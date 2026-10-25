#!/usr/bin/env bash
set -euo pipefail

echo "==> Bootstrapping solana-trading-bot development environment"

# Ensure cargo bin is on PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Verify required Linux packages
echo "==> Checking system packages"
REQUIRED_PKGS=(build-essential pkg-config libssl-dev clang cmake protobuf-compiler git curl jq unzip)
MISSING=()
for pkg in "${REQUIRED_PKGS[@]}"; do
  if ! dpkg -s "$pkg" >/dev/null 2>&1 && ! dpkg -s "$(echo "$pkg" | awk '{print $1}')" >/dev/null 2>&1; then
    if ! which "${pkg:-}" >/dev/null 2>&1; then
      MISSING+=("$pkg")
    fi
  fi
done
if [ "${#MISSING[@]}" -gt 0 ]; then
  echo "WARN: Missing system packages (install with apt-get install -y ...): ${MISSING[*]}"
fi

# Verify Rust toolchain
echo "==> Checking Rust toolchain"
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: Rust toolchain not found. Install from https://sh.rustup.rs"
  exit 1
fi
echo "cargo:  $(cargo --version)"
echo "rustc:  $(rustc --version)"

# Install Solana CLI if missing
if ! command -v solana >/dev/null 2>&1; then
  echo "==> Installing Solana CLI"
  SH_URL="https://release.solana.com/stable"
  curl -sSfL "$SH_URL/install.sh" -o /tmp/solana-install.sh
  sh /tmp/solana-install.sh -y --no-modify-path
  export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi
echo "solana: $(solana --version 2>/dev/null || echo 'not on PATH')"

# Fetch dependencies
echo "==> Fetching Rust dependencies"
cargo fetch

# Copy .env.example -> .env if missing
if [ ! -f .env ]; then
  if [ -f .env.example ]; then
    cp .env.example .env
    echo "==> Created .env from .env.example (edit it before trading)"
  else
    echo "WARN: No .env.example found; create .env manually"
  fi
else
  echo "==> .env already exists; leaving untouched"
fi

# Make helper scripts executable
if [ -f scripts/verify.sh ]; then
  chmod +x scripts/verify.sh
fi

echo ""
echo "==> Optional secrets not yet configured (build/test work without them):"
for s in HELIUS_API_KEY HELIUS_RPC_URL HELIUS_WS_URL JUPITER_API_KEY SOLANA_WALLET_PRIVATE_KEY TELEGRAM_BOT_TOKEN TELEGRAM_CHAT_ID; do
  if [ -z "${!s:-}" ] 2>/dev/null && [ -z "$(grep -E "^${s}=" .env 2>/dev/null | head -1)" ]; then
    echo "   - $s"
  fi
done

echo ""
echo "==> Bootstrap complete. Run: bash scripts/verify.sh"
