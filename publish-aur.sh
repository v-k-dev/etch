#!/bin/bash
# AUR Submission Guide for Etch v1.0.0
# Username: vkdev
# IMPORTANT: This script helps publish to AUR, but YOU need to:
#   1. Create git tag v1.0.0 on GitHub first
#   2. Have SSH key uploaded to AUR account

set -e

echo "=== Etch AUR Submission ==="
echo ""

# Check prerequisites
echo "Checking prerequisites..."
if ! command -v makepkg &> /dev/null; then
    echo "ERROR: makepkg not found. Install base-devel first."
    exit 1
fi

# Verify git tag exists
echo ""
echo "Verifying GitHub tag v1.0.0 exists..."
echo "Make sure you've tagged your release first:"
echo "  cd ~/Dokumente/development/dev2/etch"
echo "  git tag -a v1.0.0 -m 'Etch 1.0.0 - Stable Release'"
echo "  git push origin v1.0.0"
echo ""
echo "Has the tag been pushed? (y/n)"
read -r response
if [[ ! "$response" =~ ^[Yy]$ ]]; then
    echo "Please create and push the git tag first, then run this script again."
    exit 1
fi

# Step 1: Setup
echo "Step 1: Setting up AUR repository..."
cd /tmp
rm -rf etch-aur
mkdir etch-aur
cd etch-aur

# Step 2: Copy PKGBUILD
echo "Step 2: Copying PKGBUILD..."
cp ~/Dokumente/development/dev2/etch/PKGBUILD .

# Step 3: Test build locally
echo "Step 3: Testing local build..."
echo "Press Enter to test build with makepkg, or Ctrl+C to skip..."
read
makepkg -f

# Step 4: Generate .SRCINFO
echo "Step 4: Generating .SRCINFO..."
makepkg --printsrcinfo > .SRCINFO

# Step 5: Initialize git
echo "Step 5: Initializing git repository..."
git init
git add PKGBUILD .SRCINFO
git commit -m "Initial release: etch 1.0.0

- Fast, reliable ISO-to-USB writer
- 66 curated Linux distributions
- SHA256 verification
- Built-in ISO browser
- Background downloads with resume
- Smart platform detection
- Polkit integration for safety
"

# Step 6: Push to AUR
echo "Step 6: Pushing to AUR..."
echo "Make sure you have SSH access configured:"
echo "  ssh-keygen -t ed25519 -C 'vkdev@aur'"
echo "  Upload ~/.ssh/id_ed25519.pub to https://aur.archlinux.org/account/"
echo ""
echo "Press Enter to push to AUR, or Ctrl+C to cancel..."
read

git remote add origin ssh://aur@aur.archlinux.org/etch.git
git push -u origin master

echo ""
echo "=== SUCCESS ==="
echo "Etch is now published to AUR!"
echo ""
echo "Users can install with:"
echo "  yay -S etch"
echo "  paru -S etch"
echo ""
echo "View package at: https://aur.archlinux.org/packages/etch"
