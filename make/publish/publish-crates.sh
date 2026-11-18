#!/usr/bin/env bash
# Package or publish the Serenade workspace crates to crates.io in dependency order.
#
#   make/publish/publish-crates.sh check [cargo package args]  package + verify, never uploads
#   make/publish/publish-crates.sh publish                     upload versions not yet on crates.io
#   make/publish/publish-crates.sh versions <tag>              tag + version sources match, no upload
#
# Operator entry: `make publish-check`; uploads run from the stable release workflow.
#
# `publish` is safe to rerun after a partial failure: every package/version is looked up on
# crates.io before upload, already published versions are skipped (but still awaited in the
# sparse index before dependents are published), and HTTP 429 rate limits are waited out
# using the reset time reported by crates.io. It needs CARGO_REGISTRY_TOKEN and never passes
# --no-verify.
set -euo pipefail

# Split command prefixes once, without glob expansion (e.g. CARGO="cargo +stable").
read -r -a CARGO_CMD <<<"${CARGO:-cargo}"
read -r -a CURL_CMD <<<"${CURL:-curl}"
read -r -a SLEEP_CMD <<<"${SLEEP:-sleep}"
INDEX_ATTEMPTS=${INDEX_ATTEMPTS:-60}
INDEX_INTERVAL=${INDEX_INTERVAL:-10}
# Upload attempts per package (rate-limited lookups and uploads both count).
PUBLISH_MAX_ATTEMPTS=${PUBLISH_MAX_ATTEMPTS:-6}
# Wait used when a 429 carries no usable reset time.
RATE_LIMIT_FALLBACK_DELAY=${RATE_LIMIT_FALLBACK_DELAY:-120}
# Added to every reported reset time.
RATE_LIMIT_SAFETY_MARGIN=${RATE_LIMIT_SAFETY_MARGIN:-5}
# Upper bound for all rate-limit waits of one run (default 5h, below the 6h job limit).
RATE_LIMIT_MAX_TOTAL_WAIT=${RATE_LIMIT_MAX_TOTAL_WAIT:-18000}
API_URL=${CRATES_IO_API_URL:-https://crates.io/api/v1/crates}
INDEX_URL=${CRATES_IO_INDEX_URL:-https://index.crates.io}
USER_AGENT="serenade-release (https://github.com/Interchouette-ITC/Serenade)"
# Bounds every request so a stalled connection cannot hang the release.
CURL_TIMEOUTS=(--connect-timeout 10 --max-time 30)

# Dependencies before dependents (normal, build, and versioned dev-dependencies);
# `check_members` verifies this order against `cargo metadata`.
DEFAULT_CRATES=(
  serenade-component-host serenade-contracts serenade-di serenade-expression
  serenade-filesystem serenade-finder serenade-http serenade-kernel serenade-messenger
  serenade-openapi serenade-process serenade-sandbox serenade-search serenade-serializer
  serenade-string
  serenade-cache serenade-config serenade-console serenade-event serenade-http-actix
  serenade-http-axum serenade-http-client serenade-lock serenade-mailer serenade-minify
  serenade-notifier serenade-observability serenade-profiler serenade-rate-limiter
  serenade-session serenade-validator serenade-workflow
  serenade-scheduler serenade-security
  serenade-bundle serenade-form
  serenade-admin serenade-translation serenade-view serenade-testing
  serenade-cli
  serenade-framework
)
if [ -n "${PUBLISH_CRATES:-}" ]; then
  read -r -a CRATES <<<"$PUBLISH_CRATES"
else
  CRATES=("${DEFAULT_CRATES[@]}")
fi

TOTAL_WAITED=0

die() {
  echo "error: $*" >&2
  exit 1
}

validate_config() {
  ((${#CARGO_CMD[@]} > 0)) || die "CARGO must contain a command"
  ((${#CURL_CMD[@]} > 0)) || die "CURL must contain a command"
  ((${#SLEEP_CMD[@]} > 0)) || die "SLEEP must contain a command"
  local name value
  for name in INDEX_ATTEMPTS INDEX_INTERVAL PUBLISH_MAX_ATTEMPTS RATE_LIMIT_FALLBACK_DELAY \
    RATE_LIMIT_SAFETY_MARGIN RATE_LIMIT_MAX_TOTAL_WAIT; do
    value=${!name}
    [[ $value =~ ^[0-9]+$ ]] || die "$name must be a non-negative integer (got '$value')"
  done
  for name in INDEX_ATTEMPTS PUBLISH_MAX_ATTEMPTS RATE_LIMIT_FALLBACK_DELAY; do
    ((${!name} >= 1)) || die "$name must be at least 1"
  done
}

now_epoch() {
  if [ -n "${PUBLISH_NOW:-}" ]; then
    echo "$PUBLISH_NOW"
  else
    date -u +%s
  fi
}

workspace_version() {
  awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p&&/^version = /{gsub(/"/,"",$3); print $3; exit}' Cargo.toml
}

# Fails if the publish list and the publishable workspace members drift apart, if a member
# does not carry the workspace version, or if a package is listed before a workspace
# dependency that crates.io must already serve when it is uploaded.
check_members() {
  local version=$1
  "${CARGO_CMD[@]}" metadata --no-deps --format-version 1 --locked |
    python3 -c '
import json, sys
version, order = sys.argv[1], sys.argv[2:]
packages = {p["name"]: p for p in json.load(sys.stdin)["packages"]}
publishable = {n for n, p in packages.items() if p.get("publish") != []}
if len(set(order)) != len(order):
    sys.exit("publish list contains duplicates")
if set(order) != publishable:
    missing, extra = sorted(publishable - set(order)), sorted(set(order) - publishable)
    sys.exit(f"publish list drifted from workspace: missing {missing}, not publishable {extra}")
position = {name: i for i, name in enumerate(order)}
for name in order:
    package = packages[name]
    if package["version"] != version:
        sys.exit(f"{name} is {package['\''version'\'']}, expected {version}")
    for dep in package["dependencies"]:
        if dep["name"] not in packages or dep["name"] == name:
            continue
        # Unversioned dev-dependencies are stripped from the published manifest.
        if dep["kind"] == "dev" and dep["req"] == "*":
            continue
        if dep["name"] not in position:
            sys.exit(f"{name} depends on unpublished workspace package {dep['\''name'\'']}")
        if position[dep["name"]] > position[name]:
            sys.exit(f"{name} is listed before its dependency {dep['\''name'\'']}")
' "$version" "${CRATES[@]}"
}

# Fails unless the release tag is `v<workspace version>` and every application version source
# (nested example workspaces, `serenade new` template, shipped recipes, changelog) carries
# that same version.
check_release_sources() {
  local version=$1 tag=$2
  [ "$tag" = "v$version" ] || die "release tag $tag does not match workspace version $version (expected v$version)"
  python3 -c '
import glob, sys, tomllib
version = sys.argv[1]
errors = []
def load(path):
    with open(path, "rb") as f:
        return tomllib.load(f)
for path in sorted(glob.glob("examples/*/Cargo.toml")):
    found = load(path).get("workspace", {}).get("package", {}).get("version")
    if found is not None and found != version:
        errors.append(f"{path}: workspace version {found}")
template = "crates/serenade-cli/templates/app/Cargo.toml.tmpl"
for name, spec in load(template).get("dependencies", {}).items():
    found = spec if isinstance(spec, str) else spec.get("version")
    if name.startswith("serenade") and found != version:
        errors.append(f"{template}: {name} = {found!r}")
for path in sorted(glob.glob("crates/serenade-cli/recipes/*/recipe.toml")):
    for dep in load(path).get("cargo", {}).get("dependencies", []):
        name, found = dep["crate"], dep.get("version")
        if name.startswith("serenade") and found != version:
            errors.append(f"{path}: {name} version {found!r}")
with open("docs/CHANGELOG.md", encoding="utf-8") as f:
    if f"## [{version}]" not in f.read():
        errors.append(f"docs/CHANGELOG.md: no ## [{version}] section")
if errors:
    sys.exit("version sources disagree with " + version + ":\n  " + "\n  ".join(errors))
' "$version"
  # Nested example workspaces have their own lockfiles; they must already record this version.
  local manifest
  for manifest in examples/*/Cargo.toml; do
    [ -f "$(dirname "$manifest")/Cargo.lock" ] || continue
    "${CARGO_CMD[@]}" metadata --locked --format-version 1 --manifest-path "$manifest" >/dev/null ||
      die "$(dirname "$manifest")/Cargo.lock is out of date (run cargo update -w there)"
  done
}

# Prints the epoch at which a rate limit resets, read from a `Retry-After` header (seconds
# or HTTP date) or from crates.io's "try again after <date>" message. Prints nothing when
# no usable reset time is present.
parse_reset_epoch() {
  local file=$1 value parsed
  value=$(tr -d '\r' <"$file" | sed -n 's/^[Rr]etry-[Aa]fter:[[:space:]]*//p' | head -n 1)
  if [[ $value =~ ^[0-9]+$ ]]; then
    echo $(($(now_epoch) + value))
    return 0
  fi
  if [ -z "$value" ]; then
    value=$(tr -d '\r' <"$file" | tr '\n' ' ' |
      sed -n 's/.*[Tt]ry again after \(.*\(GMT\|UTC\|Z\|[+-][0-9][0-9]:\{0,1\}[0-9][0-9]\)\).*/\1/p' |
      head -n 1)
  fi
  [ -n "$value" ] || return 0
  parsed=$(date -u -d "$value" +%s 2>/dev/null) || return 0
  echo "$parsed"
}

is_rate_limited() {
  grep -Eqi '(^|[^0-9])429([^0-9]|$)|too many requests' "$1"
}

# Waits out one rate limit described by the response in $1, bounded by
# RATE_LIMIT_MAX_TOTAL_WAIT across the whole run.
rate_limit_wait() {
  local file=$1 context=$2 reset delay now
  reset=$(parse_reset_epoch "$file")
  now=$(now_epoch)
  if [ -n "$reset" ]; then
    delay=$((reset - now + RATE_LIMIT_SAFETY_MARGIN))
    ((delay >= 1)) || delay=1
    echo "$context: rate limited by crates.io until $(date -u -d "@$reset" '+%Y-%m-%dT%H:%M:%SZ'); waiting ${delay}s"
  else
    delay=$RATE_LIMIT_FALLBACK_DELAY
    echo "$context: rate limited by crates.io without a reset time; waiting ${delay}s"
  fi
  if ((TOTAL_WAITED + delay > RATE_LIMIT_MAX_TOTAL_WAIT)); then
    die "$context: rate-limit wait of ${delay}s would exceed RATE_LIMIT_MAX_TOTAL_WAIT=${RATE_LIMIT_MAX_TOTAL_WAIT}s (already waited ${TOTAL_WAITED}s); rerun the publication later to resume"
  fi
  TOTAL_WAITED=$((TOTAL_WAITED + delay))
  "${SLEEP_CMD[@]}" "$delay"
}

# Sets REGISTRY_STATUS to 200 (version published) or 404 (not published). Waits out 429s
# (consuming the caller's attempt budget through ATTEMPTS) and fails on anything else.
registry_lookup() {
  local name=$1 version=$2 headers body
  headers=$(mktemp)
  body=$(mktemp)
  while true; do
    REGISTRY_STATUS=$("${CURL_CMD[@]}" "${CURL_TIMEOUTS[@]}" -sS -o "$body" -D "$headers" -w '%{http_code}' \
      -A "$USER_AGENT" "$API_URL/$name/$version") || REGISTRY_STATUS=000
    case "$REGISTRY_STATUS" in
      200 | 404) break ;;
      429)
        ATTEMPTS=$((ATTEMPTS + 1))
        if ((ATTEMPTS > PUBLISH_MAX_ATTEMPTS)); then
          rm -f "$headers" "$body"
          die "$name $version: still rate limited after $PUBLISH_MAX_ATTEMPTS attempts"
        fi
        cat "$headers" "$body" >"$body.all"
        rate_limit_wait "$body.all" "$name $version lookup"
        rm -f "$body.all"
        ;;
      *)
        rm -f "$headers" "$body"
        die "crates.io returned HTTP $REGISTRY_STATUS for $name $version"
        ;;
    esac
  done
  rm -f "$headers" "$body"
}

wait_for_index() {
  local name=$1 version=$2 attempt body path
  case ${#name} in
    1) path="1/$name" ;;
    2) path="2/$name" ;;
    3) path="3/${name:0:1}/$name" ;;
    *) path="${name:0:2}/${name:2:2}/$name" ;;
  esac
  for ((attempt = 1; attempt <= INDEX_ATTEMPTS; attempt++)); do
    # Read the whole entry first: piping into `grep -q` under pipefail can fail
    # with SIGPIPE even when the version is present.
    body=$("${CURL_CMD[@]}" "${CURL_TIMEOUTS[@]}" -fsS -A "$USER_AGENT" "$INDEX_URL/$path" 2>/dev/null) || body=''
    if [[ $body == *"\"vers\":\"${version}\""* ]]; then
      echo "$name $version is available in the index"
      return 0
    fi
    "${SLEEP_CMD[@]}" "$INDEX_INTERVAL"
  done
  die "$name $version not in the crates.io index after $((INDEX_ATTEMPTS * INDEX_INTERVAL))s"
}

publish_one() {
  local name=$1 version=$2 log status
  ATTEMPTS=0
  log=$(mktemp)
  while true; do
    registry_lookup "$name" "$version"
    if [ "$REGISTRY_STATUS" = 200 ]; then
      # A previous attempt may have uploaded it without the client seeing the response;
      # dependents must not be published until the index serves it.
      echo "$name $version already on crates.io; skipping upload"
      break
    fi
    ATTEMPTS=$((ATTEMPTS + 1))
    if ((ATTEMPTS > PUBLISH_MAX_ATTEMPTS)); then
      rm -f "$log"
      die "$name $version: not published after $PUBLISH_MAX_ATTEMPTS attempts"
    fi
    echo "Publishing $name $version (attempt $ATTEMPTS/$PUBLISH_MAX_ATTEMPTS)"
    set +e
    "${CARGO_CMD[@]}" publish -p "$name" --locked 2>&1 | tee "$log"
    status=${PIPESTATUS[0]}
    set -e
    if [ "$status" -eq 0 ]; then
      break
    fi
    if is_rate_limited "$log"; then
      rate_limit_wait "$log" "$name $version publish"
      continue
    fi
    # The upload may have reached crates.io even though cargo reported a failure.
    registry_lookup "$name" "$version"
    if [ "$REGISTRY_STATUS" = 200 ]; then
      echo "$name $version is on crates.io despite the cargo error; continuing"
      break
    fi
    rm -f "$log"
    die "cargo publish failed for $name $version (exit $status)"
  done
  rm -f "$log"
  wait_for_index "$name" "$version"
}

publish() {
  local version=$1 name
  [ -n "${CARGO_REGISTRY_TOKEN:-}" ] || die "CARGO_REGISTRY_TOKEN is not set"
  for name in "${CRATES[@]}"; do
    publish_one "$name" "$version"
  done
  echo "All ${#CRATES[@]} packages are published at $version"
}

main() {
  local mode=${1:-} version name args=()
  case "$mode" in
    check | publish) ;;
    versions) [ -n "${2:-}" ] || die "usage: $0 versions <tag>" ;;
    *)
      echo "usage: $0 check [cargo package args] | publish | versions <tag>" >&2
      exit 2
      ;;
  esac
  validate_config
  cd "${PUBLISH_ROOT:-$(dirname "$0")/../..}"
  version=$(workspace_version)
  [ -n "$version" ] || die "no [workspace.package] version"
  check_members "$version"
  case "$mode" in
    check)
      # Verifies every package against a temporary local registry, so dependents build
      # even before their dependencies are on crates.io. Never uploads.
      for name in "${CRATES[@]}"; do
        args+=(-p "$name")
      done
      "${CARGO_CMD[@]}" package --locked "${args[@]}" "${@:2}"
      echo "Packaged and verified ${#CRATES[@]} packages at $version"
      ;;
    publish) publish "$version" ;;
    versions)
      check_release_sources "$version" "$2"
      echo "Release $2: ${#CRATES[@]} packages and all version sources are at $version"
      ;;
  esac
}

main "$@"
