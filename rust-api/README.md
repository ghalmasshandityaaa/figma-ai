# rust-api

Production-grade Rust API: Axum + PostgreSQL + OpenTelemetry (Tempo) + Grafana.
DDD-flavored, value-object driven, one use case per file, one action per use case.

## Layout

```
crates/
  kernel/          transport-agnostic primitives: AppError (+ codes), Clock, pagination
  domain/           pure domain: User aggregate, value objects, repository port
  application/      use cases (1 file = 1 action) + ports (PasswordHasher, TokenIssuer, UnitOfWork)
  infrastructure/   Postgres, Argon2, PASETO, OpenTelemetry adapters
  api/               axum HTTP binary — routes, handlers, DTOs, middleware
  worker/            background binary — drains the transactional outbox
  migrator/          first-party migration CLI (up / down / status)
migrations/          sqlx reversible migrations (`*.up.sql` / `*.down.sql`)
docker/              Dockerfiles, Tempo config, Grafana provisioning
```

Dependency direction is strictly inward: `api`/`worker` → `infrastructure` → `application` →
`domain` → `kernel`. `domain` and `application` never import axum/sqlx/tokio.

## Why these libraries (vs. an ORM / Tiberius / etc.)

- **`sqlx`, not an ORM (SeaORM/Diesel)**: runtime-checked async queries, real connection
  pooling, and — critically — full control of the query plan. An ORM buys faster CRUD
  scaffolding at the cost of hidden N+1s and less predictable SQL, which fights the
  "blazing fast" requirement. The repository pattern below gives most of an ORM's ergonomic
  win (no raw SQL scattered through use cases) without its query-plan opacity.
- **Not Tiberius**: Tiberius is a Microsoft SQL Server driver; this stack targets PostgreSQL, so
  it doesn't apply. `sqlx` is the async, non-blocking, connection-pooled equivalent for Postgres.
- **`sonic-rs`, not `serde_json`**: SIMD-accelerated encode/decode, serde-compatible, used for
  every request/response body via the custom `Sonic<T>` extractor.
- **PASETO (v4.local), not JWT**: version+purpose is fixed by the Rust type
  (`PasetoSymmetricKey<V4, Local>`), so there's no `alg: none` / algorithm-confusion class of
  bug the way there is with JWT. XChaCha20-Poly1305 authenticated encryption, not just a
  signature — the payload itself is opaque to the client.
- **Argon2id**: `argon2` crate's `Argon2::default()` already selects Argon2id (memory-hard,
  OWASP-recommended), so no change was needed there — confirmed, not assumed.

## Domain design

- **Value Objects** (`Email`, `Username`, `RawPassword`, `PasswordHash`, `UserId`) make illegal
  states unrepresentable — a `RegisterUser` use case cannot receive a malformed email, because
  `Email::parse` is the only way to construct one. All ids are UUIDv7 (`Uuid::now_v7()`) —
  time-ordered, index-friendly, no separate `created_at` needed for sort order.
- **1 use case = 1 action**: `RegisterUser`, `LoginUser`, `GetUserProfile`, `ListUsers` each
  expose a single `execute`. No god "UserService" with a dozen unrelated methods.
- **Repository pattern**: `domain::user::UserRepository` is a port; `PgUserRepository` is the
  only adapter. Read use cases (`LoginUser`, `GetUserProfile`, `ListUsers`) depend on
  `Arc<dyn UserRepository>` directly.
- **Unit of Work**: `application::ports::UnitOfWork` — write use cases that touch more than one
  repository call atomically depend on `Arc<dyn UnitOfWork>` instead. `RegisterUser` is the
  worked example: `uow.execute(|scope| async move { ... })` runs the uniqueness checks and the
  insert inside one transaction; `scope.users()` hands back a `UserRepository` bound to that
  transaction (`infrastructure::postgres::PgUnitOfWork` + `DbHandle::Tx`). Commit happens if the
  closure returns `Ok`, rollback otherwise — the `users_email_key`/`users_username_key` unique
  constraints remain the actual concurrency guarantee; the transaction gives atomicity, not
  serializability.
- **Transactional outbox**: `RegisterUser` writes the `users` row and a `user.registered` event
  in the same transaction, then `pg_notify`s. The `worker` binary `LISTEN`s and relays (5s poll
  fallback for missed notifications, `FOR UPDATE SKIP LOCKED` so multiple worker replicas never
  double-process).

## API contract

Every response is one shape — `ok` discriminates success from error, snake_case throughout:

```jsonc
// success
{ "ok": true, "data": { ... } }

// success, paginated (offset-based)
{
  "ok": true,
  "data": [ ... ],
  "metadata": {
    "total": 100, "page": 1, "page_size": 10, "page_count": 10,
    "page_start": 1, "page_end": 10, "page_current": 1,
    "page_prev": null, "page_next": 2
  }
}

// error — "error" is a stable Firebase-style `domain/reason` code, never a raw message
{ "ok": false, "error": "user/not-found" }

// validation error — "details" is the only place field-level info is ever exposed
{ "ok": false, "error": "validation/invalid-argument", "details": [{"field": "email", "message": "..."}] }

// unhandled panic — details is always empty; the real cause goes to `tracing` only
{ "ok": false, "error": "internal/server-error" }
```

