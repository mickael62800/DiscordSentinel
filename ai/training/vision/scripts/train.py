"""
Vision Sentinel - Entrainement du modele de detection NSFW / Produits illicites
"""

import os
import yaml
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, random_split
from torchvision import transforms, models
from pathlib import Path
from tqdm import tqdm

from dataset import VisionSentinelDataset


def load_config(path: str = "../configs/train_config.yaml") -> dict:
    with open(path, "r") as f:
        return yaml.safe_load(f)


def build_model(config: dict) -> nn.Module:
    backbone = config["model"]["backbone"]
    num_classes = config["model"]["num_classes"]

    if backbone == "efficientnet_v2_s":
        model = models.efficientnet_v2_s(
            weights=models.EfficientNet_V2_S_Weights.DEFAULT if config["model"]["pretrained"] else None
        )
        model.classifier[1] = nn.Linear(model.classifier[1].in_features, num_classes)
    else:
        raise ValueError(f"Backbone non supporte: {backbone}")

    return model


def get_transforms(config: dict, train: bool = True):
    size = config["model"]["input_size"]

    if train and config["data"]["augmentation"]:
        return transforms.Compose([
            transforms.Resize((size + 32, size + 32)),
            transforms.RandomCrop(size),
            transforms.RandomHorizontalFlip(),
            transforms.ColorJitter(brightness=0.2, contrast=0.2, saturation=0.2),
            transforms.RandomRotation(15),
            transforms.ToTensor(),
            transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
        ])
    else:
        return transforms.Compose([
            transforms.Resize((size, size)),
            transforms.ToTensor(),
            transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
        ])


def train_one_epoch(model, loader, criterion, optimizer, device):
    model.train()
    total_loss = 0
    correct = 0
    total = 0

    for images, labels in tqdm(loader, desc="Training"):
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

    return total_loss / len(loader), correct / total


@torch.no_grad()
def evaluate(model, loader, criterion, device):
    model.eval()
    total_loss = 0
    correct = 0
    total = 0

    for images, labels in tqdm(loader, desc="Validation"):
        images, labels = images.to(device), labels.to(device)

        outputs = model(images)
        loss = criterion(outputs, labels)

        total_loss += loss.item()
        _, predicted = outputs.max(1)
        correct += predicted.eq(labels).sum().item()
        total += labels.size(0)

    return total_loss / len(loader), correct / total


def main():
    config = load_config()
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")

    # Dataset
    dataset = VisionSentinelDataset(
        root_dir="../datasets",
        transform=get_transforms(config, train=True),
    )

    # Split
    total = len(dataset)
    train_size = int(total * config["data"]["train_split"])
    val_size = int(total * config["data"]["val_split"])
    test_size = total - train_size - val_size

    train_set, val_set, test_set = random_split(dataset, [train_size, val_size, test_size])

    # Val/test utilisent les transforms sans augmentation
    val_set.dataset.transform = get_transforms(config, train=False)

    train_loader = DataLoader(train_set, batch_size=config["training"]["batch_size"],
                              shuffle=True, num_workers=config["data"]["num_workers"])
    val_loader = DataLoader(val_set, batch_size=config["training"]["batch_size"],
                            shuffle=False, num_workers=config["data"]["num_workers"])

    # Model
    model = build_model(config).to(device)
    criterion = nn.CrossEntropyLoss()
    optimizer = torch.optim.AdamW(model.parameters(),
                                  lr=config["training"]["learning_rate"],
                                  weight_decay=config["training"]["weight_decay"])
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
        optimizer, T_max=config["training"]["epochs"]
    )

    # Training loop
    best_val_acc = 0
    patience_counter = 0

    for epoch in range(config["training"]["epochs"]):
        print(f"\n--- Epoch {epoch + 1}/{config['training']['epochs']} ---")

        train_loss, train_acc = train_one_epoch(model, train_loader, criterion, optimizer, device)
        val_loss, val_acc = evaluate(model, val_loader, criterion, device)
        scheduler.step()

        print(f"Train Loss: {train_loss:.4f} | Train Acc: {train_acc:.4f}")
        print(f"Val Loss:   {val_loss:.4f} | Val Acc:   {val_acc:.4f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            patience_counter = 0
            os.makedirs("../checkpoints", exist_ok=True)
            torch.save({
                "epoch": epoch,
                "model_state_dict": model.state_dict(),
                "optimizer_state_dict": optimizer.state_dict(),
                "val_acc": val_acc,
                "config": config,
            }, "../checkpoints/best_model.pt")
            print(f"Nouveau meilleur modele sauvegarde (acc: {val_acc:.4f})")
        else:
            patience_counter += 1
            if patience_counter >= config["training"]["early_stopping_patience"]:
                print("Early stopping!")
                break

    print(f"\nMeilleure validation accuracy: {best_val_acc:.4f}")


if __name__ == "__main__":
    main()
