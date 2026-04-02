"""
Fixtures partagees pour les tests de l'API ML Sentinel.
"""

import sys
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

# Ajouter le repertoire api au path pour les imports
API_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(API_DIR))
sys.path.insert(0, str(API_DIR.parent))


@pytest.fixture
def client():
    """Client de test FastAPI."""
    from main import app
    return TestClient(app)


@pytest.fixture
def tmp_datasets(tmp_path: Path):
    """Cree une arborescence de datasets temporaire pour les tests."""
    # Text datasets
    text_dir = tmp_path / "training" / "text" / "datasets"
    neutral_dir = text_dir / "neutral"
    toxic_dir = text_dir / "toxic"
    neutral_dir.mkdir(parents=True)
    toxic_dir.mkdir(parents=True)

    # Fichiers text neutres
    neutral_file = neutral_dir / "sample.txt"
    neutral_file.write_text("bonjour tout le monde\ncomment ca va\nbelle journee\n", encoding="utf-8")

    # Fichiers JSONL toxiques
    jsonl_file = toxic_dir / "sample.jsonl"
    jsonl_file.write_text(
        '{"text": "insulte", "label": 1}\n'
        '{"text": "menace grave", "label": 3}\n'
        '{"text": "harcelement", "label": 4}\n',
        encoding="utf-8",
    )

    # Fichier txt toxique
    toxic_txt = toxic_dir / "anger.txt"
    toxic_txt.write_text("je suis en colere\nrage absolue\n", encoding="utf-8")

    # Vision datasets
    vision_dir = tmp_path / "training" / "vision" / "datasets"
    for class_name in ("safe", "nsfw", "illicit"):
        class_dir = vision_dir / class_name
        class_dir.mkdir(parents=True)

    return tmp_path
