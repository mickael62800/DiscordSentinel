"""
Constantes et enums partages pour l'API ML Sentinel.
"""

from enum import Enum


class ModelType(str, Enum):
    """Types de modeles supportes par la plateforme."""

    TEXT_SENTIMENT = "text-sentiment"
    IMAGE_CLASSIFICATION = "image-classification"


# Taille max d'upload (100 Mo)
MAX_UPLOAD_BYTES: int = 100 * 1024 * 1024

# Extensions autorisees pour les datasets vision
VISION_EXTENSIONS: frozenset[str] = frozenset({".jpg", ".jpeg", ".png", ".webp", ".bmp"})

# Extensions autorisees pour les datasets text
TEXT_EXTENSIONS: frozenset[str] = frozenset({".jsonl", ".txt"})

# Origines CORS autorisees
ALLOWED_ORIGINS: list[str] = [
    "http://localhost:1420",
    "http://localhost:5173",
    "http://127.0.0.1:1420",
    "http://127.0.0.1:5173",
    "tauri://localhost",
]
