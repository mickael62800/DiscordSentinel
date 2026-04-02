"""Tests pour la validation Pydantic des modeles de requete."""

import sys
from pathlib import Path

import pytest
from pydantic import ValidationError

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from routes.training import TrainingRequest
from constants import ModelType


class TestTrainingRequestValidation:
    """Tests pour la validation de TrainingRequest."""

    def test_valid_defaults(self):
        req = TrainingRequest(model_type=ModelType.TEXT_SENTIMENT)
        assert req.epochs == 10
        assert req.batch_size == 32
        assert req.learning_rate == 0.001
        assert req.validation_split == 0.2

    def test_valid_custom_values(self):
        req = TrainingRequest(
            model_type=ModelType.IMAGE_CLASSIFICATION,
            epochs=50,
            batch_size=64,
            learning_rate=0.0005,
            validation_split=0.15,
        )
        assert req.epochs == 50
        assert req.batch_size == 64

    def test_valid_from_string(self):
        req = TrainingRequest(model_type="text-sentiment")
        assert req.model_type == ModelType.TEXT_SENTIMENT

    # --- Epochs ---

    def test_epochs_min_1(self):
        req = TrainingRequest(model_type="text-sentiment", epochs=1)
        assert req.epochs == 1

    def test_epochs_max_200(self):
        req = TrainingRequest(model_type="text-sentiment", epochs=200)
        assert req.epochs == 200

    def test_epochs_zero_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", epochs=0)

    def test_epochs_negative_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", epochs=-1)

    def test_epochs_over_200_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", epochs=201)

    # --- Batch size ---

    def test_batch_size_min_1(self):
        req = TrainingRequest(model_type="text-sentiment", batch_size=1)
        assert req.batch_size == 1

    def test_batch_size_max_256(self):
        req = TrainingRequest(model_type="text-sentiment", batch_size=256)
        assert req.batch_size == 256

    def test_batch_size_zero_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", batch_size=0)

    def test_batch_size_over_256_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", batch_size=257)

    # --- Learning rate ---

    def test_learning_rate_small_valid(self):
        req = TrainingRequest(model_type="text-sentiment", learning_rate=0.00001)
        assert req.learning_rate == 0.00001

    def test_learning_rate_max_1(self):
        req = TrainingRequest(model_type="text-sentiment", learning_rate=1.0)
        assert req.learning_rate == 1.0

    def test_learning_rate_zero_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", learning_rate=0)

    def test_learning_rate_negative_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", learning_rate=-0.01)

    def test_learning_rate_over_1_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", learning_rate=1.5)

    # --- Validation split ---

    def test_validation_split_valid(self):
        req = TrainingRequest(model_type="text-sentiment", validation_split=0.3)
        assert req.validation_split == 0.3

    def test_validation_split_zero_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", validation_split=0)

    def test_validation_split_one_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", validation_split=1.0)

    def test_validation_split_over_one_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", validation_split=1.5)

    # --- Early stopping ---

    def test_patience_zero_valid(self):
        req = TrainingRequest(model_type="text-sentiment", early_stopping_patience=0)
        assert req.early_stopping_patience == 0

    def test_patience_max_50(self):
        req = TrainingRequest(model_type="text-sentiment", early_stopping_patience=50)
        assert req.early_stopping_patience == 50

    def test_patience_over_50_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="text-sentiment", early_stopping_patience=51)

    # --- Model type ---

    def test_invalid_model_type_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="invalid-type")

    def test_empty_model_type_rejected(self):
        with pytest.raises(ValidationError):
            TrainingRequest(model_type="")
