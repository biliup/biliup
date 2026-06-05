---
name: biliup
description: Use the biliup command-line tool to start the WebUI server, run recording tasks from config files, inspect help, and operate common upload/download commands.
---

# biliup

Use this skill when the user wants to install or operate the `biliup` command-line tool.

`biliup` can start the WebUI server, run recording tasks from config files, log in, upload videos, append videos, inspect video information, download videos, and list uploaded videos.

## Install flow

When the user needs to install `biliup`, choose one installation path based on the user's operating system and preference.

### GitHub Releases prebuilt package

Use this path when the user wants to install a prebuilt binary from GitHub Releases.

For Linux or macOS, generate and run this script. Set `INSTALL_DIR` to the user's target directory when needed.

```bash
set -euo pipefail

REPO="biliup/biliup"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64) arch="aarch64" ;;
  armv7l|armv7*) arch="arm" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

asset="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); os=sys.argv[1]; arch=sys.argv[2]; assets=[a["name"] for a in data["assets"]]; matches=[n for n in assets if n.endswith(".tar.xz") and f"-{arch}-{os}.tar.xz" in n]; print(matches[0] if matches else "")' "$os" "$arch")"

if [ -z "$asset" ]; then
  echo "No matching biliup release asset for ${arch}-${os}" >&2
  exit 1
fi

url="https://github.com/${REPO}/releases/latest/download/${asset}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -fL "$url" -o "$tmp/$asset"
tar -xJf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 755 "$(find "$tmp" -type f -name biliup | head -n 1)" "$INSTALL_DIR/biliup"

"$INSTALL_DIR/biliup" --help
```

For Windows PowerShell, generate and run this script. Set `$InstallDir` to the user's target directory when needed.

```powershell
$ErrorActionPreference = "Stop"

$Repo = "biliup/biliup"
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE "bin" }
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { throw "Unsupported Windows architecture" }

$Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
$Asset = $Release.assets | Where-Object { $_.name -like "biliupR-v*-$Arch-windows.zip" } | Select-Object -First 1
if (-not $Asset) { throw "No matching biliup release asset for $Arch-windows" }

$Tmp = New-Item -ItemType Directory -Force (Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString()))
try {
  $Zip = Join-Path $Tmp.FullName $Asset.name
  Invoke-WebRequest $Asset.browser_download_url -OutFile $Zip
  Expand-Archive $Zip -DestinationPath $Tmp.FullName -Force
  New-Item -ItemType Directory -Force $InstallDir | Out-Null
  $Exe = Get-ChildItem $Tmp.FullName -Recurse -Filter biliup.exe | Select-Object -First 1
  if (-not $Exe) { throw "biliup.exe not found in release archive" }
  Copy-Item $Exe.FullName (Join-Path $InstallDir "biliup.exe") -Force
  & (Join-Path $InstallDir "biliup.exe") --help
} finally {
  Remove-Item $Tmp.FullName -Recurse -Force
}
```

If the target directory is not on `PATH`, tell the user to run `biliup` by its full path or add the target directory to `PATH`.

### Windows winget

Use this path when the user is on Windows and wants to install from the command line:

```bash
winget install biliup
```

Then verify:

```bash
biliup --help
```

### Linux or macOS uv

Use this path when the user is on Linux or macOS and has `uv` available:

```bash
uv tool install biliup
```

Then verify:

```bash
biliup --help
```

## Operation flow

When the user asks to start the WebUI server with authentication, use:

```bash
biliup server --auth
```

When the user provides an address or port, include them explicitly:

```bash
biliup server --bind 0.0.0.0 --port 19159 --auth
```

When the user asks to run from a biliup 1.0.7 style config file, use:

```bash
biliup server --config config.toml
```

Replace `config.toml` with the user's config file path.

When the user wants to run the server in the background on Linux or macOS, use:

```bash
nohup biliup server --auth &
```

## Help flow

When the user asks what commands are available, show:

```bash
biliup --help
```

When the user asks about server options, show:

```bash
biliup server --help
```

When the user asks about a specific command, use:

```bash
biliup <command> --help
```

Replace `<command>` with the requested subcommand.

## Commands

The CLI includes these subcommands:

```text
login
renew
upload
append
show
comments
reply
dump-flv
download
server
list
```

When generating a command, include the concrete paths required by that command, such as cookie files, config files, and video file paths.
