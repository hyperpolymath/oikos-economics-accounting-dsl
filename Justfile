# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Oikos — SFC economic modelling DSL
# justfile: project automation recipes

# ── Configuration ──────────────────────────────────────────────────────────────

# Minimum supported Rust version
msrv := "1.83"

# Cargo invocation — override to add e.g. `--target wasm32-unknown-unknown`
cargo := "cargo"

# ── Primary targets ────────────────────────────────────────────────────────────

# Run all checks: format, lint, build, test (the full local CI gate)
all: fmt check build test

# Check: clippy + deny + doc generation
check:
    {{ cargo }} clippy --workspace --all-targets --all-features -- -D warnings
    {{ cargo }} doc --workspace --no-deps --document-private-items

# Build all workspace crates in release mode
build:
    {{ cargo }} build --workspace --release

# Run the full test suite
test:
    {{ cargo }} test --workspace

# Format all Rust source files (check only — use `fmt-fix` to apply)
fmt:
    {{ cargo }} fmt --all --check

# Apply formatting in-place
fmt-fix:
    {{ cargo }} fmt --all

# ── Development helpers ────────────────────────────────────────────────────────

# Fast development build (debug, no doc, no clippy)
dev:
    {{ cargo }} build --workspace

# Watch for changes and re-run tests
watch:
    cargo watch -x "test --workspace"

# Generate documentation and open in browser
docs:
    {{ cargo }} doc --workspace --no-deps --document-private-items --open

# ── Quality gates ─────────────────────────────────────────────────────────────

# Check for security advisories in dependencies
audit:
    cargo audit

# Check for outdated dependencies
outdated:
    cargo outdated --root-deps-only

# Run minimum supported Rust version check
msrv-check:
    cargo +{{ msrv }} check --workspace

# ── Housekeeping ──────────────────────────────────────────────────────────────

# Remove build artefacts
clean:
    {{ cargo }} clean

# Print workspace dependency tree
tree:
    {{ cargo }} tree --workspace

# Verify SPDX licence headers (requires `reuse` tool)
reuse-check:
    reuse lint

secret-scan-trufflehog:
    @command -v trufflehog >/dev/null && trufflehog filesystem . --only-verified || true

# ── Estate gates ────────────────────────────────────────────────────────────
# The checks the "Estate Rules", "Lock Sync Gate" and "Workflow Security Linter"
# workflows run. None of them needs a Rust toolchain, so run them before a push,
# not after a red CI.

# Verify the repository root against .machine_readable/root-allow.txt.
# Bidirectional: a root file that is not allowlisted fails, and an allowlisted
# REQUIRED entry that is missing fails. Exit 2 means the allowlist is absent,
# which is a setup error and not a pass.
check-root-shape:
    ./scripts/check-root-shape.sh .

# Prove the root-shape gate can FAIL. A gate that has only ever printed PASS is
# indistinguishable from a gate that is not wired up, which is how this repo
# shipped a root with no allowlist at all (issue #109 item 1).
check-root-shape-self-test:
    ./tests/shape/check_root_shape_test.sh

# Verify .github/workflows/actions.lock covers every `uses:` ref, owns no
# orphans, and is transitively closed. Needs an awk with 3-argument match()
# (gawk); the script says so and exits 1 rather than reporting a false pass.
check-lock-sync:
    ./scripts/check-lock-sync.sh

# Lock parser + trust-boundary regression suite (tests/test_lock_sync.py):
# proves check-lock-sync.sh's clauses and that the Lock Sync Gate never
# executes PR-controlled code. Python is a recorded carve-out for exactly this
# suite and the gate's data pump (see .machine_readable/rsr-profile.a2ml);
# the ban on new Python files is untouched. Like `check-lock-sync`, the suite
# calls scripts/check-lock-sync.sh and therefore needs gawk.
test-lock-sync:
    python3 -m unittest discover -s tests -v

# Every relative link in the prose must resolve to a path that exists.
check-doc-links:
    ./scripts/check-doc-links.sh .

# Documentation-link gate self-test (a planted dangling link must redden it).
check-doc-links-self-test:
    ./scripts/check-doc-links.sh --self-test

# Execute the repository's own contractile (declared obligations).
# MUST_STRICT=1 makes it strict, as in CI: a check skipped for a missing tool is
# a failure there, not a pass.
mustfile:
    MUST_STRICT=1 ./scripts/run-mustfile.sh

# All repo-level gates in one go — this is what CI enforces regardless.
# test-lock-sync needs gawk (same as check-lock-sync); run the gawk-less subset
# on a machine without it and say so rather than treating a skip as a pass.
verify: check-root-shape check-root-shape-self-test check-doc-links check-doc-links-self-test check-lock-sync test-lock-sync mustfile

# Wire this clone's git hooks to .github/hooks/ (pre-commit) and to .gitmessage.
hooks-install:
    bash .github/hooks/install.sh
