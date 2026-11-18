#!/usr/bin/env bash
# Tests for make/publish/publish-crates.sh against mocked cargo, curl, sleep, and clock.
# Nothing touches the network or crates.io. Run with `make publish-test`.
set -euo pipefail

SCRIPT="$(cd "$(dirname "$0")/.." && pwd)/publish-crates.sh"
PASSED=0
FAILED=0
# 2026-11-18T14:00:00Z, the reset time used by the crates.io-style messages below.
RESET_EPOCH=1795010400
RESET_TEXT="Wed, 18 Nov 2026 14:00:00 GMT"

setup() {
  T=$(mktemp -d)
  mkdir -p "$T/bin" "$T/ws"
  printf '[workspace.package]\nversion = "1.0.0"\n' >"$T/ws/Cargo.toml"
  cat >"$T/metadata.json" <<'JSON'
{"packages": [
  {"name": "alpha", "version": "1.0.0", "publish": null, "dependencies": []},
  {"name": "beta", "version": "1.0.0", "publish": null,
   "dependencies": [{"name": "alpha", "kind": null, "req": "^1.0.0"}]},
  {"name": "demo", "version": "1.0.0", "publish": [],
   "dependencies": [{"name": "beta", "kind": null, "req": "*"}]}
]}
JSON
  : >"$T/calls"
  : >"$T/sleeps"

  cat >"$T/bin/pop" <<'SH'
#!/usr/bin/env bash
# Prints and removes the first scripted action of $1 (nothing if none are left).
[ -s "$1" ] || exit 0
head -n 1 "$1"
sed -i 1d "$1"
SH

  cat >"$T/bin/cargo" <<'SH'
#!/usr/bin/env bash
T=${MOCK_DIR:?}
if [[ ${1:-} == +* ]]; then
  echo "toolchain $1" >>"$T/calls"
  shift
fi
case "$1" in
  metadata)
    if [[ " $* " == *" --manifest-path "* ]]; then
      echo "metadata ${*: -1}" >>"$T/calls"
      [ ! -e "$T/stale-lock" ] || { echo "error: the lock file needs to be updated" >&2; exit 101; }
    else
      cat "$T/metadata.json"
    fi ;;
  package) echo "package ${*:2}" >>"$T/calls" ;;
  publish)
    name=$3
    echo "publish $name" >>"$T/calls"
    action=$("$T/bin/pop" "$T/publish.$name")
    case "${action:-ok}" in
      ok) touch "$T/published.$name"; echo "Uploaded $name" ;;
      429:*)
        echo "error: failed to publish $name to registry at https://crates.io" >&2
        echo "Caused by: the remote server responded with an error (status 429 Too Many Requests): ${action#429:}" >&2
        exit 101 ;;
      429landed:*)
        touch "$T/published.$name"
        echo "Caused by: the remote server responded with an error (status 429 Too Many Requests): ${action#429landed:}" >&2
        exit 101 ;;
      lost) touch "$T/published.$name"; echo "error: [28] Timeout was reached" >&2; exit 101 ;;
      fail:*) echo "error: ${action#fail:}" >&2; exit 101 ;;
    esac ;;
  *) echo "unexpected cargo $*" >&2; exit 99 ;;
esac
SH

  cat >"$T/bin/curl" <<'SH'
#!/usr/bin/env bash
T=${MOCK_DIR:?}
headers=/dev/null body=/dev/stdout url=
while [ $# -gt 0 ]; do
  case "$1" in
    -D) headers=$2; shift ;;
    -o) body=$2; shift ;;
    -w | -A | --connect-timeout | --max-time) shift ;;
    http*) url=$1 ;;
  esac
  shift
done
case "$url" in
  */api/v1/crates/*)
    rest=${url#*/api/v1/crates/}
    name=${rest%%/*}
    echo "lookup $name" >>"$T/calls"
    action=$("$T/bin/pop" "$T/lookup.$name")
    case "$action" in
      "429 retry-after:"*)
        printf 'HTTP/2 429\r\nretry-after: %s\r\n\r\n' "${action#429 retry-after:}" >"$headers"
        echo '{"errors":[{"detail":"rate limited"}]}' >"$body"; printf 429 ;;
      429) printf 'HTTP/2 429\r\n\r\n' >"$headers"; echo '{}' >"$body"; printf 429 ;;
      [0-9][0-9][0-9]) printf 'HTTP/2 %s\r\n\r\n' "$action" >"$headers"; printf '%s' "$action" ;;
      *)
        if [ -e "$T/published.$name" ]; then printf 200; else printf 404; fi ;;
    esac ;;
  *)
    name=${url##*/}
    echo "index $name" >>"$T/calls"
    action=$("$T/bin/pop" "$T/index.$name")
    if [ "$action" != miss ] && [ -e "$T/published.$name" ]; then
      echo "{\"name\":\"$name\",\"vers\":\"1.0.0\",\"deps\":[]}"
    else
      exit 22
    fi ;;
