# Changelog

All notable changes to ReticulumChat are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-07-28

First stable release. The v0.9.0 release candidate underwent an adversarial audit of the full codebase; all findings were fixed and covered by regression tests.

### Fixed (RC audit)

- **CLI crash on startup:** the `--host` flag's auto-assigned `-h` short collided with clap's built-in help flag, causing a debug-assertion panic on every invocation in debug builds. `--host` now uses `-H`.
- **Stack overflow on cyclic threads:** `MessageHistory::thread_root` recursed unboundedly when parent links formed a cycle (e.g. a malformed history where a message is its own ancestor). It now walks parents iteratively with cycle detection.
- **Panic on zero-capacity history:** `MessageHistory::with_capacity(0).add(...)` panicked removing from an empty history; zero-capacity histories now discard messages.
- **Config migration never reachable:** configs predating the `version` field failed to parse, so `config-migrate` could never migrate them. `Config` now uses serde defaults (missing `version` = v0), and partial configs parse with defaults for missing fields.
- **`feedback` command hung on EOF:** the interactive stdin loop spun forever after Ctrl+D; it now terminates correctly at EOF.
- **Identity data loss:** a corrupt/unreadable identity file was silently overwritten by a freshly generated one. The old file is now backed up to `<identity-path>.corrupt.bak` first.
- **GUI wrong sender name:** the GUI hardcoded the current user as `"user"` instead of the loaded identity's name.

### Added

- 22 new hermetic hardening tests (77 total) covering the regressions above plus config parsing, message serialization, identity handling, crypto robustness on malformed input, and queue edge cases
- `Default` implementation for `OfflineQueue`

### Changed

- All `cargo clippy --all-targets` and `cargo fmt --check` warnings resolved
- README corrected: removed keybindings that don't exist (`Tab`, `Page Up/Down`), updated identity-corruption troubleshooting, added a Known Limitations section
- PLAN.md checkboxes audited against the codebase; deferred items (live Reticulum transport, SQLite persistence, rooms/reactions/file transfer/voice) explicitly marked with post-1.0 notes

## [0.9.0] - 2025-06-08

### Release Candidate

This is the v0.9.0 release candidate, representing the final step before v1.0.0. All APIs are now frozen and documentation is complete.

### Added

- **Comprehensive README** with detailed usage examples for all CLI commands and options
- **Demo Mesh Network Setup Guide** (`docs/DEMO_MESH_SETUP.md`) — step-by-step instructions for setting up a multi-node mesh network with Alice, Bob, and Charlie
- **CHANGELOG.md** with full version history and release notes
- Final API freeze — all public APIs are now stable and will not change before v1.0.0

### Documentation

- Complete user documentation covering:
  - Quick start and installation instructions
  - Detailed usage examples for TUI and GUI modes
  - Configuration management (`config-migrate` command)
  - Beta feedback submission (`feedback` command)
  - Environment variables (`RUST_LOG`)
  - TUI key bindings reference
  - Docker usage
  - Troubleshooting guide
- Architecture documentation (`docs/ARCHITECTURE.md`) with system diagrams and data flow
- Demo mesh network guide with multi-machine setup instructions

### Changed

- Version bumped to 0.9.0 in `Cargo.toml`
- README updated to reflect v0.9.0 release candidate status

## [0.8.0] - 2025-05-15

### Stability Release

Focused on long-running stability, network resilience, and beta testing infrastructure.

### Added

- Long-running session tests (7+ days)
- Network partition recovery with automatic path re-establishment
- Message delivery guarantees with retry logic
- Configuration migration system (`migrate_config`) with version tracking
- Beta testing feedback collection via `reticulumchat feedback` command
- Crash reporter with panic hooks and detailed stack traces
- `ConfigMigrate` CLI subcommand for upgrading old configs
- `Feedback` CLI subcommand for submitting bug reports and feature requests

### Changed

- Improved offline message queue reliability
- Enhanced path quality measurement accuracy
- Better handling of stale nodes and paths (60-second timeout)

### Fixed

