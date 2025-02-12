# Leveraging the pre-built Docker images with
# cargo-chef and the Rust toolchain
# https://www.lpalmieri.com/posts/fast-rust-docker-builds/
FROM --platform=${BUILDPLATFORM:-linux/amd64} lukemathwalker/cargo-chef:latest-rust-1.84.1 AS chef
WORKDIR /rust_parser

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder

WORKDIR /rust_parser

COPY --from=planner /rust_parser/recipe.json recipe.json

RUN apt-get update && apt-get install -y clang cmake protobuf-compiler && rustup component add rustfmt

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

# Build actual target here
RUN cargo build --release

RUN mv target/release/rust_parser /rust_parser/rust_parser

FROM debian:11-slim
ARG APP=/rust_parser

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Etc/UTC \
    RUN_MODE=production

RUN mkdir -p ${APP}

COPY --from=builder /rust_parser/rust_parser ${APP}/rust_parser

WORKDIR ${APP}

CMD ["./rust_parser"]