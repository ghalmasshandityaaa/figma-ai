# Docker Usage

Two ways to run this stack. Pick based on what you're doing.

| Mode | When |
|---|---|
| [Full Docker](#full-docker-everything-containerized) | Just want it running, or testing the built container images |
| [Infra-only + `cargo run`](#infra-only--cargo-run-recommended-for-development) | Actively writing code — native binary, full debugger support, hot reload |

---

## Full Docker (everything containerized)

```bash
cp .env.example .env   # only read by `cargo run`; compose sets its own env — see below
docker compose up --build
```

Starts `postgres`, `tempo`, `grafana`, `api`, `worker`. Migrations run automatically on `api`
boot.

| Service | URL |
|---|---|
| API | http://localhost:8080 |
| Grafana | http://localhost:13000 (anonymous admin; Tempo datasource + dashboard provisioned) |
| Tempo query API | http://localhost:13200 |
| Postgres | `localhost:15432` (user/pass/db: `app`/`app`/`app`) |

Useful commands:

```bash
docker compose logs -f api worker      # follow logs
docker compose ps                      # container status
docker compose restart api             # restart one service after an image rebuild
docker compose down                    # stop + remove containers (keeps volumes)
docker compose down -v                 # also wipe postgres/tempo data
```

Rebuild after a code change (full Docker mode has **no** hot reload — it's a static release
binary):

```bash
docker compose build api worker
docker compose up -d api worker
```

Migrations (up/down/status) always go through the `migrator` one-off container, in either mode:

```bash
docker compose --profile tools run --rm migrator status
docker compose --profile tools run --rm migrator up
docker compose --profile tools run --rm migrator down        # revert one step
docker compose --profile tools run --rm migrator down 0      # revert to a specific version
```

---

## Infra-only + `cargo run` (recommended for development)

Run **only** the infrastructure (Postgres, Tempo, Grafana) in Docker; run `api`/`worker` as a
native `cargo` process on your machine. Faster iteration, real debugger/breakpoint support,
no image rebuild per change.

### 1. Start infra

```bash
docker compose up -d postgres tempo grafana
```

### 2. Configure environment

```bash
cp .env.example .env
```

`.env.example` already points at the **host-mapped** ports (`localhost:15432` for Postgres,
`localhost:14317` for Tempo's OTLP endpoint) — not the in-network `postgres:5432` /
`tempo:4317` names `docker-compose.yml` uses between containers. That's the only difference
between the two modes; nothing else needs to change.

`api` and `worker` call `dotenvy::dotenv()` before reading config, so `cargo run` picks up
`.env` automatically. It's a no-op if `.env` doesn't exist (e.g. inside the container images —
those get real env vars from `docker-compose.yml` instead).

### 3. Run

```bash
cargo run -p api
# in another terminal
cargo run -p worker
```

`api`/`worker` auto-run pending migrations on boot, so step 1 alone is enough — no need to run
`migrator` separately for local dev (though you still can, e.g. to test a rollback).

Verify:

```bash
curl http://localhost:8080/healthz
```

### 4. Hot reload (equivalent of `nodemon`)

Rust has no built-in watch mode, but [`cargo-watch`](https://crates.io/crates/cargo-watch)
does the same job: rebuild + restart on file save.

```bash
cargo install cargo-watch   # one-time

cargo watch -x 'run -p api'
# in another terminal
cargo watch -x 'run -p worker'
```

`-c` clears the terminal between runs if the scrollback bothers you:

```bash
cargo watch -c -x 'run -p api'
```

By default `cargo-watch` watches the whole workspace, so editing `domain`/`application`/
`infrastructure` also triggers a rebuild of `api` — same experience as `nodemon` watching your
whole source tree. Incremental compilation makes single-file edits fast after the first build
(a few seconds, not a full rebuild).

To scope the watch to just this crate's dependency graph and skip unrelated crates (e.g. only
restart `worker` when `worker`/`infrastructure`/`kernel` change, not on `api` edits):

```bash
cargo watch -w crates/api -w crates/application -w crates/domain -w crates/infrastructure -w crates/kernel -x 'run -p api'
```

### Grafana / Tempo still work in this mode

Since `api`/`worker` push traces to `http://localhost:14317` (Tempo's host-mapped OTLP port),
everything shows up in Grafana exactly like full-Docker mode — open
http://localhost:13000 → Dashboards → "rust-api — Traces".

### Tearing down

```bash
docker compose down          # stop postgres/tempo/grafana
# Ctrl+C the cargo run / cargo watch processes
```
