# ReticulumChat Architecture

## Overview

ReticulumChat is a chat client built on top of the Reticulum mesh network protocol. It is designed as a modular Rust application with clear separation between networking, messaging, cryptography, and user interface concerns.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        User Interfaces                       │
│  ┌──────────────┐  ┌──────────────┐                        │
│  │  TUI Mode    │  │  GUI Mode    │                        │
│  │ (ratatui)    │  │ (egui)       │                        │
│  └──────┬───────┘  └──────┬───────┘                        │
└─────────┼─────────────────┼────────────────────────────────┘
          │                 │
          └────────┬────────┘
                   │
┌──────────────────▼──────────────────────────────────────────┐
│                   Application Layer                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ AppState     │  │ Config       │  │ AppRuntime   │      │
│  │ (Shared)     │  │              │  │              │      │
│  └──────┬───────┘  └──────────────┘  └──────────────┘      │
└─────────┼───────────────────────────────────────────────────┘
          │
    ┌─────┴─────┬─────────────┬─────────────┐
    │           │             │             │
┌───▼───┐  ┌───▼───┐    ┌───▼───┐    ┌───▼───┐
│Crypto │  │Message│    │Network│    │Notify │
│Layer  │  │Layer  │    │Layer  │    │Layer  │
└───┬───┘  └───┬───┘    └───┬───┘    └───┬───┘
    │          │            │            │
┌───▼──────────▼────────────▼────────────▼──────────────────┐
│              Transport Layer                                │
│         ┌─────────────────────┐                           │
│         │  Reticulum Protocol │                           │
│         │  (TCP/Unix Socket)  │                           │
│         └─────────────────────┘                           │
└───────────────────────────────────────────────────────────┘
```

## Module Breakdown

### 1. Application Core (`lib.rs`)

The central hub that ties all modules together:

- **`Config`**: Serializable configuration with defaults for identity path, Reticulum host/port, and feature toggles
- **`AppState`**: The shared mutable state containing config, identity, message queue, and mesh network
- **`SharedState`**: Thread-safe wrapper (`Arc<RwLock<AppState>>`) for concurrent access across async tasks
- **`AppRuntime`**: Top-level runtime handle that owns the shared state

### 2. Identity Management (`identity.rs`)

Handles cryptographic identity generation and management:

- **`Identity`**: Wraps an age X25519 keypair
  - Generates new random identities
  - Derives a Reticulum destination hash from the public key
  - Serializes/deserializes for persistent storage
- Uses `age::x25519::Identity` for the underlying cryptography

### 3. Cryptography (`crypto.rs`)

End-to-end encryption using the age format:

- **`encrypt_message()`**: Encrypts plaintext to a recipient's public key
- **`decrypt_message()`**: Decrypts ciphertext using the local identity
- **`E2ECipher`**: High-level API with ASCII armor support
  - `encrypt()`: Returns armored ciphertext string
  - `decrypt()`: Decrypts armored ciphertext
- Uses X25519 for key exchange and ChaCha20-Poly1305 for encryption

### 4. Messaging System (`messaging.rs`)

The core chat functionality:

#### Data Structures
- **`Message`**: Individual chat message with support for:
  - Editing (tracks `edited_at` timestamp)
  - Deletion (soft delete with content clearing)
  - Replies (`parent_id` for threading)
  - Mentions (auto-extracted from `@username` syntax)
  - Pinning (`pinned`, `pinned_by`, `pinned_at`)
  - Delivery status tracking
  
- **`DeliveryConfirmation`**: Receipt confirming message delivery status

- **`OfflineQueue`**: FIFO queue for messages when network is unavailable
  - Configurable capacity (default: 1000)
  - Supports removal by message ID

- **`MessageHistory`**: Persistent conversation history
  - O(1) message lookup by UUID via index
  - Full-text search (case-insensitive)
  - Sender search
  - Mention search
  - Thread navigation (replies to parent, root finding)
  - Pin management
  - Configurable max size (default: 10000)

#### Service
- **`MessagingService`**: Central coordinator
  - Event channel (`MessageEvent`) for decoupled notifications
  - Handles incoming/outgoing messages
  - Manages delivery confirmations
  - Emits events for edits, deletes, pins

### 5. Network Layer (`network.rs`)

Mesh network management and visualization:

#### Path Quality
- **`PathQuality`**: Enum representing link quality (Excellent → Dead)
  - Derived from latency and packet loss metrics
  - Color codes for TUI display

- **`NetworkPath`**: Individual route to a destination
  - Latency, hop count, packet loss tracking
  - Redundancy flags
  - Preferred path marking
  - Staleness detection (60-second timeout)
  - Bandwidth accounting

#### Node Management
- **`NodeStatus`**: Online, Degraded, Offline, Unknown

- **`MeshNode`**: Discovered peer in the mesh
  - Path management (add, remove, best-path selection)
  - Status derived from available paths
  - Capability tracking
  - Bandwidth statistics per node

- **`BandwidthStats`**: Rolling window bandwidth tracking
  - 60-second history for send/receive rates
  - 5-second moving average for current rate
  - Peak rate tracking
  - Human-readable formatting

#### Network Manager
- **`MeshNetwork`**: Central mesh coordinator
  - Node discovery and lifecycle management
  - Topology edge tracking
  - Automatic path redundancy (configurable min/max paths)
  - Quality-based path trimming
  - Stale node/path pruning
  - ASCII topology visualization
  - Bandwidth reporting
  - Network summary statistics

### 6. Reticulum Protocol (`reticulum.rs`)

Stub interface for Reticulum network integration:

- **`ReticulumClient`**: TCP client for Reticulum instance
  - Connection management (connect/disconnect)
  - Identity announcement
  - Packet send/receive
  
- **`Destination`**: Reticulum destination address
  - Hash-based addressing
  - Named destinations
  - Trust levels

- **`LinkStatus`**: Connection state machine (Down → Connecting → Up)

*Note: Full Reticulum protocol implementation is planned for v0.2.0*

### 7. Notifications (`notification.rs`)

Desktop notification service:

- **`NotificationService`**: Cross-platform notifications via `notify-rust`
  - Message notifications
  - Mention alerts (high urgency)
  - Reply notifications
  - Delivery confirmations
  - Pin notifications
  - Configurable enable/disable

### 8. User Interfaces

#### TUI (`tui/mod.rs`)
Terminal-based interface using ratatui:

- **`TuiApp`**: Main application state
  - Multiple modes: Normal, Edit, Reply, Search, Thread, Pinned, Network
  - Contact list sidebar
  - Message display with formatting
  - Input handling
  - Real-time event processing
  - Network visualization views

Key bindings are centralized in `handle_normal_input()` and mode-specific handlers.

#### GUI (`gui/mod.rs`)
Graphical interface using egui/eframe:

- **`GuiApp`**: egui App implementation
  - Same feature set as TUI
  - Scrollable message areas
  - Context menus for message actions
  - Mode-based view switching
  - Network topology and bandwidth panels

## Data Flow

### Sending a Message

```
User Input (TUI/GUI)
    ↓
