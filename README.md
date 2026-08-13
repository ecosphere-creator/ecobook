# ecobook

A reusable **portrait book reader** LXS for eco estates. Cloned from the
`slides` domain and rebuilt around a **Phaser.js** pixel-perfect reader:
fullscreen portrait pages, warm paper background, Times New Roman at normal
size, next/prev page navigation, no toolbar, exit via the browser back
button. This first iteration has no page-turn animation.

## Structure

- `backend/` — Rust (axum) book API (`ecobook-service`): decks, catalog,
  reader sessions, files
- `frontend/` — Phaser.js portrait reader (Rust static server + `reader.html`/
  `reader.js`)
- `lxs.yml` — the LXS contract
- `scripts/` — markdown → deck JSON converter + Mongo seeder (same shape as
  slides, so decks seeded for slides work for ecobook)

## LXS

Published as `ecobook@1.0.0` to `getecosphere/lxs-registry`. Consume from an
estate:

```yaml
services:
  ecobook-backend:
    lxs: ecobook@1.0.0
    grants:
      secrets: [JWT_SECRET, MONGODB_URI]
```

Compose the reader frontend from source while in development:

```yaml
  ecobook-frontend:
    path: ecobook/frontend
    runtimes:
      - rust
```

## Build + publish

```bash
eco lxs build
eco lxs publish ecobook@1.0.0
```
