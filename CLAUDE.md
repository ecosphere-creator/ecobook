# slides

Slide decks ("books"), editor prefs, author image/poll-template libraries,
poll votes, and reader sessions. Split out of `rwid/lms` (the pre-eco
monolith) into an independent eco domain, rewritten in Rust (axum). Ported
last of the seven domains, since it's the only one that depends on both
`payments` and `community`. See
`rwid/auth/backend/docs/auth-rewrote-from-java-to-rust.md` for the origin
of this pattern.

## Status

Mostly fully ported and verified. Source: `SlideDeckController`/
`SlideDeckService`, `SlideEditorPrefsController`/`SlideEditorPrefsService`,
`AuthorImageAssetController`/`AuthorImageAssetService`,
`AuthorPollTemplateController`/`AuthorPollTemplateService`,
`PollVoteController`/`PollVoteService`, `BookSessionController`/
`BookSessionService` from `lms-backend`. One deliberate scope cut: see
below.

## Deliberately not ported: `.ktt` archive export/import

`GET /book/{id}/export` and `POST /book/import` return `501 Not
Implemented`. The Java version's `KttArchiveService` (336 lines) packages
a deck plus its referenced images/audio into a bespoke ZIP archive and
reverses it on import. This is a real, self-contained feature but not
security-relevant and not part of the everyday read/write path — porting
it faithfully (ZIP writing/reading, manifest format, referenced-asset
extraction and repacking) was judged lower value than the ~15 other
correctness and security fixes made across this domain, given the size of
everything else that needed porting first. Routes are wired to explicit
501s rather than silently dropped, so this is visible instead of a mystery
404. If this feature turns out to matter, port `KttArchiveService` next.

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
`generate_gateway_config` and `courses/CLAUDE.md`'s "File storage" section
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
