# ecobook

A reusable **portrait book reader** LXS. Cloned from the `slides` domain and
rebuilt around a Phaser.js pixel-perfect reader: fullscreen portrait pages,
paper background, Times New Roman at normal size, next/prev page navigation,
no toolbar, exit via the browser back button. This first iteration skips the
page-turn animation.

Backend is the same Rust (axum) book API as `slides` (decks, catalog, reader
sessions); the frontend is a self-contained Phaser portrait reader.

## LXS: reusable domain

`ecobook` is packaged as a reusable **LXS** (`ecobook@1.0.0`), published to
`getecosphere/lxs-registry` under Eco Creator, and consumable from any estate
via `ecompose.yml` → `services.<name>: { lxs: ecobook@1.0.0, grants: {...} }`.
It ships both a backend (`backend/`, Rust axum) and a Phaser frontend
(`frontend/`); it can also be composed from source (`path: ecobook/backend`)
while in development.

**Paywall disabled** (2026-08-12): `can_access` in
`backend/src/handlers/slide_decks.rs` short-circuits to `true` because the
`PAYWALL_ACTIVE` const is `false`. All published decks are fully public so the
LXS can be plugged into any estate (e.g. the getecosphere homepage) with
example content. The `payments`/`community` access chain is retained and
type-checked; flip `PAYWALL_ACTIVE` to `true` when the paywall ships.

## Backend notes

Most of the backend still reads as "slides" (crate `ecobook-service`, routes
`/api/book/*`) because it was cloned from `slides`. The frontend is the new
Phaser reader. Decks are authored in 16:9 presentation format and imported
through the portable markdown document format (see below); the responsive
`/ecobook` portrait reader adapts that presentation into flowing pages.

## Status

Mostly fully ported and verified. Source: `SlideDeckController`/
`SlideDeckService`, `SlideEditorPrefsController`/`SlideEditorPrefsService`,
`AuthorImageAssetController`/`AuthorImageAssetService`,
`AuthorPollTemplateController`/`AuthorPollTemplateService`,
`PollVoteController`/`PollVoteService`, `BookSessionController`/
`BookSessionService` from `lms-backend`.

## Deck document import/export (replaces the Java `.ktt` archive)

`POST /book/import` and `GET /book/{id}/export` implement a **portable
markdown deck-document format** — the modern replacement for the Java
version's bespoke `.ktt` ZIP archive (deliberately not ported as-is). A deck
document is one markdown file: YAML frontmatter (deck metadata + theme tokens)
followed by a 16:9 slide body.

```markdown
---
deck:
  name: "Ecosphere — The Software Composition Platform"
  slug: eco-investor-pitch
  subtitle: "The Software Composition Platform for the AI era"
  level: "Intermediate"
  language: "en"
  instructorName: "Ecosphere"
  tags: [pitch, ecosphere, investor]
  status: draft
theme:
  base: light
  bg: "#f7f6f2"
  ink: "#17141d"
  accent: "#5b3fd6"
  surface: "#ffffff"
  fontDisplay: "Manrope, ui-sans-serif, system-ui, sans-serif"
  fontMono: "DM Mono, ui-monospace, monospace"
---

# The next wave of lean infrastructure

A paragraph becomes a body element.

> A quote or key stat becomes a callout element.

```text
code / pipeline becomes a code element
```
```

Mapping rules: `#` starts a new slide (H1 text = slide title element),
`##`/`###` become level-2/3 sub-titles, paragraphs become body elements,
`>` blockquotes become callouts, fenced code blocks become code elements, and
a `---` rule is an explicit slide break. Elements are auto-stacked on the
1920x1080 canvas so they never overlap.

- `POST /api/book/import` — body is the document (`text/markdown`); parsed
  into a `SlideDeckInput` and persisted exactly like `POST /book` (same auth,
  slug resolution, publish validation). Returns the created deck.
- `GET /api/book/:id/export` — owner or platform `OWNER`; returns the deck as
  a downloadable `.md` document (the inverse of import).

The deck's theme tokens are stored as a JSON blob in `deck.theme`; the Phaser
reader applies them as its default palette ("✎ deck" button in the control
bar). Omitting the `theme:` block falls back to the getecosphere defaults.

## Dependencies

- `auth` — `AuthClient::is_platform_owner` resolves whether an arbitrary
  user id has the `owner` role, for the `isEditor`/`canAccess` checks
  ported from `SlideDeckService`.
- `payments` — `PaymentsClient::has_completed_slide_payment` calls
  `GET /payments/access/slide-deck/{deckId}/user/{userId}` (added to
  `payments` alongside this port) to check paid access without
  duplicating payment records locally.
- `community` — `CommunityClient::has_active_registration` calls
  `GET /events/{eventId}/registration/{userId}` (added to `community`
  alongside this port) to check event-linked access without duplicating
  registration records locally.

All three are plain HTTP calls with no end-user auth forwarded -- same
trust model used throughout this port (`courses_client` in `payments`,
`AuthClient` in `community`/`content`/`site`): assumed to run within the
estate's private network, not exposed to the public frontend routing.

## Modeling `SlideDeck`

