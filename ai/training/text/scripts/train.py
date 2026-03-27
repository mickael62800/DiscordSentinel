"""
Text Sentinel - Entrainement du modele de detection de sentiments toxiques
(colere, rage, menaces, harcelement)
"""

import os
import yaml
import torch
from torch.utils.data import DataLoader, random_split
from transformers import (
    AutoTokenizer,
    AutoModelForSequenceClassification,
    get_linear_schedule_with_warmup,
)
from tqdm import tqdm

from dataset import TextSentinelDataset


def load_config(path: str = "../configs/train_config.yaml") -> dict:
    with open(path, "r") as f:
        return yaml.safe_load(f)


def train_one_epoch(model, loader, optimizer, scheduler, device):
    model.train()
    total_loss = 0
    correct = 0
    total = 0

    for batch in tqdm(loader, desc="Training"):
        input_ids = batch["input_ids"].to(device)
        attention_mask = batch["attention_mask"].to(device)
        labels = batch["labels"].to(device)

        optimizer.zero_grad()
        outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)
        loss = outputs.loss
        loss.backward()
        optimizer.step()
        scheduler.step()

        total_loss += loss.item()
        preds = outputs.logits.argmax(dim=-1)
        correct += (preds == labels).sum().item()
        total += labels.size(0)

    return total_loss / len(loader), correct / total


@torch.no_grad()
def evaluate(model, loader, device):
    model.eval()
    total_loss = 0
    correct = 0
    total = 0

    for batch in tqdm(loader, desc="Validation"):
        input_ids = batch["input_ids"].to(device)
        attention_mask = batch["attention_mask"].to(device)
        labels = batch["labels"].to(device)

        outputs = model(input_ids=input_ids, attention_mask=attention_mask, labels=labels)

        total_loss += outputs.loss.item()
        preds = outputs.logits.argmax(dim=-1)
        correct += (preds == labels).sum().item()
        total += labels.size(0)

    return total_loss / len(loader), correct / total


def main():
    config = load_config()
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")

    backbone = config["model"]["backbone"]
    max_length = config["model"]["max_length"]

    # Tokenizer
    tokenizer = AutoTokenizer.from_pretrained(backbone)

    # Dataset
    dataset = TextSentinelDataset(
        root_dir="../datasets",
        tokenizer=tokenizer,
        max_length=max_length,
    )

    # Split
    total = len(dataset)
    train_size = int(total * config["data"]["train_split"])
    val_size = int(total * config["data"]["val_split"])
    test_size = total - train_size - val_size

    train_set, val_set, test_set = random_split(dataset, [train_size, val_size, test_size])

    train_loader = DataLoader(train_set, batch_size=config["training"]["batch_size"],
                              shuffle=True, num_workers=config["data"]["num_workers"])
    val_loader = DataLoader(val_set, batch_size=config["training"]["batch_size"],
                            shuffle=False, num_workers=config["data"]["num_workers"])

    # Model
    model = AutoModelForSequenceClassification.from_pretrained(
        backbone, num_labels=config["model"]["num_classes"]
    ).to(device)

    optimizer = torch.optim.AdamW(model.parameters(),
                                  lr=config["training"]["learning_rate"],
                                  weight_decay=config["training"]["weight_decay"])

    total_steps = len(train_loader) * config["training"]["epochs"]
    warmup_steps = int(total_steps * config["training"]["warmup_ratio"])
    scheduler = get_linear_schedule_with_warmup(optimizer, warmup_steps, total_steps)

    # Training loop
    best_val_acc = 0
    patience_counter = 0

    for epoch in range(config["training"]["epochs"]):
        print(f"\n--- Epoch {epoch + 1}/{config['training']['epochs']} ---")

        train_loss, train_acc = train_one_epoch(model, train_loader, optimizer, scheduler, device)
        val_loss, val_acc = evaluate(model, val_loader, device)

        print(f"Train Loss: {train_loss:.4f} | Train Acc: {train_acc:.4f}")
        print(f"Val Loss:   {val_loss:.4f} | Val Acc:   {val_acc:.4f}")

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            patience_counter = 0
            os.makedirs("../checkpoints", exist_ok=True)
            model.save_pretrained("../checkpoints/best_model")
            tokenizer.save_pretrained("../checkpoints/best_model")
            print(f"Nouveau meilleur modele sauvegarde (acc: {val_acc:.4f})")
        else:
            patience_counter += 1
            if patience_counter >= config["training"]["early_stopping_patience"]:
                print("Early stopping!")
                break

    print(f"\nMeilleure validation accuracy: {best_val_acc:.4f}")


if __name__ == "__main__":
    main()
