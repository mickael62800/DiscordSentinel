"""
Utilitaires partages pour l'entrainement text et vision.
"""

import torch
import torch.nn as nn
from collections import Counter


class EarlyStopping:
    """
    Arrete l'entrainement si la metrique ne s'ameliore pas
    pendant `patience` epochs consecutives.
    """

    def __init__(self, patience: int = 3, mode: str = "min"):
        self.patience = patience
        self.mode = mode  # "min" pour val_loss, "max" pour val_accuracy
        self.counter = 0
        self.best_value = float("inf") if mode == "min" else float("-inf")
        self.best_epoch = 0
        self.should_stop = False

    def step(self, value: float, epoch: int) -> bool:
        """Retourne True si on doit s'arreter."""
        improved = (
            value < self.best_value if self.mode == "min"
            else value > self.best_value
        )
        if improved:
            self.best_value = value
            self.best_epoch = epoch
            self.counter = 0
        else:
            self.counter += 1

        if self.patience > 0 and self.counter >= self.patience:
            self.should_stop = True

        return self.should_stop


def get_class_weights(
    labels: list[int],
    num_classes: int,
    device: torch.device,
) -> torch.Tensor:
    """
    Calcule les poids de classe inversement proportionnels a leur frequence.
    weight[c] = total_samples / (num_classes * count[c])
    """
    counts = Counter(labels)
    total = sum(counts.values())
    weights = []
    for c in range(num_classes):
        count = counts.get(c, 1)
        weights.append(total / (num_classes * count))
    return torch.tensor(weights, dtype=torch.float32).to(device)


def find_lr(
    model: nn.Module,
    train_loader,
    optimizer: torch.optim.Optimizer,
    criterion: nn.Module,
    device: torch.device,
    start_lr: float = 1e-7,
    end_lr: float = 1.0,
    num_steps: int = 100,
    is_text: bool = False,
) -> tuple[list[float], list[float]]:
    """
    LR Range Test : fait varier le learning rate de start_lr a end_lr
    et enregistre la loss. Le LR optimal est la ou la loss descent le plus vite.
    Retourne (lrs, losses) pour visualisation.
    """
    model.train()
    lr_mult = (end_lr / start_lr) ** (1 / num_steps)
    lr = start_lr

    # Sauvegarder l'etat du modele
    original_state = {k: v.clone() for k, v in model.state_dict().items()}
    original_lr = optimizer.param_groups[0]["lr"]

    lrs = []
    losses = []
    best_loss = float("inf")
    avg_loss = 0.0
    smoothing = 0.05

    for param_group in optimizer.param_groups:
        param_group["lr"] = lr

    data_iter = iter(train_loader)

    for step in range(num_steps):
        try:
            batch = next(data_iter)
        except StopIteration:
            data_iter = iter(train_loader)
            batch = next(data_iter)

        optimizer.zero_grad()

        if is_text:
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            labels = batch["labels"].to(device)
            outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
            loss = criterion(outputs.logits, labels) if criterion is not None else outputs.loss
        else:
            images, labels = batch
            images, labels = images.to(device), labels.to(device)
            outputs = model(images)
            loss = criterion(outputs, labels)

        # Smoothed loss
        avg_loss = smoothing * loss.item() + (1 - smoothing) * avg_loss if step > 0 else loss.item()

        if avg_loss > best_loss * 4 and step > 10:
            break  # Loss diverge, on arrete

        if avg_loss < best_loss:
            best_loss = avg_loss

        lrs.append(lr)
        losses.append(avg_loss)

        loss.backward()
        optimizer.step()

        lr *= lr_mult
        for param_group in optimizer.param_groups:
            param_group["lr"] = lr

    # Restaurer l'etat du modele
    model.load_state_dict(original_state)
    for param_group in optimizer.param_groups:
        param_group["lr"] = original_lr

    return lrs, losses
