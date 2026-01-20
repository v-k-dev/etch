# Etch v0.2.1 - Verification System & Expanded Catalog

## What's New

### SHA256 Verification System

- ✅ **Quick Verify**: Instantly check integrity of already-downloaded ISOs
- 🎯 **Direct-to-Write**: Skip download dialog for verified ISOs - one click to write!
- 🏷️ **Visual Badges**: Green checkmark for verified, yellow warning for unverified
- 🔄 **Smart Button**: Automatically changes from download to write icon based on verification

### Expanded Distribution Catalog

- 📦 **148 distros** (up from 85)
- 🎮 **Gaming**: Bazzite, SteamOS, ChimeraOS, Nobara
- 🏢 **Professional**: Proxmox, TrueNAS, pfSense, OpenMediaVault
- 🔒 **Security**: Tails, Qubes, Kali
- 📚 **Education**: Edubuntu, Sugar
- 🖥️ **Server**: Rocky, AlmaLinux, Oracle Linux

### UI Improvements

- 🎨 **Version Badges**: LTS, SRV, PRO labels with colored styling
- 🔍 **Better Filters**: Emoji labels and visual indicators
- ⚡ **Ultra-compact**: 24px filter buttons, 22×22 icons
- 💚 **Native GTK4**: Smooth, modern interface

### Under the Hood

- SQLite database integration for better catalog management
- SHA2 crate for fast verification
- Improved error handling and user feedback

## Installation

Download `etch-v0.2.1-linux-x86_64.tar.gz`, extract it, and install:

```bash
tar -xzf etch-v0.2.1-linux-x86_64.tar.gz
chmod +x etch etch-helper etch-updater
sudo mv etch etch-helper etch-updater /usr/local/bin/
etch
```

## Requirements

- Linux with GTK4
- polkit for privilege elevation
- 64-bit x86_64 architecture

## What's Fixed

- Improved download cancellation handling
- Better error messages for network issues
- Fixed progress tracking for large files

---

**Full Changelog**: https://github.com/v-k-dev/etch/compare/v0.1.5-nightly-wings...v0.2.1