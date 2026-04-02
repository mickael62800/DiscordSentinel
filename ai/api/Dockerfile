FROM python:3.12-slim AS base

WORKDIR /app

# Dependances systeme pour PyTorch + Pillow
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libgomp1 \
        libjpeg62-turbo \
        libpng16-16 \
        libwebp7 && \
    rm -rf /var/lib/apt/lists/*

# Installer les dependances Python
COPY ai/api/requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Copier le code
COPY ai/api /app/api
COPY ai/shared /app/shared

WORKDIR /app/api

ENV AI_API_PORT=8000
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health')"

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