esac
SH

  cat >"$T/bin/sleep" <<'SH'
#!/usr/bin/env bash
echo "$1" >>"${MOCK_DIR:?}/sleeps"
SH
  chmod +x "$T/bin/"*
}

# Runs the publisher with the mocks; captures OUTPUT and STATUS.
run() {
  set +e
  OUTPUT=$(env MOCK_DIR="$T" CARGO="$T/bin/cargo" CURL="$T/bin/curl" SLEEP="$T/bin/sleep" \
    PUBLISH_ROOT="$T/ws" PUBLISH_CRATES="alpha beta" PUBLISH_NOW="$((RESET_EPOCH - 100))" \
    CARGO_REGISTRY_TOKEN="${TOKEN-test-token}" INDEX_INTERVAL=3 RATE_LIMIT_FALLBACK_DELAY=7 \
    "$@" "$SCRIPT" "${MODE:-publish}" ${TAG:+"$TAG"} 2>&1)
  STATUS=$?
  set -e
}

fail() {
  echo "    $*"
  echo "    --- output ---"
  printf '%s\n' "$OUTPUT" | sed 's/^/    /'
  echo "    --- calls: $(tr '\n' ' ' <"$T/calls")"
  echo "    --- sleeps: $(tr '\n' ' ' <"$T/sleeps")"
  return 1
}

expect_status() { [ "$STATUS" -eq "$1" ] || fail "expected exit $1, got $STATUS"; }
expect_output() { [[ $OUTPUT == *"$1"* ]] || fail "output lacks: $1"; }
expect_calls() {
  local actual
  actual=$(grep -E "^${2:-publish} " "$T/calls" | tr '\n' ' ' | sed 's/ $//') || true
  [ "$actual" = "$1" ] || fail "expected ${2:-publish} calls '$1', got '$actual'"
}
expect_sleeps() {
  local actual
  actual=$(tr '\n' ' ' <"$T/sleeps" | sed 's/ $//')
  [ "$actual" = "$1" ] || fail "expected sleeps '$1', got '$actual'"
}

test_already_published_packages_are_skipped() {
  touch "$T/published.alpha" "$T/published.beta"
  run
  expect_status 0
  expect_output "alpha 1.0.0 already on crates.io; skipping upload"
  expect_output "beta 1.0.0 already on crates.io; skipping upload"
  expect_calls ""
  expect_calls "index alpha index beta" index
}

test_successful_publication_in_dependency_order() {
  run
  expect_status 0
  expect_calls "publish alpha publish beta"
  expect_output "All 2 packages are published at 1.0.0"
  expect_sleeps ""
}

test_index_propagation_delay_is_awaited() {
  printf 'miss\nmiss\n' >"$T/index.alpha"
  run
  expect_status 0
  expect_sleeps "3 3"
  expect_calls "index alpha index alpha index alpha index beta" index
}

test_index_timeout_fails() {
  printf 'miss\nmiss\nmiss\n' >"$T/index.alpha"
  run INDEX_ATTEMPTS=2
  expect_status 1
  expect_output "alpha 1.0.0 not in the crates.io index after 6s"
  expect_calls "publish alpha"
}

test_publish_429_waits_until_reported_reset() {
  echo "429:You have published too many new crates in a short period of time. Please try again after $RESET_TEXT or email help@crates.io to have your limit increased." >"$T/publish.alpha"
  run
  expect_status 0
  expect_sleeps "105"
  expect_calls "publish alpha publish alpha publish beta"
  expect_output "rate limited by crates.io until 2026-11-18T14:00:00Z; waiting 105s"
}

test_registry_lookup_429_honours_retry_after() {
  echo "429 retry-after:30" >"$T/lookup.alpha"
  run
  expect_status 0
  expect_sleeps "35"
  expect_calls "publish alpha publish beta"
  expect_output "alpha 1.0.0 lookup: rate limited"
}

