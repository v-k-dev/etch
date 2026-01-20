# Etch 1.0.0 Release Checklist

## ✅ Completed

- [x] Cleaned catalog from 151 to 66 working distros (56% reduction)
- [x] Fixed all broken URLs (96 broken → 0 broken)
- [x] Version bumped to 1.0.0 (Cargo.toml, PKGBUILD)
- [x] Download bug fixes:
  - [x] Success notification dialog
  - [x] Closeable download window
  - [x] Existing file detection
  - [x] SHA256 verification of existing files
  - [x] Non-modal download window
  - [x] Background download support
  - [x] Hide button to continue in background
- [x] Professional README (527 lines → 137 lines, 74% reduction)
- [x] AUR submission script created

## 📋 Pre-Release Tasks

### 1. Test Build
```bash
cd /home/aaronn/Dokumente/development/dev2/etch
cargo build --release
cargo test
cargo clippy
```

### 2. Final Testing
- [ ] Download an ISO (verify existing file detection works)
- [ ] Hide download window (verify continues in background)
- [ ] Complete download (verify success notification)
- [ ] Write to USB (verify write operation)
- [ ] Check all UI elements work

### 3. Git Preparation
```bash
git add -A
git commit -m "Release v1.0.0

Major improvements:
- 66 curated distros (all verified working)
- Smart download resumption
- Background downloads
- Better UX/notifications
- Professional documentation
"
git tag -a v1.0.0 -m "Etch 1.0.0 - Stable Release"
git push origin main
git push origin v1.0.0
```

### 4. AUR Publication
```bash
# Run the automated script
./publish-aur.sh

# Or manual steps:
# 1. Setup SSH key at https://aur.archlinux.org/account/
# 2. cd /tmp && mkdir etch-aur && cd etch-aur
# 3. cp ~/Dokumente/development/dev2/etch/PKGBUILD .
# 4. makepkg --printsrcinfo > .SRCINFO
# 5. git init && git add PKGBUILD .SRCINFO
# 6. git commit -m "Initial release: etch 1.0.0"
# 7. git remote add origin ssh://aur@aur.archlinux.org/etch.git
# 8. git push -u origin master
```

### 5. Post-Release
- [ ] Update AUR package page description
- [ ] Add screenshots to GitHub
- [ ] Create GitHub release notes
- [ ] Announce on r/linux, r/archlinux
- [ ] Update Flatpak manifest for Flathub submission

## 🎯 Success Metrics

**Before:**
- 151 distros (96 broken URLs - 64% failure)
- No existing file detection
- Download window bugs
- 527-line README

**After:**
- 66 distros (0 broken URLs - 100% success)
- Smart resumption
- Polished download UX
- 137-line professional README
- Ready for AUR/Flathub

## 📦 Installation Methods

**After AUR publication, users can install via:**
```bash
# Arch Linux
yay -S etch
paru -S etch

# From source
git clone https://github.com/v-k-dev/etch.git
cd etch && cargo build --release
```

## 🚀 Next Steps (Future)

- Flathub submission (universal Linux)
- Add more distros (maintain quality bar)
- Resume partial downloads (HTTP range requests)
- Multi-USB writing (parallel writes)
- Live verification during write
