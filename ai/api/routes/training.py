"""
Entrainement des modeles — demarrage, arret, status en temps reel.
Tout est integre, aucune dependance aux scripts externes.
"""

import threading
from pathlib import Path

import yaml
import torch
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

router = APIRouter()

AI_ROOT = Path(__file__).resolve().parent.parent.parent


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
        # Progression intra-epoch
        self.current_batch = 0
        self.total_batches = 0
        self.batch_loss = 0.0
        self.batch_accuracy = 0.0

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
            "epoch_history": self.epoch_history,
            "current_batch": self.current_batch,
            "total_batches": self.total_batches,
            "batch_loss": round(self.batch_loss, 4),
            "batch_accuracy": round(self.batch_accuracy, 4),
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
        self.current_batch = 0
        self.total_batches = 0
        self.batch_loss = 0.0
        self.batch_accuracy = 0.0


state = TrainingState()
_training_thread: threading.Thread | None = None


class TrainingRequest(BaseModel):
    model_type: str
    epochs: int = 10
    batch_size: int = 32
    learning_rate: float = 0.001
    validation_split: float = 0.2


@router.get("/training/status")
async def training_status():
    return state.to_dict()


@router.post("/training/start")
async def start_training(req: TrainingRequest):
    global _training_thread

    # Si un thread tourne encore apres un stop, attendre qu'il finisse
    if state.running or (_training_thread is not None and _training_thread.is_alive()):
        state._stop_flag = True
        if _training_thread is not None and _training_thread.is_alive():
            state.phase = "arret en cours"
            _training_thread.join(timeout=15)
            if _training_thread.is_alive():
                raise HTTPException(409, "L'entrainement precedent ne s'est pas arrete a temps")

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
        from torch.utils.data import DataLoader, random_split
        from transformers import AutoTokenizer, AutoModelForSequenceClassification, get_linear_schedule_with_warmup
        from ml.text_dataset import TextSentinelDataset

        config_path = AI_ROOT / "training" / "text" / "configs" / "train_config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f)

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        state.phase = f"chargement modele ({device})"

        backbone = config["model"]["backbone"]
        max_length = config["model"]["max_length"]
        tokenizer = AutoTokenizer.from_pretrained(backbone)

        dataset_dir = AI_ROOT / "training" / "text" / "datasets"
        dataset = TextSentinelDataset(str(dataset_dir), tokenizer, max_length)

        if len(dataset) == 0:
            state.phase = "erreur: dataset vide"
            state.running = False
            return

        state.phase = "preparation donnees"
        train_size = int(len(dataset) * (1 - req.validation_split))
        val_size = len(dataset) - train_size
        train_set, val_set = random_split(dataset, [train_size, val_size])

        train_loader = DataLoader(train_set, batch_size=req.batch_size, shuffle=True)
        val_loader = DataLoader(val_set, batch_size=req.batch_size, shuffle=False)

        model = AutoModelForSequenceClassification.from_pretrained(
            backbone, num_labels=config["model"]["num_classes"]
        ).to(device)

        optimizer = torch.optim.AdamW(model.parameters(), lr=req.learning_rate, weight_decay=0.01)
        total_steps = len(train_loader) * req.epochs
        scheduler = get_linear_schedule_with_warmup(optimizer, int(total_steps * 0.1), total_steps)

        best_val_acc = 0.0
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
                outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
                outputs.loss.backward()
                optimizer.step()
                scheduler.step()

                total_loss += outputs.loss.item()
                preds = outputs.logits.argmax(dim=-1)
                correct += (preds == labels).sum().item()
                total += labels.size(0)

                state.update_batch(batch_idx, num_train_batches, total_loss, correct, total)

            state.loss = total_loss / max(num_train_batches, 1)
            state.accuracy = correct / max(total, 1)

            state.phase = f"validation epoch {epoch + 1}/{req.epochs}"
            model.eval()
            val_loss, val_correct, val_total = 0.0, 0, 0
            with torch.no_grad():
                for batch in val_loader:
                    input_ids = batch["input_ids"].to(device)
                    attention_mask = batch["attention_mask"].to(device)
                    labels = batch["labels"].to(device)
                    outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
                    val_loss += outputs.loss.item()
                    preds = outputs.logits.argmax(dim=-1)
                    val_correct += (preds == labels).sum().item()
                    val_total += labels.size(0)

            state.val_loss = val_loss / max(len(val_loader), 1)
            state.val_accuracy = val_correct / max(val_total, 1)
            state.record_epoch()

            if state.val_accuracy > best_val_acc:
                best_val_acc = state.val_accuracy
                checkpoint_dir = AI_ROOT / "training" / "text" / "checkpoints" / "best_model"
                checkpoint_dir.mkdir(parents=True, exist_ok=True)
                model.save_pretrained(str(checkpoint_dir))
                tokenizer.save_pretrained(str(checkpoint_dir))

        state.phase = "termine" if not state._stop_flag else "arrete"

    except Exception as e:
        state.phase = f"erreur: {e}"
    finally:
        state.running = False


