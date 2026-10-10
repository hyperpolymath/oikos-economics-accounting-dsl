#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# run-mustfile.sh — execute the obligations declared in a contractile.
#
# Why this exists: .machine_readable/contractiles/must/Mustfile.a2ml said
# "Run with: must check" while nothing in this repository provided or ran `must`
# (no `must` executable, no workflow step, no just recipe — grep found only the
# claim). The consequence was measurable: six of its checks named root files that
# had been deleted or relocated, `security-policy` still demanded a root
# `SECURITY.md` that issue #109 removed, and no one noticed, because a contract
# that is never evaluated is decoration. It also asserted directories that this
# repo has never had (`descriptiles/anchors`, `.machine_readable/self-validating`)
# because the file was copied verbatim from rsr-template-repo and never
# instantiated.
#
# So: the declarations are now executable. A check whose command exits non-zero
# is a violation; `severity: critical` violations fail the run, `warning` and
# `advisory` are reported without blocking (the estate's promote-honestly rule —
# arming a gate on a condition that predates it manufactures red noise, while
# leaving the finding unreported manufactures rot).
#
# Usage: run-mustfile.sh [contractile.a2ml]     # default: the must contractile
#        run-mustfile.sh --list                 # ids + severities, run nothing
# Exit:  0 — no critical violation | 1 — critical violation(s) | 2 — setup error
#
# Records are TAB-separated (severity, id, description, run, requires).
#
# Environment:
#   MUST_STRICT=1   an unevaluated check is a violation AT ITS DECLARED
#                   SEVERITY: a critical check skipped for a missing tool fails
#                   the run, a warning-severity one is reported as a warning. CI
#                   sets it, because a check that could not run in CI is a check
#                   that did not run and must not read as green. Strict mode does
#                   NOT escalate severities — see the skip and WARN branches for
#                   why that distinction is load-bearing.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LIST_ONLY=0
if [ "${1:-}" = "--list" ]; then LIST_ONLY=1; shift; fi
FILE="${1:-.machine_readable/contractiles/must/Mustfile.a2ml}"
[ -f "$FILE" ] || { echo "run-mustfile: FATAL: no contractile at $FILE" >&2; exit 2; }

STRICT="${MUST_STRICT:-0}"

