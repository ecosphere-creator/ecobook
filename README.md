# slides

Slide deck / presentation editor, polls, image assets, archive export

Split out of `rwid/lms` (the original pre-eco monolith) as an independent,
eco-managed domain — see `rwid/auth`'s
`backend/docs/auth-rewrote-from-java-to-rust.md` for the reasoning and
pattern this split follows (explicit dependencies instead of direct
cross-domain database access, security hardening baseline, etc.).

## Status

Scaffold only. Boots, connects to MongoDB, validates JWTs issued by `auth`,
has the estate's standard security baseline (rate limiting, security
headers, body size limits, CORS) wired up. The actual domain logic has not
been ported from `lms-backend` yet.

## Split from (lms-backend)

SlideDeckController/SlideDeckService, SlideEditorPrefsController/SlideEditorPrefsService, AuthorImageAssetController/AuthorImageAssetService, AuthorPollTemplateController/AuthorPollTemplateService, PollVoteController/PollVoteService, KttArchiveService

## Depends on

auth, community, payments

## Structure

- `backend/` — Rust (axum) service
