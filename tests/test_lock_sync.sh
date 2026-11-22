#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# Lock parser and trust-boundary regressions. Run: bash tests/test_lock_sync.sh
# (`just test-lock-sync`). Needs gawk, like scripts/check-lock-sync.sh.
#
# It proves that check-lock-sync.sh's clauses can fail, and that the Lock Sync
# Gate (.github/workflows/lock-sync-gate.yml) never executes PR-controlled code.
# The gate's own run block is extracted from the workflow and rehearsed against
# fixture repositories, exactly as the previous Python suite did. The ten test
# names are unchanged.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VALIDATOR_SRC="$ROOT/scripts/check-lock-sync.sh"
WORKFLOW_FILE="$ROOT/.github/workflows/lock-sync-gate.yml"
WORKFLOW="$(cat "$WORKFLOW_FILE")"

TMP_ALL="$(mktemp -d)"
trap 'rm -rf -- "${TMP_ALL:?}"' EXIT

# The gate's run block: everything after the first `        run: |` line, with
# the ten-space indent of the block removed.
RUN="$(sed -n '/^        run: |$/,$p' "$WORKFLOW_FILE" | tail -n +2 | sed 's/^          //')"
[ -n "$RUN" ] || { echo "cannot extract the run block from $WORKFLOW_FILE" >&2; exit 2; }

LOCK="version: 'v0.0.2'
workflows:
    '.github/workflows/test.yml':
        - 'example/action@v1'
dependencies:
    'example/action@v1':
        uses:
"
YAML=$'jobs:\n  test:\n    steps:\n      - uses: example/action@v1\n'

# ---- assertions: a failure prints why and ends the current test --------------

die() { echo "    ASSERT: $*" >&2; exit 1; }
assert_eq() { [ "$1" = "$2" ] || die "expected '$2', got '$1' ($3)"; }
assert_in() { case "$2" in *"$1"*) ;; *) die "'$1' not found in: $2";; esac; }
assert_not_in() { case "$2" in *"$1"*) die "'$1' unexpectedly found in: $2";; *) ;; esac; }

# ---- fixture -----------------------------------------------------------------

fixture() {
  root="$(mktemp -d "$TMP_ALL/fx.XXXXXX")"
  wf="$root/.github/workflows"
  mkdir -p "$wf"
  printf '%s' "$YAML" > "$wf/test.yml"
  printf '%s' "$LOCK" > "$wf/actions.lock"
}

validate() { # sets out (stdout) and rc; stderr is appended to out for messages
  out="$(bash "$VALIDATOR_SRC" "$wf" 2>/dev/null)"; rc=$?
}

# ---- LockSyncTests -----------------------------------------------------------

test_nested_string_forms_and_transitive_closure() {
  fixture
  local ref nested closed
  for ref in "'example/child@v2'" '"example/child@v2"' "example/child@v2"; do
    nested="${LOCK}            - ${ref} # nested dependency"$'\n'
    printf '%s' "$nested" > "$wf/actions.lock"
    validate
    assert_eq "$rc" 1 "dangling edge, ref $ref"
    assert_in "DANGLING EDGES" "$out"
    assert_in "example/child@v2" "$out"
    assert_in "named by: dependencies:example/action@v1" "$out"
    closed="${nested}    'example/child@v2': {}"$'\n'
    printf '%s' "$closed" > "$wf/actions.lock"
    validate
    assert_eq "$rc" 0 "closed edge, ref $ref"
    printf '%s' "${nested}    'example/child@v2':"$'\n'"        uses:"$'\n'"            - \"example/grandchild@v3\""$'\n' > "$wf/actions.lock"
    validate
    assert_eq "$rc" 1 "transitive edge, ref $ref"
    assert_in "named by: dependencies:example/child@v2" "$out"
  done
}

test_existing_validation_clauses() {
  fixture
  local yaml lock diag i
  local -a yamls locks diags
  yamls=("${YAML//@v1/@v2}" $'jobs: {}\n' "$YAML" $'jobs: {}\n')
  locks=("$LOCK" "$LOCK" "${LOCK%%dependencies:*}" $'workflows:\ndependencies:\n')
  diags=("refs missing from the lockfile" "stale lockfile entries" "DANGLING EDGES" "UNLISTED WORKFLOWS")
  for i in 0 1 2 3; do
    yaml="${yamls[$i]}"; lock="${locks[$i]}"; diag="${diags[$i]}"
    printf '%s' "$yaml" > "$wf/test.yml"
    printf '%s' "$lock" > "$wf/actions.lock"
    validate
    assert_eq "$rc" 1 "$diag"
    assert_in "$diag" "$out"
  done
}

# ---- GateTests ---------------------------------------------------------------

