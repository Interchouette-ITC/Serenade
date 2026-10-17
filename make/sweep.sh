#!/bin/sh
# Content sweep over tracked product files (git grep only).
# Fails on Unicode em/en dashes and forbidden clippy allows under crates/.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

status=0

fail() {
	echo "SWEEP FAIL: $1" >&2
	status=1
}

ok() {
	echo "$1: zero hits"
}

check_dashes() {
	if git grep -nE '—|–' -- \
		'*.md' '*.rs' '*.toml' '*.yml' \
		'Makefile' '*.mk' >/dev/null; then
		fail "em or en dash"
		git grep -nE '—|–' -- \
			'*.md' '*.rs' '*.toml' '*.yml' \
			'Makefile' '*.mk' | head -20 >&2
	else
		ok dashes
	fi
}

check_forbidden_allows() {
	if git grep -nE 'allow\(clippy::(too_many_arguments|too_many_lines|dead_code)\)' -- \
		crates >/dev/null; then
		fail "forbidden clippy allow under crates/"
		git grep -nE 'allow\(clippy::(too_many_arguments|too_many_lines|dead_code)\)' -- \
			crates | head -20 >&2
	else
		ok forbidden-allows
	fi
}

check_dashes
check_forbidden_allows

exit "$status"
