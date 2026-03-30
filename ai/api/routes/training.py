"""
Entrainement des modeles — demarrage, arret, status en temps reel.
Tout est integre, aucune dependance aux scripts externes.
"""

import sys
import os
import threading
from pathlib import Path

import yaml
import torch
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

# Ajouter le repertoire parent au path pour les imports shared
AI_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(AI_ROOT))

_state_lock = threading.Lock()

router = APIRouter()


class TrainingState:
    def __init__(self):
        self.running = False
        self.model_type: str | None = None
        self.current_epoch = 0
        self.total_epochs = 0
        self.loss = 0.0
        self.accuracy = 0.0
        self.val_loss = 0.0
        self.val_accuracy = 0.0
        self.phase = "idle"
        self.epoch_history: list[dict] = []
        self._stop_flag = False
        self.early_stopped = False
        self.best_epoch = 0
        # Progression intra-epoch
        self.current_batch = 0
        self.total_batches = 0
        self.batch_loss = 0.0
        self.batch_accuracy = 0.0
        # Metriques avancees (Fix 10)
        self.final_metrics: dict | None = None

    def to_dict(self):
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

    def record_epoch(self):
        self.epoch_history.append({
            "epoch": self.current_epoch,
            "loss": round(self.loss, 4),
            "accuracy": round(self.accuracy, 4),
            "val_loss": round(self.val_loss, 4),
            "val_accuracy": round(self.val_accuracy, 4),
        })

    def update_batch(self, batch_idx: int, total_batches: int, running_loss: float, running_correct: int, running_total: int):
        self.current_batch = batch_idx + 1
        self.total_batches = total_batches
        self.batch_loss = running_loss / (batch_idx + 1)
        self.batch_accuracy = running_correct / max(running_total, 1)

    def reset(self):
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


class TrainingRequest(BaseModel):
    model_type: str
    epochs: int = 10
    batch_size: int = 32
    learning_rate: float = 0.001
    validation_split: float = 0.2
    early_stopping_patience: int = 3
    use_class_weights: bool = True
    use_mixed_precision: bool = True   # Fix 7: AMP
    run_lr_finder: bool = False        # Fix 8: LR range test


@router.get("/training/status")
async def training_status():
    return state.to_dict()


@router.post("/training/start")
async def start_training(req: TrainingRequest):
    global _training_thread

    # Si un thread tourne encore apres un stop, attendre qu'il finisse
    if _training_thread is not None and _training_thread.is_alive():
        state._stop_flag = True
        state.phase = "arret en cours"
        _training_thread.join(timeout=10)
        if _training_thread.is_alive():
            raise HTTPException(409, "L'entrainement precedent ne s'est pas arrete a temps")
    _training_thread = None

    if req.model_type == "text-sentiment":
        thread = threading.Thread(target=_train_text, args=(req,), daemon=True)
    elif req.model_type == "image-classification":
        thread = threading.Thread(target=_train_vision, args=(req,), daemon=True)
    else:
        raise HTTPException(400, f"Type de modele inconnu: {req.model_type}")

    state.reset()
    state.running = True
    state.model_type = req.model_type
    state.total_epochs = req.epochs
    state.phase = "initialisation"
    _training_thread = thread
    thread.start()

    return {"message": "Entrainement demarre", "model_type": req.model_type}


@router.post("/training/stop")
async def stop_training():
    if not state.running:
        raise HTTPException(400, "Aucun entrainement en cours")

    state._stop_flag = True
    state.phase = "arret en cours"
    return {"message": "Arret demande"}


# ── Text Training ──

