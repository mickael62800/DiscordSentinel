"""Tests pour les metriques partagees."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from shared.metrics import compute_metrics, build_confusion_matrix


class TestComputeMetrics:
    """Tests pour compute_metrics."""

    def test_perfect_predictions(self):
        preds = [0, 1, 2, 0, 1, 2]
        labels = [0, 1, 2, 0, 1, 2]
        result = compute_metrics(preds, labels, ["a", "b", "c"])
        assert result["accuracy"] == 1.0
        assert result["macro_f1"] == 1.0
        assert result["macro_precision"] == 1.0
        assert result["macro_recall"] == 1.0

    def test_all_wrong_predictions(self):
        preds = [1, 0, 1, 0]
        labels = [0, 1, 0, 1]
        result = compute_metrics(preds, labels, ["a", "b"])
        assert result["accuracy"] == 0.0

    def test_partial_predictions(self):
        preds = [0, 0, 1, 1]
        labels = [0, 1, 1, 0]
        result = compute_metrics(preds, labels, ["a", "b"])
        assert result["accuracy"] == 0.5

    def test_per_class_metrics(self):
        preds = [0, 0, 0, 1, 1, 1]
        labels = [0, 0, 1, 1, 1, 0]
        result = compute_metrics(preds, labels, ["a", "b"])
        assert "a" in result["per_class"]
        assert "b" in result["per_class"]
        assert "precision" in result["per_class"]["a"]
        assert "recall" in result["per_class"]["a"]
        assert "f1" in result["per_class"]["a"]
        assert "support" in result["per_class"]["a"]

    def test_single_class(self):
        preds = [0, 0, 0]
        labels = [0, 0, 0]
        result = compute_metrics(preds, labels, ["a"])
        assert result["accuracy"] == 1.0

    def test_empty_predictions(self):
        result = compute_metrics([], [], ["a", "b"])
        assert result["accuracy"] == 0.0

    def test_three_classes(self):
        preds = [0, 1, 2, 0, 1, 2, 0, 1, 2]
        labels = [0, 1, 2, 0, 1, 2, 1, 2, 0]
        result = compute_metrics(preds, labels, ["safe", "nsfw", "illicit"])
        assert 0 < result["accuracy"] < 1
        assert len(result["per_class"]) == 3


class TestBuildConfusionMatrix:
    """Tests pour build_confusion_matrix."""

    def test_perfect_diagonal(self):
        preds = [0, 1, 2, 0, 1, 2]
        labels = [0, 1, 2, 0, 1, 2]
        matrix = build_confusion_matrix(preds, labels, 3)
        assert matrix == [[2, 0, 0], [0, 2, 0], [0, 0, 2]]

    def test_all_predicted_class_0(self):
        preds = [0, 0, 0, 0]
        labels = [0, 1, 2, 0]
        matrix = build_confusion_matrix(preds, labels, 3)
        assert matrix[0][0] == 2  # TP class 0
        assert matrix[1][0] == 1  # FN class 1 -> pred 0
        assert matrix[2][0] == 1  # FN class 2 -> pred 0

    def test_empty_inputs(self):
        matrix = build_confusion_matrix([], [], 3)
        assert matrix == [[0, 0, 0], [0, 0, 0], [0, 0, 0]]

    def test_2x2_matrix(self):
        preds = [0, 1, 0, 1]
        labels = [0, 0, 1, 1]
        matrix = build_confusion_matrix(preds, labels, 2)
        assert matrix[0][0] == 1  # TN
        assert matrix[0][1] == 1  # FP
        assert matrix[1][0] == 1  # FN
        assert matrix[1][1] == 1  # TP

    def test_out_of_range_ignored(self):
        preds = [0, 5, 1]
        labels = [0, 1, 1]
        matrix = build_confusion_matrix(preds, labels, 2)
        # pred=5 should be ignored (out of range)
        assert matrix[0][0] == 1
        assert matrix[1][1] == 1
