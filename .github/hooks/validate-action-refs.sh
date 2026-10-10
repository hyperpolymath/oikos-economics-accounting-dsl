#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
# Pre-commit hook: validate-action-refs.sh — every `uses:` ref in a workflow
# must be RESOLVABLE from .github/workflows/actions.lock.
#
# Renamed from validate-sha-pins.sh on 2026-10-10 (issue #109 item 1, together
# with the hooks/ -> .github/hooks/ move), because the check it performs is not
# "is there a 40-hex after the @" any more. A name that describes the old rule
# would be a stale signpost on the way to the fix.
#
# The previous copy enforced "actions must use SHA pins":
#
#     if ! echo "$line" | grep -qE '@[a-f0-9]{40}'
#
# That rule contradicts this repository's actual supply-chain model. Oikos pins
# by VERSION in the YAML (`actions/checkout@v7.0.1`) and records the immutable
# SHA, once, in `actions.lock` — which is also what carries the transitive
# closure of the `hyperpolymath/standards` reusable workflows (see #109 item 2
# and .github/workflows/lock-sync-gate.yml). On the tree as it stands the old
# hook reported 27 "unpinned actions", every one of them correctly locked.
# A hook whose only output is false alarms gets ignored, and then the rule it
# claims to enforce is not enforced anywhere.
#
# So this file no longer asks "is there a SHA after the @". It asks the question
# GitHub itself asks at run start — "is this ref in the lockfile?" — by
# delegating to scripts/check-lock-sync.sh, the same validator the CI gate runs.
# One rule, one implementation, no divergence between local and CI.

set -euo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)"

CHECK="scripts/check-lock-sync.sh"
if [ ! -x "$CHECK" ] && [ ! -f "$CHECK" ]; then
  echo "ERROR: $CHECK is missing. The lockfile gate cannot be skipped locally:" >&2
  echo "restore it, or commit with --no-verify and expect Lock Sync Gate to fail in CI." >&2
  exit 1
fi

# No capability probe here on purpose. check-lock-sync.sh probes for an awk with
# 3-argument match() itself and exits 1 with an install hint if there is none —
# a local hook that cannot run must not report success. Duplicating that probe
# would give two places to keep in step and one more way for local and CI to
# disagree.
bash "$CHECK"
