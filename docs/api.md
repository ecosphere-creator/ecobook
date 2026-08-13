# API

Base path: `/api` (estate gateway routes `/api/book/*` to `ecobook-backend`).

## Health

- `GET /health` — liveness (`{"status":"UP"}`)

## Decks ("books")

- `POST /book` — create a deck. Auth required. Body is `SlideDeckInput`;
  `ownerId` is always the authenticated caller.
- `GET /book/catalog` — published decks for the marketplace/catalog.
- `GET /book/owner/:ownerId` — a user's decks (owner or platform OWNER).
- `GET /book/event/:eventId` — decks linked to an event.
- `GET /book/:id` — full deck (all slides). Auth optional; paywall disabled
  so full content is returned to everyone.
- `GET /book/:id_or_slug/public` — public detail (catalog metadata +
  curriculum).
- `PUT /book/:id` — update. Owner or platform OWNER.
- `DELETE /book/:id` — delete. Owner or platform OWNER.
- `GET /book/:id/export` / `POST /book/import` — `501 Not Implemented`
  (the `.ktt` archive port was deliberately cut).

## Editor prefs

- `GET/PUT /book/:deckId/editor-prefs`

## Sessions

- `GET/POST /book/:deckId/session`
- `POST /book/:deckId/session/merge`

## Poll votes

- `GET/POST /book/:deckId/slides/:slideId/elements/:elementId/poll-votes`

## Author assets

- `GET/POST/DELETE /author-assets/images`, `GET /author-assets/images/usage`
- `GET/POST /author-poll-templates`, `PUT/DELETE /author-poll-templates/:templateId`,
  `GET /author-poll-templates/:templateId/usage`, `POST /author-poll-templates/:templateId/sync`

## Files

- `POST /files/upload`, `GET /files/view/:fileId`, `DELETE /files/:fileId`
- Also reachable under `/api/ecobook-files/*` for gateway routing
  (same handlers).

## Errors

Uniform JSON envelope:

```json
{ "code": "RESOURCE_NOT_FOUND", "message": "...", "timestamp": "..." }
```

Common codes: `RESOURCE_NOT_FOUND`, `FORBIDDEN`, `BAD_REQUEST`,
`UNAUTHORIZED`. Unexpected errors return `INTERNAL_SERVER_ERROR`.