test_missing_reset_time_uses_fallback_delay() {
  echo "429:Too Many Requests" >"$T/publish.alpha"
  echo 429 >"$T/lookup.beta"
  run
  expect_status 0
  expect_sleeps "7 7"
  expect_output "without a reset time; waiting 7s"
}

test_rate_limited_upload_that_landed_is_not_repeated() {
  echo "429landed:Please try again after $RESET_TEXT" >"$T/publish.alpha"
  run
  expect_status 0
  expect_calls "publish alpha publish beta"
  expect_output "alpha 1.0.0 already on crates.io; skipping upload"
}

test_ambiguous_upload_failure_already_published_is_skipped() {
  echo lost >"$T/publish.alpha"
  run
  expect_status 0
  expect_calls "publish alpha publish beta"
  expect_output "alpha 1.0.0 is on crates.io despite the cargo error; continuing"
}

test_retry_limit_stops_publication() {
  printf '429:Too Many Requests\n%.0s' 1 2 3 >"$T/publish.alpha"
  run PUBLISH_MAX_ATTEMPTS=2
  expect_status 1
  expect_output "alpha 1.0.0: not published after 2 attempts"
  expect_calls "publish alpha publish alpha"
}

test_lookup_retry_limit_stops_publication() {
  printf '429\n429\n429\n' >"$T/lookup.alpha"
  run PUBLISH_MAX_ATTEMPTS=2
  expect_status 1
  expect_output "alpha 1.0.0: still rate limited after 2 attempts"
  expect_calls ""
}

test_total_wait_limit_stops_publication() {
  echo "429:Please try again after $RESET_TEXT" >"$T/publish.alpha"
  run RATE_LIMIT_MAX_TOTAL_WAIT=60
  expect_status 1
  expect_output "would exceed RATE_LIMIT_MAX_TOTAL_WAIT=60s"
  expect_sleeps ""
}

test_non_429_publish_error_fails_immediately() {
  echo "fail:failed to verify package tarball" >"$T/publish.alpha"
  run
  expect_status 1
  expect_output "cargo publish failed for alpha 1.0.0 (exit 101)"
  expect_calls "publish alpha"
  expect_sleeps ""
}

test_non_429_lookup_error_fails_immediately() {
  echo 500 >"$T/lookup.alpha"
  run
  expect_status 1
  expect_output "crates.io returned HTTP 500 for alpha 1.0.0"
  expect_calls ""
}

test_missing_token_fails_before_upload() {
  TOKEN='' run
  expect_status 1
  expect_output "CARGO_REGISTRY_TOKEN is not set"
  expect_calls ""
  expect_calls "" lookup
}

test_invalid_configuration_is_rejected() {
  run PUBLISH_MAX_ATTEMPTS=abc
  expect_status 1
  expect_output "PUBLISH_MAX_ATTEMPTS must be a non-negative integer (got 'abc')"
  run RATE_LIMIT_FALLBACK_DELAY=0
  expect_status 1
  expect_output "RATE_LIMIT_FALLBACK_DELAY must be at least 1"
  run RATE_LIMIT_MAX_TOTAL_WAIT=-5
  expect_status 1
  expect_output "RATE_LIMIT_MAX_TOTAL_WAIT must be a non-negative integer"
  expect_calls ""
}

test_package_list_drift_is_rejected() {
  run PUBLISH_CRATES="alpha"
  expect_status 1
  expect_output "publish list drifted from workspace: missing ['beta']"
  expect_calls ""
}

test_dependency_order_is_enforced() {
  run PUBLISH_CRATES="beta alpha"
  expect_status 1
  expect_output "beta is listed before its dependency alpha"
}

test_version_mismatch_is_rejected() {
  sed -i 's/"name": "beta", "version": "1.0.0"/"name": "beta", "version": "0.9.0"/' "$T/metadata.json"
  run
  expect_status 1
  expect_output "beta is 0.9.0, expected 1.0.0"
}

test_check_mode_packages_without_uploading() {
  MODE=check TOKEN='' run
  expect_status 0
  expect_calls "package --locked -p alpha -p beta" package
  expect_calls ""
  expect_calls "" lookup
}

test_check_mode_preserves_cargo_toolchain_prefix() {
  MODE=check TOKEN='' run CARGO="$T/bin/cargo +stable"
  expect_status 0 || return
  expect_calls "toolchain +stable toolchain +stable" toolchain || return
  expect_calls "package --locked -p alpha -p beta" package || return
  expect_calls "" || return
  expect_calls "" lookup
}

