"""Tests pour les endpoints de l'API ML."""

import pytest


class TestHealthEndpoint:
    """Tests pour /health."""

    def test_health_returns_ok(self, client):
        response = client.get("/health")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert data["service"] == "sentinel-ai"


class TestDatasetsEndpoints:
    """Tests pour /api/ai/datasets."""

    def test_list_datasets(self, client):
        response = client.get("/api/ai/datasets")
        assert response.status_code == 200
        data = response.json()
        assert isinstance(data, list)
        assert len(data) == 2
        types = {d["model_type"] for d in data}
        assert "text-sentiment" in types
        assert "image-classification" in types

    def test_list_datasets_has_required_fields(self, client):
        response = client.get("/api/ai/datasets")
        data = response.json()
        for dataset in data:
            assert "model_type" in dataset
            assert "total_samples" in dataset
            assert "label_distribution" in dataset
            assert "last_updated" in dataset

    def test_upload_invalid_model_type(self, client):
        response = client.post(
            "/api/ai/datasets/invalid-type/upload",
            files={"file": ("test.txt", b"content", "text/plain")},
        )
        assert response.status_code == 422

    def test_upload_empty_filename(self, client):
        response = client.post(
            "/api/ai/datasets/text-sentiment/upload",
            files={"file": ("", b"content", "text/plain")},
        )
        # FastAPI retourne 422 pour les fichiers sans nom valide
        assert response.status_code in (400, 422)

    def test_upload_dotfile_rejected(self, client):
        response = client.post(
            "/api/ai/datasets/text-sentiment/upload",
            files={"file": (".hidden", b"content", "text/plain")},
        )
        assert response.status_code == 400

    def test_clear_invalid_model_type(self, client):
        response = client.delete("/api/ai/datasets/invalid-type")
        assert response.status_code == 422


class TestTrainingEndpoints:
    """Tests pour /api/ai/training."""

    def test_status_when_idle(self, client):
        response = client.get("/api/ai/training/status")
        assert response.status_code == 200
        data = response.json()
        assert data["running"] is False
        assert data["phase"] == "idle" or data["phase"] == "termine" or "erreur" in data["phase"] or data["phase"] == "arrete"

    def test_status_has_all_fields(self, client):
        response = client.get("/api/ai/training/status")
        data = response.json()
        expected_fields = [
            "running", "model_type", "current_epoch", "total_epochs",
            "loss", "accuracy", "val_loss", "val_accuracy", "phase",
            "epoch_history", "current_batch", "total_batches",
            "batch_loss", "batch_accuracy", "early_stopped",
            "best_epoch", "final_metrics",
        ]
        for field in expected_fields:
            assert field in data, f"Champ manquant: {field}"

    def test_start_invalid_model_type(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "invalid-type",
        })
        assert response.status_code == 422

    def test_start_invalid_epochs_zero(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "epochs": 0,
        })
        assert response.status_code == 422

    def test_start_invalid_epochs_negative(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "epochs": -5,
        })
        assert response.status_code == 422

    def test_start_invalid_epochs_too_large(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "epochs": 999,
        })
        assert response.status_code == 422

    def test_start_invalid_batch_size_zero(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "batch_size": 0,
        })
        assert response.status_code == 422

    def test_start_invalid_batch_size_too_large(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "batch_size": 512,
        })
        assert response.status_code == 422

    def test_start_invalid_learning_rate_zero(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "learning_rate": 0,
        })
        assert response.status_code == 422

    def test_start_invalid_learning_rate_too_large(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "learning_rate": 5.0,
        })
        assert response.status_code == 422

    def test_start_invalid_validation_split(self, client):
        response = client.post("/api/ai/training/start", json={
            "model_type": "text-sentiment",
            "validation_split": 1.0,
        })
        assert response.status_code == 422

    def test_stop_when_not_running(self, client):
        # Reset state
        from routes.training import state
        state.running = False
        response = client.post("/api/ai/training/stop")
        assert response.status_code == 400


class TestExportEndpoints:
    """Tests pour /api/ai/export."""

    def test_export_invalid_model_type(self, client):
        response = client.post("/api/ai/export/invalid-type")
        assert response.status_code == 422

    def test_list_models(self, client):
        response = client.get("/api/ai/models")
        assert response.status_code == 200
        assert isinstance(response.json(), list)

    def test_export_text_no_checkpoint(self, client):
        response = client.post("/api/ai/export/text-sentiment")
        # 404 si pas de checkpoint, 500 si module onnx pas installe (CI)
        assert response.status_code in (404, 500)

    def test_export_vision_no_checkpoint(self, client):
        response = client.post("/api/ai/export/image-classification")
        assert response.status_code in (404, 500)
