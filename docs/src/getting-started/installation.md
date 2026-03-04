# Installation

## Nix (Recommended)

Ennio provides a Nix flake with pre-built packages.

```bash
# Run directly without installing
nix run github:glottologist/ennio -- --help

# Build the CLI
nix build github:glottologist/ennio

# Build the remote node daemon
nix build github:glottologist/ennio#ennio-node

# Build the WASM dashboard
nix build github:glottologist/ennio#ennio-dashboard
```

From a local checkout:

```bash
nix build              # ennio CLI (default package)
nix build .#ennio-node # remote daemon
nix run                # run directly
nix flake check        # clippy, fmt, tests, docs, audit
nix develop            # dev shell with all tools
```

The dev shell includes: `cargo-nextest`, `cargo-audit`, `cargo-watch`, `cargo-bloat`, `bacon`, and `rust-analyzer`.

## Cargo

Requires Rust 1.88+ and `protoc` (protobuf compiler).

```bash
# Install protoc (Debian/Ubuntu)
sudo apt install protobuf-compiler

# Install protoc (macOS)
brew install protobuf

# Build both binaries
cargo build --release -p ennio-cli -p ennio-node
```

Binaries are at `target/release/ennio` and `target/release/ennio-node`.

## Docker

```bash
# Build the image
docker build -t ennio .

# Run the CLI
docker run --rm ennio --help

# Run the node daemon
docker run --rm --entrypoint ennio-node ennio --help
```

Pre-built images are published to `glottologist/ennio` on Docker Hub for tagged releases (`v*`).

The Docker image is based on `debian:bookworm-slim` and includes `git`, `tmux`, and `openssh-client` since Ennio shells out to these at runtime.

## Runtime Dependencies

Ennio requires these tools to be available on `$PATH`:

| Tool | Required By | Purpose |
|------|-------------|---------|
| `git` | All workspaces | Clone repos, create worktrees, check status |
| `tmux` | TmuxRuntime, TmuxStrategy | Manage agent terminal sessions |
| `ssh` | SSH strategies | Connect to remote machines |
| `tmate` | TmateStrategy only | Shared terminal sessions |

Optional services:

| Service | Purpose |
|---------|---------|
| NATS | Event messaging between components |
| SQLite | Session and event persistence (auto-created) |
