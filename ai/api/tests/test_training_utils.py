"""Tests pour les utilitaires d'entrainement."""

import sys
from pathlib import Path

import torch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from shared.training_utils import EarlyStopping, get_class_weights


class TestEarlyStopping:
    """Tests pour EarlyStopping."""

    def test_no_stop_when_improving_min(self):
        es = EarlyStopping(patience=3, mode="min")
        assert es.step(1.0, 1) is False
        assert es.step(0.9, 2) is False
        assert es.step(0.8, 3) is False
        assert es.counter == 0

    def test_no_stop_when_improving_max(self):
        es = EarlyStopping(patience=3, mode="max")
        assert es.step(0.5, 1) is False
        assert es.step(0.6, 2) is False
        assert es.step(0.7, 3) is False
        assert es.counter == 0

    def test_stops_after_patience_min(self):
        es = EarlyStopping(patience=2, mode="min")
        es.step(0.5, 1)  # best
        es.step(0.6, 2)  # worse, counter=1
        result = es.step(0.7, 3)  # worse, counter=2 => stop
        assert result is True
        assert es.should_stop is True

    def test_stops_after_patience_max(self):
        es = EarlyStopping(patience=2, mode="max")
        es.step(0.9, 1)  # best
        es.step(0.8, 2)  # worse, counter=1
        result = es.step(0.7, 3)  # worse, counter=2 => stop
        assert result is True

    def test_counter_resets_on_improvement(self):
        es = EarlyStopping(patience=3, mode="min")
        es.step(1.0, 1)
        es.step(1.1, 2)  # counter=1
        es.step(1.2, 3)  # counter=2
        es.step(0.5, 4)  # improved! counter=0
        assert es.counter == 0
        assert es.best_value == 0.5
        assert es.best_epoch == 4

    def test_tracks_best_epoch(self):
        es = EarlyStopping(patience=5, mode="min")
        es.step(1.0, 1)
        es.step(0.8, 2)
        es.step(0.9, 3)
        es.step(0.7, 4)
        assert es.best_epoch == 4
        assert es.best_value == 0.7

    def test_patience_zero_never_stops(self):
        es = EarlyStopping(patience=0, mode="min")
        es.step(1.0, 1)
        es.step(2.0, 2)
        es.step(3.0, 3)
        es.step(4.0, 4)
        assert es.should_stop is False

    def test_patience_one(self):
        es = EarlyStopping(patience=1, mode="min")
        es.step(1.0, 1)
        result = es.step(2.0, 2)
        assert result is True


class TestGetClassWeights:
    """Tests pour get_class_weights."""

    def test_balanced_dataset(self):
        labels = [0, 1, 2, 0, 1, 2]
        weights = get_class_weights(labels, 3, torch.device("cpu"))
        assert weights.shape == (3,)
        # Balanced: all weights should be ~1.0
        assert torch.allclose(weights, torch.ones(3), atol=0.01)

    def test_imbalanced_dataset(self):
        labels = [0, 0, 0, 0, 1]  # 4:1 ratio
        weights = get_class_weights(labels, 2, torch.device("cpu"))
        # Class 0 (frequent) should have lower weight
        assert weights[0] < weights[1]

    def test_single_class(self):
        labels = [0, 0, 0]
        weights = get_class_weights(labels, 2, torch.device("cpu"))
        assert weights.shape == (2,)
        # Class 0 has all samples, class 1 has none (count defaults to 1)
        assert weights[0] < weights[1]

    def test_returns_tensor(self):
        labels = [0, 1]
        weights = get_class_weights(labels, 2, torch.device("cpu"))
        assert isinstance(weights, torch.Tensor)
        assert weights.dtype == torch.float32

    def test_empty_labels(self):
        labels = []
        weights = get_class_weights(labels, 2, torch.device("cpu"))
        assert weights.shape == (2,)
