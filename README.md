# ReticulumChat

[![CI](https://github.com/reticulumchat/reticulumchat/actions/workflows/ci.yml/badge.svg)](https://github.com/reticulumchat/reticulumchat/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.82%2B-blue.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-1.0.0-green.svg)](./CHANGELOG.md)

A chat client for the [Reticulum](https://reticulum.network/) mesh network, built in Rust with support for both terminal (TUI) and graphical (GUI) interfaces.

## Features

- **Dual Interface**: Run in TUI mode (ratatui) or GUI mode (egui/eframe)
- **Mesh Network Visualization**: View discovered nodes, path quality, and network topology in real-time
- **End-to-End Encryption**: Age-based X25519 encryption for private messages
- **Advanced Messaging**:
  - Message editing and deletion
  - Reply threads
  - @mentions with notifications
  - Full-text message search
  - Pinned messages
  - Delivery confirmations
- **Offline Support**: Message queue for when network is unavailable
- **Desktop Notifications**: System notifications for mentions and messages via `notify-rust`
- **Cross-Platform**: Linux, macOS, and Windows support
- **Config Migration**: Built-in configuration migration system
- **Beta Feedback**: Integrated feedback collection for beta testing

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) 1.82 or later
- (Optional) For GUI mode: system libraries for your platform

### Installation

```bash
# Clone the repository
git clone https://github.com/reticulumchat/reticulumchat.git
cd reticulumchat

# Build in TUI mode (default)
cargo build --release

# Or build with GUI support
cargo build --release --features gui

# Run
cargo run
```

### Basic Usage

```bash
# TUI mode (default)
./target/release/reticulumchat

# GUI mode
./target/release/reticulumchat --mode gui

# With custom options
./target/release/reticulumchat \
  --host 127.0.0.1 \
  --port 3742 \
  --name alice \
  --identity ~/.reticulumchat/identity
```

## Usage Examples

### Starting the Client

```bash
# Start in TUI mode with default settings
reticulumchat

# Start in GUI mode
reticulumchat --mode gui

# Connect to a remote Reticulum instance
reticulumchat --host 192.168.1.1 --port 4242

# Use a custom identity and display name
reticulumchat --name bob --identity ~/.reticulumchat/bob-id

# Disable desktop notifications
reticulumchat --no-notifications

# Disable end-to-end encryption
reticulumchat --no-encryption
```

### Configuration Management

```bash
# Migrate an old configuration file to the current format
reticulumchat config-migrate --config ~/.reticulumchat/config.json

# Migrate with backup disabled
reticulumchat config-migrate --config ~/.reticulumchat/config.json --backup false
```

### Beta Feedback

```bash
# Submit general feedback
reticulumchat feedback --message "Love the new thread feature!"

# Submit a bug report with attachment
reticulumchat feedback \
  --message "App crashes when viewing large network topologies" \
  --category bug \
  --attachment ./logs/crash.log

# Submit a feature request
reticulumchat feedback \
  --message "Please add voice message support" \
  --category feature

# Interactive feedback (reads from stdin)
reticulumchat feedback --category general
```

### Environment Variables

```bash
# Control log level
RUST_LOG=debug reticulumchat

# Or for specific modules
RUST_LOG=reticulumchat::network=trace reticulumchat
```

## TUI Controls

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Ctrl+Q` / `Esc` | Quit |
| `Ctrl+E` | Edit last message |
| `Ctrl+R` | Reply to last message |
| `Ctrl+F` | Search messages |
| `Ctrl+P` | Pin/unpin message |
| `Ctrl+D` | Delete message |
| `Ctrl+T` | View thread |
| `Ctrl+V` | View pinned messages |
| `Ctrl+N` | Network view |
| `Up/Down` | Navigate contacts |

## Configuration

ReticulumChat stores its configuration and identity in `~/.reticulumchat/` by default.

### Default Config File (`~/.reticulumchat/config.json`)

```json
{
  "identity_path": "~/.reticulumchat/identity",
  "reticulum_host": "127.0.0.1",
  "reticulum_port": 3742,
  "enable_notifications": true,
  "enable_encryption": true,
  "version": 1
}
```

### Identity File (`~/.reticulumchat/identity`)

The identity file contains your X25519 keypair used for encryption and Reticulum destination generation. It is created automatically on first run if it does not exist.

## Docker

```bash
# Build Docker image
docker build -t reticulumchat .

# Run
docker run -it --rm \
  -v ~/.reticulumchat:/data \
  -p 3742:3742 \
  reticulumchat
```

## Development

### Running Tests

```bash
# Run all tests
cargo test --all-features

# Run unit tests only
cargo test

# Run integration tests
cargo test --test integration_test
```

### Project Structure

```
reticulumchat/
├── src/
│   ├── main.rs          # CLI entry point with clap argument parsing
│   ├── lib.rs           # Library exports, Config, AppState, AppRuntime
│   ├── crypto.rs        # Age-based E2E encryption (X25519, ChaCha20-Poly1305)
│   ├── identity.rs      # Identity generation and Reticulum destination hash
│   ├── messaging.rs     # Messages, history, offline queue, delivery confirmations
│   ├── network.rs       # Mesh network, paths, bandwidth stats, topology
│   ├── notification.rs  # Desktop notifications via notify-rust
│   ├── reticulum.rs     # Reticulum client (stub; live transport planned post-1.0)
│   ├── crash_reporter.rs # Panic hook and crash reporting
│   ├── tui/             # Terminal UI implementation (ratatui)
│   └── gui/             # Graphical UI implementation (egui/eframe)
├── tests/               # Integration tests
├── docs/                # Documentation
│   ├── ARCHITECTURE.md  # Detailed architecture documentation
│   └── DEMO_MESH_SETUP.md # Demo mesh network setup guide
└── scripts/             # Build scripts
```

### Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed architecture documentation including:
- System architecture diagram
- Module breakdown
- Data flow for sending/receiving messages
- Network discovery process
- Concurrency model
- Testing strategy

## Mesh Network Setup

See [docs/DEMO_MESH_SETUP.md](docs/DEMO_MESH_SETUP.md) for a complete guide to setting up a demo mesh network with multiple ReticulumChat nodes.

## Building for Release

### Cross-Platform Builds

Use the included build script:

```bash
# Build for current platform
./scripts/build.sh

# Build for specific target
./scripts/build.sh x86_64-unknown-linux-gnu
./scripts/build.sh x86_64-apple-darwin
./scripts/build.sh x86_64-pc-windows-msvc
```

### Manual Release Build

```bash
# Linux/macOS
cargo build --release

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## Troubleshooting

### GUI mode fails to start

If you get an error about the `gui` feature not being enabled, rebuild with:
```bash
cargo build --release --features gui
```

### Cannot connect to Reticulum instance

Ensure your Reticulum instance is running and accessible at the host/port specified. The default is `127.0.0.1:3742`.

### Identity file issues

If your identity file becomes corrupted, ReticulumChat automatically backs it up to `<identity-path>.corrupt.bak` and generates a fresh identity, so the old key material is never silently destroyed.

### Config migration errors

If you encounter config parsing errors after upgrading, run:
```bash
reticulumchat config-migrate --config ~/.reticulumchat/config.json
```

## Known Limitations (v1.0.0)

- The Reticulum transport (`ReticulumClient`) is currently a stub: the full messaging, encryption, and mesh data models are implemented and tested, but packets are not yet sent over a live RNS instance. Wiring the transport is the first post-1.0 priority (see [PLAN.md](PLAN.md)).
- Message history and the offline queue are in-memory; on-disk persistence (SQLite) is planned post-1.0.
- Rooms, reactions, file transfer, and voice messages are deferred (see [PLAN.md](PLAN.md)).

## Roadmap

See [PLAN.md](PLAN.md) for the full development roadmap.

Current version: **v1.0.0** — First stable release

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and release notes.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Acknowledgments

- [Reticulum](https://reticulum.network/) — The mesh networking protocol
- [ratatui](https://ratatui.rs/) — Terminal UI framework
- [egui](https://www.egui.rs/) — Immediate mode GUI framework
- [age](https://age-encryption.org/) — Simple file encryption tool

---

## ☕ Support the Developer

If this project saved you time, solved a problem, or just made your day a little more neon, you can fuel the next one:

[![Buy Me A Coffee](https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png)](https://buymeacoffee.com/synthalorian)
