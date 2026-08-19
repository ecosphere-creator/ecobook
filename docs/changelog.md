# Changelog

## 2.0.0 (2026-08-19)
- Logging contract: service logs now emitted as newline-delimited JSON (NDJSON) to stdout per the platform LXS logging contract (`ts`/`level`/`msg` + optional `service`,`request_id`,`status`,`latency_ms`,`user_id`,`error`). Breaking change — log output format changed.

## 1.0.0

- Cloned from `slides` and renamed to `ecobook`.
- Added the Phaser.js portrait reader frontend (`frontend/`): fullscreen
  portrait pages, paper background, Times New Roman, normal size, next/prev
  navigation, no toolbar, exit via browser back button, no page-turn
  animation in this first iteration.
- Paywall disabled (`PAYWALL_ACTIVE=false`): all decks fully public.

### Breaking changes

- New LXS identity `ecobook` (distinct from `slides`); estate references
  should use `lxs: ecobook@1.0.0`.
