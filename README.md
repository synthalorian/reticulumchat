# ReticulumChat

[![CI](https://github.com/reticulumchat/reticulumchat/actions/workflows/ci.yml/badge.svg)](https://github.com/reticulumchat/reticulumchat/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.82%2B-blue.svg)](https://www.rust-lang.org)

A chat client for the [Reticulum](https://reticulum.network/) mesh network, built in Rust with support for both terminal (TUI) and graphical (GUI) interfaces.

## Features

- **Dual Interface**: Run in TUI mode (ratatui) or GUI mode (egui/eframe)
- **Mesh Network Visualization**: View discovered nodes, path quality, and network topology
- **End-to-End Encryption**: Age-based X25519 encryption for private messages
- **Advanced Messaging**:
  - Message editing and deletion
  - Reply threads
  - @mentions with notifications
  - Message search
  - Pinned messages
  - Delivery confirmations
- **Offline Support**: Message queue for when network is unavailable
- **Desktop Notifications**: System notifications for mentions and messages
- **Cross-Platform**: Linux, macOS, and Windows support

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

### Usage

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

### TUI Controls

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
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── crypto.rs        # Age-based E2E encryption
│   ├── identity.rs      # Identity generation (X25519)
│   ├── messaging.rs     # Messages, history, queue
│   ├── network.rs       # Mesh network, paths, bandwidth
│   ├── notification.rs  # Desktop notifications
│   ├── reticulum.rs     # Reticulum protocol client
│   ├── tui/             # Terminal UI implementation
│   └── gui/             # Graphical UI implementation
├── tests/               # Integration tests
├── docs/                # Documentation
└── scripts/             # Build scripts
```

### Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed architecture documentation.

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

## Roadmap

See [PLAN.md](PLAN.md) for the full development roadmap.

Current version: **v0.7.0** — Pre-release polish

- [x] Comprehensive test suite
- [x] CI/CD with GitHub Actions
- [x] Cross-platform builds
- [x] Docker image
- [x] Documentation

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
