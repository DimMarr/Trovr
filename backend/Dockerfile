# syntax=docker/dockerfile:1

# Builds the `trovr` binary. Queries are checked against the committed `.sqlx`
# cache, so no database is needed at build time; migrations are embedded.
FROM rust:1-bookworm AS build
WORKDIR /src
ENV SQLX_OFFLINE=true
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p trovr \
    && cp target/release/trovr /trovr

# Distroless: glibc, CA certificates and a non-root user, nothing else.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /trovr /usr/local/bin/trovr
ENV APP__BIND_ADDR=0.0.0.0:8080 \
    APP__LOG_FORMAT=json
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/trovr"]