# Parse with awk rather than a bash while-loop over the file: the `- run:` bodies
# contain `&&`, quotes and command substitution, and a parser that reads the file
# line-by-line in the same shell that then executes them inherits any quoting
# mistake in the contractile. Emit TAB-separated records; NUL-free by assumption
# on single-line runs (enforced: a `### ` block with a wrapped `- run:` is
# reported as malformed instead of silently half-parsed).
mapfile -t RECORDS < <(awk -v file="$FILE" '
  function flush() {
    if (id != "") {
      if (run == "")
        printf "MALFORMED\t%s\t%s\tno - run: line\n", id, sev
      else
        printf "%s\t%s\t%s\t%s\t%s\n", sev, id, desc, run, req
    }
    id=""; desc=""; sev="advisory"; run=""; req=""
  }
  # Three or more '#' at column 0 is an `### id` block heading (#### too, which
  # the trust/dust contractiles use). NOT /^#{3,4} /: mawk — the default awk on
  # Debian and Ubuntu images, and the one every runner here has for `awk` — has
  # no interval expressions, and a pattern that silently never matches makes this
  # parser report an empty contract as a clean pass. Heading levels are therefore
  # expressed by repetition, not by an interval.
  /^###/ { h = $0; sub(/^#+ +/, "", h); flush(); id = h; next }
  /^- description: / { desc = substr($0, 16); next }
  /^- severity: /    { sev  = substr($0, 13); next }
  /^- run: /         { if (run != "") printf "MALFORMED\t%s\t%s\tmulti-line - run: is not supported\n", id, sev
                       run = substr($0, 8); next }
  /^- requires: /    { req  = substr($0, 13); next }
  END { flush() }
' "$FILE")

[ "${#RECORDS[@]}" -gt 0 ] || { echo "run-mustfile: FATAL: no checks parsed from $FILE" >&2; exit 2; }

fails=0; warns=0; checks=0; malformed=0
FAILED_IDS=""; WARN_IDS=""
printf '%-8s %-34s %s\n' "SEVERITY" "CHECK" "RESULT"
printf '%-8s %-34s %s\n' "--------" "-----" "------"

for rec in "${RECORDS[@]}"; do
  case "$rec" in
    MALFORMED*)
      malformed=$((malformed + 1))
      IFS=$'\t' read -r _ mid mdesc mdetail <<<"$rec"
      printf '%-8s %-34s %s\n' "malformed" "$mid" "cannot parse: $mdetail"
      fails=$((fails + 1)); FAILED_IDS="$FAILED_IDS ${mid}:malformed"
      continue ;;
  esac
  IFS=$'\t' read -r sev id desc run req <<<"$rec"
  [ -n "$id" ] || continue
  checks=$((checks + 1))
  # `- requires: <tool>` declares the check's environment, so that a tool being
  # absent is reported as a SETUP GAP and not as a violated obligation. It is not
  # an escape hatch: the skip is printed, counted, and MUST_STRICT=1 (which CI
  # sets) turns every skip into a failure. The alternative — letting each check
  # start with `command -v X || exit 0` — is how a gate ends up asserting nothing
  # on the machines where nobody has the tool.
  if [ -n "${req:-}" ] && ! command -v "$req" >/dev/null 2>&1; then
    if [ "$STRICT" = "1" ] && [ "$sev" = "critical" ]; then
      printf '%-8s %-34s %s\n' "$sev" "$id" "SKIP (needs $req) — unevaluated, fatal in strict mode"
      fails=$((fails + 1)); FAILED_IDS="$FAILED_IDS $id:skipped-needs-$req"
    else
      # A skip never lifts a check above its declared severity: an unevaluated
      # `warning` is reported as a warning, not laundered into a blocker. That
      # escalation is what reddened this gate's first CI run, where the contract
      # itself records `signed-commits` as declared-unmet (no signing key exists in
      # this repository) — a debt the contract priced as advisory, printed as a
      # failure of the machine rather than of the repo.
      printf '%-8s %-34s %s\n' "$sev" "$id" "SKIP (needs $req)"
      warns=$((warns + 1)); WARN_IDS="$WARN_IDS $id:skipped-needs-$req"
    fi
    continue
  fi
  if [ "$LIST_ONLY" -eq 1 ]; then
    printf '%-8s %-34s %s\n' "$sev" "$id" "(not run)"
    continue
  fi
  if out=$(bash -c "$run" 2>&1); then
    printf '%-8s %-34s %s\n' "$sev" "$id" "ok"
  else
    # A missing tool is not a violated obligation, and reporting it as one would
    # make the local run depend on what happens to be installed. Say which it is.
    first=$(printf '%s' "$out" | head -c 300 | tr '\n' ' ')
    case "$sev" in
      critical)
        printf '%-8s %-34s %s\n' "$sev" "$id" "FAIL"
        [ -n "$desc" ] && printf '%-8s %-34s %s\n' "" "" "$desc"
        printf '%-8s %-34s %s\n' "" "\$ $run" "${first:-no output}"
        fails=$((fails + 1)); FAILED_IDS="$FAILED_IDS $id:failed" ;;
      *)
        printf '%-8s %-34s %s\n' "$sev" "$id" "WARN"
        # `warning`/`advisory` failures are non-blocking in every mode, strict
        # included: strict governs whether a check was EVALUATED, and must not
        # rewrite the contract's own severity ladder into a second set of criticals.
        printf '%-8s %-34s %s\n' "" "\$ $run" "${first:-no output}"
        warns=$((warns + 1)); WARN_IDS="$WARN_IDS $id:warn" ;;
    esac
  fi
done

if [ "$LIST_ONLY" -eq 1 ]; then
  echo "$checks check(s) declared in $FILE"; exit 0
fi

echo
# One workflow command per offender, on both verdicts. A runner's step log is not
# always reachable — the log API redirects to blob storage that some environments
# cannot fetch — and a job whose only signal is "exit 1" then costs an hour of
# guessing. Annotations are the channel that survives, and the API exposes them.
for cid in $WARN_IDS; do echo "::warning::run-mustfile: $cid"; done
if [ "$fails" -gt 0 ]; then
  for cid in $FAILED_IDS; do echo "::error::run-mustfile: $cid"; done
  echo "run-mustfile: FAIL — $fails critical violation(s), $warns warning(s) across $checks check(s)"
  exit 1
fi
echo "run-mustfile: PASS — $checks check(s), 0 critical violations, $warns warning(s)"
[ "$malformed" -gt 0 ] && exit 1
exit 0
