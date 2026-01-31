# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Stook is a Rust service that automatically redeploys Portainer stacks when Docker registry webhooks indicate new images have been pushed. Add a `stook: redeploy` label to a Docker Compose service, and stook handles the rest.

## Commands

All commands use `just` (justfile task runner):

```bash
just build          # cargo build --release
just test           # cargo test
just lint           # cargo clippy -- -D warnings
just fmt            # cargo fmt
just fmt-check      # cargo fmt -- --check
just check          # cargo check
just run            # Run locally
just docker-build   # Build Docker image
just test-webhook   # Send test webhook payload to /webhook
just health         # Check /health endpoint
```

## Architecture

**Data flow:** Docker Registry → `POST /webhook` → parse notification → filter push events → discover matching containers via Docker socket → redeploy stack via Portainer API.

Four modules in `src/`:

- **`main.rs`** — Server init, env config, Axum router setup
- **`routes.rs`** — `POST /webhook` and `GET /health` handlers; `AppState` holds `Discovery` and `Redeployer`
- **`discovery.rs`** — Queries Docker socket for containers with `stook`/`stook.image` labels, maps repository names to stack names (from `com.docker.compose.project` label), TTL-cached
- **`redeployer.rs`** — Calls Portainer API: finds stack by name, fetches stack file, PUTs update with `pullImage: true`
- **`registry.rs`** — Deserializes Docker Registry V2 notification format, filters for push events

**Key traits** (used for test mocking): `WebhookLookup` (discovery) and `StackRedeployer` (redeployment).

**Integration tests** in `tests/webhook.rs` use mock implementations of both traits.

## Environment Variables

| Variable | Default | Required |
|----------|---------|----------|
| `PORTAINER_API_KEY` | — | Yes |
| `PORTAINER_URL` | `https://localhost:9443` | No |
| `LISTEN_PORT` | `3000` | No |
| `CACHE_TTL_SECS` | `60` | No |
| `LOG_LEVEL` | `info` | No |
