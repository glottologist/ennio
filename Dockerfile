FROM rust:1.88-bookworm AS planner
RUN cargo install cargo-chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.88-bookworm AS builder
RUN apt-get update && apt-get install -y cmake pkg-config protobuf-compiler && rm -rf /var/lib/apt/lists/*
ENV PROTOC=/usr/bin/protoc
RUN cargo install cargo-chef
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p ennio-cli -p ennio-node

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y git tmux openssh-client ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ennio /usr/local/bin/ennio
COPY --from=builder /app/target/release/ennio-node /usr/local/bin/ennio-node
ENTRYPOINT ["ennio"]
