#!/usr/bin/env bash
set -euo pipefail

# ── Color helpers ──────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${GREEN}✓${RESET} $1"; }
err()   { echo -e "${RED}✗${RESET} $1" >&2; }
warn()  { echo -e "${YELLOW}!${RESET} $1" >&2; }

# ── Help ───────────────────────────────────────────────────────────────────
usage() {
  cat <<'EOF'
${BOLD}titen${RESET} — Install the latest Titen release from GitHub

Usage:
  install.sh [OPTIONS]

Options:
  --help                Show this help message

Environment variables:
  TITEN_INSTALL_DIR     Override install directory
                        (default: $HOME/.codecora/titen/bin)

Installs: titen, titen-api, titen-mcp
Platforms: Linux (x86_64, aarch64), macOS (x86_64, aarch64)
EOF
  exit 0
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
fi

# ── Detect OS ──────────────────────────────────────────────────────────────
os_name=""
case "$(uname -s)" in
  Linux*)   os_name="linux"   ;;
  Darwin*)  os_name="darwin"  ;;
  *)        err "Unsupported OS: $(uname -s)"; exit 1 ;;
esac

# ── Detect architecture ────────────────────────────────────────────────────
arch_name=""
case "$(uname -m)" in
  x86_64|amd64)    arch_name="x86_64"  ;;
  aarch64|arm64)  arch_name="aarch64" ;;
  *)               err "Unsupported architecture: $(uname -m)"; exit 1 ;;
esac

echo -e "${BOLD}Installing Titen${RESET} — ${os_name}/${arch_name}"

# ── Resolve install directory ──────────────────────────────────────────────
install_dir="${TITEN_INSTALL_DIR:-$HOME/.codecora/titen/bin}"
mkdir -p "${install_dir}"

# ── Download URL ───────────────────────────────────────────────────────────
release_url="https://github.com/codecoradev/titen/releases/latest/download/titen-${os_name}-${arch_name}.tar.gz"

# ── Temp directory ─────────────────────────────────────────────────────────
tmp_dir=$(mktemp -d)
trap 'rm -rf "${tmp_dir}"' EXIT

# ── Download ───────────────────────────────────────────────────────────────
archive="${tmp_dir}/titen.tar.gz"
echo "Downloading from ${release_url} …"
if command -v curl &>/dev/null; then
  curl --fail --silent --show-error --location --output "${archive}" "${release_url}"
elif command -v wget &>/dev/null; then
  wget --quiet --output-document="${archive}" "${release_url}"
else
  err "Neither curl nor wget is available"; exit 1
fi

# ── Extract ────────────────────────────────────────────────────────────────
echo "Extracting …"
tar -xzf "${archive}" -C "${tmp_dir}"

# ── Install binaries ───────────────────────────────────────────────────────
for bin in titen titen-api titen-mcp; do
  if [[ -f "${tmp_dir}/${bin}" ]]; then
    cp "${tmp_dir}/${bin}" "${install_dir}/${bin}"
    chmod +x "${install_dir}/${bin}"
    info "Installed ${bin} → ${install_dir}/${bin}"
  else
    warn "Binary ${bin} not found in archive — skipping"
  fi
done

# ── Verify ─────────────────────────────────────────────────────────────────
if "${install_dir}/titen" --help &>/dev/null; then
  info "Verified: titen binary runs"
else
  warn "titen binary did not run (may need runtime deps)"
fi

# ── Add to PATH ────────────────────────────────────────────────────────────
add_to_path() {
  local shell_rc="$1"
  local entry="export PATH=\"${install_dir}:\$PATH\""

  if [[ -f "${shell_rc}" ]] && grep -qF "${install_dir}" "${shell_rc}" 2>/dev/null; then
    info "PATH already configured in ${shell_rc}"
    return 0
  fi

  echo "" >> "${shell_rc}"
  echo "# Added by Titen installer" >> "${shell_rc}"
  echo "${entry}" >> "${shell_rc}"
  info "Added ${install_dir} to PATH in ${shell_rc}"
}

path_added=false
if [[ -f "$HOME/.bashrc" ]]; then
  add_to_path "$HOME/.bashrc"
  path_added=true
fi
if [[ -f "$HOME/.zshrc" ]]; then
  add_to_path "$HOME/.zshrc"
  path_added=true
fi

if [[ "${path_added}" == "false" ]]; then
  # Create .bashrc as a reasonable default
  touch "$HOME/.bashrc"
  add_to_path "$HOME/.bashrc"
fi

# ── Done ───────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}Titen installed successfully!${RESET}"
echo -e "  Binaries : ${install_dir}/"
echo -e "  Next step: source ~/.bashrc (or reopen your terminal)"
echo ""
