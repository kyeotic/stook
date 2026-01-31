# Portainer Webhook Router — Implementation Plan

## Overview

A lightweight Rust service that receives Docker registry v2 webhook notifications, discovers which Portainer stack to notify by reading Docker container labels, and forwards the webhook to the correct Portainer endpoint. Fully stateless — no config files or volumes (beyond the Docker socket).

## Architecture

```
Docker Registry --POST /webhook--> Router --POST--> Portainer webhook URL
                                     |
                                     +-- reads container labels via Docker socket
```

On each incoming registry push event:
1. Parse the registry notification payload
2. Extract `repository` name from push events
3. Look up the matching Portainer webhook URL from cached Docker label data
4. Forward an HTTP POST to that Portainer webhook URL
5. Ignore unmatched images

## Discovery via Docker Labels

Instead of a config file, stacks opt-in to webhook routing by adding labels to their services:

```yaml
# In the target stack's compose file (e.g. myapp-stack.yml)
services:
  myapp:
    image: registry.local/myapp:latest
    labels:
      webhook-router.image: "myapp"
      webhook-router.url: "http://portainer:9443/api/webhooks/<token>"
```

The router queries the Docker API for all containers with `webhook-router.image` labels and builds an in-memory map of `image_name -> webhook_url`.

**Adding a new stack to the router = adding two labels to that stack's compose file.** No changes to the router or its deployment.

## Caching

- Cache the Docker label discovery results in memory with a configurable TTL (default: 60 seconds)
- On cache miss/expiry, re-query Docker for labeled containers
- No persistent storage needed

## Tech Stack

- **Language:** Rust
- **HTTP server:** `axum`
- **HTTP client:** `reqwest`
- **Docker API:** `bollard` (Rust Docker client, communicates via socket)
- **Container:** `FROM scratch` or distroless — single static binary via musl

## API

| Endpoint | Method | Description |
|---|---|---|
| `/webhook` | POST | Receives registry v2 notification, routes to Portainer |
| `/health` | GET | Returns 200 OK |

## Registry Notification Payload

```json
{
  "events": [
    {
      "action": "push",
      "target": {
        "repository": "myapp",
        "tag": "latest",
        "url": "..."
      }
    }
  ]
}
```

The router:
- Filters to `action == "push"` events only
- Matches `target.repository` against the cached label map
- Forwards to the corresponding Portainer webhook URL

## Deployment

```yaml
# registry stack compose
services:
  registry:
    image: registry:2
    ports:
      - "5000:5000"
    volumes:
      - registry-data:/var/lib/registry
    environment:
      REGISTRY_NOTIFICATIONS_ENDPOINTS: >
        - name: webhook-router
          url: http://webhook-router:3000/webhook
          timeout: 5s
          backoff: 1s

  webhook-router:
    image: registry.local/webhook-router:latest
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - LISTEN_PORT=3000
      - CACHE_TTL_SECS=60
      - LOG_LEVEL=info
```

## Configuration (Environment Variables)

| Variable | Default | Description |
|---|---|---|
| `LISTEN_PORT` | `3000` | Port to listen on |
| `CACHE_TTL_SECS` | `60` | Seconds to cache Docker label discovery |
| `LOG_LEVEL` | `info` | Log level (trace/debug/info/warn/error) |

## Project Structure

```
src/
  main.rs          — entrypoint, axum server setup
  routes.rs        — POST /webhook, GET /health handlers
  registry.rs      — registry notification payload types + parsing
  discovery.rs     — Docker label discovery via bollard, caching
  forwarder.rs     — HTTP POST forwarding to Portainer
Cargo.toml
Dockerfile
```

## Build & Container

- Build with `cargo build --release --target x86_64-unknown-linux-musl`
- Multi-stage Dockerfile: rust builder -> scratch/distroless final image
- Single static binary, minimal image size

## Design Decisions

- **Exact match only** on image names — no glob/prefix patterns
- **Fire-and-forget forwarding** — log failures but don't retry (the registry has its own retry logic)
- **Structured logging only** (via `tracing` crate) — no metrics endpoint