- Memory leaks during extended mesh network operation
- Race conditions in shared state access
- Incorrect bandwidth calculation during high packet loss

## [0.7.0] - 2025-04-20

### Pre-Release Polish

Infrastructure, testing, and packaging improvements preparing for stable release.

### Added

- Comprehensive test suite with unit and integration tests
- CI/CD pipeline with GitHub Actions
- Cross-platform build scripts (`scripts/build.sh`)
- Docker image with multi-stage build
- Project documentation structure (`docs/`)
- Integration tests covering full mesh lifecycle and message workflows

### Changed

- Build system optimized for release binaries
- Test coverage expanded to all major modules

## [0.6.0] - 2025-03-10

### Network Features

Advanced mesh network visualization and management capabilities.

### Added

- Mesh network visualization with ASCII topology display
- Node discovery and status tracking (`NodeStatus`: Online, Degraded, Offline, Unknown)
- Path quality indicators (`PathQuality`: Excellent, Good, Fair, Poor, Dead)
- Automatic path redundancy with configurable min/max paths
- Bandwidth usage statistics with rolling window tracking
- `MeshNetwork` manager for node lifecycle and topology edges
- `NetworkPath` with latency, hop count, packet loss, and staleness detection
- `BandwidthStats` with 60-second history and 5-second moving average

### Changed

- Network layer redesigned for real-time path quality feedback
- Bandwidth reporting integrated into TUI and GUI network views

## [0.5.0] - 2025-02-01

### Advanced Messaging

Rich messaging features for group chat and conversation management.

### Added

- Message editing with `edited_at` timestamp tracking
- Message deletion (soft delete)
- Reply threads with `parent_id` navigation
- @mentions with auto-extraction and desktop notifications
- Full-text message search (case-insensitive)
- Pinned messages with `pinned_by` and `pinned_at` metadata
- `MessageHistory` with O(1) UUID lookup and comprehensive search
- `MessagingService` event channel for decoupled notifications

### Changed

- Message structure expanded to support editing, threading, and pinning
- UI modes added: Edit, Reply, Search, Thread, Pinned

## [0.4.0] - 2024-12-15

### Polish & Encryption

Security, reliability, and user experience enhancements.

### Added

- End-to-end encryption using age (X25519 key exchange, ChaCha20-Poly1305)
- `E2ECipher` high-level API with ASCII armor support
- Message delivery confirmations (`DeliveryConfirmation`)
- Offline message queue (`OfflineQueue`) with configurable capacity (default: 1000)
- Desktop notifications via `notify-rust` (`NotificationService`)
- TUI + GUI dual mode support
- GUI mode using egui/eframe (feature-gated with `--features gui`)

### Changed

- Encryption enabled by default (`--no-encryption` to disable)
- Notifications enabled by default (`--no-notifications` to disable)

## [0.3.0] - 2024-10-20

### Rooms & Features

Group chat and media sharing capabilities.

### Added

- Group chat / room support
- Message reactions
- File transfer over Reticulum (preparation for protocol integration)
- Voice message support (Opus compression preparation)

## [0.2.0] - 2024-09-05

### Reticulum Integration

Core protocol connectivity and contact management.

### Added

- TCP client for Reticulum instance connection (`ReticulumClient`)
- Destination management with hash-based addressing
- Send/receive plain text messages
- Contact list management
- Message history with local storage
- `Destination` and `LinkStatus` types for protocol abstraction

## [0.1.0] - 2024-08-01

### Initial Scaffold

Project foundation with basic UI and identity generation.

### Added

- Project scaffold in Rust
- Terminal UI with ratatui (`TuiApp`)
- Identity generation using Ed25519/X25519 keys (`Identity`)
- Basic CLI argument parsing with clap
- Application state management (`AppState`, `SharedState`)
- Configuration structure (`Config`)

[0.9.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.9.0
[0.8.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.8.0
[0.7.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.7.0
[0.6.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.6.0
[0.5.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.5.0
[0.4.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.4.0
[0.3.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.3.0
[0.2.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.2.0
[0.1.0]: https://github.com/reticulumchat/reticulumchat/releases/tag/v0.1.0