def _train_text(req: TrainingRequest):
    try:
        import random
        import torch.nn as nn
        from torch.utils.data import DataLoader, Subset
        from transformers import AutoTokenizer, AutoModelForSequenceClassification, get_linear_schedule_with_warmup
        from ml.text_dataset import TextSentinelDataset
        from shared.training_utils import EarlyStopping, get_class_weights, find_lr
        from shared.metrics import compute_metrics, build_confusion_matrix, collect_predictions

        config_path = AI_ROOT / "training" / "text" / "configs" / "train_config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f)

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        use_amp = req.use_mixed_precision and device.type == "cuda"  # Fix 7: AMP only on CUDA
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
            return

        # ── Split stratifie train/val/test (Fix 3) ──
        state.phase = "preparation donnees"
        random.seed(42)
        indices_by_label: dict[int, list[int]] = {}
        for i in range(len(dataset)):
            label = dataset[i]["labels"]
            indices_by_label.setdefault(label, []).append(i)

        test_ratio = config.get("data", {}).get("test_split", 0.1)
        val_ratio = req.validation_split

        train_indices, val_indices, test_indices = [], [], []
        for label, indices in indices_by_label.items():
            random.shuffle(indices)
            n = len(indices)
            n_test = max(1, int(n * test_ratio))
            n_val = max(1, int(n * val_ratio))
            test_indices.extend(indices[:n_test])
            val_indices.extend(indices[n_test:n_test + n_val])
            train_indices.extend(indices[n_test + n_val:])

        random.shuffle(train_indices)
        random.shuffle(val_indices)

        train_set = Subset(dataset, train_indices)
        val_set = Subset(dataset, val_indices)
        test_set = Subset(dataset, test_indices)

        train_loader = DataLoader(train_set, batch_size=req.batch_size, shuffle=True)
        val_loader = DataLoader(val_set, batch_size=req.batch_size, shuffle=False)
        test_loader = DataLoader(test_set, batch_size=req.batch_size, shuffle=False)

        # ── Class weights ──
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

        # ── LR Finder (Fix 8) ──
        if req.run_lr_finder:
            state.phase = "LR range test"
            lrs, losses = find_lr(model, train_loader, optimizer, loss_fn, device, is_text=True)
            if lrs:
                # Trouver le LR ou la loss descent le plus vite
                min_loss_idx = losses.index(min(losses))
                suggested_lr = lrs[max(0, min_loss_idx - 10)]  # Un peu avant le minimum
                for pg in optimizer.param_groups:
                    pg["lr"] = suggested_lr
                state.phase = f"LR suggere: {suggested_lr:.2e}"

        total_steps = len(train_loader) * req.epochs
        scheduler = get_linear_schedule_with_warmup(optimizer, int(total_steps * 0.1), total_steps)

        # ── Early stopping (shared) ──
        early_stopping = EarlyStopping(patience=req.early_stopping_patience, mode="min")
        num_train_batches = len(train_loader)

        for epoch in range(req.epochs):
            if state._stop_flag:
                state.phase = "arrete"
                break

            state.current_epoch = epoch + 1
            state.phase = f"entrainement epoch {epoch + 1}/{req.epochs}"

            model.train()
            total_loss, correct, total = 0.0, 0, 0
            for batch_idx, batch in enumerate(train_loader):
                if state._stop_flag:
                    break
                input_ids = batch["input_ids"].to(device)
                attention_mask = batch["attention_mask"].to(device)
                labels = batch["labels"].to(device)

                optimizer.zero_grad()

                # Fix 7: Mixed precision
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

                state.update_batch(batch_idx, num_train_batches, total_loss, correct, total)

            state.loss = total_loss / max(num_train_batches, 1)
            state.accuracy = correct / max(total, 1)

            # ── Validation ──
            state.phase = f"validation epoch {epoch + 1}/{req.epochs}"
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

            state.val_loss = val_loss / max(len(val_loader), 1)
            state.val_accuracy = val_correct / max(val_total, 1)
            state.record_epoch()

            # ── Sauvegarde du meilleur modele ──
            if not early_stopping.step(state.val_loss, epoch + 1):
                if early_stopping.counter == 0:  # Just improved
                    state.best_epoch = epoch + 1
                    checkpoint_dir = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
                    checkpoint_dir.mkdir(parents=True, exist_ok=True)
                    model.save_pretrained(str(checkpoint_dir))
                    tokenizer.save_pretrained(str(checkpoint_dir))
            else:
                state.early_stopped = True
                state.phase = f"early stop epoch {epoch + 1} (patience {req.early_stopping_patience}, best epoch {early_stopping.best_epoch})"
                break

        # ── Metriques finales sur test set (Fix 10) ──
        if len(test_set) > 0:
            state.phase = "evaluation sur test set"
            all_preds, all_labels = collect_predictions(model, test_loader, device, is_text=True)
            metrics = compute_metrics(all_preds, all_labels, class_names)
            metrics["confusion_matrix"] = build_confusion_matrix(all_preds, all_labels, num_classes)
            state.final_metrics = metrics

        if not state._stop_flag and not state.early_stopped:
            state.phase = "termine"
        elif state._stop_flag:
            state.phase = "arrete"

    except Exception as e:
        state.phase = f"erreur: {e}"
    finally:
        state.running = False
        if torch.cuda.is_available():
            torch.cuda.empty_cache()


# ── Vision Training ──

