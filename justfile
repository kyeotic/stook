default:
    @just --list

# Run the service locally (requires Docker socket access)
run:
    cargo run

# Build in release mode
build:
    cargo build --release

# Check compilation without building
check:
    cargo check

# Run tests
test:
    cargo test

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying files
fmt-check:
    cargo fmt -- --check

# Build Docker image
docker-build:
    docker build -t stook .

# Run Docker image locally
docker-run:
    docker run --rm -p 3000:3000 -v /var/run/docker.sock:/var/run/docker.sock:ro stook

# Send a test webhook payload
test-webhook:
    curl -s -X POST http://localhost:3000/webhook \
        -H 'Content-Type: application/json' \
        -d '{"events":[{"action":"push","target":{"repository":"myapp","tag":"latest"}}]}'

# Check health endpoint
health:
    curl -s http://localhost:3000/health
