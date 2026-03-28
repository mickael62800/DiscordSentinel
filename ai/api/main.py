"""
Sentinel AI Training API — FastAPI
Pilote l'entrainement des modeles text et vision depuis l'app desktop.
"""

import os
from pathlib import Path
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from dotenv import load_dotenv

load_dotenv()

AI_ROOT = Path(__file__).resolve().parent.parent

from routes import datasets, training, export


@asynccontextmanager
async def lifespan(app: FastAPI):
    print("Sentinel AI API demarree")
    yield
    print("Sentinel AI API arretee")


app = FastAPI(
    title="Sentinel AI Training API",
    version="0.1.0",
    description="API pour l'entrainement et l'export des modeles IA de DiscordSentinel",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(datasets.router, prefix="/api/ai", tags=["Datasets"])
app.include_router(training.router, prefix="/api/ai", tags=["Training"])
app.include_router(export.router, prefix="/api/ai", tags=["Export"])


@app.get("/health")
async def health():
    return {"status": "ok", "service": "sentinel-ai"}


if __name__ == "__main__":
    import uvicorn

    port = int(os.getenv("AI_API_PORT", "8000"))
    uvicorn.run("main:app", host="0.0.0.0", port=port)
