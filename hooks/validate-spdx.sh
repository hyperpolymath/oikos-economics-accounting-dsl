#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# Pre-commit hook: Validate SPDX headers in workflow files
#
# The identifier is looked for in the file's LEADING COMMENT BLOCK, not on
# line 1: `gh actions-lock` prepends `# This workflow is managed by gh
# actions-lock.` whenever it mints a lockfile, which displaces the SPDX line
# to line 2. A line-1 test re-fails every workflow on every lockfile refresh
# and invites a "fix" that prepends a second, possibly wrong, licence. This
# matches the canonical check in rsr-template-repo's workflow-linter.yml and
# standards' governance-reusable.yml.

set -euo pipefail

ERRORS=0

for workflow in .github/workflows/*.yml .github/workflows/*.yaml; do
    [ -f "$workflow" ] || continue

    # Read the leading run of comment lines (tolerating a YAML document
    # marker), stopping at the first line of the workflow body.
    if ! awk '/^---[[:space:]]*$/ { next } /^#/ { print; next } { exit }' "$workflow" \
         | grep -qE "^# SPDX-License-Identifier:"; then
        echo "ERROR: Missing SPDX header in $workflow"
        echo "  The leading comment block should carry the MPL-2.0 SPDX licence identifier"
        ERRORS=$((ERRORS + 1))
    fi
done

if [ $ERRORS -gt 0 ]; then
    exit 1
fi

exit 0
