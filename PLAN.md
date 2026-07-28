# ReticulumChat Roadmap

_Status audited against the codebase for the v1.0.0 release (2026-07-28)._

## v0.1.0 — Scaffold
- [x] Project scaffold (Rust with TUI)
- [x] Basic terminal UI with ratatui
- [x] Identity generation — **deviation:** uses age X25519 keys instead of Ed25519 (see `src/identity.rs`)

## v0.2.0 — Reticulum integration
- [ ] Connect to local Reticulum instance — **stub only** (`src/reticulum.rs` `ReticulumClient` logs and returns Ok; no TCP/RNS protocol). Requires external RNS instance to develop/test against; deferred post-1.0
- [ ] Send/receive plain text messages — messaging model complete (`src/messaging.rs`), but network send/receive is unwired pending the stub above; TUI/GUI operate on local state
- [x] Contact list / destination management — `Destination` type + TUI/GUI contact sidebar (static "General" contact; dynamic population pending network layer)
- [ ] Message history (local SQLite) — **deviation:** in-memory `MessageHistory` with indexing/search instead of SQLite; no on-disk persistence yet

## v0.3.0 — Rooms & features
- [ ] Group chat / room support — deferred (single "General" room hardcoded)
- [ ] Message reactions — deferred
- [ ] File transfer over Reticulum — deferred (needs network layer)
- [ ] Voice message (opus compression) — deferred

## v0.4.0 — Phase 4: Polish & encryption
- [x] End-to-end encryption (age X25519) — `src/crypto.rs`, armored + binary roundtrips tested
- [x] Message delivery confirmations — `DeliveryConfirmation` type + events + TUI status updates
- [x] Offline message queue — bounded `OfflineQueue` (default 1000)
- [x] Desktop notifications — `notify-rust` integration (messages, mentions, replies, delivery, pins)
- [x] TUI + GUI dual mode — `--mode tui|gui`, ratatui + egui/eframe

## v0.5.0 — Advanced messaging
- [x] Message editing and deletion
- [x] Reply threads
- [x] Mentions and notifications
- [x] Message search
- [x] Pinned messages

## v0.6.0 — Network features
- [x] Mesh network visualization — TUI network view + ASCII topology (`visualize_topology`)
- [x] Node discovery and status — `MeshNode`/`NodeStatus` model; live discovery pending network layer
- [x] Path quality indicators — `PathQuality::from_metrics` with boundary-tested thresholds
- [x] Automatic path redundancy — `check_redundancy` maintains min/max path counts
- [x] Bandwidth usage stats — `BandwidthStats` with rolling 60s window, rates, peaks

## v0.7.0 — Pre-release polish
- [x] Comprehensive test suite — 77 tests (unit + integration + RC hardening), all hermetic
- [x] CI/CD with GitHub Actions — `.github/workflows/ci.yml`
- [x] Cross-platform builds — via CI matrix
- [x] Docker image — `Dockerfile`
- [x] Documentation — README, `docs/ARCHITECTURE.md`, `docs/DEMO_MESH_SETUP.md`

## v0.8.0 — Stability
- [ ] Long-running session test (7d+) — not performed; blocked on real Reticulum integration
- [ ] Network partition recovery — blocked on real Reticulum integration
- [~] Message delivery guarantees — model + confirmations implemented; end-to-end guarantees need live network
- [x] Config migration system — `config-migrate` command, v0→v1 verified end-to-end
- [x] Beta testing feedback integration — `feedback` command writes timestamped reports

## v0.9.0 — Release candidate
- [x] Final API freeze
- [x] Documentation complete
- [x] Demo mesh network setup — `docs/DEMO_MESH_SETUP.md`
- [x] Release notes draft — `CHANGELOG.md`

## v1.0.0 — Ship it
- [x] RC audit + hardening — 6 bugs fixed (CLI panic, stack overflow, config migration, stdin hang, identity data-loss, GUI identity); 22 regression tests added
- [ ] Tag v1.0.0 — release-day action
- [ ] GitHub release with binaries — release-day action
- [ ] Announcement post — release-day action
- [ ] Comparison with other mesh chat tools — release-day action
- [ ] Community feedback channel — release-day action

## Post-1.0 priorities (deferred from above)
1. Real Reticulum/RNS transport behind the existing `ReticulumClient` API
2. SQLite persistence for message history and offline queue
3. Rooms, reactions, file transfer, voice messages
4. Long-running soak test and partition recovery once transport exists
