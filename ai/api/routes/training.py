"""
Entrainement des modeles — demarrage, arret, status en temps reel.
Tout est integre, aucune dependance aux scripts externes.
"""

import logging
import sys
import threading
from pathlib import Path

import yaml
import torch
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from constants import ModelType

logger = logging.getLogger("sentinel.ai.training")

# Ajouter le repertoire parent au path pour les imports shared
AI_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(AI_ROOT))

_state_lock = threading.Lock()

router = APIRouter()


# ── Training State ──


class TrainingState:
    """Etat mutable de l'entrainement en cours, partage entre le thread et l'API."""

    def __init__(self) -> None:
        self.running: bool = False
        self.model_type: str | None = None
        self.current_epoch: int = 0
        self.total_epochs: int = 0
        self.loss: float = 0.0
        self.accuracy: float = 0.0
        self.val_loss: float = 0.0
        self.val_accuracy: float = 0.0
        self.phase: str = "idle"
        self.epoch_history: list[dict] = []
        self._stop_flag: bool = False
        self.early_stopped: bool = False
        self.best_epoch: int = 0
        # Progression intra-epoch
        self.current_batch: int = 0
        self.total_batches: int = 0
        self.batch_loss: float = 0.0
        self.batch_accuracy: float = 0.0
        # Metriques avancees
        self.final_metrics: dict | None = None

    def to_dict(self) -> dict:
        """Serialise l'etat pour l'endpoint /status."""
        with _state_lock:
            return {
                "running": self.running,
                "model_type": self.model_type,
                "current_epoch": self.current_epoch,
                "total_epochs": self.total_epochs,
                "loss": round(self.loss, 4),
                "accuracy": round(self.accuracy, 4),
                "val_loss": round(self.val_loss, 4),
                "val_accuracy": round(self.val_accuracy, 4),
                "phase": self.phase,
                "epoch_history": list(self.epoch_history),
                "current_batch": self.current_batch,
                "total_batches": self.total_batches,
                "batch_loss": round(self.batch_loss, 4),
                "batch_accuracy": round(self.batch_accuracy, 4),
                "early_stopped": self.early_stopped,
                "best_epoch": self.best_epoch,
                "final_metrics": self.final_metrics,
            }

    def record_epoch(self) -> None:
        """Enregistre les metriques de l'epoch courante dans l'historique."""
        with _state_lock:
            self.epoch_history.append({
                "epoch": self.current_epoch,
                "loss": round(self.loss, 4),
                "accuracy": round(self.accuracy, 4),
                "val_loss": round(self.val_loss, 4),
                "val_accuracy": round(self.val_accuracy, 4),
            })

    def update_batch(
        self,
        batch_idx: int,
        total_batches: int,
        running_loss: float,
        running_correct: int,
        running_total: int,
    ) -> None:
        """Met a jour la progression intra-epoch."""
        with _state_lock:
            self.current_batch = batch_idx + 1
            self.total_batches = total_batches
            self.batch_loss = running_loss / (batch_idx + 1)
            self.batch_accuracy = running_correct / max(running_total, 1)

    def reset(self) -> None:
        """Reinitialise l'etat pour un nouvel entrainement."""
        with _state_lock:
            self.current_epoch = 0
            self.total_epochs = 0
            self.loss = 0.0
            self.accuracy = 0.0
            self.val_loss = 0.0
            self.val_accuracy = 0.0
            self.phase = "idle"
            self.epoch_history = []
            self._stop_flag = False
            self.early_stopped = False
            self.best_epoch = 0
            self.current_batch = 0
            self.total_batches = 0
            self.batch_loss = 0.0
            self.batch_accuracy = 0.0
            self.final_metrics = None


state = TrainingState()
_training_thread: threading.Thread | None = None


# ── Request Model ──


