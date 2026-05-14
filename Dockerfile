# syntax=docker/dockerfile:1.7
# Image for the Python crawler (sunsetRollercoaster/main.py).

FROM python:3.12-slim-bookworm AS base

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    UV_LINK_MODE=copy \
    UV_PROJECT_ENVIRONMENT=/opt/venv \
    PATH="/opt/venv/bin:$PATH"

COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

WORKDIR /app

# Install deps first for cache-friendliness.
COPY pyproject.toml uv.lock ./
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-install-project --no-dev

# Then copy source and install the project itself.
COPY sunsetRollercoaster ./sunsetRollercoaster
COPY alembic ./alembic
COPY alembic.ini main.py ./
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-dev

# env_config.yml is mounted at runtime.
CMD ["python", "main.py"]