MessagingService::queue_message()
    ↓
Message added to OfflineQueue
    ↓
Message added to MessageHistory
    ↓
MessageEvent::QueueUpdated emitted
    ↓
Network layer attempts delivery
    ↓
DeliveryConfirmation received
    ↓
Message status updated
```

### Receiving a Message

```
Reticulum packet received
    ↓
Decrypted (if encrypted)
    ↓
MessagingService::receive_message()
    ↓
Mentions extracted
    ↓
MessageEvent::MentionReceived (if mentioned)
    ↓
Message added to MessageHistory
    ↓
MessageEvent::MessageReceived
    ↓
NotificationService triggers desktop notification
    ↓
UI updates
```

### Network Discovery

```
Node discovered via Reticulum
    ↓
MeshNetwork::discover_node()
    ↓
Node added to nodes HashMap
    ↓
Paths measured and added
    ↓
NodeStatus derived from paths
    ↓
Topology edges rebuilt
    ↓
Redundancy check triggered (if needed)
    ↓
UI updates node list
```

## Concurrency Model

The application uses Tokio for async runtime:

- **Main thread**: UI rendering and input handling
- **Async tasks**: 
  - Network I/O (Reticulum connection)
  - Event processing (message events)
  - Periodic tasks (bandwidth ticks, stale pruning)
- **Shared state**: `Arc<RwLock<AppState>>` for safe concurrent access

## Testing Strategy

### Unit Tests
Each module contains `#[cfg(test)]` blocks testing:
- Individual struct behavior
- Method correctness
- Edge cases and boundary conditions

### Integration Tests (`tests/integration_test.rs`)
Cross-module tests covering:
- Full mesh network lifecycle
- Message workflow (send, edit, delete, pin)
- Thread navigation
- Search functionality
- Event flow through messaging service
- End-to-end chat scenarios

## Future Architecture (Planned)

### v0.2.0: Full Reticulum Integration
- Real TCP/Unix socket connection to Reticulum instance
- Destination registration and link establishment
- Actual packet encoding/decoding
- Link quality measurement from real metrics

### v0.8.0+: Stability
- SQLite persistence for message history
- Config migration system
- Long-running session management
- Network partition recovery
