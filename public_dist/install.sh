#!/usr/bin/env bash
# Loctree CLI installer
#
# Usage:
#   curl -fsSL https://loct.io/install.sh | bash
#
# Env overrides:
#   LOCTREE_VERSION   release version (default: 0.14.2)
#   INSTALL_DIR       where binaries/wrappers are placed (default: ~/.local/bin)
#   CARGO_HOME        cargo home for source fallback (default: ~/.cargo)
#   LOCTREE_BASE_URL  release base URL (default: https://loct.io/releases)
#   LOCTREE_GPG_KEY_URL  release signing public key URL (default: https://loct.io/loctree-signing.asc)
#   LOCTREE_GPG_FINGERPRINT  expected release key fingerprint
#   LOCTREE_REQUIRE_GPG=1  fail if the GPG key or .sig sidecar is unavailable
#   LOCTREE_NO_PROFILE_UPDATE=1  do not edit ~/.zshrc when PATH is missing
#   LOCTREE_ALLOW_SOURCE_FALLBACK=1  contributor fallback when no bundle exists

# --- Bash guard --------------------------------------------------------------
# This installer relies on `set -o pipefail` and other bash-only features.
# When users pipe into `sh` (Debian/Ubuntu /bin/sh = dash, strict POSIX),
# pipefail is rejected and the script aborts on line 17 before doing anything.
# Detect that and either re-exec under bash or fail fast with a fix command.
if [ -z "${BASH_VERSION:-}" ]; then
  if command -v bash >/dev/null 2>&1; then
    # If $0 points to a real file (i.e. the script was saved locally), we
    # can simply re-execute it under bash. When piped from stdin
    # (`curl ... | sh`), $0 is the calling shell name and there is no
    # readable file to re-exec — fall through to the friendly error below.
    if [ -f "$0" ] && [ -r "$0" ]; then
      exec bash "$0" "$@"
    fi
    printf 'Loctree installer needs bash, but it was invoked through %s.\n' "${0##*/}" >&2
    printf 'Re-run with bash:\n' >&2
    printf '  curl -fsSL https://loct.io/install.sh | bash\n' >&2
    exit 1
  fi
  printf 'Error: bash is required (your /bin/sh appears to be POSIX-only).\n' >&2
  printf 'Install bash, then run:\n' >&2
  printf '  curl -fsSL https://loct.io/install.sh | bash\n' >&2
  exit 1
fi

set -euo pipefail
umask 022

# Keep this default in lockstep with [workspace.package] version in Cargo.toml.
# `scripts/sync-version.sh` updates it and `make version-assert` enforces it.
VERSION="${LOCTREE_VERSION:-0.14.2}"
INSTALL_DIR="${INSTALL_DIR:-"$HOME/.local/bin"}"
CARGO_HOME="${CARGO_HOME:-"$HOME/.cargo"}"
CARGO_BIN="$CARGO_HOME/bin"
BASE_URL="${LOCTREE_BASE_URL:-https://loct.io/releases}"
GPG_KEY_URL="${LOCTREE_GPG_KEY_URL:-${BASE_URL%/releases}/loctree-signing.asc}"
GPG_FINGERPRINT="${LOCTREE_GPG_FINGERPRINT:-8868139E8A9A2291D067135FB979B60C7079E4D4}"
REQUIRE_GPG="${LOCTREE_REQUIRE_GPG:-1}"
RELEASE_BINARIES="loct loctree loctree-mcp loctree-lsp aicx aicx-mcp"

red() { printf '\033[0;31m%s\033[0m\n' "$*"; }
green() { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$*"; }
blue() { printf '\033[0;34m%s\033[0m\n' "$*"; }

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    red "missing required command: $1"
    exit 1
  fi
}

sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    red "missing shasum or sha256sum for release verification"
    exit 1
  fi
}

skip_signature_verification() {
  reason="$1"
  if [ "$REQUIRE_GPG" = "1" ]; then
    red "$reason"
    exit 1
  fi
  yellow "$reason; skipping signature verification"
}

normalize_fingerprint() {
  printf '%s' "$1" | tr -d '[:space:]' | tr '[:lower:]' '[:upper:]'
}

detect_libc() {
  # Pick `musl` for systems whose libc family is musl OR whose glibc is older
  # than the minimum required by the gnu bundle (currently 2.39). Otherwise
  # return `gnu`. The musl bundle is a static binary that runs anywhere on
  # Linux regardless of distro / glibc version — used as the wide-compat
  # fallback. Override with LOCTREE_LIBC=gnu|musl to force a specific variant.
  if [ -n "${LOCTREE_LIBC:-}" ]; then
    printf '%s' "$LOCTREE_LIBC"
    return
  fi
  if ! command -v ldd >/dev/null 2>&1; then
    printf 'gnu'
    return
  fi
  if ldd --version 2>&1 | head -1 | grep -qi musl; then
    printf 'musl'
    return
  fi
  glibc_ver="$(ldd --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+' | head -1)"
  if [ -z "$glibc_ver" ]; then
    printf 'gnu'
    return
  fi
  # awk numeric comparison: glibc_ver < 2.39  →  use musl static bundle.
  awk -v v="$glibc_ver" -v min="2.39" 'BEGIN { print (v + 0 < min + 0) ? "musl" : "gnu" }'
}

