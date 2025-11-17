# Rower

A Minecraft server proxy written in Rust.

## Features

- [x] **Multi-version Support**
- [x] **Server Switching** - Seamlessly switch players between backend servers
- [x] **Packet Interception** (e.g., modifying the server brand)
- [x] **Compression Support**
- [ ] **Online Mode**

## Getting Started

### Building

```bash
cargo build --release
```

### Configuration

On first run, Rower creates a `config.toml` file with default settings:

```toml
bind = "0.0.0.0:25565"              # Address to listen on
compression_threshold = 256         # Packet compression threshold (-1 to disable)
compression_level = 4               # Compression level (1-12)
online = true                       # Enable Mojang authentication (wip)
backend_server = "127.0.0.1:25566"  # Primary backend server
fallback_server = "127.0.0.1:25567" # Fallback server for disconnects
```

## Credits

- **[Velocity](https://github.com/PaperMC/Velocity)** - Proxy reference code
- **[potoq](https://github.com/Craftserve/potoq)** - Proxy reference code
- **[minecraft.wiki](https://minecraft.wiki/w/Minecraft_Wiki:Protocol_documentation)** - Protocol documentation