class TrainingRequest(BaseModel):
    """Parametres de lancement d'un entrainement."""

    model_type: ModelType
    epochs: int = Field(default=10, ge=1, le=200, description="Nombre d'epochs (1-200)")
    batch_size: int = Field(default=32, ge=1, le=256, description="Taille de batch (1-256)")
    learning_rate: float = Field(default=0.001, gt=0, le=1.0, description="Learning rate (0-1)")
    validation_split: float = Field(default=0.2, gt=0, lt=1.0, description="Ratio validation (0-1)")
    early_stopping_patience: int = Field(default=3, ge=0, le=50, description="Patience early stopping (0=desactive)")
    use_class_weights: bool = True
    use_mixed_precision: bool = True
    run_lr_finder: bool = False


class TrainingResponse(BaseModel):
    """Reponse au demarrage d'un entrainement."""

    message: str
    model_type: str


class StopResponse(BaseModel):
    """Reponse a l'arret d'un entrainement."""

    message: str


# ── Routes ──


@router.get("/training/status")
async def training_status() -> dict:
    """Retourne l'etat de l'entrainement en cours."""
    return state.to_dict()


@router.post("/training/start")
async def start_training(req: TrainingRequest) -> TrainingResponse:
    """Demarre un entrainement dans un thread separe."""
    global _training_thread

    # Si un thread tourne encore apres un stop, attendre qu'il finisse
    if _training_thread is not None and _training_thread.is_alive():
        state._stop_flag = True
        state.phase = "arret en cours"
        _training_thread.join(timeout=10)
        if _training_thread.is_alive():
            raise HTTPException(409, "L'entrainement precedent ne s'est pas arrete a temps")
    _training_thread = None

    if req.model_type == ModelType.TEXT_SENTIMENT:
        thread = threading.Thread(target=_train_text, args=(req,), daemon=True)
    else:
        thread = threading.Thread(target=_train_vision, args=(req,), daemon=True)

    state.reset()
    state.running = True
    state.model_type = req.model_type.value
    state.total_epochs = req.epochs
    state.phase = "initialisation"
    _training_thread = thread
    thread.start()

    logger.info("Entrainement demarre: %s (%d epochs, batch=%d, lr=%s)",
                req.model_type.value, req.epochs, req.batch_size, req.learning_rate)

    return TrainingResponse(message="Entrainement demarre", model_type=req.model_type.value)


@router.post("/training/stop")
async def stop_training() -> StopResponse:
    """Demande l'arret de l'entrainement en cours."""
    if not state.running:
        raise HTTPException(400, "Aucun entrainement en cours")

    state._stop_flag = True
    state.phase = "arret en cours"
    logger.info("Arret de l'entrainement demande")
    return StopResponse(message="Arret demande")


# ── Helpers ──


def _load_training_config(model_type: ModelType) -> dict:
    """Charge la configuration YAML d'entrainement."""
    if model_type == ModelType.TEXT_SENTIMENT:
        config_path = AI_ROOT / "training" / "text" / "configs" / "train_config.yaml"
    else:
        config_path = AI_ROOT / "training" / "vision" / "configs" / "train_config.yaml"

    if not config_path.exists():
        raise FileNotFoundError(f"Config introuvable: {config_path}")

    with open(config_path) as f:
        config = yaml.safe_load(f)

    if not config:
        raise ValueError(f"Config vide: {config_path}")

    return config


def _create_stratified_splits(
    dataset_size: int,
    get_label_fn,
    val_ratio: float,
    test_ratio: float,
) -> tuple[list[int], list[int], list[int]]:
    """Cree des splits train/val/test stratifies par label."""
    import random
    random.seed(42)

    indices_by_label: dict[int, list[int]] = {}
    for i in range(dataset_size):
        label = get_label_fn(i)
        indices_by_label.setdefault(label, []).append(i)

    train_indices: list[int] = []
    val_indices: list[int] = []
    test_indices: list[int] = []

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


