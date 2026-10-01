# LOD Manager Telegram Mini App & Web API Backend

This integration connects **LOD Manager** to the Python Quart **Loglan Bot** backend running on your server.

It exposes a RESTful API (`/api/v1`) that implements the `DataAdapter` contract required by LOD Manager, handles Telegram WebApp `initData` HMAC-SHA256 signature verification, and allows dictionary search and management both in Telegram and in standard web browsers.

---

## 1. Architecture Overview

```
                      ┌──────────────────────────────────────┐
                      │    Telegram Client / Web Browser    │
                      └──────────────────┬───────────────────┘
                                         │
                   HTTPS requests to https://<your-backend-domain>
                                         │
                      ┌──────────────────▼───────────────────┐
                      │       Dokploy Traefik / Caddy        │
                      │   (Automatic SSL via Let's Encrypt)   │
                      └────────┬──────────────────────┬──────┘
                               │                      │
                  / (Static Frontend)        /api/v1 (REST API)
                               │                      │
                      ┌────────▼───────┐     ┌────────▼───────┐
                      │   dist/ HTML   │     │  Quart ASGI    │
                      │  Svelte 5 App  │     │   (Python)     │
                      └────────────────┘     └────────┬───────┘
                                                      │
                                             ┌────────▼───────┐
                                             │   export.db    │
                                             │ (SQLite + FTS) │
                                             └────────────────┘
```

---

## 2. Directory Structure

```
integrations/loglan_bot/
├── README.md                 # Deployment & Dokploy integration guide
└── app/
    └── api/
        ├── __init__.py       # Quart Blueprint definition ('api_v1')
        ├── auth.py           # Telegram initData HMAC-SHA256 authentication
        ├── serializers.py    # DTO serializers matching LOD TypeScript types
        └── routes.py         # REST endpoints (/words, /search, /stats, etc.)
```

---

## 3. Registering the API Blueprint in Quart

In your Quart application entrypoint (e.g. `app/__init__.py` or `main.py`):

```python
from quart import Quart
from quart_cors import cors
from app.api import api_bp

app = Quart(__name__)
cors_origins = [o.strip() for o in os.environ.get("CORS_ORIGINS", "https://torrua.github.io,http://localhost:5173").split(",") if o.strip()]
app = cors(app, allow_origin=cors_origins)

# Configuration
app.config["BOT_TOKEN"] = os.environ.get("BOT_TOKEN")
app.config["ADMIN_IDS"] = os.environ.get("ADMIN_IDS", "")  # e.g. "1234567,987654"
app.config["DATABASE_PATH"] = os.environ.get("DATABASE_PATH", "/app/data/export.db")

# Register REST API
app.register_blueprint(api_bp)
```

---

## 4. Serving the Svelte 5 Frontend

### Option A: Static Files via Quart

Copy the built frontend bundle (`dist/`) into your Quart app's `static/` directory:

```python
from quart import send_from_directory

@app.route("/")
@app.route("/<path:path>")
async def serve_spa(path=""):
    if path and os.path.exists(os.path.join("dist", path)):
        return await send_from_directory("dist", path)
    return await send_from_directory("dist", "index.html")
```

### Option B: High-Performance Multi-Stage Dockerfile (Recommended for Dokploy)

```dockerfile
# Stage 1: Build Svelte 5 Frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Stage 2: Python Quart Backend
FROM python:3.11-slim
WORKDIR /app

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt hypercorn

# Copy backend code
COPY . .

# Copy built frontend assets
COPY --from=frontend-builder /app/dist /app/dist

EXPOSE 8000

CMD ["hypercorn", "main:app", "--bind", "0.0.0.0:8000"]
```

---

## 5. Dokploy Deployment Instructions

1. **Create Project / Application in Dokploy**:
   - In the Dokploy Dashboard, create a new Application.
   - Set Build Type to **Dockerfile** (or **Docker Compose**).
   - Set Repository to your Git repository and target branch.

2. **Configure Domain & SSL**:
   - Domain: `your-api-domain.com`
   - Port: `8000` (or `80` if using reverse proxy)
   - Enable **HTTPS / SSL (Let's Encrypt)**.

3. **Set Environment Variables**:

   | Variable        | Value / Description                                                          |
   | --------------- | ---------------------------------------------------------------------------- |
   | `BOT_TOKEN`     | Telegram Bot Token from `@BotFather`                                         |
   | `ADMIN_IDS`     | Comma-separated list of admin Telegram IDs (e.g. `1234567,9876543`)          |
   | `CORS_ORIGINS`  | Allowed web origins (e.g. `https://torrua.github.io,http://localhost:5173`)  |
   | `DATABASE_PATH` | Path to `export.db` or database credentials                                  |
   | `VITE_API_URL`  | Optional if served on same domain; otherwise `https://<your-api-domain.com>` |

4. **Persistent Storage / Volume**:
   - Mount persistent volume to `/app/data` to store `export.db` (if using SQLite).

5. **Deploy**:
   - Click **Deploy**. Dokploy will pull the code, build the container, issue SSL certificates, and start the service.

---

## 6. Configuring Telegram BotFather for TMA

1. Open [@BotFather](https://t.me/BotFather) in Telegram.
2. Send `/mybots` and select your Loglan Bot.
3. Configure **Bot Settings → Menu Button**:
   - Choose **Configure menu button**.
   - Enter URL: `https://torrua.github.io/LOD_manager/` (or your hosted frontend URL)
   - Enter Button Text: `LOD Dictionary`
4. Configure **Web App Direct Link (Mini App)**:
   - Send `/newapp` to BotFather.
   - Select your bot.
   - Provide Title: `LOD Manager`
   - Provide Description: `Loglan Online Dictionary Browser`
   - Set WebApp URL: `https://torrua.github.io/LOD_manager/` (or your hosted frontend URL)
   - Set Short Name: e.g. `lod`
5. Test deep links:
   - Opening `https://t.me/your_bot/lod?startapp=w_42` opens word ID `42` directly in the Mini App!
