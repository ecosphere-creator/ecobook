# ecobook

A reusable **portrait book reader** LXS. Cloned from the `slides` domain and
rebuilt around a **Phaser.js** pixel-perfect reader.

- `backend/` — Rust (axum) book API (`ecobook-service`)
- `frontend/` — Phaser.js portrait reader (static server + `reader.html`/`reader.js`)
- `lxs.yml` — the LXS contract

## What this LXS owns

- Slide decks ("books") — the document model, catalog, publish lifecycle,
  paywall metadata (currently disabled)
- Reader sessions and poll votes for decks
- Author image / poll-template libraries

## What this LXS must NEVER own

- Identity/session concerns — delegated to the `auth` domain
- Payments/entitlement — delegated to `payments` / `community`
- Object storage — delegated to the estate `storage` domain

## Contracts (public API)

See `docs/api.md` for the full endpoint reference.

## Composition

```yaml
services:
  ecobook-backend:
    lxs: ecobook@1.0.0
    grants:
      secrets: [JWT_SECRET, MONGODB_URI]
  ecobook-frontend:
    path: ecobook/frontend
    runtimes:
      - rust
```

## Runtime

- Rust (axum), self-contained static binary (musl)
- MongoDB `@7`

## Environment variables

- `SERVER_PORT` — listen port
- `MONGODB_URI` — MongoDB connection string (must include a database name)
- `JWT_SECRET` — shared HS512 estate secret (validate only, never issue)
- `AUTH_BASE_URL` / `PAYMENTS_BASE_URL` / `COMMUNITY_BASE_URL` — peer APIs
- `STORAGE_BACKEND` — `local` (default) or `s3`
- `API_BASE_URL` — for gateway-routable file URLs
- `CORS_ALLOWED_ORIGINS` — comma-separated
