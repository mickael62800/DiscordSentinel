"""
Dataset pour la detection vision (NSFW / Produits illicites / Safe).
Integre directement dans l'API.
"""

from pathlib import Path
from PIL import Image
from torch.utils.data import Dataset


CLASS_MAP = {
    "safe": 0,
    "nsfw": 1,
    "illicit": 2,
}

SUPPORTED_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}


class VisionSentinelDataset(Dataset):
    def __init__(self, root_dir: str, transform=None):
        self.root_dir = Path(root_dir)
        self.transform = transform
        self.samples = []

        for class_name, label in CLASS_MAP.items():
            class_dir = self.root_dir / class_name
            if not class_dir.exists():
                continue

            for img_path in class_dir.iterdir():
                if img_path.suffix.lower() in SUPPORTED_EXTENSIONS:
                    self.samples.append((str(img_path), label))

        print(f"Dataset charge: {len(self.samples)} images")
        for class_name, label in CLASS_MAP.items():
            count = sum(1 for _, l in self.samples if l == label)
            print(f"  {class_name}: {count}")

    def __len__(self):
        return len(self.samples)

    def __getitem__(self, idx):
        img_path, label = self.samples[idx]
        with Image.open(img_path) as img:
            image = img.convert("RGB")

        if self.transform:
            image = self.transform(image)

        return image, label
