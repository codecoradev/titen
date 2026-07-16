# Stage 1: chef
FROM rust:1.88-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --workspace

# Stage 4: runtime
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/titen-api /usr/local/bin/
COPY --from=builder /app/target/release/titen-cli /usr/local/bin/
COPY --from=builder /app/target/release/titen-mcp /usr/local/bin/
EXPOSE 7845
CMD ["titen-api"]
