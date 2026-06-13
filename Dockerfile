FROM rust:1.96-slim-bookworm AS builder

# protoc: build.rs(tonic-prost-build), cmake/build-essential/libcurl: rdkafka(cmake-build) 소스 빌드.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        protobuf-compiler cmake build-essential libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# 캐시 마운트는 이미지 레이어에 남지 않으므로 같은 RUN에서 바이너리를 복사한다.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release \
        --bin matching-engine \
        --bin runtime_stress \
        --bin integration_stress \
    && cp target/release/matching-engine /usr/local/bin/matching-engine \
    && cp target/release/runtime_stress /usr/local/bin/runtime_stress \
    && cp target/release/integration_stress /usr/local/bin/integration_stress

FROM debian:bookworm-slim AS runtime-stress

COPY --from=builder /usr/local/bin/runtime_stress /usr/local/bin/runtime_stress

ENTRYPOINT ["/usr/local/bin/runtime_stress"]

FROM debian:bookworm-slim AS matching-engine

COPY --from=builder /usr/local/bin/matching-engine /usr/local/bin/matching-engine

EXPOSE 50051
ENTRYPOINT ["/usr/local/bin/matching-engine"]

FROM debian:bookworm-slim AS integration-stress

COPY --from=builder /usr/local/bin/integration_stress /usr/local/bin/integration_stress

ENTRYPOINT ["/usr/local/bin/integration_stress"]
