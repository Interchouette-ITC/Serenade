#!/usr/bin/env bash
# Packages the stable serenade-cli release assets for one Rust target (Linux, macOS,
# Windows Git Bash). Used by .github/workflows/release.yml after
# `cargo build --release --locked -p serenade-cli --target <target>`.
#
#   make/release/package-cli.sh <version> <target> <dist dir>
#
# Writes to <dist dir>:
#   serenade-cli-v<version>-<target>.<ext>     the six CLI executables
#   serenade-trunk-v<version>-<target>.<ext>   verified Trunk in serenade-cli's install
#                                              layout (only for targets the fork ships)
# plus a `.sha256` per archive. <ext> is `zip` on Windows, `tar.gz` elsewhere.
#
# Trunk is never rebuilt: the fork's `trunk-itc-dev` archive is checked against its
# published SHA-256 and its build provenance attestation (`gh attestation verify`, needs
# GH_TOKEN), and the binary must report the attested commit, exactly as serenade-cli
# checks it before first use. Distinct names keep these assets apart from the rolling
# `serenade-cli-pre-release` release.
set -euo pipefail

VERSION=${1:?usage: $0 <version> <target> <dist dir>}
TARGET=${2:?usage: $0 <version> <target> <dist dir>}
DIST=${3:?usage: $0 <version> <target> <dist dir>}
CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target}
TRUNK_REPO=${TRUNK_REPO:-Interchouette-ITC/trunk}
TRUNK_RELEASE_URL=${TRUNK_RELEASE_URL:-https://github.com/${TRUNK_REPO}/releases/download/trunk-itc-dev}
BINS=(serenade serenade-spa serenade-html serenade-universal serenade-ssg serenade-trunk)

case "$TARGET" in
  *-windows-*) EXE=.exe EXT=zip ;;
  *) EXE='' EXT=tar.gz ;;
esac
# Targets of the fork's trunk-itc-dev release; mirrors `release_target` in src/trunk.rs.
case "$TARGET" in
  x86_64-unknown-linux-musl | x86_64-apple-darwin | aarch64-apple-darwin) TRUNK_ASSET="trunk-${TARGET}.tar.gz" ;;
  x86_64-pc-windows-msvc) TRUNK_ASSET="trunk-${TARGET}.zip" ;;
  *) TRUNK_ASSET='' ;;
esac

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | awk '{print $1}'
}

# archive <output> <dir> <entries...>: zip on Windows, gzip tarball elsewhere; plus .sha256.
archive() {
  local output=$1 dir=$2 name
  shift 2
  if [ "$EXT" = zip ]; then
    (cd "$dir" && 7z a -tzip -bso0 "$output" "$@")
  else
    tar -czf "$output" -C "$dir" "$@"
  fi
  name=$(basename "$output")
  printf '%s  %s\n' "$(sha256 "$output")" "$name" >"${output}.sha256"
}

mkdir -p "$DIST"
DIST=$(cd "$DIST" && pwd)
BIN_DIR="${CARGO_TARGET_DIR}/${TARGET}/release"
names=()
for bin in "${BINS[@]}"; do
  test -x "${BIN_DIR}/${bin}${EXE}" || {
    echo "missing ${BIN_DIR}/${bin}${EXE}" >&2
    exit 1
  }
  names+=("${bin}${EXE}")
done
reported=$("${BIN_DIR}/serenade${EXE}" --version | tr -d '\r')
[ "$reported" = "serenade ${VERSION}" ] || {
  echo "serenade reports '$reported', expected 'serenade ${VERSION}'" >&2
  exit 1
}
"${BIN_DIR}/serenade-ssg${EXE}" --help >/dev/null
archive "${DIST}/serenade-cli-v${VERSION}-${TARGET}.${EXT}" "$BIN_DIR" "${names[@]}"

if [ -z "$TRUNK_ASSET" ]; then
  echo "no trunk-itc-dev build for ${TARGET}; Trunk must be installed separately there"
  ls -la "$DIST"
  exit 0
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL --retry 3 -o "${work}/${TRUNK_ASSET}" "${TRUNK_RELEASE_URL}/${TRUNK_ASSET}"
TRUNK_SHA256=$(curl -fsSL --retry 3 "${TRUNK_RELEASE_URL}/${TRUNK_ASSET}.sha256" | awk '{print $1}')
[ "$(sha256 "${work}/${TRUNK_ASSET}")" = "$TRUNK_SHA256" ] || {
  echo "checksum mismatch for ${TRUNK_ASSET}" >&2
  exit 1
}
TRUNK_COMMIT=$(gh attestation verify "${work}/${TRUNK_ASSET}" \
  --repo "$TRUNK_REPO" \
  --signer-workflow "${TRUNK_REPO}/.github/workflows/release.yaml" \
  --source-ref refs/heads/main \
  --predicate-type https://slsa.dev/provenance/v1 \
  --format json \
  --jq "[.[].verificationResult.statement.predicate.buildDefinition.resolvedDependencies[]? | select((.uri // \"\") | startswith(\"git+https://github.com/${TRUNK_REPO}@\")) | .digest.gitCommit // empty][0]")
[[ $TRUNK_COMMIT =~ ^[0-9a-f]{40}$ ]] || {
  echo "no attested commit for ${TRUNK_ASSET}" >&2
  exit 1
}

INSTALL_DIR="${work}/tools/trunk-itc-dev"
mkdir -p "${INSTALL_DIR}/bin"
if [ "$EXT" = zip ]; then
  7z e -bso0 -o"${INSTALL_DIR}/bin" "${work}/${TRUNK_ASSET}" "trunk${EXE}"
else
  tar -xzf "${work}/${TRUNK_ASSET}" -C "${INSTALL_DIR}/bin" trunk
fi
TRUNK_BIN="${INSTALL_DIR}/bin/trunk${EXE}"
TRUNK_VERSION=$("$TRUNK_BIN" --version | tr -d '\r')
case "$TRUNK_VERSION" in
  "trunk "?*"+itc."?*) ;;
  *)
    echo "missing fork build revision: $TRUNK_VERSION" >&2
    exit 1
    ;;
esac
case "$TRUNK_COMMIT" in
  "${TRUNK_VERSION##*+itc.}"*) ;;
  *)
    echo "${TRUNK_VERSION} was not built from attested commit ${TRUNK_COMMIT}" >&2
    exit 1
    ;;
esac

# Markers read by serenade-cli (`is_installed` in src/trunk.rs): the install record binds
# the executable's SHA-256 to the release checksum, attested commit and version.
printf '%s\n' "$TRUNK_SHA256" >"${INSTALL_DIR}/.serenade-trunk-sha256"
printf '%s\n' "$TRUNK_VERSION" >"${INSTALL_DIR}/.serenade-trunk-version"
printf 'commit %s\nsigner https://github.com/%s/.github/workflows/release.yaml@refs/heads/main\n' \
  "$TRUNK_COMMIT" "$TRUNK_REPO" >"${INSTALL_DIR}/.serenade-trunk-provenance"
printf '%s\nrelease %s\ncommit %s\nversion %s\n' "$(sha256 "$TRUNK_BIN")" \
  "$TRUNK_SHA256" "$TRUNK_COMMIT" "$TRUNK_VERSION" >"${INSTALL_DIR}/.serenade-trunk-bin-sha256"
archive "${DIST}/serenade-trunk-v${VERSION}-${TARGET}.${EXT}" "${work}/tools" trunk-itc-dev
echo "Trunk: ${TRUNK_VERSION} (sha256 ${TRUNK_SHA256}, provenance verified: ${TRUNK_COMMIT})"
ls -la "$DIST"
