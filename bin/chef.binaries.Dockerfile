## Builds the Komodo Core, Periphery, and Util binaries
## for a specific architecture.

## Dependency caching to help speed it up for multiple builds on same host.

FROM lukemathwalker/cargo-chef:latest-rust-1.89.0 AS chef
WORKDIR /builder

# Plan just the RECIPE to see if things have changed
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /builder/recipe.json recipe.json
# Build JUST dependencies - cached layer
RUN cargo chef cook --release --recipe-path recipe.json
# NOW build app
RUN \
  cargo build -p komodo_core --release && \
  cargo build -p komodo_periphery --release && \
  cargo build -p komodo_cli --release

# Copy just the binaries to scratch image
FROM scratch

COPY --from=builder /builder/target/release/core /core
COPY --from=builder /builder/target/release/periphery /periphery
COPY --from=builder /builder/target/release/km /km

LABEL org.opencontainers.image.source=https://github.com/moghtech/komodo
LABEL org.opencontainers.image.description="Komodo Binaries"
LABEL org.opencontainers.image.licenses=GPL-3.0