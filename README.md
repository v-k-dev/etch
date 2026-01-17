# ETCH

<p align="center">
  <img src="https://github.com/user-attachments/assets/6bc3f4eb-f075-4448-9971-ac02c6c5a8d3"
       alt="ETCH logo"
       width="360" />
</p>




A minimal, fast, and reliable USB/ISO writer for Linux.

Etch is a native GTK4 application written in Rust that writes ISO images to USB drives with verification. Built for correctness, transparency, and simplicity.

<p align="center">
  <img
    src="https://github.com/user-attachments/assets/47e5244d-28d6-4718-a82a-61e89d1ee52f"
    alt="image"
    width="720"
  />
</p>

## Status

**Version 0.1: NIGHTLY** · CODE 2

**Live development - actively updated**

This is the nightly development branch with latest features. For stable releases, see the [stable branch](https://github.com/v-k-dev/etch/tree/main-stable).

## Features

- Write ISO images to removable USB drives
- Verify written data for integrity
- Real-time progress reporting (bytes written, speed, ETA)
- Clean, minimal interface following modern Linux design principles
- No root required until write operation
- PolicyKit integration for privilege elevation

## Philosophy

- Correctness over convenience
- No fake progress or placeholder implementations
- One tool, one job - write ISOs safely
- Clean runtime - zero GTK warnings
- Simple, readable code

## Requirements

- Linux (GTK4)
- Rust toolchain (for building from source)
- PolicyKit (for privilege elevation)

## Building from Source

```bash
# Clone repository
git clone https://github.com/v-k-dev/etch.git
cd etch

# Build
cargo build --release

# Run
cargo run --release
```

## Installation

### Arch Linux (and derivatives)

**With yay/paru (AUR - recommended):**
```bash
yay -S etch
# or
paru -S etch
```

**Manual from AUR:**
```bash
git clone https://aur.archlinux.org/etch.git
cd etch
makepkg -si
```

**Build from source:**
```bash
git clone https://github.com/v-k-dev/etch.git
cd etch
cargo build --release
sudo install -Dm755 target/release/etch /usr/local/bin/
sudo install -Dm755 target/release/etch-helper /usr/local/bin/
```

### Flatpak (Universal - all distributions)

```bash
flatpak install flathub org.etch.Etch
flatpak run org.etch.Etch
```

Or install from [Flathub](https://flathub.org/apps/org.etch.Etch)

### Other distributions

Build from source:
```bash
git clone https://github.com/v-k-dev/etch.git
cd etch
cargo build --release
sudo install -Dm755 target/release/etch /usr/local/bin/etch
sudo install -Dm755 target/release/etch-helper /usr/local/bin/etch-helper
```

## Support

If you find Etch useful, consider supporting development:

- ☕ [Buy me a coffee](https://ko-fi.com) (Coming soon)
- ⭐ Star this repository
- 🐛 Report bugs and suggest features

## Usage

1. Launch Etch
2. Select an ISO file
3. Select target USB drive
4. Click Write
5. Authenticate when prompted (PolicyKit)
6. Wait for write and verification to complete

**Warning:** All data on the target drive will be permanently erased.

## Architecture

- `src/main.rs` - Application entry point
- `src/ui/` - GTK4 interface, styling
- `src/core/` - Business logic, models, verification
- `src/io/` - Device detection, disk I/O operations

## Development

```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Run with warnings visible
cargo run
```

## Contributing

This project is in active development. Contributions welcome but please discuss major changes first.

Design principles:
- Keep it simple
- No over-engineering
- Professional alignment and spacing
- Testable, incremental changes

## License

To be determined

## Credits

Developed with focus on reliability and transparency for the Linux desktop.

## What Etch Does NOT Do

- No partition management
- No multi-boot support
- No network features
- No telemetry or analytics
- No theme customization
- No cross-platform support (Linux only)

## License

MIT OR Apache-2.0

## Safety

⚠️ **Warning:** Etch performs destructive operations on block devices. Always:
- Double-check the selected target device
- Ensure you have backups of important data
- Verify the device path before confirming

Etch will validate devices and prevent writing to mounted devices, but user vigilance is essential.

## Development Philosophy

Etch prioritizes correctness and transparency over features. Every operation reports actual system state. No simulated progress, no fake timings, no hidden operations.

If you can't trust your ISO writer, what can you trust?
