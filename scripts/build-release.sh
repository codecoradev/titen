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

# ── Configuration ──────────────────────────────────────────────────────────
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUTPUT_DIR="${REPO_ROOT}/target/release-artifacts"
CRATE_NAME="titen"

# All supported targets
TARGETS=(
  "x86_64-unknown-linux-gnu"
  "aarch64-unknown-linux-gnu"
  "x86_64-apple-darwin"
  "aarch64-apple-darwin"
)

# Map target triple → archive name fragment (os-arch)
declare -A ARCHIVE_NAME
ARCHIVE_NAME["x86_64-unknown-linux-gnu"]="linux-x86_64"
ARCHIVE_NAME["aarch64-unknown-linux-gnu"]="linux-aarch64"
ARCHIVE_NAME["x86_64-apple-darwin"]="darwin-x86_64"
ARCHIVE_NAME["aarch64-apple-darwin"]="darwin-aarch64"

# Native target (for cargo vs cross decision)
NATIVE_TARGET=$(rustup target list --installed 2>/dev/null | head -1 || echo "unknown")

# ── Preflight ──────────────────────────────────────────────────────────────
command -v cargo &>/dev/null || { err "cargo not found — install Rust first"; exit 1; }

# Check for cross-rs
HAS_CROSS=false
if command -v cross &>/dev/null; then
  HAS_CROSS=true
  info "Found cross-rs — will use it for non-native targets"
else
  warn "cross not found — falling back to cargo for all targets"
  warn "Install cross with:  cargo install cross --locked"
fi

rm -rf "${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}"

# ── Build function ─────────────────────────────────────────────────────────
build_target() {
  local target="$1"
  local archive_fragment="${ARCHIVE_NAME[$target]}"
  local archive_name="titen-${archive_fragment}.tar.gz"
  local staging="${OUTPUT_DIR}/${archive_fragment}"
  local tool

  echo ""
  echo -e "${BOLD}Building ${CRATE_NAME} for ${target} …${RESET}"

  # Select build tool: cross for non-native, cargo for native
  if [[ "${target}" == "${NATIVE_TARGET}" ]]; then
    tool="cargo"
  elif [[ "${HAS_CROSS}" == "true" ]]; then
    tool="cross"
  else
    # cargo can still cross-compile if the target is installed and linker is configured
    if rustup target list --installed 2>/dev/null | grep -q "^${target}"; then
      tool="cargo"
      warn "Using cargo for cross-target ${target} — ensure linker is configured"
    else
      err "Target ${target} not installed and cross not available"
      err "Run:  rustup target add ${target}  (or install cross)"
      return 1
    fi
  fi

  # Build release
  if [[ "${tool}" == "cross" ]]; then
    cross build --release --target "${target}"
  else
    cargo build --release --target "${target}"
  fi

  # Stage binaries
  mkdir -p "${staging}"
  local target_dir="${REPO_ROOT}/target/${target}/release"

  for bin in titen titen-api titen-mcp; do
    if [[ -f "${target_dir}/${bin}" ]]; then
      cp "${target_dir}/${bin}" "${staging}/${bin}"
      info "Staged ${bin} for ${archive_fragment}"
    else
      warn "${bin} not found at ${target_dir}/${bin} — skipping"
    fi
  done

  # Create tar.gz
  tar -czf "${OUTPUT_DIR}/${archive_name}" -C "${OUTPUT_DIR}" "${archive_fragment}/"
  info "Created ${archive_name}"

  # Clean up staging
  rm -rf "${staging}"
}

# ── Main ───────────────────────────────────────────────────────────────────
echo -e "${BOLD}Titen Release Builder${RESET}"
echo "Repo root : ${REPO_ROOT}"
echo "Output    : ${OUTPUT_DIR}"
echo "Targets   : ${TARGETS[*]}"
echo ""

failed=()

for target in "${TARGETS[@]}"; do
  if ! build_target "${target}"; then
    failed+=("${target}")
  fi
done

# ── Checksums ──────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}Generating SHA256 checksums …${RESET}"
(
  cd "${OUTPUT_DIR}"
  sha256sum titen-*.tar.gz > SHA256SUMS.txt
)
info "Wrote SHA256SUMS.txt"

echo ""
if [[ ${#failed[@]} -eq 0 ]]; then
  echo -e "${GREEN}${BOLD}All targets built successfully!${RESET}"
  echo ""
  echo "Artifacts:"
  ls -lh "${OUTPUT_DIR}/titen-"*.tar.gz "${OUTPUT_DIR}/SHA256SUMS.txt"
else
  echo -e "${RED}${BOLD}Failed targets: ${failed[*]}${RESET}"
  exit 1
fi