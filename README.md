# King God Castle Toolkit CLI

A Rust command-line toolkit for **King God Castle** (com.awesomepiece.castle) asset management and API analysis.
This tool created by vibecode, inspired by the work of [Reverse-Game-Android-Toolkit](https://github.com/nowl-it/Reverse-Game-Android-Toolkit)

## Features

- 📥 **XAPK Download** - Download game versions from APKPure
- 🔄 **C2U Convert** - Convert XAPK to Unity project using AssetRipper
- 📊 **Compare** - Compare two Unity project versions (diff assets, detect new heroes)
- 🔒 **MITM Proxy** - Capture and decrypt game API traffic
- 🎮 **Unity Tools** - Parse prefabs, list heroes, export assets
- ⚙️ **Config** - Manage XML configuration bundles
- 📦 **Dependencies** - Auto-install required tools (AssetRipper, Il2CppDumper)
- 🖥️ **TUI** - Interactive terminal user interface

## Installation

### Build from source

```bash
git clone https://github.com/nowl-it/kgc-toolkit-cli.git
cd kgc-toolkit-cli
cargo build --release
```

Binary will be at `target/release/kgc`.

### Requirements

- Rust 1.75+ (for building)
- dotnet-runtime-7.0+ (for Il2CppDumper on Linux)

## Usage

### Download XAPK

```bash
# List available versions
kgc download --list

# Download latest version
kgc download

# Download specific version
kgc download --version 167.0.01

# Download to custom directory
kgc download -o ./downloads/
```

### Convert to Unity Project

```bash
# Convert XAPK to Unity project
kgc c2u ./kgc.xapk -o ./unity_project/

# Skip extraction (use existing extracted files)
kgc c2u ./kgc.xapk -o ./unity_project/ --skip-extract
```

### Compare Versions

```bash
# Compare two Unity projects
kgc compare ./v166 ./v167

# Filter by pattern
kgc compare ./v166 ./v167 --filter "01_Fx"

# Output to JSON
kgc compare ./v166 ./v167 --output diff.json
```

### MITM Proxy

```bash
# Start proxy on default port (8888)
kgc proxy start

# Start on custom port
kgc proxy start --port 9999

# List captured traffic
kgc proxy list

# Decrypt a response (auto-uses KGC AES key)
kgc proxy decrypt <request_id>

# Export traffic to JSON
kgc proxy export -o traffic.json

# Clear captured traffic
kgc proxy clear

# Show proxy status
kgc proxy status
```

### Unity Tools

```bash
# Parse prefab hierarchy
kgc unity parse-prefab ./Assets/01_Fx/1_Hero/Fx_10001/Fx_10001.prefab

# List heroes in project
kgc unity list-heroes ./unity_project/

# Export hero assets
kgc unity export-hero 10001 --project ./unity_project/ -o ./export/
```

### Config Management

```bash
# Fetch XML bundle from CDN
kgc config fetch

# Extract XML files
kgc config extract ./ConfigBundle.unity3d

# List extracted configs
kgc config list
```

### Dependencies

```bash
# Check tool status
kgc deps check

# Install specific tool
kgc deps install asset-ripper

# Install all tools
kgc deps install-all

# Show tool paths
kgc deps paths
```

### Interactive TUI

```bash
kgc tui
```

Use `Tab` to switch tabs, arrow keys to navigate, and `Ctrl+Q` to quit.

## Configuration

Config file: `~/.config/kgc-toolkit/config.toml`

```toml
[paths]
tools_directory = "~/.local/share/kgc-toolkit/tools"

[proxy]
default_port = 8888
```

## Data Directories

- **Tools**: `~/.local/share/kgc-toolkit/tools/`
- **Certificates**: `~/.local/share/kgc-toolkit/certificates/`
- **Traffic DB**: `~/.local/share/kgc-toolkit/traffic.db`
- **Config**: `~/.config/kgc-toolkit/`

## License

MIT

## Credits

- [AssetRipper](https://github.com/AssetRipper/AssetRipper) - Unity asset extraction
- [Il2CppDumper](https://github.com/Perfare/Il2CppDumper) - IL2CPP metadata dump