def _run_train_epoch_text(
    model,
    train_loader,
    optimizer,
    scheduler,
    loss_fn,
    device: torch.device,
    use_amp: bool,
    scaler,
) -> tuple[float, int, int]:
    """Execute une epoch d'entrainement text et retourne (total_loss, correct, total)."""
    model.train()
    total_loss, correct, total = 0.0, 0, 0
    num_batches = len(train_loader)

    for batch_idx, batch in enumerate(train_loader):
        if state._stop_flag:
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

        state.update_batch(batch_idx, num_batches, total_loss, correct, total)

    return total_loss, correct, total


def _run_val_epoch_text(
    model,
    val_loader,
    loss_fn,
    device: torch.device,
    use_amp: bool,
) -> tuple[float, int, int]:
    """Execute une epoch de validation text et retourne (val_loss, correct, total)."""
    model.eval()
    val_loss, val_correct, val_total = 0.0, 0, 0

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

            val_loss += v_loss.item()
            preds = outputs.logits.argmax(dim=-1)
            val_correct += (preds == labels).sum().item()
            val_total += labels.size(0)

    return val_loss, val_correct, val_total


def _run_train_epoch_vision(
    model,
    train_loader,
    optimizer,
    criterion,
    device: torch.device,
    use_amp: bool,
    scaler,
) -> tuple[float, int, int]:
    """Execute une epoch d'entrainement vision et retourne (total_loss, correct, total)."""
    model.train()
    total_loss, correct, total = 0.0, 0, 0
    num_batches = len(train_loader)

    for batch_idx, (images, labels) in enumerate(train_loader):
        if state._stop_flag:
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

        state.update_batch(batch_idx, num_batches, total_loss, correct, total)

    return total_loss, correct, total


def _run_val_epoch_vision(
    model,
    val_loader,
    criterion,
    device: torch.device,
    use_amp: bool,
) -> tuple[float, int, int]:
    """Execute une epoch de validation vision et retourne (val_loss, correct, total)."""
    model.eval()
    val_loss, val_correct, val_total = 0.0, 0, 0

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

            val_loss += loss.item()
            _, predicted = outputs.max(1)
            val_correct += predicted.eq(labels).sum().item()
            val_total += labels.size(0)

    return val_loss, val_correct, val_total


def _evaluate_test_set(
    model,
    test_loader,
    device: torch.device,
    class_names: list[str],
    num_classes: int,
    is_text: bool,
) -> dict:
    """Evalue le modele sur le test set et retourne les metriques finales."""
    from shared.metrics import compute_metrics, build_confusion_matrix, collect_predictions

    all_preds, all_labels = collect_predictions(model, test_loader, device, is_text=is_text)
    metrics = compute_metrics(all_preds, all_labels, class_names)
    metrics["confusion_matrix"] = build_confusion_matrix(all_preds, all_labels, num_classes)
    return metrics


def _run_lr_finder(model, train_loader, optimizer, criterion, device: torch.device, is_text: bool) -> float | None:
    """Execute le LR range test et retourne le LR suggere."""
    from shared.training_utils import find_lr

    lrs, losses = find_lr(model, train_loader, optimizer, criterion, device, is_text=is_text)
    if not lrs:
        return None

    # Le LR optimal est un peu avant le minimum de loss
    min_loss_idx = losses.index(min(losses))
    lr_offset = min(10, min_loss_idx)  # Pas depasser l'index 0
    return lrs[max(0, min_loss_idx - lr_offset)]


# ── Text Training ──


