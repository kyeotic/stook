# stook

A lightweight Rust service that routes Docker registry v2 webhook notifications to Portainer stack webhooks. Fully stateless — no config files or volumes (beyond the Docker socket).

## How It Works

1. A Docker registry sends push notifications to stook's `/webhook` endpoint
2. stook extracts the repository name from the notification
3. It queries the Docker socket for containers with matching `webhook-router.*` labels
4. It forwards an HTTP POST to the corresponding Portainer webhook URL

## Discovery via Docker Labels

Stacks opt-in to webhook routing by adding labels to their services:

```yaml
services:
  myapp:
    image: registry.local/myapp:latest
    labels:
      webhook-router.image: "myapp"
      webhook-router.url: "http://portainer:9443/api/webhooks/<token>"
```

Adding a new stack to the router = adding two labels to that stack's compose file. No changes to the router or its deployment.

## Deployment

```yaml
services:
  registry:
    image: registry:2
    environment:
      REGISTRY_NOTIFICATIONS_ENDPOINTS: >
        - name: webhook-router
          url: http://webhook-router:3000/webhook
          timeout: 5s
          backoff: 1s

  webhook-router:
    image: registry.local/stook:latest
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - LISTEN_PORT=3000
      - CACHE_TTL_SECS=60
      - LOG_LEVEL=info
```

## Configuration

| Variable | Default | Description |
|---|---|---|
| `LISTEN_PORT` | `3000` | Port to listen on |
| `CACHE_TTL_SECS` | `60` | Seconds to cache Docker label discovery |
| `LOG_LEVEL` | `info` | Log level (trace/debug/info/warn/error) |

## API

| Endpoint | Method | Description |
|---|---|---|
| `/webhook` | POST | Receives registry v2 notification, routes to Portainer |
| `/health` | GET | Returns 200 OK |

## Building

```sh
cargo build --release
```

### Docker

```sh
docker build -t stook .
```
