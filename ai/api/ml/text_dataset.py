"""
Dataset pour la detection de sentiments toxiques.
Integre directement dans l'API — plus de dependance aux scripts externes.
"""

import json
import logging
from pathlib import Path
from typing import Any

from torch.utils.data import Dataset

logger = logging.getLogger("sentinel.ai.dataset.text")

CLASS_MAP: dict[str, int] = {
    "neutral": 0,
    "anger": 1,
    "rage": 2,
    "threat": 3,
    "harassment": 4,
}


class TextSentinelDataset(Dataset):
    """Dataset PyTorch pour le text-sentiment.

    Structure attendue :
        root_dir/
            neutral/    -> fichiers .txt (une phrase par ligne)
            toxic/      -> fichiers .jsonl ({"text": "...", "label": 1})
                        -> fichiers .txt  (une phrase par ligne, label=1)
    """

    def __init__(self, root_dir: str, tokenizer: Any, max_length: int = 128) -> None:
        self.root_dir = Path(root_dir)
        self.tokenizer = tokenizer
        self.max_length = max_length
        self.samples: list[tuple[str, int]] = []

        self._load_neutral()
        self._load_toxic()

        logger.info("TextSentinelDataset charge: %d textes", len(self.samples))
        for name, label in CLASS_MAP.items():
            count = sum(1 for _, l in self.samples if l == label)
            if count > 0:
                logger.info("  %s: %d", name, count)

    def _load_neutral(self) -> None:
        """Charge les textes neutres depuis root_dir/neutral/*.txt."""
        neutral_dir = self.root_dir / "neutral"
        if not neutral_dir.exists():
            return

        for file in neutral_dir.iterdir():
            if file.suffix == ".txt":
                for line in file.read_text(encoding="utf-8").splitlines():
                    line = line.strip()
                    if line:
                        self.samples.append((line, 0))

    def _load_toxic(self) -> None:
        """Charge les textes toxiques depuis root_dir/toxic/*.jsonl et *.txt."""
        toxic_dir = self.root_dir / "toxic"
        if not toxic_dir.exists():
            return

        for file in toxic_dir.iterdir():
            if file.suffix == ".jsonl":
                skipped = 0
                for line in file.read_text(encoding="utf-8").splitlines():
                    line = line.strip()
                    if line:
                        try:
                            entry = json.loads(line)
                            self.samples.append((entry["text"], entry["label"]))
                        except (json.JSONDecodeError, KeyError):
                            skipped += 1
                            continue
                if skipped > 0:
                    logger.warning("Fichier %s: %d lignes ignorees (malformees)", file.name, skipped)
            elif file.suffix == ".txt":
                for line in file.read_text(encoding="utf-8").splitlines():
                    line = line.strip()
                    if line:
                        self.samples.append((line, 1))

    def __len__(self) -> int:
        return len(self.samples)

    def __getitem__(self, idx: int) -> dict[str, Any]:
        text, label = self.samples[idx]

        encoding = self.tokenizer(
            text,
            max_length=self.max_length,
            padding="max_length",
            truncation=True,
            return_tensors="pt",
        )

        return {
            "input_ids": encoding["input_ids"].squeeze(0),
            "attention_mask": encoding["attention_mask"].squeeze(0),
            "labels": label,
        }