### Endpoints

```
POST /api/v1/users                  register  { username, email, password }
GET  /api/v1/users                  list, paginated (?page=&page_size=)
GET  /api/v1/users/:id              profile
POST /api/v1/users/:id/activate     reinstate a deactivated account
POST /api/v1/users/:id/deactivate   suspend an account (blocks future logins)
POST /api/v1/auth/login             login     { email, password } -> PASETO access_token
POST /api/v1/products               create a product (bearer auth required) { name, description, price_cents, currency }
GET  /api/v1/products               list, paginated (?page=&page_size=)
GET  /api/v1/products/:id           product detail (public)
DELETE /api/v1/products/:id         delete a product (bearer auth required, owner only)
GET  /healthz                         liveness probe
```

## Logging & tracing

- **Non-blocking**: `tracing_subscriber::fmt` writes go through
  `tracing_appender::non_blocking` — a channel to a dedicated I/O thread, so stdout never
  stalls the async executor on the request path.
- **Request id**: `x-request-id` is generated (or passed through) by `SetRequestIdLayer`,
  echoed on the response by `PropagateRequestIdLayer`, and recorded as a span field.
- **Full lifecycle**: `middleware::request_logging` wraps every request in one span and logs
  `"request started"` on entry and `"request completed"` (status, latency_ms) after `next`
  returns — open/close of every request is visible in logs.
- **Correlated with Tempo**: the same span is exported via `tracing-opentelemetry`, so a
  `request_id` in a log line and a trace in Grafana → Tempo point at the same request.

## Security / resilience middleware (outside-in)

`set_request_id` → `CatchPanicLayer` → CORS → security headers → body size cap (1 MiB default)
→ per-IP rate limit (`governor`, 120 req/min + burst 20 by default) → request logging span →
compression → concurrency cap (4096 in-flight) → 15s timeout → `propagate_request_id` → router
→ `route/not-found` fallback.

- **CORS**: deny-by-default; set `CORS_ALLOWED_ORIGINS` (comma-separated) to allow specific
  origins. Methods/headers are an explicit allow-list, not `Any`.
- **Panics never crash the process**: `CatchPanicLayer` requires `panic = "unwind"` (the
  default) — the release profile deliberately does *not* set `panic = "abort"`, or panic
  recovery silently stops working. Verified: firing a panic mid-request returns a clean
  `internal/server-error` 500 and the process keeps serving.
- **Rate limiting** is keyed by `X-Forwarded-For` (falls back to the direct peer via
  `ConnectInfo`), backed by a `dashmap`-based token bucket — no lock contention on the hot path.

## Running

See [`DOCKER.md`](./DOCKER.md) for full instructions — two modes:

- **Full Docker**: `docker compose up --build` — everything containerized, zero local toolchain
  needed beyond Docker.
- **Infra-only + `cargo run`** (recommended while developing): Postgres/Tempo/Grafana in
  Docker, `api`/`worker` run natively via `cargo run`, with `cargo-watch` for hot reload
  (`nodemon`-equivalent).

Quick reference:

- API: `http://localhost:8080`
- Grafana: `http://localhost:13000` (anonymous admin; Tempo datasource + traces dashboard
  provisioned; host port remapped to avoid clashing with a local Grafana on 3000)
- Tempo API: `http://localhost:13200`, OTLP gRPC: `localhost:14317`
- Postgres: `localhost:15432`

### Migrations

```bash
docker compose --profile tools run --rm migrator status
docker compose --profile tools run --rm migrator up
docker compose --profile tools run --rm migrator down          # revert one step
docker compose --profile tools run --rm migrator down 0        # revert to a specific version
```

No `sqlx-cli` install required. `api`/`worker` also auto-run pending migrations on boot for
convenience — `migrator` is for explicit up/down/status control (rollback, unlike Prisma's
migration story, is a first-class one-liner here).

## Trying every endpoint (`xh`)

```bash
cargo install xh   # https://github.com/ducaale/xh — httpie-compatible, single binary
./examples.sh       # exercises every route: happy path + validation/conflict/auth/404 cases
```

Runs against `$BASE` (defaults to `http://localhost:8080`); each call is copy-pasteable on its
own for manual poking. Requires the API + Postgres up (either `Running` mode above) and `jq`.

## Verifying observability

Register a user, then open Grafana → Dashboards → "rust-api — Traces" (or Explore → Tempo →
TraceQL `{ resource.service.name = "rust-api" }`) to see the request span, correlated with the
`request_id` in the JSON logs.

## Known scope decision

`GET /users/:id` and `GET /users` are public reads (no bearer-auth required) — this is a
deliberate choice, not an oversight. `AuthenticatedUser`, the `FromRequestParts<AppState>`
extractor that verifies a PASETO access token into a `UserId` (see ADR-0001), exists and is
covered by unit tests; it's opt-in per handler, so a route stays public until a handler names
it in its signature.
