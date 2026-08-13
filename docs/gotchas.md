# Gotchas

- **Paywall disabled**: `can_access` short-circuits to `true` while
  `PAYWALL_ACTIVE` is `false`. Do not re-enable the `payments`/`community`
  entitlement chain without turning that flag on and testing against real
  estates.
- **JWT_SECRET must match the estate**: this service only validates tokens
  issued by `auth`. A wrong secret makes every request `UNAUTHORIZED`.
- **MONGODB_URI must include a database name**: the backend derives its DB
  from the URI path. `mongodb://localhost:27017` alone fails startup.
- **Gateway-routable file URLs**: uploads are returned as
  `{API_BASE_URL}/ecobook-files/view/{id}`. Several domains serve their own
  `/files/*`, so the shared gateway needs the domain-unique path segment to
  route correctly.
- **`.ktt` archive export/import is 501**: the routes exist and return an
  explicit `501 Not Implemented` rather than a mystery 404.
- **Frontend first iteration**: the Phaser reader is portrait fullscreen with
  no toolbar/animation. Deck content is rendered one slide per page as
  wrapped text; rich media (images, polls, flows) is not yet surfaced.
