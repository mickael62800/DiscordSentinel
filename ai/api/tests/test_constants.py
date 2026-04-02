"""Tests pour les constantes et enums."""

from constants import ModelType, MAX_UPLOAD_BYTES, VISION_EXTENSIONS, ALLOWED_ORIGINS


class TestModelType:
    """Tests pour l'enum ModelType."""

    def test_text_sentiment_value(self):
        assert ModelType.TEXT_SENTIMENT.value == "text-sentiment"

    def test_image_classification_value(self):
        assert ModelType.IMAGE_CLASSIFICATION.value == "image-classification"

    def test_from_string(self):
        assert ModelType("text-sentiment") == ModelType.TEXT_SENTIMENT
        assert ModelType("image-classification") == ModelType.IMAGE_CLASSIFICATION

    def test_invalid_raises(self):
        import pytest
        with pytest.raises(ValueError):
            ModelType("invalid-type")

    def test_is_string(self):
        """ModelType herite de str, donc utilisable comme string."""
        assert isinstance(ModelType.TEXT_SENTIMENT, str)
        assert "text-sentiment" == ModelType.TEXT_SENTIMENT


class TestConstants:
    """Tests pour les constantes."""

    def test_max_upload_bytes_is_100mb(self):
        assert MAX_UPLOAD_BYTES == 100 * 1024 * 1024

    def test_vision_extensions_contains_common_formats(self):
        assert ".jpg" in VISION_EXTENSIONS
        assert ".jpeg" in VISION_EXTENSIONS
        assert ".png" in VISION_EXTENSIONS
        assert ".webp" in VISION_EXTENSIONS
        assert ".bmp" in VISION_EXTENSIONS

    def test_vision_extensions_is_frozenset(self):
        assert isinstance(VISION_EXTENSIONS, frozenset)

    def test_allowed_origins_not_wildcard(self):
        """CORS ne doit pas etre ouvert a tout le monde."""
        assert "*" not in ALLOWED_ORIGINS

    def test_allowed_origins_has_localhost(self):
        assert any("localhost" in origin for origin in ALLOWED_ORIGINS)
