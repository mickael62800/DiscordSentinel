"""
Sentinel AI Training API — FastAPI
Pilote l'entrainement des modeles text et vision depuis l'app desktop.
"""

import logging
import os
from pathlib import Path
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from dotenv import load_dotenv

from constants import ALLOWED_ORIGINS

load_dotenv()

AI_ROOT = Path(__file__).resolve().parent.parent

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("sentinel.ai")

from routes import datasets, training, export


@asynccontextmanager
async def lifespan(app: FastAPI):
    logger.info("Sentinel AI API demarree")
    yield
    logger.info("Sentinel AI API arretee")


app = FastAPI(
    title="Sentinel AI Training API",
    version="0.2.0",
    description="API pour l'entrainement et l'export des modeles IA de DiscordSentinel",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=ALLOWED_ORIGINS,
    allow_methods=["GET", "POST", "DELETE"],
    allow_headers=["*"],
)

app.include_router(datasets.router, prefix="/api/ai", tags=["Datasets"])
app.include_router(training.router, prefix="/api/ai", tags=["Training"])
app.include_router(export.router, prefix="/api/ai", tags=["Export"])


@app.get("/health")
async def health() -> dict[str, str]:
    """Verifie que l'API est en ligne."""
    return {"status": "ok", "service": "sentinel-ai"}


if __name__ == "__main__":
    import uvicorn

    port = int(os.getenv("AI_API_PORT", "8000"))
    uvicorn.run("main:app", host="0.0.0.0", port=port)