def _train_text(req: TrainingRequest) -> None:
    """Thread principal d'entrainement du modele text-sentiment."""
    try:
        import torch.nn as nn
        from torch.utils.data import DataLoader, Subset
        from transformers import AutoTokenizer, AutoModelForSequenceClassification, get_linear_schedule_with_warmup
        from ml.text_dataset import TextSentinelDataset
        from shared.training_utils import EarlyStopping, get_class_weights

        config = _load_training_config(ModelType.TEXT_SENTIMENT)

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        use_amp = req.use_mixed_precision and device.type == "cuda"
        scaler = torch.amp.GradScaler("cuda") if use_amp else None
        state.phase = f"chargement modele ({device}{'+ AMP' if use_amp else ''})"

        backbone = config["model"]["backbone"]
        max_length = config["model"]["max_length"]
        num_classes = config["model"]["num_classes"]
        class_names = [config["classes"][i] for i in range(num_classes)]
        label_smoothing = config.get("training", {}).get("label_smoothing", 0.0)
        tokenizer = AutoTokenizer.from_pretrained(backbone)

        dataset_dir = AI_ROOT / "training" / "text" / "datasets"
        dataset = TextSentinelDataset(str(dataset_dir), tokenizer, max_length)

        if len(dataset) == 0:
            state.phase = "erreur: dataset vide"
            state.running = False
            logger.error("Entrainement text annule: dataset vide")
            return

        # Split stratifie train/val/test
        state.phase = "preparation donnees"
        test_ratio = config.get("data", {}).get("test_split", 0.1)
        train_indices, val_indices, test_indices = _create_stratified_splits(
            len(dataset),
            lambda i: dataset[i]["labels"],
            req.validation_split,
            test_ratio,
        )

        train_set = Subset(dataset, train_indices)
        val_set = Subset(dataset, val_indices)
        test_set = Subset(dataset, test_indices)

        train_loader = DataLoader(train_set, batch_size=req.batch_size, shuffle=True)
        val_loader = DataLoader(val_set, batch_size=req.batch_size, shuffle=False)
        test_loader = DataLoader(test_set, batch_size=req.batch_size, shuffle=False)

        logger.info("Text splits: train=%d, val=%d, test=%d", len(train_indices), len(val_indices), len(test_indices))

        # Class weights
        loss_fn = None
        if req.use_class_weights:
            state.phase = "calcul class weights"
            train_labels = [dataset[idx]["labels"] for idx in train_indices]
            class_weights = get_class_weights(train_labels, num_classes, device)
            loss_fn = nn.CrossEntropyLoss(weight=class_weights, label_smoothing=label_smoothing)
        elif label_smoothing > 0:
            loss_fn = nn.CrossEntropyLoss(label_smoothing=label_smoothing)

        model = AutoModelForSequenceClassification.from_pretrained(
            backbone, num_labels=num_classes
        ).to(device)

        optimizer = torch.optim.AdamW(model.parameters(), lr=req.learning_rate, weight_decay=0.01)

        # LR Finder
        if req.run_lr_finder:
            state.phase = "LR range test"
            suggested_lr = _run_lr_finder(model, train_loader, optimizer, loss_fn, device, is_text=True)
            if suggested_lr is not None:
                for pg in optimizer.param_groups:
                    pg["lr"] = suggested_lr
                state.phase = f"LR suggere: {suggested_lr:.2e}"
                logger.info("LR range test: LR suggere = %.2e", suggested_lr)

        total_steps = len(train_loader) * req.epochs
        scheduler = get_linear_schedule_with_warmup(optimizer, int(total_steps * 0.1), total_steps)

        early_stopping = EarlyStopping(patience=req.early_stopping_patience, mode="min")

        for epoch in range(req.epochs):
            if state._stop_flag:
                state.phase = "arrete"
                break

            state.current_epoch = epoch + 1
            state.phase = f"entrainement epoch {epoch + 1}/{req.epochs}"

            total_loss, correct, total = _run_train_epoch_text(
                model, train_loader, optimizer, scheduler, loss_fn, device, use_amp, scaler,
            )
            state.loss = total_loss / max(len(train_loader), 1)
            state.accuracy = correct / max(total, 1)

            # Validation
            state.phase = f"validation epoch {epoch + 1}/{req.epochs}"
            val_loss, val_correct, val_total = _run_val_epoch_text(
                model, val_loader, loss_fn, device, use_amp,
            )
            state.val_loss = val_loss / max(len(val_loader), 1)
            state.val_accuracy = val_correct / max(val_total, 1)
            state.record_epoch()

            logger.info(
                "Text epoch %d/%d — loss=%.4f acc=%.4f val_loss=%.4f val_acc=%.4f",
                epoch + 1, req.epochs, state.loss, state.accuracy, state.val_loss, state.val_accuracy,
            )

            # Sauvegarde du meilleur modele
            if not early_stopping.step(state.val_loss, epoch + 1):
                if early_stopping.counter == 0:
                    state.best_epoch = epoch + 1
                    checkpoint_dir = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
                    checkpoint_dir.mkdir(parents=True, exist_ok=True)
                    model.save_pretrained(str(checkpoint_dir))
                    tokenizer.save_pretrained(str(checkpoint_dir))
            else:
                state.early_stopped = True
                state.phase = f"early stop epoch {epoch + 1} (patience {req.early_stopping_patience}, best epoch {early_stopping.best_epoch})"
                logger.info("Early stopping a epoch %d (best: %d)", epoch + 1, early_stopping.best_epoch)
                break

        # Metriques finales sur test set
        if len(test_set) > 0:
            state.phase = "evaluation sur test set"
            state.final_metrics = _evaluate_test_set(model, test_loader, device, class_names, num_classes, is_text=True)
            logger.info("Metriques test: accuracy=%.4f macro_f1=%.4f",
                        state.final_metrics["accuracy"], state.final_metrics["macro_f1"])

        if not state._stop_flag and not state.early_stopped:
            state.phase = "termine"

    except Exception as e:
        logger.exception("Erreur entrainement text-sentiment")
        state.phase = f"erreur: {e}"
    finally:
        state.running = False
        if torch.cuda.is_available():
            torch.cuda.empty_cache()


