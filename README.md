# stook

Automatically redeploy Portainer stacks when you push a new image to your Docker registry. Just add one label to a stack's compose file — no config files, no webhook plumbing, no manual redeployment.

## Quick Start

Add stook to your Docker Compose stack alongside your registry:

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
    image: ghcr.io/kyeotic/stook:latest
    environment:
      # PORTAINER_URL: "http://portainer:9443" # Only needed if non-default
      PORTAINER_API_KEY: "${PORTAINER_API_KEY}"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
```

Then add a label to any service you want to auto-deploy on push:

```yaml
services:
  myapp:
    image: registry.local/myapp:latest
    labels:
      webhook-router.image: "myapp"
```

The stack name is read automatically from the `com.docker.compose.project` label that Docker Compose sets on every container.

That's it. Push an image to your registry and stook will redeploy the stack via the Portainer API.

## How It Works

1. A Docker registry sends push notifications to stook's `/webhook` endpoint
2. stook extracts the repository name from the notification
3. It queries the Docker socket for containers with matching `webhook-router.image` labels and reads the `com.docker.compose.project` label to determine the stack name
4. It calls the Portainer API to redeploy the stack (pull latest images, preserve env vars and compose file)

## Configuration

| Variable            | Default                 | Description                                        |
| ------------------- | ----------------------- | -------------------------------------------------- |
| `LISTEN_PORT`       | `3000`                  | Port to listen on                                  |
| `CACHE_TTL_SECS`    | `60`                    | Seconds to cache Docker label discovery            |
| `LOG_LEVEL`         | `info`                  | Log level (trace/debug/info/warn/error)            |
| `PORTAINER_URL`     | `http://portainer:9443` | Portainer base URL                                 |
| `PORTAINER_API_KEY` | *(required)*            | Portainer API token (create in Portainer UI → API) |

## API

| Endpoint   | Method | Description                                            |
| ---------- | ------ | ------------------------------------------------------ |
| `/webhook` | POST   | Receives registry v2 notification, routes to Portainer |
| `/health`  | GET    | Returns 200 OK                                         |
