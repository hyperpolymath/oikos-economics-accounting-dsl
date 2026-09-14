<!--
SPDX-License-Identifier: CC-BY-SA-4.0
Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
-->
## Machine-Readable Artefacts

The following files in `.machine_readable/descriptiles/` contain structured project metadata (a2ml format; the prior `.scm` Guile-Scheme files were retired estate-wide):

- `STATE.a2ml` - Current project state and progress
- `META.a2ml` - Architecture decisions and development practices
- `ECOSYSTEM.a2ml` - Position in the ecosystem and related projects
- `AGENTIC.a2ml` - AI agent interaction patterns
- `NEUROSYM.a2ml` - Neurosymbolic integration config
- `PLAYBOOK.a2ml` - Operational runbook

---

# CLAUDE.md - AI Assistant Instructions

## Language Policy (Hyperpolymath Standard)

> **Policy refresh 2026-05-28**: This repo has two products. (1) The **Oikos DSL** at the repo root is **Rust** (compiler crates `oikos-syntax`, `oikos-parser`, `oikos-desugar`, `oikos-check`); (2) **OikosBot** at `bot-integration-affine/` is **AffineScript** as of the 2026-05-28 legacy-shutoff (oikos#41) — the previous ReScript implementation at `bot-integration/` was removed in the same PR. Write no new ReScript or TypeScript here. MPL-1.0 / MPL-1.0-or-later are banned; rewrite to MPL-2.0 wherever encountered (DR-010 supersedes DR-002).

### ALLOWED Languages & Tools

| Language/Tool | Use Case | Notes |
|---------------|----------|-------|
| **Rust** | Oikos DSL compiler crates | Repo root; performance-critical |
| **AffineScript** (`.affine`) | OikosBot (`bot-integration-affine/`) | Affine types, dependent types, row polymorphism, extensible effects |
| **Haskell** | OikosBot analyser backend (`analyzers/`) | Existing surface; pure analysis |
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

1. **No new TypeScript files** - Write new code in AffineScript
2. **No new ReScript files** - Legacy `bot-integration/` ReScript was retired 2026-05-28 (oikos#41); the AS port lives at `bot-integration-affine/`
3. **Use `package.json` + `bun.lock` for JS runtime deps** - Bun is npm-compatible; a manifest is REQUIRED
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

