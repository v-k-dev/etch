# 🚀 Etch with Wings - v0.1: NIGHTLY (CODE 3)

**Etch with Wings** is the **experimental nightly release** of Etch, featuring expanded platform support and cutting-edge capabilities.

> **Note:** This is a **nightly/development** release. Use [Etch Stable](https://github.com/your-username/etch) for production systems.

---

## ✨ What's New in "Wings"

Etch with Wings extends the core ISO-to-USB writer with support for **embedded systems and microcontrollers**:

### 🎯 Supported Platforms

| Platform | Support | Details |
|----------|---------|---------|
| **Windows ISO** | ✅ Full | Standard Windows ISO flashing |
| **Linux ISO** | ✅ Full | All distributions supported |
| **Raspberry Pi** | 🔄 Experimental | Optimized SD card writing & verification |
| **Orange Pi** | 🔄 Experimental | ARM-based SBC support |
| **ESP32** | 🔄 In Progress | Firmware flashing via USB-Serial |
| **Arduino** | 🔄 In Progress | Bootloader & sketch upload |
| **Generic ISO** | ✅ Full | Any ISO image format |

### 🆕 New Features

- **🎨 Platform Detection** - Automatically identifies target platform from filename
- **🎯 Smart Icons** - Visual indicators for each platform type
- **⚡ Optimized Write Paths** - Specialized handling for SD cards (RasPi/OrangePi)
- **🔧 Firmware Support** - Direct microcontroller programming
- **📊 Platform-aware Verification** - Different verification methods per platform
- **🔄 Update Checker** - Built-in version checking (improved from Stable)

---

## 🏃 Quick Start

### Installation

#### 1️⃣ **Source Build** (Recommended for testing)

```bash
git clone https://github.com/your-username/etch.git
cd etch
git checkout main-nightly-wings

# Build release binary
cargo build --release

# Run directly
./target/release/etch
```

**Requirements:**
- Rust 1.70+ (`rustup`)
- GTK4 development headers
- Cargo

#### 2️⃣ **Cachy OS / Arch-based Systems** (Coming soon)

```bash
# AUR coming soon - stay tuned!
yay -S etch-wings
```

#### 3️⃣ **Flatpak** (Coming soon)

```bash
flatpak install flathub org.etch.Etch
flatpak run org.etch.Etch
```

---

## 🎮 Usage

### Basic Workflow

```
1. Click "Choose File" → Select your ISO/image file
2. Platform is auto-detected (see icon indicator)
3. Select target device (USB drive / SD card / Serial port)
4. Click "Write" → Real-time progress
5. Verification runs automatically
```

### Platform-Specific Usage

#### Raspberry Pi
- Select `.img` files with "raspberrypi" in filename
- Automatically optimized for SD card writing
- Proper file system finalization included

#### Orange Pi
- Select any ARM Linux distribution
- Similar SD card optimization as Raspberry Pi
- Works with most Orange Pi boards

#### ESP32 / Arduino
- Select `.bin` firmware files
- Requires USB connection
- Serial port auto-detection (work in progress)

---

## 🛠️ Building from Source

### Full Build Setup

```bash
# Clone nightly-wings branch
git clone -b main-nightly-wings https://github.com/your-username/etch.git
cd etch

# Install dependencies (Arch-based)
sudo pacman -S base-devel gtk4 rust

# Or Fedora/Ubuntu
sudo apt install build-essential libgtk-4-dev cargo

# Build
cargo build --release

# Install locally (optional)
cargo install --path .
```

### Build Flags

```bash
# Development build (faster)
cargo build

# Release (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --release
```

---

## 📋 Feature Matrix

| Feature | Stable | Wings |
|---------|--------|-------|
| ISO → USB | ✅ | ✅ |
| Byte verification | ✅ | ✅ |
| Real-time progress | ✅ | ✅ |
| Platform detection | ❌ | ✅ |
| RasPi support | ❌ | 🔄 |
| OrangePi support | ❌ | 🔄 |
| ESP32 support | ❌ | 🔄 |
| Arduino support | ❌ | 🔄 |
| Update checker | ❌ | ✅ |

---

## 📊 Performance Benchmarks

**Release Build (optimized):**
- Binary size: ~920KB
- Memory usage: ~45MB (idle)
- Write speed: Native USB throughput (up to 100MB/s)
- Verification speed: Real-time byte-by-byte with speed calculation

**Build Time:**
- Clean build: ~37s (first time includes dependencies)
- Incremental build: ~0.75s

---

## 🔒 Security

- ✅ **Polkit/pkexec** for privilege escalation
- ✅ **Byte-by-byte verification** prevents write corruption
- ✅ **No network calls** (except optional update check)
- ✅ **No persistent state** - memory-only settings
- ✅ **Open source** - audit-friendly Rust code

---

## 🐛 Known Issues

- 🔄 ESP32/Arduino support is experimental - USB detection needs refinement
- 🔄 Some OrangePi boards may need additional driver setup
- 📝 Platform detection based on filename only (could improve with magic bytes)

---

## 🤝 Contributing

This is the **experimental branch**. Contributions welcome!

```bash
git checkout main-nightly-wings
# Make changes
git commit -am "feat: cool new thing"
git push origin main-nightly-wings
```

Then open a PR to `main-nightly-wings` branch.

---

## 📝 Roadmap

**v0.1 (Current - Wings):**
- ✅ Platform detection framework
- 🔄 RasPi/OrangePi SD card optimization
- 🔄 ESP32/Arduino support
- ⏳ Advanced verification per-platform

**v0.2 (Planned):**
- Hardware device manager UI
- Batch writing support
- Custom partition table support

**v1.0 (Stable):**
- All features from Wings production-ready
- Native packages (AUR, Flatpak, Snap)
- Internationalization (i18n)

---

## 📄 License

MIT OR Apache-2.0

**Etch with Wings** - Making firmware and ISO flashing dead simple.

---

## 🆘 Support

- **Issues:** [GitHub Issues](https://github.com/your-username/etch/issues)
- **Discussions:** [GitHub Discussions](https://github.com/your-username/etch/discussions)
- **Email:** Not available (open-source project)

---

**Made with ❤️ by the Etch community**

*Etch: Correctness over convenience. No fake progress. Just write.*
