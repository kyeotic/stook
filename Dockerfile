FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM scratch
COPY --from=builder /app/target/release/stook /stook
EXPOSE 3000
ENTRYPOINT ["/stook"]