def _train_vision(req: TrainingRequest):
    try:
        import torch.nn as nn
        from collections import Counter
        from torch.utils.data import DataLoader, Subset
        from torchvision import transforms, models
        from ml.vision_dataset import VisionSentinelDataset
        from shared.training_utils import EarlyStopping, get_class_weights, find_lr
        from shared.metrics import compute_metrics, build_confusion_matrix, collect_predictions

        config_path = AI_ROOT / "training" / "vision" / "configs" / "train_config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f)

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        use_amp = req.use_mixed_precision and device.type == "cuda"  # Fix 7
        scaler = torch.amp.GradScaler("cuda") if use_amp else None
        state.phase = f"chargement modele ({device}{'+ AMP' if use_amp else ''})"

        size = config["model"]["input_size"]
        num_classes = config["model"]["num_classes"]
        class_names = [config["classes"][i] for i in range(num_classes)]
        patience = config.get("training", {}).get("early_stopping_patience", 10)

        # Fix 4: Augmentation enrichie
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

        # Fix 9: Eval transform avec CenterCrop
        val_transform = transforms.Compose([
            transforms.Resize((size + 32, size + 32)),
            transforms.CenterCrop(size),
            transforms.ToTensor(),
            transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
        ])

        dataset_dir = AI_ROOT / "training" / "vision" / "datasets"
        # Charger deux fois avec transforms differentes
        train_dataset = VisionSentinelDataset(str(dataset_dir), transform=train_transform)
        val_dataset = VisionSentinelDataset(str(dataset_dir), transform=val_transform)

        if len(train_dataset) == 0:
            state.phase = "erreur: dataset vide"
            state.running = False
            return

        # ── Split stratifie (Fix 3 equivalent) ──
        state.phase = "preparation donnees"
        import random
        random.seed(42)

        indices_by_label: dict[int, list[int]] = {}
        for i in range(len(train_dataset)):
            _, label = train_dataset.samples[i]
            indices_by_label.setdefault(label, []).append(i)

        test_ratio = config.get("data", {}).get("test_split", 0.1)
        val_ratio = req.validation_split

        train_indices, val_indices, test_indices = [], [], []
        for label, indices in indices_by_label.items():
            random.shuffle(indices)
            n = len(indices)
            n_test = max(1, int(n * test_ratio))
            n_val = max(1, int(n * val_ratio))
            test_indices.extend(indices[:n_test])
            val_indices.extend(indices[n_test:n_test + n_val])
            train_indices.extend(indices[n_test + n_val:])

        train_set = Subset(train_dataset, train_indices)   # Avec augmentation
        val_set = Subset(val_dataset, val_indices)          # Avec CenterCrop
        test_set = Subset(val_dataset, test_indices)        # Avec CenterCrop

        train_loader = DataLoader(train_set, batch_size=req.batch_size, shuffle=True)
        val_loader = DataLoader(val_set, batch_size=req.batch_size, shuffle=False)
        test_loader = DataLoader(test_set, batch_size=req.batch_size, shuffle=False)

        model = models.efficientnet_v2_s(weights=models.EfficientNet_V2_S_Weights.DEFAULT)
        model.classifier[1] = nn.Linear(model.classifier[1].in_features, num_classes)
        model = model.to(device)

        # Fix 5: Class weights pour vision
        if req.use_class_weights:
            state.phase = "calcul class weights"
            train_labels = [train_dataset.samples[idx][1] for idx in train_indices]
            class_weights = get_class_weights(train_labels, num_classes, device)
            criterion = nn.CrossEntropyLoss(weight=class_weights)
        else:
            criterion = nn.CrossEntropyLoss()

        optimizer = torch.optim.AdamW(model.parameters(), lr=req.learning_rate, weight_decay=0.0001)

        # ── LR Finder (Fix 8) ──
        if req.run_lr_finder:
            state.phase = "LR range test"
            lrs, losses = find_lr(model, train_loader, optimizer, criterion, device, is_text=False)
            if lrs:
                min_loss_idx = losses.index(min(losses))
                suggested_lr = lrs[max(0, min_loss_idx - 10)]
                for pg in optimizer.param_groups:
                    pg["lr"] = suggested_lr
                state.phase = f"LR suggere: {suggested_lr:.2e}"

        scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=req.epochs)

        # Fix 2: Early stopping pour vision (utilise shared)
        early_stopping = EarlyStopping(patience=patience, mode="max")  # Surveille val_accuracy
        num_train_batches = len(train_loader)

        for epoch in range(req.epochs):
            if state._stop_flag:
                state.phase = "arrete"
                break

            state.current_epoch = epoch + 1
            state.phase = f"entrainement epoch {epoch + 1}/{req.epochs}"

            model.train()
            total_loss, correct, total = 0.0, 0, 0
            for batch_idx, (images, labels) in enumerate(train_loader):
                if state._stop_flag:
                    break
                images, labels = images.to(device), labels.to(device)
                optimizer.zero_grad()

                # Fix 7: Mixed precision
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

                state.update_batch(batch_idx, num_train_batches, total_loss, correct, total)

            scheduler.step()
            state.loss = total_loss / max(num_train_batches, 1)
            state.accuracy = correct / max(total, 1)

            # ── Validation ──
            state.phase = f"validation epoch {epoch + 1}/{req.epochs}"
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

            state.val_loss = val_loss / max(len(val_loader), 1)
            state.val_accuracy = val_correct / max(val_total, 1)
            state.record_epoch()

            # ── Sauvegarde + early stopping (Fix 2) ──
            if not early_stopping.step(state.val_accuracy, epoch + 1):
                if early_stopping.counter == 0:  # Just improved
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
                break

        # ── Metriques finales sur test set (Fix 10) ──
        if len(test_set) > 0:
            state.phase = "evaluation sur test set"
            all_preds, all_labels = collect_predictions(model, test_loader, device, is_text=False)
            metrics = compute_metrics(all_preds, all_labels, class_names)
            metrics["confusion_matrix"] = build_confusion_matrix(all_preds, all_labels, num_classes)
            state.final_metrics = metrics

        if not state._stop_flag and not state.early_stopped:
            state.phase = "termine"
        elif state._stop_flag:
            state.phase = "arrete"

    except Exception as e:
        state.phase = f"erreur: {e}"
    finally:
        state.running = False
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
