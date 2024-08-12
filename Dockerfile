ARG DISTROLESS_TAG=latest

FROM rust:alpine AS builder

RUN \
   --mount=type=cache,target=/var/cache/apk,sharing=locked \
    apk add clang lld musl-dev

WORKDIR /usr/local/src
RUN \
    --mount=type=cache,target=/usr/local/cargo/git/db,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=bind,target=. \
    cargo build --release --target-dir=/build

FROM gcr.io/distroless/static-debian12:$DISTROLESS_TAG

COPY --from=builder /build/release/remo-graph /

ENTRYPOINT ["/remo-graph"]
