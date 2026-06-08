# Demo Mesh Network Setup Guide

This guide walks you through setting up a demo mesh network with multiple ReticulumChat nodes on a single machine or across multiple machines on the same network.

## Overview

In this demo, you will:

1. Start three ReticulumChat nodes (Alice, Bob, and Charlie)
2. Connect them to separate Reticulum transport instances
3. Send messages between nodes
4. Observe mesh network discovery and path quality

## Prerequisites

- ReticulumChat built from source (see [README.md](../README.md))
- [Reticulum](https://reticulum.network/) installed and available on your system
- Three free TCP ports (default: 3742, 3743, 3744)

## Step 1: Prepare Reticulum Transport Instances

Each ReticulumChat node needs its own Reticulum transport instance. Create three separate configuration directories:

```bash
mkdir -p ~/reticulum-demos/{alice,bob,charlie}
```

### Alice's Reticulum Config (`~/reticulum-demos/alice/config`)

```bash
cat > ~/reticulum-demos/alice/config << 'EOF'
[reticulum]
  enable_transport = True
  share_instance = Yes
  shared_instance_port = 3742
  instance_control_port = 37421
  permit_joining = True

[logging]
  loglevel = 4
EOF
```

### Bob's Reticulum Config (`~/reticulum-demos/bob/config`)

```bash
cat > ~/reticulum-demos/bob/config << 'EOF'
[reticulum]
  enable_transport = True
  share_instance = Yes
  shared_instance_port = 3743
  instance_control_port = 37431
  permit_joining = True

[interfaces]
  [[Alice]]
    type = TCPClientInterface
    interface_enabled = Yes
    target_host = 127.0.0.1
    target_port = 3742

[logging]
  loglevel = 4
EOF
```

### Charlie's Reticulum Config (`~/reticulum-demos/charlie/config`)

```bash
cat > ~/reticulum-demos/charlie/config << 'EOF'
[reticulum]
  enable_transport = True
  share_instance = Yes
  shared_instance_port = 3744
  instance_control_port = 37441
  permit_joining = True

[interfaces]
  [[Alice]]
    type = TCPClientInterface
    interface_enabled = Yes
    target_host = 127.0.0.1
    target_port = 3742

[logging]
  loglevel = 4
EOF
```

## Step 2: Start Reticulum Transport Instances

Open three separate terminals and start each instance:

### Terminal 1: Alice's Reticulum

```bash
rnsd --config ~/reticulum-demos/alice
```

### Terminal 2: Bob's Reticulum

```bash
rnsd --config ~/reticulum-demos/bob
```

### Terminal 3: Charlie's Reticulum

```bash
rnsd --config ~/reticulum-demos/charlie
```

Wait a few seconds for the nodes to discover each other. You should see log messages indicating link establishment.

## Step 3: Start ReticulumChat Nodes

Open three more terminals and start each chat client:

### Terminal 4: Alice

```bash
reticulumchat \
  --name alice \
  --host 127.0.0.1 \
  --port 3742 \
  --identity ~/.reticulumchat/alice-id
```

### Terminal 5: Bob

```bash
reticulumchat \
  --name bob \
  --host 127.0.0.1 \
  --port 3743 \
  --identity ~/.reticulumchat/bob-id
```

### Terminal 6: Charlie

```bash
reticulumchat \
  --name charlie \
  --host 127.0.0.1 \
  --port 3744 \
  --identity ~/.reticulumchat/charlie-id
```

## Step 4: Send Messages

### From Alice to Bob

1. In Alice's terminal, press `Tab` to focus the contact list
2. Select Bob from the list (use `Up/Down` arrows)
3. Press `Tab` to return to the message input
4. Type a message and press `Enter`

Bob should see the message appear in his chat window.

### Group Chat

ReticulumChat supports group chat via room destinations. All nodes on the same mesh can participate in shared rooms.

### Using @mentions

Type `@bob` in your message to mention Bob. Bob will receive a desktop notification (if enabled) and the mention will be highlighted in the message.

## Step 5: Explore Mesh Network Visualization

Press `Ctrl+N` in any ReticulumChat terminal to view the mesh network topology.

You should see:
- **Alice** with direct connections to Bob and Charlie
- **Bob** with a path to Alice (and indirectly to Charlie via Alice)
- **Charlie** with a path to Alice (and indirectly to Bob via Alice)

### Understanding Path Quality

The network view shows path quality indicators:

| Indicator | Meaning |
|-----------|---------|
| 🟢 Excellent | Latency < 50ms, no packet loss |
| 🟢 Good | Latency < 100ms, minimal packet loss |
| 🟡 Fair | Latency < 300ms, some packet loss |
| 🔴 Poor | Latency < 1000ms, high packet loss |
| ⚫ Dead | No response, path timed out |

### Bandwidth Statistics

The network view also displays bandwidth usage:
- **Send Rate**: Current outgoing data rate (5-second moving average)
- **Receive Rate**: Current incoming data rate
- **Peak Rate**: Highest observed rate in the last 60 seconds

## Step 6: Test Advanced Features

### Reply Threads

1. Navigate to a message using `Up/Down`
2. Press `Ctrl+R` to reply
3. Type your reply and press `Enter`
4. Press `Ctrl+T` to view the thread

### Message Search

1. Press `Ctrl+F` to enter search mode
2. Type a keyword and press `Enter`
3. Use `Up/Down` to navigate results
4. Press `Esc` to exit search mode

### Pinning Messages

1. Navigate to a message
2. Press `Ctrl+P` to pin it
3. Press `Ctrl+V` to view all pinned messages

### Editing and Deleting

1. Press `Ctrl+E` to edit your last sent message
2. Press `Ctrl+D` to delete your last sent message (soft delete)

## Step 7: Simulate Network Partition

To test offline message queuing and network partition recovery:

1. Stop Bob's Reticulum instance (`Ctrl+C` in Terminal 2)
2. From Alice, send a message to Bob
3. The message will be queued (indicated by a ⏳ icon)
4. Restart Bob's Reticulum instance
5. The queued message will be delivered automatically

## Multi-Machine Setup

To run the demo across multiple machines on the same LAN:

1. Choose one machine as the "hub" (Alice)
2. On Alice's machine, use the local IP in the Reticulum config:
   ```
   shared_instance_port = 3742
   ```
3. On Bob's and Charlie's machines, point to Alice's IP:
   ```
   target_host = 192.168.1.10  # Alice's IP
   target_port = 3742
   ```
4. Ensure firewalls allow TCP traffic on port 3742

## Troubleshooting

### Nodes not discovering each other

- Check that Reticulum instances are running and logs show link establishment
- Verify TCP ports are not blocked by firewalls
- Ensure `permit_joining = True` in Reticulum configs

### Messages not delivering

- Check the network view (`Ctrl+N`) for path quality
- Verify destination hashes are correct
- Check the offline queue status in the UI

### High latency or packet loss

- This is normal in mesh networks with multiple hops
- The network view will show degraded path quality
- The system automatically seeks redundant paths when available

## Cleanup

To stop all demo processes:

```bash
# Stop Reticulum instances
pkill -f "rnsd.*reticulum-demos"

# Remove demo data (optional)
rm -rf ~/reticulum-demos
rm -rf ~/.reticulumchat/alice-id ~/.reticulumchat/bob-id ~/.reticulumchat/charlie-id
```

## Next Steps

- Read [ARCHITECTURE.md](ARCHITECTURE.md) to understand the internal design
- Explore the codebase in `src/` to see how features are implemented
- Join the community and share your feedback with `reticulumchat feedback`
