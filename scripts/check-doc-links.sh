#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# check-doc-links.sh — every RELATIVE link in the prose must resolve to a path
# that exists.
#
# Why this exists in this repository specifically: issue #109 item 1 relocated
# a dozen root documents (AFFIRMATION.adoc, EXPLAINME.adoc, GOVERNANCE.adoc,
# MAINTAINERS.adoc, PALIMPSEST.adoc, ARCHITECTURE.adoc, spec/SPEC.adoc,
# SECURITY.md + SECURITY.adoc, CODE_OF_CONDUCT.adoc, the root ROADMAP.adoc
# pointer). A move is half of a move; the other half is the links. This repo's
# `www/.well-known/security.txt` pointed at a DIFFERENT repository for months
# and no gate noticed — the same defect one level up: a pointer nobody checks.
#
# Checked forms:
#   * AsciiDoc   link:TARGET[TEXT]
#   * Markdown   [TEXT](TARGET)  and  [TEXT]: TARGET
# Targets resolve relative to the file containing the link, then relative to the
# repository root (the AsciiDoc convention this repo's docs use). Anything that
# is not a repo-relative path — http(s):, mailto:, anchors, attribute-bearing
# targets — is skipped, and the skip count is printed so "nothing was checked"
# can never masquerade as "everything passed".
#
# `--self-test` plants a dangling link in a scratch fixture and requires a FAIL.
# Adopted from the template's doctrine (see tests/shape/check_root_shape_test.sh
# and .github/workflows/lock-sync-gate.yml): a gate that has only ever returned
# green is indistinguishable from a gate that does not work.
#
# Exit: 0 all relative links resolve | 1 a link is dangling | 2 setup error.

set -uo pipefail

if [ "${1:-}" = "--self-test" ]; then
  FIX=$(mktemp -d); trap 'rm -rf "$FIX"' EXIT
  git -C "$FIX" init -q
  git -C "$FIX" config user.name fixture
  git -C "$FIX" config user.email fixture@example.invalid
  mkdir -p "$FIX/docs"
  printf '= Top\n\nlink:docs/child.adoc[child]\nlink:https://example.com[external]\n' > "$FIX/README.adoc"
  printf '= Child\n\nlink:gone.adoc[missing]\n' > "$FIX/docs/child.adoc"
  git -C "$FIX" add -A
  self() { bash "$(dirname "$0")/check-doc-links.sh" "$@"; }
  if self "$FIX" >/dev/null 2>&1; then
    echo "check-doc-links --self-test: FAIL — a dangling link did not red the gate" >&2; exit 1
  fi
  out=$(self "$FIX" 2>&1) || true
  grep -q 'gone.adoc' <<<"$out" || { echo "check-doc-links --self-test: FAIL — the verdict did not name the dangling target" >&2; echo "$out" >&2; exit 1; }
  grep -q 'docs/child.adoc' <<<"$out" || { echo "check-doc-links --self-test: FAIL — the verdict did not name the containing file" >&2; exit 1; }
  grep -qF 'README.adoc ->' <<<"$out" && { echo "check-doc-links --self-test: FAIL — a resolving link was reported as dangling" >&2; exit 1; }
  printf '= Child\n\nlink:../README.adoc[top]\n' > "$FIX/docs/child.adoc"
  git -C "$FIX" add -A
  self "$FIX" >/dev/null || { echo "check-doc-links --self-test: FAIL — a resolving link did not green the gate" >&2; exit 1; }
  echo "check-doc-links --self-test: PASS (fails on a dangling link, names it, passes when fixed)"
  exit 0
fi

[ "${1:-}" = "--help" ] && { sed -n '2,36p' "$0"; exit 0; }

ROOT="${1:-.}"
[ -d "$ROOT" ] || { echo "check-doc-links: FATAL: no such directory: $ROOT" >&2; exit 2; }

extract() { # $1 = file; prints raw link targets, one per line
  {
    grep -oE 'link:[^[]+\[[^]]*\]' "$1" 2>/dev/null | sed -E 's/^link://; s/\[.*$//'
    grep -oE '\]\([^) ]+'           "$1" 2>/dev/null | sed -E 's/^\]\(//'
    grep -oE '^\[[^]]+\]:[[:space:]]*[^[:space:]]+' "$1" 2>/dev/null | sed -E 's/^\[[^]]+\]:[[:space:]]*//'
  } | sed -E 's/[[:space:]]+$//' | sort -u || true
}

is_skipped() { # $1 = target; 0 = not a repo-relative path, do not check
  case "$1" in
    ''|'#'*|'{'*|'$'*|'!'*|'['*) return 0 ;;        # empty, anchor, attribute, image/include
    http://*|https://*|mailto:*|ftp:*|irc:*|data:*|xref:*) return 0 ;;
  esac
  return 1
}

exists_relative() { # $1 = containing dir, $2 = target (anchor already stripped)
  [ -e "$1/$2" ] || [ -e "$ROOT/$2" ]
}

FILES=()
while IFS= read -r -d '' f; do FILES+=("$f"); done < <(
  cd "$ROOT" && { git ls-files -z '*.adoc' '*.md' '*.markdown' 2>/dev/null || true; }
)
if [ "${#FILES[@]}" -eq 0 ]; then
  # Fall back to a plain scan outside a git worktree (a local rehearsal of this
  # script must not be able to pass by finding nothing).
  while IFS= read -r -d '' f; do FILES+=("${f#./}"); done < <(
    cd "$ROOT" && find . -type f \( -name '*.adoc' -o -name '*.md' \) -not -path './.git/*' -print0 2>/dev/null
  )
fi
[ "${#FILES[@]}" -gt 0 ] || { echo "check-doc-links: FATAL: no .adoc/.md files under $ROOT" >&2; exit 2; }

total=0; checked=0; failed=0; report=""
for f in "${FILES[@]}"; do
  while IFS= read -r t; do
    [ -n "$t" ] || continue
    total=$((total + 1))
    is_skipped "$t" && continue
    target="${t%%#*}"                       # strip trailing anchor
    [ -n "$target" ] || continue
    checked=$((checked + 1))
    if ! exists_relative "$ROOT/$(dirname "$f")" "$target"; then
      failed=$((failed + 1))
      report+="  $f -> $t"$'\n'
    fi
  done < <(extract "$ROOT/$f")
done

if [ "$failed" -gt 0 ]; then
  echo "check-doc-links: FAIL — $failed dangling link(s) ($checked checked, $total seen):" >&2
  printf '%s' "$report" >&2
  echo "Fix the link, or move the target back. A dangling link is how a relocated" >&2
  echo "document stops existing for everyone who followed it." >&2
  exit 1
fi
echo "check-doc-links: PASS — $checked relative link(s) resolve across ${#FILES[@]} file(s); $((total - checked)) external/anchor/attribute target(s) skipped"
exit 0
