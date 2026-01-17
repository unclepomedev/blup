# blup

[![Crates.io](https://img.shields.io/crates/v/blup.svg)](https://crates.io/crates/blup)
[![CI](https://github.com/unclepomedev/blup/actions/workflows/release.yml/badge.svg)](https://github.com/unclepomedev/blup/actions)
[![License](https://img.shields.io/crates/l/blup.svg)](https://github.com/unclepomedev/blup/blob/main/LICENSE)

**Blender Version Manager** 🦀

A CLI tool to manage Blender versions (`rustup` for Blender).
Supports `.blender-version` files, context-aware execution, and script path injection.

Works on **Windows**, **macOS**, and **Linux**.

## Installation

### macOS/Linux (Automated Install or Package Managers)

```shell
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/unclepomedev/blup/releases/latest/download/blup-installer.sh | sh
# or
brew install unclepomedev/blup/blup
```

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/unclepomedev/blup/releases/latest/download/blup-installer.ps1 | iex"
```

### From cargo

If you have Rust installed:

```shell
cargo install blup
# or
git clone https://github.com/unclepomedev/blup.git
cd blup
cargo install --path .
```

## Quick Start

### Basic Usage

```shell
blup install 5.0.0                   # Download & Install
blup default 5.0.0                   # Set global default
blup list                            # Check installed versions
blup run                             # Run default version
blup run -- file.blend --background  # Run with arguments
```

### Daily Builds

```shell
blup list --remote               # List available versions (Active Stable & Daily)
blup install 4.2 --daily         # Install latest 4.2 experimental
blup install daily --daily       # Install latest main branch
```

**Note**: The remote list reflects the active build pipeline. Intermediate stable versions (e.g., `4.5.5`) disappear from the list once superseded, but can still be installed directly: `blup install 4.5.5`.

### Version Control

```shell
# Run specific version
blup run 5.0.0

# Pin version for project (Context Aware)
echo "5.0.0" > .blender-version
blup run  # Auto-detects 5.0.0 from file
```

### For Developers

```shell
# Inject add-on scripts path (BLENDER_USER_SCRIPTS)
blup run --scripts ./my_addon
```

You can set the `BLUP_MIRROR_URL` environment variable.
The URL must point to the directory structure equivalent to `https://download.blender.org/release`.

```shell
# Linux / macOS
export BLUP_MIRROR_URL=https://mirror.example.com/blender/release
# Windows (PowerShell)
$env:BLUP_MIRROR_URL="https://mirror.example.com/blender/release"
````

### Storage Location

* **Linux**: `~/.local/share/blup/versions`
* **macOS**: `~/Library/Application Support/blup/versions`
* **Windows**: `%LOCALAPPDATA%\blup\versions`

### Uninstall

```shell
blup remove 4.2.0
```

## License

MIT License
