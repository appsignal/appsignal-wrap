#!/bin/sh

set -eu

if ! command -v curl >/dev/null; then
  echo "Error: \`curl\` is required to download the \`appsignal-wrap\` binary"
  exit 1
fi

if ! command -v tar >/dev/null; then
  echo "Error: \`tar\` is required to extract the \`appsignal-wrap\` binary"
  exit 1
fi

VERSION="${APPSIGNAL_WRAP_VERSION:-"latest"}"
INSTALL_FOLDER="${APPSIGNAL_WRAP_INSTALL_FOLDER:-"/usr/local/bin"}"

# Expected values are "linux" or "darwin".
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"

if [ "$OS" != "linux" ]; then
  echo "Error: Unsupported OS: $OS"
  exit 1
fi

if [ "$OS" = "linux" ]; then
  OS_FRIENDLY="Linux"
  VENDOR="unknown"
fi

# Expected values are "x86_64", "aarch64" or "arm64".
ARCH="$(uname -m)"
ARCH_FRIENDLY="$ARCH"

if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "arm64" ] && [ "$ARCH" != "aarch64" ]; then
  echo "Error: Unsupported architecture: $ARCH"
  exit 1
fi

# Rename "arm64" to "aarch64" to match the naming convention used by the Rust
# toolchain target triples.
if [ "$ARCH" = "arm64" ]; then
  ARCH="aarch64"
fi

# Rename "aarch64" to "arm64" for the user-friendly architecture name.
if [ "$ARCH" = "aarch64" ]; then
  ARCH_FRIENDLY="arm64"
fi

EXTRA=""
EXTRA_FRIENDLY=""

# Only check for musl with ldd on Linux; macOS doesn't have either.
if [ "$OS" = "linux" ]; then
  ldd_has_musl() {
    ldd --version 2>&1 | grep -q musl
  }

  if ldd_has_musl; then
    EXTRA="musl"
    EXTRA_FRIENDLY="musl"
  else
    EXTRA="gnu"
  fi
fi

if [ -z "$EXTRA" ]; then
  TRIPLE="${ARCH}-${VENDOR}-${OS}"
else
  TRIPLE="${ARCH}-${VENDOR}-${OS}-${EXTRA}"
fi

if [ -z "$EXTRA_FRIENDLY" ]; then
  FRIENDLY="${OS_FRIENDLY} (${ARCH_FRIENDLY})"
else
  FRIENDLY="${OS_FRIENDLY} (${ARCH_FRIENDLY}, ${EXTRA_FRIENDLY})"
fi

echo "Downloading $VERSION version of the \`appsignal-wrap\` binary for $FRIENDLY..."

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/appsignal/appsignal-wrap/releases/latest/download/$TRIPLE.tar.gz"
else
  URL="https://github.com/appsignal/appsignal-wrap/releases/download/v$VERSION/$TRIPLE.tar.gz"
fi

curl --progress-bar -SL "$URL" | tar -C "$INSTALL_FOLDER" -xz

echo "Done! Installed \`appsignal-wrap\` binary at \`$INSTALL_FOLDER/appsignal-wrap\`."
