#!/bin/bash
# Complete Etch 1.0.0 Release Workflow
# This script guides you through the entire release process

set -e

VERSION="1.0.0"
ETCH_DIR="$HOME/Dokumente/development/dev2/etch"

echo "╔════════════════════════════════════════════╗"
echo "║   Etch v${VERSION} Release Workflow         ║"
echo "╚════════════════════════════════════════════╝"
echo ""

# Step 1: Build and test
echo "━━━ Step 1: Build and Test ━━━"
cd "$ETCH_DIR"
echo "Building release version..."
cargo build --release
echo "✓ Build successful"
echo ""
echo "Running tests..."
cargo test
echo "✓ Tests passed"
echo ""

# Step 2: Git commit and tag
echo "━━━ Step 2: Git Commit & Tag ━━━"
echo "Current git status:"
git status --short
echo ""
echo "Commit all changes? (y/n)"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    git add -A
    git commit -m "Release v${VERSION}

Major improvements:
- 66 curated distros (all verified working)
- Smart download resumption with existing file detection
- Background downloads with hide functionality
- Improved UX with success notifications
- Professional documentation (74% smaller README)
- SHA256 verification for all downloads
- Polished download window (non-modal, closeable)
" || echo "No changes to commit"
fi

echo ""
echo "Create git tag v${VERSION}? (y/n)"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    git tag -a "v${VERSION}" -m "Etch ${VERSION} - Stable Release"
    echo "✓ Tag created"
fi

echo ""
echo "Push to GitHub? (y/n)"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    git push origin main
    git push origin "v${VERSION}"
    echo "✓ Pushed to GitHub"
fi

# Step 3: AUR Submission
echo ""
echo "━━━ Step 3: AUR Submission ━━━"
echo "Ready to publish to AUR?"
echo "Make sure you have:"
echo "  - SSH key uploaded to https://aur.archlinux.org/account/"
echo "  - Username: vkdev"
echo ""
echo "Continue with AUR submission? (y/n)"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    cd /tmp
    rm -rf etch-aur
    mkdir etch-aur
    cd etch-aur
    
    # Copy PKGBUILD
    cp "$ETCH_DIR/PKGBUILD" .
    
    # Generate .SRCINFO
    makepkg --printsrcinfo > .SRCINFO
    
    # Show what will be committed
    echo ""
    echo "PKGBUILD contents:"
    cat PKGBUILD
    echo ""
    echo ".SRCINFO contents:"
    cat .SRCINFO
    echo ""
    
    # Initialize git
    git init
    git add PKGBUILD .SRCINFO
    git commit -m "Initial release: etch ${VERSION}

- Fast, reliable ISO-to-USB writer
- 66 curated Linux distributions
- SHA256 verification
- Built-in ISO browser
- Background downloads with resume
- Smart platform detection
- Polkit integration for safety
"
    
    # Push to AUR
    echo ""
    echo "Pushing to AUR..."
    git remote add origin ssh://aur@aur.archlinux.org/etch.git
    git push -u origin master
    
    echo ""
    echo "✓ Published to AUR!"
fi

# Step 4: Done
echo ""
echo "╔════════════════════════════════════════════╗"
echo "║          Release Complete! 🚀              ║"
echo "╚════════════════════════════════════════════╝"
echo ""
echo "Etch v${VERSION} is now available at:"
echo "  • GitHub: https://github.com/v-k-dev/etch/releases/tag/v${VERSION}"
echo "  • AUR: https://aur.archlinux.org/packages/etch"
echo ""
echo "Users can install with:"
echo "  yay -S etch"
echo "  paru -S etch"
echo ""
echo "Next steps:"
echo "  - Add release notes on GitHub"
echo "  - Announce on r/archlinux"
echo "  - Submit to Flathub (optional)"
