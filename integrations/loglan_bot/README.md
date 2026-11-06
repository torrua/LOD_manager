# LOD Manager & Loglan Bot Backend Integration

This directory contains integration templates and deployment configurations connecting **LOD Manager** (Telegram Mini App & Web frontend) to the official **Loglan Bot** backend service.

---

## 1. Upstream Backend Repository

The official, canonical backend and Telegram Bot is maintained in its dedicated repository:
👉 **[torrua/loglan_bot](https://github.com/torrua/loglan_bot)**

All API endpoints (`/api/v1/*`), database models (`loglan_core`), Telegram webhook handlers, authentication, and test suites are developed and maintained in that repository.

To avoid code drift and dual maintenance, Python API sources are not duplicated here.

---

## 2. Architecture Overview

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
                      │   dist/ HTML   │     │   loglan_bot   │
                      │  Svelte 5 App  │     │  (Quart ASGI)  │
                      └────────────────┘     └────────┬───────┘
                                                      │
                                             ┌────────▼───────┐
                                             │   PostgreSQL   │
                                             │   / SQLite     │
                                             └────────────────┘
```

---

## 3. Deployment with Docker Compose

You can easily run the backend alongside the frontend using the provided `docker-compose.yml`:

```bash
cd integrations/loglan_bot
docker compose up -d
```

### Environment Variables

| Variable           | Description                                  | Default                                                                |
| :----------------- | :------------------------------------------- | :--------------------------------------------------------------------- |
| `BOT_TOKEN`        | Telegram Bot Token from `@BotFather`         | Required for TMA authentication                                        |
| `ADMIN_IDS`        | Comma-separated list of admin Telegram IDs   | Admin permissions                                                      |
| `CORS_ORIGINS`     | Allowed origins for web/TMA access           | `https://torrua.github.io,http://localhost:5173`                       |
| `LOD_DATABASE_URL` | Async PostgreSQL or SQLite connection string | `postgresql+asyncpg://...` or `sqlite+aiosqlite:///app/data/export.db` |

---

## 4. Telegram Bot & Mini App Configuration

1. In [@BotFather](https://t.me/BotFather), configure the bot's Menu Button or Web App Link to point to your frontend URL (e.g. `https://torrua.github.io/LOD_manager/`).
2. The Mini App automatically sends Telegram `initData` in HTTP headers (`X-Telegram-Init-Data` and `Authorization: tma <initData>`) to the backend.
3. The backend validates the cryptographic signature using `BOT_TOKEN` and handles all dictionary queries.
