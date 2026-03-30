"""
Metriques partagees entre les pipelines text et vision.
"""

import torch
import numpy as np
from collections import defaultdict


def compute_metrics(
    all_preds: list[int],
    all_labels: list[int],
    class_names: list[str],
) -> dict:
    """
    Calcule precision, recall, F1 par classe + macro average.
    Retourne un dict pret pour le logging.
    """
    num_classes = len(class_names)
    tp = defaultdict(int)
    fp = defaultdict(int)
    fn = defaultdict(int)

    for pred, label in zip(all_preds, all_labels):
        if pred == label:
            tp[pred] += 1
        else:
            fp[pred] += 1
            fn[label] += 1

    per_class = {}
    macro_p, macro_r, macro_f1 = 0.0, 0.0, 0.0

    for c in range(num_classes):
        precision = tp[c] / max(tp[c] + fp[c], 1)
        recall = tp[c] / max(tp[c] + fn[c], 1)
        f1 = 2 * precision * recall / max(precision + recall, 1e-8)
        per_class[class_names[c]] = {
            "precision": round(precision, 4),
            "recall": round(recall, 4),
            "f1": round(f1, 4),
            "support": tp[c] + fn[c],
        }
        macro_p += precision
        macro_r += recall
        macro_f1 += f1

    accuracy = sum(tp.values()) / max(len(all_preds), 1)

    return {
        "accuracy": round(accuracy, 4),
        "macro_precision": round(macro_p / num_classes, 4),
        "macro_recall": round(macro_r / num_classes, 4),
        "macro_f1": round(macro_f1 / num_classes, 4),
        "per_class": per_class,
    }


def build_confusion_matrix(
    all_preds: list[int],
    all_labels: list[int],
    num_classes: int,
) -> list[list[int]]:
    """
    Matrice de confusion [num_classes x num_classes].
    matrix[true][pred] = count
    """
    matrix = [[0] * num_classes for _ in range(num_classes)]
    for pred, label in zip(all_preds, all_labels):
        if 0 <= label < num_classes and 0 <= pred < num_classes:
            matrix[label][pred] += 1
    return matrix


@torch.no_grad()
def collect_predictions(
    model,
    dataloader,
    device: torch.device,
    is_text: bool = False,
) -> tuple[list[int], list[int]]:
    """
    Collecte predictions et labels sur un dataloader complet.
    Fonctionne pour text (dict batches) et vision (tuple batches).
    """
    model.eval()
    all_preds = []
    all_labels = []

    for batch in dataloader:
        if is_text:
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            labels = batch["labels"].to(device)
            outputs = model(input_ids=input_ids, attention_mask=attention_mask)
            logits = outputs.logits
        else:
            images, labels = batch
            images, labels = images.to(device), labels.to(device)
            logits = model(images)

        preds = logits.argmax(dim=-1)
        all_preds.extend(preds.cpu().tolist())
        all_labels.extend(labels.cpu().tolist())

    return all_preds, all_labels
