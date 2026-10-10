#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Point this clone's git hooks at .github/hooks/ so the local checks run before
# a commit leaves, and wire the commit-message template. Idempotent; safe to
# re-run. Mirrors rsr-template-repo's .github/hooks/install.sh (same two
# settings, same reason: a hook that is not installed is documentation, not a
# gate).

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

git config core.hooksPath .github/hooks
git config commit.template .gitmessage
chmod +x .github/hooks/pre-commit .github/hooks/validate-*.sh 2>/dev/null || true

echo "Installed:"
echo "  core.hooksPath   -> .github/hooks   (pre-commit gate active)"
echo "  commit.template  -> .gitmessage"
echo
echo "Uninstall with: git config --unset core.hooksPath && git config --unset commit.template"
