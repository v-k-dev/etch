# Etch

**A fast, reliable ISO-to-USB writer for Linux**

![Version](https://img.shields.io/badge/version-1.0.0-blue?style=flat-square)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green?style=flat-square)

## Features

- 🚀 **Fast** - Native Rust/GTK4 performance
- ✅ **Reliable** - SHA256 verification for all downloads
- 📦 **66 Distros** - Curated catalog with working download links
- 🔍 **Smart Detection** - Identifies platform from ISO content
- 🛡️ **Safe** - Polkit integration prevents accidental overwrites
- 📥 **Auto-Download** - Built-in ISO browser with resume support

## Installation

### Arch Linux (AUR)

```bash
yay -S etch
# or
paru -S etch
```

### From Source

```bash
git clone https://github.com/v-k-dev/etch.git
cd etch
cargo build --release
sudo install -Dm755 target/release/etch /usr/bin/etch
sudo install -Dm755 target/release/etch-helper /usr/bin/etch-helper
```

## Quick Start

1. Launch Etch
2. Browse and download an ISO from the catalog
3. Select your USB device
4. Click "Write to USB"
5. Verify automatically on completion

## Supported ISOs

Popular distributions included:
- Ubuntu, Fedora, Debian, Arch Linux
- Manjaro, Pop!_OS, Linux Mint, Zorin
- Kali Linux, Parrot Security
- Gaming: Nobara, ChimeraOS, Garuda
- Servers: Rocky Linux, AlmaLinux, Proxmox, TrueNAS
- And 50+ more...

## Architecture

```
etch/
├── src/
│   ├── main.rs              # GTK4 UI
│   ├── bin/
│   │   ├── etch-helper.rs   # Privileged operations (polkit)
│   │   └── etch-updater.rs  # Auto-update system
│   ├── core/                # Platform detection, models
│   ├── download/            # ISO fetcher with verification
│   ├── io/                  # USB device detection, writing
│   └── ui/                  # GTK4 components
└── catalog.json             # Curated distro database
```

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Requirements

**Runtime:**
- GTK4 >= 4.0
- GLib >= 2.66
- Polkit >= 0.105

**Build:**
- Rust >= 1.70
- pkg-config
- GTK4 development headers

## Security

- All downloads verified with SHA256
- Privileged operations isolated in `etch-helper`
- Polkit authentication required for disk writes
- USB devices filtered (prevents writing to system disks)

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

For distro additions, update `catalog.json` with:
- Working download URL
- SHA256 hash (or PLACEHOLDER for verification skip)
- Accurate size information

## License

Licensed under either:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## Author

Created by Aaron ([@v-k-dev](https://github.com/v-k-dev))

## Links

- **Repository:** https://github.com/v-k-dev/etch
- **Issues:** https://github.com/v-k-dev/etch/issues
- **AUR Package:** https://aur.archlinux.org/packages/etch