The Java model is a deeply nested authoring-tool document (`Slide` ->
`Element`, `Flow` -> `FlowEdge`, `GuidedReveal` -> `RevealCue`). Most of it
is typed exactly like the Java version (`Slide`, `Element`, `Flow`, etc, in
`src/models/slide_deck.rs`) because business logic actually reaches into
it: curriculum-title extraction reads `element.type`/`element.content`,
image/poll-template usage scans read `element.type`/`element.content`/
`element.style`. Only the genuinely free-form leaves stay
`serde_json::Value` passthrough (`Slide.rpp`, `Flow.condition`,
`Flow.nodePositions`, `Flow.editorState`).

`SlideDeck` itself needed a real DTO split (`SlideDeckDto` for responses,
`SlideDeckInput` for requests) rather than reusing the model directly the
way courses/community did for simpler shapes — Slide/Element/Flow/etc
carry no datetime fields (their ids are plain client-generated strings,
not Mongo ObjectIds) so those are shared as-is, but the outer struct's
`id`/`createdAt`/`updatedAt` do need the bson -> String/chrono conversion.
Skipping that for the smaller sub-resources (`SlideEditorPrefs`,
`AuthorImageAsset`, `AuthorPollTemplate`, `BookSession`) was an actual bug
caught during integration testing, not just a style choice: returning
those models straight through `axum::Json` serialized `bson::DateTime`/
`ObjectId` fields as BSON Extended JSON (`{"$date": {"$numberLong":
"..."}}`, `{"$oid": "..."}`) instead of plain JSON -- the exact failure
mode documented in auth's original bson/chrono write-up. Fixed by adding
`SlideEditorPrefsDto`/`AuthorImageAssetDto`/`AuthorPollTemplateDto`/
`BookSessionDto` in `src/dto.rs` and converting at every response
boundary.

## Security fixes made during the port

- **`createSlideDeck` took the entire request body verbatim, including
  `ownerId`.** A client could set someone else's id (or omit it) as the
  new deck's owner. Fixed: always the authenticated caller.
- **`GET /book/owner/{ownerId}` had no auth requirement at all**
  (`SecurityConfig` blanket-permitted every `GET /book/**`) **and returned
  full, un-redacted `SlideDeck` documents** — including unpublished
  drafts and paywalled slide content — **for any owner id, to anyone.**
  This was the most serious finding in the whole port: an anonymous
  caller could dump any author's entire draft/paid content library for
  free by guessing or observing a user id. Fixed: caller must be that
  owner or a platform `OWNER` (the same `isEditor` check `update`/
  `delete` already used).

Everything else in `SlideDeckService` (`updateSlideDeck`'s ownership
check, `canAccess`'s owner/platform-owner/paid/event-registered chain,
the paywall preview redaction, slug immutability once published) already
had correct logic in Java and was carried over as-is — including the
`layoutFormat` field on `SlideDeck` that exists in the model but has no
dedicated handling anywhere in the Java service either (frontend-only
concern, stored and echoed back unchanged).

## Gateway-routable file URLs

Slide-image/slide-cover/narration uploads are returned as
`{API_BASE_URL}/slides-files/view/{id}` (also registered as second routes
alongside the original bare `/files/*`), not the bare path. Production
puts every domain behind one shared gateway origin, and several domains
each implement their own `/files/*` — the gateway can't tell them apart
without a domain-unique path segment. See `eco configure`'s
`generate_gateway_config` and `courses/README.md`'s "File storage" section
for the fuller explanation.

## Observability (added 2026-07-09)

Logs are structured JSON (`tracing_subscriber::fmt().json()`), not the
default human-readable text — prep for centralized log aggregation
(Grafana Loki is the leading candidate, self-hosted alongside the rest of
the estate rather than a SaaS product, in keeping with `eco`'s host-native
philosophy).

Every request gets a correlation id (`src/request_id.rs`): reused from an
incoming `x-request-id` header if present, otherwise a fresh UUID,
recorded on the request's tracing span (so every JSON log line during
that request carries it) and echoed back on the response. All three peer
clients (`AuthClient::is_platform_owner`, `PaymentsClient::
has_completed_slide_payment`, `CommunityClient::has_active_registration`)
now forward this same header on their outbound calls, so a single
`canAccess` check that fans out to all three peers is fully
reconstructable as one trail under a single `request_id`, once logs are
aggregated somewhere queryable — the domain where this matters most,
given it's the one with the widest fan-out.

## Verified

Built, ran against live `auth`, `payments`, and `community` instances and
local MongoDB: `ownerId` spoofing on deck creation correctly ignored,
paywall slide redaction for both an authenticated non-entitled user and a
fully anonymous request, the `GET /book/owner/{id}` fix (a second user and
an anonymous caller both correctly rejected; the real owner sees full
unredacted content), publish-field validation, the public catalog and
curriculum-with-locking endpoints, cross-service paid-access unlocking a
previously-locked deck end-to-end through the real `payments` service,
editor prefs with padding clamping, poll vote submission and anonymous
aggregation, anonymous book sessions, slide image upload with WebP
conversion and view, and ownership-checked deck deletion. Also caught and
fixed a `camelCase` deserialization bug in `author-assets/images`'s add
endpoint during testing (`fileUrl` silently defaulted to empty because the
request struct was missing `#[serde(rename_all = "camelCase")]`) — worth
flagging since it's an easy mistake to repeat in future request DTOs on
this domain.