# ── Vision Training ──

def _train_vision(req: TrainingRequest):
    try:
        import torch.nn as nn
        from torch.utils.data import DataLoader, random_split
        from torchvision import transforms, models
        from ml.vision_dataset import VisionSentinelDataset

        config_path = AI_ROOT / "training" / "vision" / "configs" / "train_config.yaml"
        with open(config_path) as f:
            config = yaml.safe_load(f)

        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        state.phase = f"chargement modele ({device})"

        size = config["model"]["input_size"]
        train_transform = transforms.Compose([
            transforms.Resize((size + 32, size + 32)),
            transforms.RandomCrop(size),
            transforms.RandomHorizontalFlip(),
            transforms.ToTensor(),
            transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
        ])

        dataset_dir = AI_ROOT / "training" / "vision" / "datasets"
        dataset = VisionSentinelDataset(str(dataset_dir), transform=train_transform)

        if len(dataset) == 0:
            state.phase = "erreur: dataset vide"
            state.running = False
            return

        state.phase = "preparation donnees"
        train_size = int(len(dataset) * (1 - req.validation_split))
        val_size = len(dataset) - train_size
        train_set, val_set = random_split(dataset, [train_size, val_size])

        train_loader = DataLoader(train_set, batch_size=req.batch_size, shuffle=True)
        val_loader = DataLoader(val_set, batch_size=req.batch_size, shuffle=False)

        model = models.efficientnet_v2_s(weights=models.EfficientNet_V2_S_Weights.DEFAULT)
        model.classifier[1] = nn.Linear(model.classifier[1].in_features, config["model"]["num_classes"])
        model = model.to(device)

        criterion = nn.CrossEntropyLoss()
        optimizer = torch.optim.AdamW(model.parameters(), lr=req.learning_rate, weight_decay=0.0001)
        scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=req.epochs)

        best_val_acc = 0.0
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

            state.phase = f"validation epoch {epoch + 1}/{req.epochs}"
            model.eval()
            val_loss, val_correct, val_total = 0.0, 0, 0
            with torch.no_grad():
                for images, labels in val_loader:
                    images, labels = images.to(device), labels.to(device)
                    outputs = model(images)
                    loss = criterion(outputs, labels)
                    val_loss += loss.item()
                    _, predicted = outputs.max(1)
                    val_correct += predicted.eq(labels).sum().item()
                    val_total += labels.size(0)

            state.val_loss = val_loss / max(len(val_loader), 1)
            state.val_accuracy = val_correct / max(val_total, 1)
            state.record_epoch()

            if state.val_accuracy > best_val_acc:
                best_val_acc = state.val_accuracy
                checkpoint_dir = AI_ROOT / "training" / "vision" / "checkpoints"
                checkpoint_dir.mkdir(parents=True, exist_ok=True)
                torch.save({
                    "epoch": epoch,
                    "model_state_dict": model.state_dict(),
                    "val_acc": state.val_accuracy,
                    "config": config,
                }, str(checkpoint_dir / "best_model.pt"))

        state.phase = "termine" if not state._stop_flag else "arrete"

    except Exception as e:
        state.phase = f"erreur: {e}"
    finally:
        state.running = False
