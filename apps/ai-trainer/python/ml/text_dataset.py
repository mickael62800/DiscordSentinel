"""Dataset PyTorch pour la classification de sentiment texte."""

import json
import logging
from pathlib import Path
from typing import Any

from torch.utils.data import Dataset

logger = logging.getLogger("sentinel.trainer.dataset.text")

CLASS_MAP: dict[str, int] = {
    "safe": 0,
    "severe": 1,
}

# Projette les 5 labels bruts du dataset (toxifrench) sur 2 classes binaires.
# neutral + anger + harassment -> safe ; rage + threat -> severe.
_LABEL_REMAP: dict[int, int] = {0: 0, 1: 0, 2: 1, 3: 1, 4: 0}


class TextSentinelDataset(Dataset):
    """root_dir/neutral/*.txt + root_dir/toxic/*.{jsonl,txt}."""

    def __init__(self, root_dir: str, tokenizer: Any, max_length: int = 128) -> None:
        self.root_dir = Path(root_dir)
        self.tokenizer = tokenizer
        self.max_length = max_length
        self.samples: list[tuple[str, int]] = []

        self._load_neutral()
        self._load_toxic()

    def _load_neutral(self) -> None:
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
        toxic_dir = self.root_dir / "toxic"
        if not toxic_dir.exists():
            return
        for file in toxic_dir.iterdir():
            if file.suffix == ".jsonl":
                for line in file.read_text(encoding="utf-8").splitlines():
                    line = line.strip()
                    if line:
                        try:
                            entry = json.loads(line)
                            raw_label = entry["label"]
                            label = _LABEL_REMAP.get(raw_label, raw_label)
                            self.samples.append((entry["text"], label))
                        except (json.JSONDecodeError, KeyError):
                            continue
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
