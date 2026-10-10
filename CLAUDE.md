<!--
SPDX-License-Identifier: CC-BY-SA-4.0
Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
-->
## Machine-Readable Artefacts

The following files in `.machine_readable/descriptiles/` contain structured project metadata (a2ml format; the prior `.scm` Guile-Scheme files were retired estate-wide).

> `.a2ml` is itself being retired in favour of repo-deed clauses
> (rsr-template-repo#209). The rule for agents: **do not create new `.a2ml`
> files**, and do not delete the ones below — they retire in the commit that
> mints this repo's `oikos-economics-accounting-dsl_chora.deed`, which has not
> happened. Deleting state before the replacement exists leaves the repo with no
> machine-readable state at all.

- `STATE.a2ml` - Current project state and progress
- `META.a2ml` - Architecture decisions and development practices
- `ECOSYSTEM.a2ml` - Position in the ecosystem and related projects
- `AGENTIC.a2ml` - AI agent interaction patterns
- `NEUROSYM.a2ml` - Neurosymbolic integration config
- `PLAYBOOK.a2ml` - Operational runbook

---

# CLAUDE.md - AI Assistant Instructions

## Repository shape and gates (read first)

The root layout is **enforced**, not conventional:

* `.machine_readable/root-allow.txt` allowlists the root bidirectionally;
  `scripts/check-root-shape.sh` checks it (CI: **Estate Rules**). A new root file
  needs a justified line there — moving it into `docs/`, `build/`, `ci/`,
  `scripts/`, `tests/` or `.github/` is nearly always the better answer.
* `.github/workflows/actions.lock` must cover every `uses:` ref, transitively:
  `scripts/check-lock-sync.sh` (CI: **Lock Sync Gate**). A desync is not cosmetic —
  GitHub refuses the run at startup with zero jobs and no log. Dependabot rewrites
  the YAML and cannot touch the lockfile, which is how #105 and #112 broke `main`.
* `scripts/check-doc-links.sh` requires every relative link in the prose to resolve.
* `scripts/run-mustfile.sh` executes the repo's declared obligations
  (`.machine_readable/contractiles/must/Mustfile.a2ml`).

Run them all locally with `just verify` (needs `gawk` for the lock check). Install
the push/commit-time versions with `just hooks-install`.

Rust source lives **only** in the four crates: there is no root `src/` and there
must not be one.

## Language Policy (Hyperpolymath Standard)

> **Scope (corrected 2026-10-10, issue #109).** This repository is **one** product: the
> **Oikos DSL**, in **Rust**, as four workspace crates (`oikos-syntax`, `oikos-parser`,
> `oikos-desugar`, `oikos-check`). There is no `bot-integration-affine/` and no
> `analyzers/` directory here — those lines of this file described OikosBot, which
> was extracted to
> https://github.com/hyperpolymath/oikosbot[`hyperpolymath/oikosbot`] in the
> 2026-05-28 legacy-shutoff (oikos#41). Read that repository's own `CLAUDE.md`
> before touching AffineScript or the ReScript→AffineScript port; do not act on
> bot rules from here.
>
> MPL-1.0 / MPL-1.0-or-later are banned estate-wide; rewrite to MPL-2.0 wherever
> encountered (DR-010 supersedes DR-002).

### ALLOWED Languages & Tools

| Language/Tool | Use Case | Notes |
|---------------|----------|-------|
| **Rust** | Oikos DSL compiler crates | Repo root; performance-critical |
| **AffineScript** (`.affine`) | *not in this repo* — OikosBot (`hyperpolymath/oikosbot`) | Listed so an agent does not re-add it here |
| **Haskell** | *not in this repo* | Same reason |
| **Bun** | JS runtime & package management (tier 1) | Default for all new work. Runs compiled ESM/JS directly — no bundler step. Uses an npm-compatible `package.json` plus `bun.lock` — both are expected, not anti-patterns. |
| **Tauri 2.0+** | Mobile apps (iOS/Android) | Rust backend + web UI |
| **Dioxus** | Mobile apps (native UI) | Pure Rust, React-like |
| **Gleam** | Backend services | Runs on BEAM or compiles to JS |
| **Bash/POSIX Shell** | Scripts, automation | Keep minimal |
| **Nickel** | Configuration language | For complex configs |
| **A2ML** | State/meta files | `.machine_readable/descriptiles/*.a2ml` |
| **Julia** | Batch scripts, data processing | Per RSR |
| **OCaml** | AffineScript compiler upstream | Not in this repo (lives in `hyperpolymath/affinescript`) |
| **Ada** | Safety-critical systems | Where required |

### BANNED - Do Not Use

| Banned | Replacement |
|--------|-------------|
| TypeScript | AffineScript (for OikosBot) / Rust (for DSL) |
| **ReScript** (new files) | **AffineScript** — the legacy `bot-integration/` ReScript was removed 2026-05-28 (oikos#41) |
| JavaScript (new files) | AffineScript |
| Deno | Bun |
| Node.js | Bun |
| npm | Bun |
| pnpm/yarn | Bun |
| Go | Rust |
| Python | Julia/Rust/AffineScript |
| Java/Kotlin | Rust/Tauri/Dioxus |
| Swift | Tauri/Dioxus |
| React Native | Tauri/Dioxus |
| Flutter/Dart | Tauri/Dioxus |

### Mobile Development

**No exceptions for Kotlin/Swift** - use Rust-first approach:

1. **Tauri 2.0+** - Web UI (AffineScript) + Rust backend, MIT/Apache-2.0
2. **Dioxus** - Pure Rust native UI, MIT/Apache-2.0

Both are FOSS with independent governance (no Big Tech).

### Enforcement Rules

1. **No new TypeScript files** - This repo is Rust; write new code in Rust
2. **No new ReScript files** - Retired 2026-05-28 (oikos#41). The AffineScript port lives in the separate `hyperpolymath/oikosbot` repository, not here
3. **JS runtime, if any, is Bun** (`package.json` + `bun.lock`). There is no JS in this repo today
4. **`bun install --production` for production deps** - resolved from `package.json`, pinned via `bun.lock`
5. **No Go code** - Use Rust instead
6. **No Python anywhere** - Use Julia for data/batch, Rust for systems
7. **No Kotlin/Swift for mobile** - Use Tauri 2.0+ or Dioxus
8. **MPL-1.0 / MPL-1.0-or-later are non-conforming** - Rewrite to MPL-2.0 (DR-010)

### Package Management

- **Primary**: Guix (guix.scm)
- **Fallback**: Nix (flake.nix)
- **JS deps**: Bun (`package.json` + `bun.lock`). Declare tooling as a devDependency and run `bunx --no-install --bun <tool>` — a bare `bunx <tool>` can fetch an unpinned package and may start Node via its shebang.

### Security Requirements

- No MD5/SHA1 for security (use SHA256+)
- HTTPS only (no HTTP URLs)
- No hardcoded secrets
- SHA-pinned dependencies
- SPDX license headers on all files