target_triple() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os:$arch" in
    darwin:arm64|darwin:aarch64) printf 'aarch64-apple-darwin' ;;
    linux:x86_64|linux:amd64) printf 'x86_64-unknown-linux-%s' "$(detect_libc)" ;;
    linux:aarch64|linux:arm64) printf 'aarch64-unknown-linux-%s' "$(detect_libc)" ;;
    *) printf '' ;;
  esac
}

# Explains *why* target_triple() came back empty. The release-bundles CI
# matrix (.github/workflows/release-bundles.yml) and its default target list
# (distribution/build-bundle.sh) only publish aarch64-apple-darwin,
# x86_64-unknown-linux-gnu and x86_64-unknown-linux-musl — Intel macOS
# (darwin:x86_64) is a real, currently-unpublished gap, not a typo.
unsupported_platform_reason() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os:$arch" in
    darwin:x86_64)
      printf 'Intel macOS (darwin/x86_64) has no published prebuilt bundle yet — only aarch64-apple-darwin (Apple Silicon) ships today.\n'
      printf 'Install via cargo instead: cargo install --locked loctree (loct/loctree CLIs; loctree-mcp/loctree-lsp separately), or re-run this installer with LOCTREE_ALLOW_SOURCE_FALLBACK=1 to do the same automatically.\n'
      ;;
    *)
      printf 'No prebuilt bundle target maps to %s/%s.\n' "$os" "$arch"
      ;;
  esac
}

verify_signature() {
  file="$1"
  base_url="$2"
  tmp="$3"
  sig_file="$file.sig"
  pub_file="$tmp/loctree-signing.asc"
  sums_file="$tmp/SHA256SUMS"
  gnupg_home="$tmp/gnupg"
  expected_fingerprint="$(normalize_fingerprint "$GPG_FINGERPRINT")"

  if ! command -v gpg >/dev/null 2>&1; then
    skip_signature_verification "gpg unavailable"
    return 0
  fi
  if ! curl -fsSL "$GPG_KEY_URL" -o "$pub_file" 2>/dev/null; then
    skip_signature_verification "GPG signing key unavailable"
    return 0
  fi
  actual_fingerprint="$(gpg --batch --with-colons --import-options show-only --import "$pub_file" 2>/dev/null | awk -F: '$1 == "fpr" {print $10; exit}')"
  actual_fingerprint="$(normalize_fingerprint "$actual_fingerprint")"
  if [ -z "$actual_fingerprint" ]; then
    red "GPG signing key fingerprint could not be read"
    exit 1
  fi
  if [ "$actual_fingerprint" != "$expected_fingerprint" ]; then
    red "GPG signing key fingerprint mismatch"
    printf 'expected: %s\nactual:   %s\n' "$expected_fingerprint" "$actual_fingerprint"
    exit 1
  fi
  if curl -fsSL "$base_url/SHA256SUMS" -o "$sums_file" 2>/dev/null; then
    file_name="$(basename "$file")"
    expected="$(awk -v name="$file_name" '$2 == name {print $1}' "$sums_file")"
    if [ -n "$expected" ]; then
      actual="$(sha256_file "$file")"
      if [ "$actual" != "$expected" ]; then
        red "checksum mismatch for $(basename "$file")"
        printf 'expected: %s\nactual:   %s\n' "$expected" "$actual"
        exit 1
      fi
      green "SHA256SUMS ok: $actual"
    fi
  fi
  if curl -fsSL "$base_url/$(basename "$sig_file")" -o "$sig_file" 2>/dev/null; then
    mkdir -p "$gnupg_home"
    chmod 700 "$gnupg_home"
    if GNUPGHOME="$gnupg_home" gpg --batch --quiet --import "$pub_file" >/dev/null 2>&1 \
      && GNUPGHOME="$gnupg_home" gpg --batch --quiet --verify "$sig_file" "$file" >/dev/null 2>&1; then
      green "signature ok"
    else
      red "signature verification failed for $(basename "$file")"
      exit 1
    fi
  else
    skip_signature_verification "signature sidecar unavailable"
  fi
}

install_binary_from_payload() {
  bin="$1"
  src="$2"
  install -m 0755 "$src" "$INSTALL_DIR/$bin"
  printf '  %s -> %s\n' "$bin" "$INSTALL_DIR/$bin"
}

install_payload_binaries() {
  payload="$1"
  mkdir -p "$INSTALL_DIR"
  installed_names=""
  installed_count=0

  for bin in $RELEASE_BINARIES; do
    if [ -f "$payload/$bin" ]; then
      install_binary_from_payload "$bin" "$payload/$bin"
      installed_names="$installed_names $bin"
      installed_count=$((installed_count + 1))
    fi
  done

  for bin_path in "$payload"/*; do
    [ -e "$bin_path" ] || continue
    [ -f "$bin_path" ] && [ -x "$bin_path" ] || continue
    bin="${bin_path##*/}"
    case " $installed_names " in
      *" $bin "*) continue ;;
    esac
    install_binary_from_payload "$bin" "$bin_path"
    installed_names="$installed_names $bin"
    installed_count=$((installed_count + 1))
  done

  if [ "$installed_count" -eq 0 ]; then
    red "no installable binaries found in $payload"
    exit 1
  fi
}

