"""Constantes partagees du pipeline d'entrainement."""

from enum import Enum


class ModelType(str, Enum):
    TEXT_SENTIMENT = "text-sentiment"
    IMAGE_CLASSIFICATION = "image-classification"


MAX_UPLOAD_BYTES: int = 100 * 1024 * 1024
VISION_EXTENSIONS: frozenset[str] = frozenset({".jpg", ".jpeg", ".png", ".webp", ".bmp"})
TEXT_EXTENSIONS: frozenset[str] = frozenset({".jsonl", ".txt"})
