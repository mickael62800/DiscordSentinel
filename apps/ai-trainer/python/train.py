"""
CLI d'entrainement — autonome, sans FastAPI.

Emet sa progression en JSON Lines sur stdout pour que le parent Rust puisse
la relayer au frontend Tauri sous forme d'events.

Evenements emis:
    start    — debut d'entrainement
    phase    — changement de phase (chargement, validation, etc.)
    batch    — progression intra-epoch (throttled a ~150ms)
    epoch    — fin d'une epoch
    metrics  — metriques finales sur test set (accuracy, per-class, confusion)
    done     — entrainement termine (succes, stop, early stop)
    error    — erreur fatale
"""

import argparse
import io
import logging
import sys
from pathlib import Path

import yaml
import torch

# Fix Windows console encoding pour eviter les erreurs d'emoji
if sys.stdout.encoding != "utf-8":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

from constants import ModelType  # noqa: E402
from emitter import Emitter  # noqa: E402

logging.basicConfig(level=logging.WARNING, format="%(levelname)s: %(message)s", stream=sys.stderr)
logger = logging.getLogger("sentinel.trainer")


def _load_config(model_type: ModelType, data_root: Path) -> dict:
    if model_type == ModelType.TEXT_SENTIMENT:
        config_path = SCRIPT_DIR / "configs" / "text" / "train_config.yaml"
    else:
        config_path = SCRIPT_DIR / "configs" / "vision" / "train_config.yaml"
    if not config_path.exists():
        raise FileNotFoundError(f"Config introuvable: {config_path}")
    with open(config_path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def _create_stratified_splits(dataset_size, get_label_fn, val_ratio, test_ratio):
    import random
    random.seed(42)
    indices_by_label: dict[int, list[int]] = {}
    for i in range(dataset_size):
        label = get_label_fn(i)
        indices_by_label.setdefault(label, []).append(i)

    train_indices, val_indices, test_indices = [], [], []
    for _label, indices in indices_by_label.items():
        random.shuffle(indices)
        n = len(indices)
        n_test = max(1, int(n * test_ratio))
        n_val = max(1, int(n * val_ratio))
        test_indices.extend(indices[:n_test])
        val_indices.extend(indices[n_test:n_test + n_val])
        train_indices.extend(indices[n_test + n_val:])
    random.shuffle(train_indices)
    random.shuffle(val_indices)
    return train_indices, val_indices, test_indices


# ──────────────────────────────────────────────────────────────
# TEXT TRAINING
# ──────────────────────────────────────────────────────────────


def train_text(args, emitter: Emitter, data_root: Path) -> None:
    import torch.nn as nn
    from torch.utils.data import DataLoader, Subset
    from transformers import AutoTokenizer, AutoModelForSequenceClassification, get_linear_schedule_with_warmup
    from ml.text_dataset import TextSentinelDataset
    from shared.training_utils import EarlyStopping, get_class_weights
    from shared.metrics import compute_metrics, build_confusion_matrix, collect_predictions

    config = _load_config(ModelType.TEXT_SENTIMENT, data_root)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    use_amp = args.use_mixed_precision and device.type == "cuda"
    scaler = torch.amp.GradScaler("cuda") if use_amp else None
    emitter.emit("phase", phase=f"chargement modele ({device}{' + AMP' if use_amp else ''})")

    backbone = config["model"]["backbone"]
    max_length = args.max_length if args.max_length else config["model"]["max_length"]
    num_classes = config["model"]["num_classes"]
    class_names = [config["classes"][i] for i in range(num_classes)]
    label_smoothing = (
        args.label_smoothing if args.label_smoothing is not None
        else config.get("training", {}).get("label_smoothing", 0.0)
    )

    tokenizer = AutoTokenizer.from_pretrained(backbone)

    dataset_dir = data_root / "text" / "datasets"
    dataset = TextSentinelDataset(str(dataset_dir), tokenizer, max_length)

    if len(dataset) == 0:
        emitter.emit("error", message="dataset vide")
        return

    emitter.emit("phase", phase="preparation donnees")
    test_ratio = config.get("data", {}).get("test_split", 0.1)
    all_labels = [lbl for _, lbl in dataset.samples]
    train_indices, val_indices, test_indices = _create_stratified_splits(
        len(dataset), lambda i: all_labels[i], args.validation_split, test_ratio,
    )

    if args.neutral_cap and args.neutral_cap > 0:
        import random as _rnd
        _rnd.seed(42)
        neutral_idx = [i for i in train_indices if all_labels[i] == 0]
        other_idx = [i for i in train_indices if all_labels[i] != 0]
        if len(neutral_idx) > args.neutral_cap:
            _rnd.shuffle(neutral_idx)
            neutral_idx = neutral_idx[:args.neutral_cap]
        train_indices = other_idx + neutral_idx
        _rnd.shuffle(train_indices)

    train_set = Subset(dataset, train_indices)
    val_set = Subset(dataset, val_indices)
    test_set = Subset(dataset, test_indices)

    loader_kwargs = {
        "num_workers": min(4, len(train_indices) // args.batch_size),
        "pin_memory": device.type == "cuda",
    }
    if loader_kwargs["num_workers"] > 0:
        loader_kwargs["persistent_workers"] = True
    train_loader = DataLoader(train_set, batch_size=args.batch_size, shuffle=True, **loader_kwargs)
    val_loader = DataLoader(val_set, batch_size=args.batch_size, shuffle=False, **loader_kwargs)
    test_loader = DataLoader(test_set, batch_size=args.batch_size, shuffle=False, **loader_kwargs)

    loss_fn = None
    if args.use_class_weights:
        emitter.emit("phase", phase="calcul class weights")
        train_labels = [dataset[idx]["labels"] for idx in train_indices]
        class_weights = get_class_weights(train_labels, num_classes, device)
        loss_fn = nn.CrossEntropyLoss(weight=class_weights, label_smoothing=label_smoothing)
    elif label_smoothing > 0:
        loss_fn = nn.CrossEntropyLoss(label_smoothing=label_smoothing)

    model = AutoModelForSequenceClassification.from_pretrained(backbone, num_labels=num_classes).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.learning_rate, weight_decay=args.weight_decay)

    total_steps = len(train_loader) * args.epochs
    scheduler = get_linear_schedule_with_warmup(optimizer, int(total_steps * args.warmup_ratio), total_steps)
    early_stopping = EarlyStopping(patience=args.early_stopping_patience, mode="min")

    emitter.emit("start", total_epochs=args.epochs, model_type="text-sentiment",
                 train_samples=len(train_indices), val_samples=len(val_indices), test_samples=len(test_indices))

    best_epoch = 0
    early_stopped = False

    for epoch in range(args.epochs):
        if emitter.should_stop():
            emitter.emit("phase", phase="arrete")
            break

        emitter.emit("phase", phase=f"entrainement epoch {epoch + 1}/{args.epochs}")
        model.train()
        total_loss, correct, total = 0.0, 0, 0
        num_batches = len(train_loader)

        for batch_idx, batch in enumerate(train_loader):
            if emitter.should_stop():
                break
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            labels = batch["labels"].to(device)
            optimizer.zero_grad()

            if use_amp:
                with torch.amp.autocast("cuda"):
                    outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
                    loss = loss_fn(outputs.logits, labels) if loss_fn is not None else outputs.loss
                scaler.scale(loss).backward()
                scaler.unscale_(optimizer)
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
                scaler.step(optimizer)
                scaler.update()
            else:
                outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
                loss = loss_fn(outputs.logits, labels) if loss_fn is not None else outputs.loss
                loss.backward()
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
                optimizer.step()

            scheduler.step()
            total_loss += loss.item()
            preds = outputs.logits.argmax(dim=-1)
            correct += (preds == labels).sum().item()
            total += labels.size(0)

            emitter.emit_batch_throttled(
                epoch=epoch + 1, current=batch_idx + 1, total=num_batches,
                loss=round(total_loss / (batch_idx + 1), 4),
                accuracy=round(correct / max(total, 1), 4),
            )

        train_loss = total_loss / max(len(train_loader), 1)
        train_acc = correct / max(total, 1)

        emitter.emit("phase", phase=f"validation epoch {epoch + 1}/{args.epochs}")
        model.eval()
        val_loss_sum, val_correct, val_total = 0.0, 0, 0
        with torch.no_grad():
            for batch in val_loader:
                input_ids = batch["input_ids"].to(device)
                attention_mask = batch["attention_mask"].to(device)
                labels = batch["labels"].to(device)
                if use_amp:
                    with torch.amp.autocast("cuda"):
                        outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
                        v_loss = loss_fn(outputs.logits, labels) if loss_fn is not None else outputs.loss
                else:
                    outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
                    v_loss = loss_fn(outputs.logits, labels) if loss_fn is not None else outputs.loss
                val_loss_sum += v_loss.item()
                preds = outputs.logits.argmax(dim=-1)
                val_correct += (preds == labels).sum().item()
                val_total += labels.size(0)

        val_loss = val_loss_sum / max(len(val_loader), 1)
        val_acc = val_correct / max(val_total, 1)

        improved = not early_stopping.step(val_loss, epoch + 1)
        if improved and early_stopping.counter == 0:
            best_epoch = epoch + 1
            checkpoint_dir = data_root / "text" / "checkpoints" / "best_model"
            checkpoint_dir.mkdir(parents=True, exist_ok=True)
            for old in checkpoint_dir.glob("model.safetensors*"):
                try:
                    old.unlink()
                except OSError:
                    pass
            model.save_pretrained(str(checkpoint_dir), safe_serialization=False)
            tokenizer.save_pretrained(str(checkpoint_dir))

        emitter.emit("epoch", epoch=epoch + 1, loss=round(train_loss, 4), accuracy=round(train_acc, 4),
                     val_loss=round(val_loss, 4), val_accuracy=round(val_acc, 4), best_epoch=best_epoch)

        if early_stopping.should_stop:
            early_stopped = True
            emitter.emit("phase", phase=f"early stop epoch {epoch + 1} (best {best_epoch})")
            break

    if len(test_set) > 0 and not emitter.should_stop():
        emitter.emit("phase", phase="evaluation sur test set")
        all_preds, all_true = collect_predictions(model, test_loader, device, is_text=True)
        metrics = compute_metrics(all_preds, all_true, class_names)
        metrics["confusion_matrix"] = build_confusion_matrix(all_preds, all_true, num_classes)
        emitter.emit("metrics", **metrics)

    emitter.emit("done", phase="termine", early_stopped=early_stopped, best_epoch=best_epoch)

    if torch.cuda.is_available():
        torch.cuda.empty_cache()


# ──────────────────────────────────────────────────────────────
# VISION TRAINING
# ──────────────────────────────────────────────────────────────


def train_vision(args, emitter: Emitter, data_root: Path) -> None:
    import torch.nn as nn
    from torch.utils.data import DataLoader, Subset
    from torchvision import transforms, models
    from ml.vision_dataset import VisionSentinelDataset
    from shared.training_utils import EarlyStopping, get_class_weights
    from shared.metrics import compute_metrics, build_confusion_matrix, collect_predictions

    config = _load_config(ModelType.IMAGE_CLASSIFICATION, data_root)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    use_amp = args.use_mixed_precision and device.type == "cuda"
    scaler = torch.amp.GradScaler("cuda") if use_amp else None
    emitter.emit("phase", phase=f"chargement modele ({device}{' + AMP' if use_amp else ''})")

    size = config["model"]["input_size"]
    num_classes = config["model"]["num_classes"]
    class_names = [config["classes"][i] for i in range(num_classes)]
    patience = args.early_stopping_patience or config.get("training", {}).get("early_stopping_patience", 10)

    train_transform = transforms.Compose([
        transforms.Resize((size + 32, size + 32)),
        transforms.RandomCrop(size),
        transforms.RandomHorizontalFlip(),
        transforms.ColorJitter(brightness=0.2, contrast=0.2, saturation=0.2, hue=0.1),
        transforms.RandomRotation(15),
        transforms.ToTensor(),
        transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
        transforms.RandomErasing(p=0.1),
    ])
    val_transform = transforms.Compose([
        transforms.Resize((size + 32, size + 32)),
        transforms.CenterCrop(size),
        transforms.ToTensor(),
        transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
    ])

    dataset_dir = data_root / "vision" / "datasets"
    train_dataset = VisionSentinelDataset(str(dataset_dir), transform=train_transform)
    val_dataset = VisionSentinelDataset(str(dataset_dir), transform=val_transform)

    if len(train_dataset) == 0:
        emitter.emit("error", message="dataset vide")
        return

    emitter.emit("phase", phase="preparation donnees")
    test_ratio = config.get("data", {}).get("test_split", 0.1)
    train_indices, val_indices, test_indices = _create_stratified_splits(
        len(train_dataset), lambda i: train_dataset.samples[i][1], args.validation_split, test_ratio,
    )

    train_set = Subset(train_dataset, train_indices)
    val_set = Subset(val_dataset, val_indices)
    test_set = Subset(val_dataset, test_indices)

    loader_kwargs = {
        "num_workers": min(4, len(train_indices) // args.batch_size),
        "pin_memory": device.type == "cuda",
    }
    if loader_kwargs["num_workers"] > 0:
        loader_kwargs["persistent_workers"] = True
    train_loader = DataLoader(train_set, batch_size=args.batch_size, shuffle=True, **loader_kwargs)
    val_loader = DataLoader(val_set, batch_size=args.batch_size, shuffle=False, **loader_kwargs)
    test_loader = DataLoader(test_set, batch_size=args.batch_size, shuffle=False, **loader_kwargs)

    model = models.efficientnet_v2_s(weights=models.EfficientNet_V2_S_Weights.DEFAULT)
    model.classifier[1] = nn.Linear(model.classifier[1].in_features, num_classes)
    model = model.to(device)

    if args.use_class_weights:
        emitter.emit("phase", phase="calcul class weights")
        train_labels = [train_dataset.samples[idx][1] for idx in train_indices]
        class_weights = get_class_weights(train_labels, num_classes, device)
        criterion = nn.CrossEntropyLoss(weight=class_weights)
    else:
        criterion = nn.CrossEntropyLoss()

    optimizer = torch.optim.AdamW(model.parameters(), lr=args.learning_rate, weight_decay=args.weight_decay)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=args.epochs)
    early_stopping = EarlyStopping(patience=patience, mode="max")

    emitter.emit("start", total_epochs=args.epochs, model_type="image-classification",
                 train_samples=len(train_indices), val_samples=len(val_indices), test_samples=len(test_indices))

    best_epoch = 0
    early_stopped = False

    for epoch in range(args.epochs):
        if emitter.should_stop():
            emitter.emit("phase", phase="arrete")
            break

        emitter.emit("phase", phase=f"entrainement epoch {epoch + 1}/{args.epochs}")
        model.train()
        total_loss, correct, total = 0.0, 0, 0
        num_batches = len(train_loader)

        for batch_idx, (images, labels) in enumerate(train_loader):
            if emitter.should_stop():
                break
            images, labels = images.to(device), labels.to(device)
            optimizer.zero_grad()

            if use_amp:
                with torch.amp.autocast("cuda"):
                    outputs = model(images)
                    loss = criterion(outputs, labels)
                scaler.scale(loss).backward()
                scaler.unscale_(optimizer)
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
                scaler.step(optimizer)
                scaler.update()
            else:
                outputs = model(images)
                loss = criterion(outputs, labels)
                loss.backward()
                optimizer.step()

            total_loss += loss.item()
            _, predicted = outputs.max(1)
            correct += predicted.eq(labels).sum().item()
            total += labels.size(0)

            emitter.emit_batch_throttled(
                epoch=epoch + 1, current=batch_idx + 1, total=num_batches,
                loss=round(total_loss / (batch_idx + 1), 4),
                accuracy=round(correct / max(total, 1), 4),
            )

        scheduler.step()
        train_loss = total_loss / max(len(train_loader), 1)
        train_acc = correct / max(total, 1)

        emitter.emit("phase", phase=f"validation epoch {epoch + 1}/{args.epochs}")
        model.eval()
        val_loss_sum, val_correct, val_total = 0.0, 0, 0
        with torch.no_grad():
            for images, labels in val_loader:
                images, labels = images.to(device), labels.to(device)
                if use_amp:
                    with torch.amp.autocast("cuda"):
                        outputs = model(images)
                        loss = criterion(outputs, labels)
                else:
                    outputs = model(images)
                    loss = criterion(outputs, labels)
                val_loss_sum += loss.item()
                _, predicted = outputs.max(1)
                val_correct += predicted.eq(labels).sum().item()
                val_total += labels.size(0)

        val_loss = val_loss_sum / max(len(val_loader), 1)
        val_acc = val_correct / max(val_total, 1)

        improved = not early_stopping.step(val_acc, epoch + 1)
        if improved and early_stopping.counter == 0:
            best_epoch = epoch + 1
            checkpoint_dir = data_root / "vision" / "checkpoints"
            checkpoint_dir.mkdir(parents=True, exist_ok=True)
            torch.save({
                "epoch": epoch,
                "model_state_dict": model.state_dict(),
                "val_acc": val_acc,
                "config": config,
            }, str(checkpoint_dir / "best_model.pt"))

        emitter.emit("epoch", epoch=epoch + 1, loss=round(train_loss, 4), accuracy=round(train_acc, 4),
                     val_loss=round(val_loss, 4), val_accuracy=round(val_acc, 4), best_epoch=best_epoch)

        if early_stopping.should_stop:
            early_stopped = True
            emitter.emit("phase", phase=f"early stop epoch {epoch + 1} (best {best_epoch})")
            break

    if len(test_set) > 0 and not emitter.should_stop():
        emitter.emit("phase", phase="evaluation sur test set")
        all_preds, all_true = collect_predictions(model, test_loader, device, is_text=False)
        metrics = compute_metrics(all_preds, all_true, class_names)
        metrics["confusion_matrix"] = build_confusion_matrix(all_preds, all_true, num_classes)
        emitter.emit("metrics", **metrics)

    emitter.emit("done", phase="termine", early_stopped=early_stopped, best_epoch=best_epoch)

    if torch.cuda.is_available():
        torch.cuda.empty_cache()


# ──────────────────────────────────────────────────────────────
# MAIN
# ──────────────────────────────────────────────────────────────


def main() -> int:
    parser = argparse.ArgumentParser(description="Sentinel AI Trainer CLI")
    parser.add_argument("--model-type", required=True, choices=["text-sentiment", "image-classification"])
    parser.add_argument("--data-root", required=True, help="Racine du repertoire data (text/ vision/)")
    parser.add_argument("--epochs", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=32)
    parser.add_argument("--learning-rate", type=float, default=0.001)
    parser.add_argument("--validation-split", type=float, default=0.2)
    parser.add_argument("--early-stopping-patience", type=int, default=3)
    parser.add_argument("--use-class-weights", type=lambda s: s.lower() == "true", default=True)
    parser.add_argument("--use-mixed-precision", type=lambda s: s.lower() == "true", default=True)
    parser.add_argument("--label-smoothing", type=float, default=None)
    parser.add_argument("--weight-decay", type=float, default=0.01)
    parser.add_argument("--warmup-ratio", type=float, default=0.1)
    parser.add_argument("--max-length", type=int, default=None)
    parser.add_argument("--neutral-cap", type=int, default=0)
    parser.add_argument("--stop-flag", default=None, help="Chemin d'un fichier-flag. Sa presence demande l'arret.")
    args = parser.parse_args()

    emitter = Emitter(args.stop_flag)
    data_root = Path(args.data_root).resolve()
    data_root.mkdir(parents=True, exist_ok=True)

    try:
        if args.model_type == "text-sentiment":
            train_text(args, emitter, data_root)
        else:
            train_vision(args, emitter, data_root)
    except Exception as e:
        logger.exception("Erreur entrainement")
        emitter.emit("error", message=str(e))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
