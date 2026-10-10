<!--
SPDX-License-Identifier: CC-BY-SA-4.0
SPDX-FileCopyrightText: 2024-2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
-->

# Security Policy

This is the single security policy for `oikos-economics-accounting-dsl` (the
Oikos stock-flow consistent economics DSL). It follows *Rhodium Standard Repo
(RSR)* principles: reversibility, attestation, and a minimal attack surface.

> **History.** Until 2026-10-10 this repository carried two security files — a
> root `SECURITY.md` pointer and a root `SECURITY.adoc`. The `.adoc` described
> **SustainaBot / scarcity-bot** (waste metrics, BOINC audit nodes, cloud
> backup memories, `SCARCITY_API_KEY`), which is not this project: it was
> inherited when the repository was re-purposed. Both files are gone; this one
> is canonical and describes only what is true here. The lineage is recorded in
> link:../docs/PALIMPSEST.adoc[`docs/PALIMPSEST.adoc`].

## Supported versions

| Version | Supported | Notes |
|---|---|---|
| 0.x.x | :white_check_mark: | Only the latest minor version receives security updates. |

Oikos is pre-1.0 (`0.1.0-dev`); there are no LTS lines and no backport
commits. Severity triage assumes a moving `main`.

## Reporting a vulnerability

**Do not report vulnerabilities publicly** (no GitHub or GitLab issues, no
discussions, no PR review comments).

1. **GitHub private security advisory** — preferred, for traceability:
   <https://github.com/hyperpolymath/oikos-economics-accounting-dsl/security/advisories/new>
2. **Email** — `security@hyperpolymath.org` (for urgent or sensitive
   disclosures; state the repository name in the subject line).

### What to include

* **Type of issue** — e.g. memory unsafety, denial of service via a crafted
  model file, supply-chain tampering, secret exposure in CI.
* **Affected components** — crate and source paths (e.g.
  `oikos-parser/src/lexer.rs`), plus the commit SHA, tag, or branch.
* **Reproduction** — the offending `.oikos` source (minimal form), the
  `cargo` / `just` invocation, and any config or workflow fragment involved.
* **Impact** — what an attacker gains, and which trust boundary is crossed.
* **Proof of concept** — code or a compressed input file. Do not attach
  anything that contains live credentials.

A malformed-model DoS report is in scope: the parser and desugaring passes
take untrusted input, and Oikos is a compiler, so "a file that makes
`cargo run` spin" is a real finding.

## Response timeline

| Phase | Target | Notes |
|---|---|---|
| Initial response | ≤ 48 hours | Acknowledgement of receipt. |
| Status update | ≤ 7 days | Progress, or mitigation advice. |
| Resolution | ≤ 30 days | For critical issues (RCE, credential leak). |
| Advisory publication | Post-resolution | GitHub security advisory + `CHANGELOG.adoc` entry. |

## Hardening already in place

Reporting these areas is welcome; they are the parts most likely to break.

* **Pinned, locked CI actions.** Every `uses:` ref in `.github/workflows/` is
  SHA-pinned and recorded in `.github/workflows/actions.lock`, including the
  transitive closure of the reusable workflows called from
  `hyperpolymath/standards`. `scripts/check-lock-sync.sh` (Lock Sync Gate)
  fails any PR where the two drift apart.
* **Least-privilege workflow tokens.** Every workflow declares top-level
  `permissions:`; `.github/hooks/validate-permissions.sh` checks it locally.
* **Secret scanning.** `.github/workflows/secret-scanner.yml` runs on every
  push and PR; reviewed historical false positives are exempted by exact
  fingerprint, never by rule.
* **Dependency advisories.** `.github/workflows/cargo-audit.yml` audits
  `Cargo.lock`.
* **No secrets in the tree.** The repository contains no example assignments
  or fallback values for any token; nothing here reads a credential from a
  file other than the runner's environment.

## Notes for reporters

* This repository is largely scaffolding: `oikos-desugar` and `oikos-check`
  are not yet load-bearing (see link:../docs/ROADMAP.adoc[`docs/ROADMAP.adoc`]).
  A finding in unshipped code is still a finding — it just usually lands at
  moderate severity.
* Expect one acknowledgement from a maintainer, not a triage team.
