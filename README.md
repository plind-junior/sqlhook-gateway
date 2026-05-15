# sqlhook

A webhook gateway with HMAC-signed ingress, jq transforms, and a durable retry queue. Single binary, SQLite-backed, config-as-code.

## What it does

```
inbound HTTP
  ─► HMAC signature verify
  ─► enqueue (SQLite)
       ─► worker picks up
            ─► jq transform
            ─► POST to destination (per-route timeout)
            ─► success: log delivery, mark done
            ─► failure: exponential backoff retry
            ─► budget exhausted: dead-letter
```

Routes are loaded from a YAML file at startup. Each route binds a `source_path`, a signature scheme, a jq filter, and a destination URL with retry policy. Secrets are referenced by environment variable name in the config and resolved at load time.

## Status

MVP. End-to-end happy path, retry, dead-letter, and replay are all verified. See [Not yet implemented](#not-yet-implemented) for what's deliberately missing.

## Quickstart

Requires Rust 1.85+ (edition 2021), and `cargo`.

```bash
git clone <your-fork-url> sqlhook
cd sqlhook
cargo build --release
```

Write a config (`config.yaml`):

```yaml
server:
  bind: 127.0.0.1:8080
  database_url: sqlite://./data/sqlhook.db?mode=rwc

worker:
  poll_interval_ms: 500
  concurrency: 4

routes:
  - id: github-push
    source_path: /gh/push
    signature:
      header: X-Hub-Signature-256
      algo: hmac-sha256
      secret_env: GH_WEBHOOK_SECRET
      prefix: "sha256="
    transform: |
      {
        repo: .repository.full_name,
        pusher: .pusher.name,
        sha: .after,
        commits: (.commits | length)
      }
    destination:
      url: https://internal.example.com/notify
      timeout_ms: 5000
    retry:
      max_attempts: 5
      initial_backoff_ms: 500
      max_backoff_ms: 60000
```

Run:

```bash
mkdir -p data
GH_WEBHOOK_SECRET='your-shared-secret' \
  ./target/release/sqlhook serve --config config.yaml
```

Send a signed event:

```bash
SECRET='your-shared-secret'
BODY='{"repository":{"full_name":"acme/widgets"},"pusher":{"name":"alice"},"after":"abc123","commits":[{"id":"abc123"}]}'
SIG=$(printf '%s' "$BODY" | openssl dgst -sha256 -hmac "$SECRET" -hex | awk '{print $2}')

curl -X POST http://127.0.0.1:8080/ingest/gh/push \
  -H "Content-Type: application/json" \
  -H "X-Hub-Signature-256: sha256=$SIG" \
  -d "$BODY"
```

## CLI

```
sqlhook serve            --config <path>   # start HTTP server + worker pool
sqlhook validate-config  --config <path>   # parse + resolve secrets; print "ok" or fail
sqlhook transform-test   --config <path> --route <id>   # stdin JSON → transform → stdout
sqlhook replay           --config <path> <job-id>       # re-queue a dead-lettered job
```

`transform-test` is the fastest way to iterate on a jq filter:

```bash
echo '{"user":{"name":"alice"},"items":[1,2,3]}' \
  | sqlhook transform-test --config config.yaml --route smoke
# {
#   "count": 3,
#   "name": "alice"
# }
```

## Operational endpoints

- `GET /health` — returns `{"status":"ok"}` once the server is bound and the worker is running.
- `GET /metrics` — Prometheus text format. Useful counters:
  - `sqlhook_ingest_accepted_total`
  - `sqlhook_ingest_signature_failed_total`
  - `sqlhook_ingest_unknown_route_total`
  - `sqlhook_ingest_bad_payload_total`
  - `sqlhook_delivery_attempts_total{route, outcome}` — outcome is `success` or `failure`
  - `sqlhook_delivery_retries_total`
  - `sqlhook_delivery_dead_total`

## Configuration reference

```yaml
server:
  bind: <host:port>                       # required
  database_url: sqlite://<path>?mode=rwc  # required; rwc creates the file if absent

worker:
  poll_interval_ms: <u64>   # default 500
  concurrency: <usize>      # default 4

routes:
  - id: <string>            # unique
    source_path: /<path>    # must start with /; unique
    signature:
      header: <header-name>
      algo: hmac-sha256     # only scheme supported in the MVP
      secret_env: <ENV_VAR> # resolved at startup; must be set
      prefix: <string>      # optional; stripped from header value before hex-decoding
    transform: |
      <jq expression>       # compiled once per process via `jaq`
    destination:
      url: <https-or-http-url>
      timeout_ms: <u64>     # default 5000
    retry:
      max_attempts: <u32>       # default 5
      initial_backoff_ms: <u64> # default 500
      max_backoff_ms: <u64>     # default 60000
```

## Architecture

```
src/
  main.rs           # CLI dispatch + serve()/replay()/transform-test()
  cli.rs            # clap definitions
  config/           # YAML loader + secret resolution
  error.rs          # AppError + AppResult
  signature.rs      # HMAC-SHA256 verify (constant-time)
  transform.rs      # jaq compile/apply wrapper
  delivery.rs       # reqwest client with per-call timeout
  retry.rs          # exponential backoff (capped)
  queue/
    mod.rs          # Queue trait + Job/JobOutcome types
    sqlite.rs       # claim-via-UPDATE-RETURNING implementation
  metrics.rs        # prometheus registry
  server/
    mod.rs          # axum Router assembly
    health.rs       # GET /health
    metrics.rs      # GET /metrics
    ingest.rs       # POST /ingest/{*path}
  worker.rs         # claim → transform → deliver → report
migrations/
  0001_init.sql     # jobs + deliveries tables
```

### Queue semantics

The `jobs` table has four statuses: `pending`, `processing`, `done`, `dead`. Workers claim with a single statement that's safe under contention (SQLite serializes writes):

```sql
UPDATE jobs
SET status = 'processing', attempts = attempts + 1, updated_at = ?
WHERE id = (
  SELECT id FROM jobs
  WHERE status = 'pending' AND visible_at <= ?
  ORDER BY visible_at
  LIMIT 1
)
RETURNING id, route_id, source_path, raw_payload, attempts
```

On failure with retry budget remaining, the worker writes the next `visible_at` and flips the status back to `pending`. On exhaustion, the row is set to `dead` and can be re-queued via `sqlhook replay <id>`.

Every attempt writes one row to `deliveries` (audit log), with the transformed payload, destination URL, response code/body, error, and duration — useful for forensics and for understanding why a delivery failed.

### Why SQLite

The MVP targets single-node deployments. SQLite handles the queue, audit log, and visibility-timer state in one file with no operational dependency. If you need multi-node, the natural upgrade path is Postgres (`FOR UPDATE SKIP LOCKED` replaces the SQLite `UPDATE ... RETURNING` claim); the `Queue` trait is the seam. Litestream/LiteFS can give you durable replication on top of SQLite for a wider set of failure modes without changing the data model.

## Differences from the inspiration

The original `duckdb-webhook-gateway` (Python + FastAPI + DuckDB + React) demonstrates the same core idea — a programmable pipeline between inbound webhooks and downstream services. `sqlhook` reimplements the idea with different design choices:

| Axis | duckdb-webhook-gateway | sqlhook |
| --- | --- | --- |
| Language | Python | Rust |
| Transform engine | DuckDB SQL with `{{payload}}` templating | jaq (jq-compatible), compiled once per route |
| State | DuckDB for everything | SQLite for queue + audit; no analytics DB |
| Topology | FastAPI `BackgroundTasks` (fire-and-forget) | Durable queue + worker pool, survives restarts |
| Inbound auth | Single API key on control plane | HMAC signature verify per event, per route |
| Retry | None | Exponential backoff with cap + dead-letter |
| Config | Stored in DB, mutated via REST | YAML loaded at startup (config-as-code) |
| UI | React SPA | None — CLI + Prometheus + structured logs |
| Audit | One row per event | One row per delivery attempt |

If you want a UI for managing routes and an interactive SQL playground, use upstream. If you want a single binary you can drop on a server that survives restarts, verifies signatures, and retries durably, this is what `sqlhook` is for.

## Not yet implemented

Deliberately deferred from the MVP:

- **Multiple signature schemes.** Only `hmac-sha256` with bare hex digest is supported. Stripe's `t=...,v1=...` format and Slack's `v0:timestamp:body` need scheme-specific parsers.
- **Hot config reload.** Edits to `config.yaml` require a process restart.
- **Multi-tenancy / quotas.** No per-tenant scoping, no rate limits.
- **Outbound mTLS or per-route auth headers.** The destination request goes out as a plain `POST` with `Content-Type: application/json`.
- **Distributed deployment.** Single-node only; see [Why SQLite](#why-sqlite).
- **Schema validation of payloads.** Transforms are responsible for shape; bad input produces a transform error and a dead-lettered job.
- **Replay-from-DLQ in bulk.** `sqlhook replay` takes one job id at a time.

## Development

```bash
cargo test             # 7 unit tests covering signature, transform, retry math
cargo build            # debug build
cargo build --release  # release binary at ./target/release/sqlhook
```

Logging is via `tracing`; set `RUST_LOG=sqlhook=debug,info` to see worker decisions.

## License

Apache-2.0. See [LICENSE](LICENSE) (matches the upstream project's license).
