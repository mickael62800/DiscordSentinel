"""
Dataset pour la detection de sentiments toxiques.
Integre directement dans l'API — plus de dependance aux scripts externes.
"""

import json
from pathlib import Path
from torch.utils.data import Dataset


CLASS_MAP = {
    "neutral": 0,
    "anger": 1,
    "rage": 2,
    "threat": 3,
    "harassment": 4,
}


class TextSentinelDataset(Dataset):
    def __init__(self, root_dir: str, tokenizer, max_length: int = 128):
        self.root_dir = Path(root_dir)
        self.tokenizer = tokenizer
        self.max_length = max_length
        self.samples = []

        # Charger les textes neutres
        neutral_dir = self.root_dir / "neutral"
        if neutral_dir.exists():
            for file in neutral_dir.iterdir():
                if file.suffix == ".txt":
                    for line in file.read_text(encoding="utf-8").splitlines():
                        line = line.strip()
                        if line:
                            self.samples.append((line, 0))

        # Charger les textes toxiques (JSONL avec label)
        toxic_dir = self.root_dir / "toxic"
        if toxic_dir.exists():
            for file in toxic_dir.iterdir():
                if file.suffix == ".jsonl":
                    for line in file.read_text(encoding="utf-8").splitlines():
                        line = line.strip()
                        if line:
                            try:
                                entry = json.loads(line)
                                self.samples.append((entry["text"], entry["label"]))
                            except (json.JSONDecodeError, KeyError):
                                continue
                elif file.suffix == ".txt":
                    for line in file.read_text(encoding="utf-8").splitlines():
                        line = line.strip()
                        if line:
                            self.samples.append((line, 1))

        print(f"Dataset charge: {len(self.samples)} textes")
        for name, label in CLASS_MAP.items():
            count = sum(1 for _, l in self.samples if l == label)
            print(f"  {name}: {count}")

    def __len__(self):
        return len(self.samples)

    def __getitem__(self, idx):
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
