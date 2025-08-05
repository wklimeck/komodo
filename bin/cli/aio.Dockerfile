FROM rust:1.88.0-bullseye AS builder

WORKDIR /builder
COPY Cargo.toml Cargo.lock ./
COPY ./lib ./lib
COPY ./client/core/rs ./client/core/rs
COPY ./client/periphery ./client/periphery
COPY ./bin/cli ./bin/cli

# Compile bin
RUN cargo build -p komodo_cli --release

# Copy binaries to distroless base
FROM gcr.io/distroless/cc

COPY --from=builder /builder/target/release/km /usr/local/bin/km

CMD [ "km" ]

LABEL org.opencontainers.image.source=https://github.com/moghtech/komodo
LABEL org.opencontainers.image.description="Komodo CLI"
LABEL org.opencontainers.image.licenses=GPL-3.0