install_prebuilt() {
  target="$1"
  archive="loctree-$VERSION-$target.tar.gz"
  url="$BASE_URL/$VERSION/$archive"
  artifact_base="$BASE_URL/$VERSION"
  sha_url="$url.sha256"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  blue "[1/4] downloading release bundle"
  need_cmd curl
  need_cmd tar
  if ! curl -fsSL "$url" -o "$tmp/$archive"; then
    rm -rf "$tmp"
    trap - EXIT
    install_from_cargo "$target"
    return
  fi
  if ! curl -fsSL "$sha_url" -o "$tmp/$archive.sha256"; then
    rm -rf "$tmp"
    trap - EXIT
    install_from_cargo "$target"
    return
  fi

  blue "[2/4] verifying SHA256"
  expected="$(awk '{print $1}' "$tmp/$archive.sha256")"
  actual="$(sha256_file "$tmp/$archive")"
  if [ "$actual" != "$expected" ]; then
    red "checksum mismatch for $archive"
    printf 'expected: %s\nactual:   %s\n' "$expected" "$actual"
    exit 1
  fi
  green "checksum ok: $actual"
  verify_signature "$tmp/$archive" "$artifact_base" "$tmp"

  blue "[3/4] installing binaries"
  tar -xzf "$tmp/$archive" -C "$tmp"
  payload="$tmp/loctree-$VERSION-$target/bin"
  install_payload_binaries "$payload"
}

install_from_cargo() {
  attempted_target="${1:-}"
  blue "[1/4] prebuilt bundle unavailable for this platform"
  if [ "${LOCTREE_ALLOW_SOURCE_FALLBACK:-0}" != "1" ]; then
    if [ -z "$attempted_target" ]; then
      red "no prebuilt Loctree bundle target for this platform"
      printf '\n'
      unsupported_platform_reason
    else
      red "no prebuilt Loctree bundle is available for $attempted_target at version $VERSION"
      printf '\n'
      printf 'This installer is binary-first and does not build the Rust workspace for first-time users.\n'
      printf 'Try LOCTREE_VERSION set to a published version, or the contributor fallback below.\n'
    fi
    printf 'Contributor fallback, if you really want a source install:\n'
    printf '  LOCTREE_ALLOW_SOURCE_FALLBACK=1 curl -fsSL https://loct.io/install.sh | bash\n'
    exit 1
  fi
  need_cmd cargo
  mkdir -p "$CARGO_BIN"

  blue "[2/4] installing contributor fallback from Cargo"
  cargo install loctree --force
  cargo install loctree-mcp --force || true
  cargo install loctree-lsp --force || true
  yellow "aicx/aicx-mcp are distributed via prebuilt bundles; source fallback installs Loctree binaries only"

  mkdir -p "$INSTALL_DIR"
  for bin in loct loctree loctree-mcp loctree-lsp; do
    if [ -x "$CARGO_BIN/$bin" ]; then
      ln -sf "$CARGO_BIN/$bin" "$INSTALL_DIR/$bin" 2>/dev/null || cp "$CARGO_BIN/$bin" "$INSTALL_DIR/$bin"
      printf '  %s -> %s\n' "$bin" "$INSTALL_DIR/$bin"
    fi
  done
}

ensure_path() {
  blue "[4/4] checking PATH"
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) green "$INSTALL_DIR is already in PATH"; return ;;
  esac

  if [ "${LOCTREE_NO_PROFILE_UPDATE:-}" = "1" ]; then
    yellow "$INSTALL_DIR is not in PATH; profile update skipped"
    return
  fi

  profile="$HOME/.zshrc"
  if [ -w "$profile" ] && ! grep -q "loctree installer" "$profile" 2>/dev/null; then
    {
      printf '\n# loctree installer\n'
      # shellcheck disable=SC2016
      printf 'export PATH="%s:$PATH"\n' "$INSTALL_DIR"
    } >>"$profile"
    yellow "added $INSTALL_DIR to $profile; reload your shell or run: source $profile"
  else
    yellow "$INSTALL_DIR is not in PATH"
  fi
}

printf '\n'
blue "Loctree installer"
printf 'version: %s\ninstall: %s\n\n' "$VERSION" "$INSTALL_DIR"

target="$(target_triple)"
if [ -n "$target" ]; then
  install_prebuilt "$target"
else
  install_from_cargo ""
fi

ensure_path

printf '\n'
green "Installation complete"
printf 'try:\n'
printf '  %s/loct --version\n' "$INSTALL_DIR"
printf '  %s/loct scan\n' "$INSTALL_DIR"
printf '  %s/loct --for-ai\n' "$INSTALL_DIR"
printf '\n'
