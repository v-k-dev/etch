# Etch Update System

## Overview

Etch includes a built-in update system that checks GitHub for new releases and can automatically update the application with user permission.

## Components

### 1. Polkit Policy (`org.etch.Etch.policy`)
Provides privilege escalation for:
- Writing ISO images to disks
- Updating the Etch application

Install to: `/usr/share/polkit-1/actions/org.etch.Etch.policy`

### 2. Updater Binary (`etch-updater`)
A separate binary that:
- Downloads new releases from GitHub
- Verifies the downloaded binary
- Replaces the old binary with elevated permissions
- Preserves backups

Runs via `pkexec` for privilege escalation.

### 3. GUI Update Checker
Integrated into the main application:
- Checks GitHub API for latest releases
- Compares versions
- Downloads and installs updates
- Restarts the application

## How It Works

1. User clicks the update button in Etch
2. Application queries GitHub API for latest release
3. If update available, shows confirmation dialog
4. User approves → downloads release binary
5. Launches `pkexec etch-updater <download-url> <temp-file> <target-binary>`
6. Polkit prompts for authentication
7. Updater replaces `/usr/bin/etch` with new version
8. Application restarts automatically

## Requirements

- `curl` for downloading from GitHub
- `polkit` for privilege escalation
- Internet connection for update checks

## Manual Update

If automatic update fails:
```bash
# Download latest release
curl -L -o /tmp/etch https://github.com/v-k-dev/etch/releases/latest/download/etch-x86_64

# Install with elevated permissions
pkexec install -m 755 /tmp/etch /usr/bin/etch
```

## For Package Maintainers

When building the package, ensure:
1. All three binaries are built: `etch`, `etch-helper`, `etch-updater`
2. Polkit policy is installed to `/usr/share/polkit-1/actions/`
3. Binaries have executable permissions (755)

The PKGBUILD includes all necessary installation steps.

## Security

- Updates only download from official GitHub releases
- Binary verification checks ELF format
- Polkit authentication required for installation
- Backup created before update
- Rollback possible if update fails
