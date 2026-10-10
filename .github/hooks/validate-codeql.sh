#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# Pre-commit hook: Validate the CodeQL language matrix against the repo's
# actual languages.
#
# Rewritten 2026-10-10 while relocating hooks/ -> .github/hooks/ (issue #109
# item 1). The previous copy could not be wired into anything, because it
# exited 1 ON A CLEAN TREE with no output at all:
#
#     set -euo pipefail
#     HAS_JS=$(find . -name "*.js" | grep -v node_modules | head -1)
#
# With `pipefail`, an empty scan makes `grep` exit 1, the substitution propagates
# that status to the assignment, and `set -e` kills the script before any check
# runs. In this repository — a Rust workspace, no JS — that was every run. A
# validator that fails closed on the absence of the thing it looks for is worse
# than no validator: it is why these hooks sat unreferenced in `hooks/` while
# the workflows duplicated their rules.
#
# Probes now tolerate empty results explicitly, and the matrix rules are stated
# as rules rather than as a pile of greps.

set -euo pipefail

CODEQL_FILE=".github/workflows/codeql.yml"
[ -f "$CODEQL_FILE" ] || { echo "note: $CODEQL_FILE absent — nothing to validate"; exit 0; }

# `|| true` on every probe: an empty result is a legitimate answer, not an error.
probe() {  # $@ = find predicates
  local found
  found=$(find . -type f \( "$@" \) -not -path './node_modules/*' -not -path './target/*' \
            -not -path './.git/*' 2>/dev/null | head -1 || true)
  [ -n "$found" ]
}

HAS_JS=0; probe -name '*.js' -o -name '*.ts' -o -name '*.jsx' -o -name '*.tsx' && HAS_JS=1
HAS_PY=0; probe -name '*.py' && HAS_PY=1
HAS_GO=0; probe -name '*.go' && HAS_GO=1
HAS_RS=0; probe -name '*.rs' && HAS_RS=1

# `language: actions` is always permitted: it scans the workflow files
# themselves, which is what every RSR repo owes regardless of source language.
# A declared language with no matching source is a false sense of coverage —
# CodeQL creates an empty database and reports green.
status=0
declared=$(grep -oE '^[[:space:]]*-?[[:space:]]*language:[[:space:]]*[A-Za-z_]+' "$CODEQL_FILE" \
             | sed -E 's/.*language:[[:space:]]*//' | sort -u || true)

for lang in $declared; do
  case "$lang" in
    actions) ;;
    javascript-typescript)
      [ "$HAS_JS" -eq 1 ] || { echo "WARNING: CodeQL declares $lang but no JS/TS source exists"; status=1; } ;;
    python)
      [ "$HAS_PY" -eq 1 ] || { echo "WARNING: CodeQL declares python but no .py source exists"; status=1; } ;;
    go)
      [ "$HAS_GO" -eq 1 ] || { echo "WARNING: CodeQL declares go but no .go source exists"; status=1; } ;;
    java|csharp|ruby|php|kotlin|swift)
      echo "WARNING: CodeQL declares $lang with no matching source in this repo"; status=1 ;;
    rust)
      # Hard error, not a warning: CodeQL has no Rust engine, so the run cannot
      # ever analyse the Rust that this repo is made of.
      echo "ERROR: CodeQL does not support Rust. Use 'language: actions' (build-mode: none) and rely on cargo clippy / cargo audit for the crates."
      status=1 ;;
    *)
      echo "WARNING: unrecognised CodeQL language '$lang' in $CODEQL_FILE"; status=1 ;;
  esac
done

if [ "$status" -ne 0 ]; then
  echo "Fix the matrix in $CODEQL_FILE: every declared language must have source, and Rust must not be declared."
fi
exit "$status"