gate_setup() {
  fixture
  export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 \
         GIT_AUTHOR_NAME=Test GIT_AUTHOR_EMAIL=test@example.invalid \
         GIT_COMMITTER_NAME=Test GIT_COMMITTER_EMAIL=test@example.invalid
  g init -q
  mkdir "$root/scripts"
  cp "$VALIDATOR_SRC" "$root/scripts/check-lock-sync.sh"
  base="$(commit)"
  marker="$root/pr-code-executed"
  printf "touch '%s'\nexit 0\n" "$marker" > "$root/scripts/check-lock-sync.sh"
  head="$(commit)"
  merge
}

g() { git -C "$root" "$@"; }
commit() { g add . >/dev/null; g commit -qm fixture; g rev-parse HEAD; }
merge() {
  local tree sha
  g add . >/dev/null
  tree="$(g write-tree)"
  sha="$(g commit-tree "$tree" -p "$base" -p "$head" -m merge)"
  g update-ref refs/pull/1/merge "$sha"
}

# gate [pr|nopr] [head-sha]: runs the extracted run block; sets out, err, rc
gate() {
  local mode="${1:-pr}" prhead="${2:-$head}" runner prnum=1
  [ "$mode" = nopr ] && prnum=""
  runner="$(mktemp -d "$TMP_ALL/runner.XXXXXX")"
  out="$(cd "$runner" && env RUNNER_TEMP="$runner" REPO=fixture/repo SHA="$base" \
          PR_NUMBER="$prnum" PR_HEAD="$prhead" TOKEN=synthetic \
          GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0="url.file://$root.insteadOf" \
          GIT_CONFIG_VALUE_0="https://github.com/fixture/repo.git" \
          bash -c "$RUN" 2>"$runner/stderr")"; rc=$?
  err="$(cat "$runner/stderr")"
  [ ! -e "$marker" ] || die "PR-controlled script was executed"
}

test_trusted_workflow_configuration() {
  local code
  assert_in $'\n  pull_request_target:\n' "$WORKFLOW"
  assert_not_in $'\n  pull_request:\n' "$WORKFLOW"
  assert_in "github.event.pull_request.number || github.ref" "$WORKFLOW"
  code="$(grep -v '^[[:space:]]*#' "$WORKFLOW_FILE")"
  assert_not_in "uses:" "$code"
}

test_valid_merge_ignores_replaced_validator() {
  gate_setup; gate
  assert_eq "$rc" 0 "$out $err"
}

test_merge_data_is_validated_even_when_head_is_clean() {
  gate_setup
  printf '%s' "${YAML//@v1/@v2}" > "$wf/test.yml"
  merge; gate
  assert_eq "$rc" 1 "$out $err"
  assert_in "refs missing from the lockfile" "$out"
}

test_deleted_pr_validator_is_ignored() {
  gate_setup
  rm "$root/scripts/check-lock-sync.sh"
  merge; gate
  assert_eq "$rc" 0 "$out $err"
}

test_symlink_data_is_rejected() {
  gate_setup
  rm "$wf/test.yml"
  ln -s ../../scripts/check-lock-sync.sh "$wf/test.yml"
  merge; gate
  [ "$rc" -ne 0 ] || die "symlink data was accepted"
  assert_in "must be regular files" "$err"
}

test_stale_merge_is_rejected() {
  gate_setup; gate pr "$base"
  [ "$rc" -ne 0 ] || die "stale merge was accepted"
  assert_in "does not match" "$out"
}

test_missing_merge_is_rejected() {
  gate_setup
  g update-ref -d refs/pull/1/merge
  gate
  [ "$rc" -ne 0 ] || die "missing merge ref was accepted"
}

test_push_and_dispatch_use_event_sha() {
  gate_setup; gate nopr
  assert_eq "$rc" 0 "$out $err"
}

# ---- runner ------------------------------------------------------------------

tests=(
  test_nested_string_forms_and_transitive_closure
  test_existing_validation_clauses
  test_trusted_workflow_configuration
  test_valid_merge_ignores_replaced_validator
  test_merge_data_is_validated_even_when_head_is_clean
  test_deleted_pr_validator_is_ignored
  test_symlink_data_is_rejected
  test_stale_merge_is_rejected
  test_missing_merge_is_rejected
  test_push_and_dispatch_use_event_sha
)

failed=0
for t in "${tests[@]}"; do
  if ( set -u; "$t" ); then
    echo "$t ... ok"
  else
    echo "$t ... FAIL"; failed=$((failed + 1))
  fi
done
echo
echo "Ran ${#tests[@]} tests, $failed failed"
[ "$failed" -eq 0 ] && echo OK || { echo FAILED; exit 1; }
