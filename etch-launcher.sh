#!/usr/bin/env bash
# Etch Launcher - Handles sudo elevation smoothly

# Store current display variables
ORIGINAL_DISPLAY="${DISPLAY}"
ORIGINAL_WAYLAND_DISPLAY="${WAYLAND_DISPLAY}"
ORIGINAL_XAUTHORITY="${XAUTHORITY}"
ORIGINAL_USER="${USER}"

# Check if already root
if [ "$EUID" -eq 0 ]; then
    exec ./target/release/etch "$@"
fi

# Use sudo with preserved environment for GUI
echo "🔐 Etch requires root privileges to write to block devices"
echo "Please enter your password:"
exec sudo -E \
    DISPLAY="${ORIGINAL_DISPLAY}" \
    WAYLAND_DISPLAY="${ORIGINAL_WAYLAND_DISPLAY}" \
    XDG_RUNTIME_DIR="/run/user/$(id -u)" \
    XAUTHORITY="${ORIGINAL_XAUTHORITY}" \
    ./target/release/etch "$@"