# ── Vision Training ──


def _train_vision(req: TrainingRequest) -> None:
    """Thread principal d'entrainement du modele image-classification."""
    try:
        import torch.nn as nn
        from torch.utils.data import DataLoader, Subset
        from torchvision import transforms, models
        from ml.vision_dataset import VisionSentinelDataset
        from shared.training_utils import EarlyStopping, get_class_weights

        config = _load_training_config(ModelType.IMAGE_CLASSIFICATION)

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        use_amp = req.use_mixed_precision and device.type == "cuda"
        scaler = torch.amp.GradScaler("cuda") if use_amp else None
        state.phase = f"chargement modele ({device}{'+ AMP' if use_amp else ''})"

        size = config["model"]["input_size"]
        num_classes = config["model"]["num_classes"]
        class_names = [config["classes"][i] for i in range(num_classes)]
        patience = config.get("training", {}).get("early_stopping_patience", 10)

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

        dataset_dir = AI_ROOT / "training" / "vision" / "datasets"
        train_dataset = VisionSentinelDataset(str(dataset_dir), transform=train_transform)
        val_dataset = VisionSentinelDataset(str(dataset_dir), transform=val_transform)

        if len(train_dataset) == 0:
            state.phase = "erreur: dataset vide"
            state.running = False
            logger.error("Entrainement vision annule: dataset vide")
            return

        # Split stratifie
        state.phase = "preparation donnees"
        test_ratio = config.get("data", {}).get("test_split", 0.1)
        train_indices, val_indices, test_indices = _create_stratified_splits(
            len(train_dataset),
            lambda i: train_dataset.samples[i][1],
            req.validation_split,
            test_ratio,
        )

        train_set = Subset(train_dataset, train_indices)
        val_set = Subset(val_dataset, val_indices)
        test_set = Subset(val_dataset, test_indices)

        train_loader = DataLoader(train_set, batch_size=req.batch_size, shuffle=True)
        val_loader = DataLoader(val_set, batch_size=req.batch_size, shuffle=False)
        test_loader = DataLoader(test_set, batch_size=req.batch_size, shuffle=False)

        logger.info("Vision splits: train=%d, val=%d, test=%d", len(train_indices), len(val_indices), len(test_indices))

        model = models.efficientnet_v2_s(weights=models.EfficientNet_V2_S_Weights.DEFAULT)
        model.classifier[1] = nn.Linear(model.classifier[1].in_features, num_classes)
        model = model.to(device)

        # Class weights
        if req.use_class_weights:
            state.phase = "calcul class weights"
            train_labels = [train_dataset.samples[idx][1] for idx in train_indices]
            class_weights = get_class_weights(train_labels, num_classes, device)
            criterion = nn.CrossEntropyLoss(weight=class_weights)
        else:
            criterion = nn.CrossEntropyLoss()

        optimizer = torch.optim.AdamW(model.parameters(), lr=req.learning_rate, weight_decay=0.0001)

        # LR Finder
        if req.run_lr_finder:
            state.phase = "LR range test"
            suggested_lr = _run_lr_finder(model, train_loader, optimizer, criterion, device, is_text=False)
            if suggested_lr is not None:
                for pg in optimizer.param_groups:
                    pg["lr"] = suggested_lr
                state.phase = f"LR suggere: {suggested_lr:.2e}"
                logger.info("LR range test: LR suggere = %.2e", suggested_lr)

        scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=req.epochs)

        early_stopping = EarlyStopping(patience=patience, mode="max")

        for epoch in range(req.epochs):
            if state._stop_flag:
                state.phase = "arrete"
                break

            state.current_epoch = epoch + 1
            state.phase = f"entrainement epoch {epoch + 1}/{req.epochs}"

            total_loss, correct, total = _run_train_epoch_vision(
                model, train_loader, optimizer, criterion, device, use_amp, scaler,
            )
            scheduler.step()
            state.loss = total_loss / max(len(train_loader), 1)
            state.accuracy = correct / max(total, 1)

            # Validation
            state.phase = f"validation epoch {epoch + 1}/{req.epochs}"
            val_loss, val_correct, val_total = _run_val_epoch_vision(
                model, val_loader, criterion, device, use_amp,
            )
            state.val_loss = val_loss / max(len(val_loader), 1)
            state.val_accuracy = val_correct / max(val_total, 1)
            state.record_epoch()

            logger.info(
                "Vision epoch %d/%d — loss=%.4f acc=%.4f val_loss=%.4f val_acc=%.4f",
                epoch + 1, req.epochs, state.loss, state.accuracy, state.val_loss, state.val_accuracy,
            )

            # Sauvegarde + early stopping
            if not early_stopping.step(state.val_accuracy, epoch + 1):
                if early_stopping.counter == 0:
                    state.best_epoch = epoch + 1
                    checkpoint_dir = AI_ROOT / "training" / "vision" / "checkpoints"
                    checkpoint_dir.mkdir(parents=True, exist_ok=True)
                    torch.save({
                        "epoch": epoch,
                        "model_state_dict": model.state_dict(),
                        "val_acc": state.val_accuracy,
                        "config": config,
                    }, str(checkpoint_dir / "best_model.pt"))
            else:
                state.early_stopped = True
                state.phase = f"early stop epoch {epoch + 1} (patience {patience}, best epoch {early_stopping.best_epoch})"
                logger.info("Early stopping a epoch %d (best: %d)", epoch + 1, early_stopping.best_epoch)
                break

        # Metriques finales sur test set
        if len(test_set) > 0:
            state.phase = "evaluation sur test set"
            state.final_metrics = _evaluate_test_set(model, test_loader, device, class_names, num_classes, is_text=False)
            logger.info("Metriques test: accuracy=%.4f macro_f1=%.4f",
                        state.final_metrics["accuracy"], state.final_metrics["macro_f1"])

        if not state._stop_flag and not state.early_stopped:
            state.phase = "termine"

    except Exception as e:
        logger.exception("Erreur entrainement image-classification")
        state.phase = f"erreur: {e}"
    finally:
        state.running = False
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