test_command_prefix_arguments_are_not_glob_expanded() {
  cat >"$T/bin/command-prefix" <<'SH'
#!/usr/bin/env bash
if [ "$1" != 'literal*' ]; then
  echo "unexpected command prefix argument: $1" >&2
  exit 99
fi
shift
exec "$@"
SH
  chmod +x "$T/bin/command-prefix"
  touch "$T/ws/literal-expanded"
  echo '429:Too Many Requests' >"$T/publish.alpha"
  echo miss >"$T/index.alpha"
  run CARGO="$T/bin/command-prefix literal* $T/bin/cargo" \
    CURL="$T/bin/command-prefix literal* $T/bin/curl" \
    SLEEP="$T/bin/command-prefix literal* $T/bin/sleep"
  expect_status 0 || return
  expect_calls "publish alpha publish alpha publish beta" || return
  expect_sleeps "7 3"
}

test_empty_command_prefixes_are_rejected() {
  local command
  for command in CARGO CURL SLEEP; do
    run "$command= "
    expect_status 1 || return
    expect_output "$command must contain a command" || return
    expect_calls "" || return
    expect_calls "" lookup || return
  done
}

release_sources() {
  mkdir -p "$T/ws/examples/demo" "$T/ws/crates/serenade-cli/templates/app" \
    "$T/ws/crates/serenade-cli/recipes/framework" "$T/ws/docs"
  printf '[workspace.package]\nversion = "%s"\n' "$1" >"$T/ws/examples/demo/Cargo.toml"
  : >"$T/ws/examples/demo/Cargo.lock"
  printf '[dependencies]\nserenade-kernel = "%s"\nserde = "1"\n' "$1" \
    >"$T/ws/crates/serenade-cli/templates/app/Cargo.toml.tmpl"
  printf '[cargo]\ndependencies = [{ crate = "serenade-di", version = "%s" }]\n' "$1" \
    >"$T/ws/crates/serenade-cli/recipes/framework/recipe.toml"
  printf '# Changelog\n\n## [%s]\n' "$1" >"$T/ws/docs/CHANGELOG.md"
}

test_versions_mode_accepts_matching_release() {
  release_sources 1.0.0
  MODE=versions TAG=v1.0.0 TOKEN='' run CARGO="$T/bin/cargo +stable"
  expect_status 0
  expect_output "Release v1.0.0: 2 packages and all version sources are at 1.0.0"
  expect_calls "toolchain +stable toolchain +stable" toolchain || return
  expect_calls "metadata examples/demo/Cargo.toml" metadata
  expect_calls ""
  expect_calls "" lookup
}

test_versions_mode_rejects_tag_and_source_drift() {
  release_sources 1.0.0
  MODE=versions TAG=v1.0.1 TOKEN='' run
  expect_status 1
  expect_output "release tag v1.0.1 does not match workspace version 1.0.0"
  release_sources 0.9.0
  MODE=versions TAG=v1.0.0 TOKEN='' run
  expect_status 1
  expect_output "examples/demo/Cargo.toml: workspace version 0.9.0"
  expect_output "Cargo.toml.tmpl: serenade-kernel = '0.9.0'"
  expect_output "recipe.toml: serenade-di version '0.9.0'"
  expect_output "docs/CHANGELOG.md: no ## [1.0.0] section"
}

test_versions_mode_rejects_stale_example_lockfile() {
  release_sources 1.0.0
  touch "$T/stale-lock"
  MODE=versions TAG=v1.0.0 TOKEN='' run
  expect_status 1
  expect_output "examples/demo/Cargo.lock is out of date"
}

mapfile -t tests < <(declare -F | awk '$3 ~ /^test_/ {print $3}')
if [ "${#tests[@]}" -eq 0 ]; then
  echo "publish-crates tests: no test_ functions discovered" >&2
  exit 1
fi
for test in "${tests[@]}"; do
  setup
  if (set -e; "$test"); then
    echo "ok   $test"
    PASSED=$((PASSED + 1))
  else
    echo "FAIL $test"
    FAILED=$((FAILED + 1))
  fi
  rm -rf "$T"
done
echo "publish-crates tests: $PASSED passed, $FAILED failed"
[ "$FAILED" -eq 0 ]
