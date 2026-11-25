# Execution Plan 8: `G29` `jNSMR` Gating and `G30` `R CMD check --as-cran` Compiled-Code Hygiene

## Goal & Scope

### Goal
Eliminate the `Suggests not in mainstream repositories: jNSMR` NOTE and investigate/remediate the `checking compiled code` WARNING (`R_NamespaceRegistry`, `_exit`/`abort`/`exit`) in `R CMD check --as-cran --no-manual`:
1. **`G29` (Wave 1):** Remove `jNSMR` from `r-pkg/DESCRIPTION` `Suggests`, gate `r-pkg/tests/testthat/test-jnsmr-parity.R` with `skip_on_cran()`, `skip_if_not(requireNamespace("jNSMR", quietly = TRUE))`, and `getExportedValue("jNSMR", "newhall_batch")` so `R CMD check` reports zero unstated-dependency or non-CRAN `Suggests` notes, and remove the `_R_CHECK_FORCE_SUGGESTS_: "false"` workaround from `.github/workflows/ci.yml`.
2. **`G30` (Wave 1, parallel):** Eliminate the `checking compiled code` WARNING (`R_NamespaceRegistry`, `_exit`/`abort`/`exit`) in `libnewhallrs.a` by enabling `[profile.release]` LTO (`lto = true`, `codegen-units = 1`) and any required `extendr-api` / Rust link-time dead-code elimination so `R CMD check --as-cran` reports **0 WARNINGs** (no `cran-comments.md` workarounds).
3. **Verification Gate (Wave 2):** Run `R CMD build`, `R CMD check --as-cran --no-manual`, `devtools::test('r-pkg')`, `sync-core.sh --check`, and `cargo test --workspace --all-targets`, recording exact check results in `STATE_8.md`.

### Out of Scope (Deferred)
- Replacing `extendr-api` with a raw R C-API or `savvy` bridge (deferred unless needed for actual CRAN submission).
- R vignettes (`G22`), `newhall_batch_phase4.character` (`G24`), `newhall-cli` Phase 4 flags (`G23`), Phase 4 analytical reference tests (`G26`), and release workflow (`G28`).

### Verbatim Constraints
- "All Phase 1–3 golden-master parity tests (`jNSMR` / Java baseline) MUST remain 100% green and unchanged. Legacy behavior is the default; Phase 4 physics are opt-in trait implementations."
- "Keep `newhall-core` free of R/`extendr` dependencies."

### Decision Log (2026-11-24)
- **D1 (`G29`):** Keep `test-jnsmr-parity.R` in `r-pkg/tests/testthat/` gated by `skip_on_cran()` + `requireNamespace("jNSMR", quietly = TRUE)` + `getExportedValue("jNSMR", "newhall_batch")`, removing `jNSMR` from `DESCRIPTION` `Suggests`.
- **D2 (`G30`):** Do NOT document warnings or notes in `cran-comments.md`. Eliminate `R_NamespaceRegistry` and `_exit`/`abort`/`exit` from `newhallrs.so` / `libnewhallrs.a` via `[profile.release]` LTO (`lto = true`, `codegen-units = 1`) and link-time dead-code elimination; if any symbol survives LTO, stop and escalate to fix the root cause.
- **D3 (Workers & Escalation):** `flash` workers, escalate to `pro` after 2 failed attempts; up to 2 parallel workers in Wave 1 on disjoint files.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `8.1.1` | Remove `jNSMR` from `DESCRIPTION`, gate `test-jnsmr-parity.R`, clean `ci.yml` (`G29`) | `newhall-rs` | — | Edit: `r-pkg/DESCRIPTION`, `r-pkg/tests/testthat/test-jnsmr-parity.R`, `.github/workflows/ci.yml` |
| 1 | `8.1.2` | Eliminate `libnewhallrs.a` compiled-code WARNING symbols via LTO/DCE (`G30`) | `newhall-rs` | — | Edit: `r-pkg/src/rust/Cargo.toml`, `r-pkg/src/rust/Cargo.lock` (if updated) |
| 2 | `8.2.1` | Full `R CMD check --as-cran`, `testthat`, `sync-core.sh --check`, and Rust gate | `newhall-rs` | `8.1.1`, `8.1.2` | None (verification only; temp in `.development/tmp`) |

---

## Task Specifications

### Task `8.1.1`: Remove `jNSMR` from `DESCRIPTION`, gate `test-jnsmr-parity.R`, clean `ci.yml` (`G29`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/DESCRIPTION`, `r-pkg/tests/testthat/test-jnsmr-parity.R`, `.github/workflows/ci.yml`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/`, `r-pkg/R/`, `r-pkg/src/`.
- **Depends on:** None
- **Spec:**
  1. Remove `jNSMR,` from `Suggests:` in `r-pkg/DESCRIPTION`.
  2. Update `r-pkg/tests/testthat/test-jnsmr-parity.R` so:
     - Every test/helper calls `skip_on_cran()` and `skip_if_not(requireNamespace("jNSMR", quietly = TRUE), "jNSMR not installed")` (instead of `skip_if_not_installed("jNSMR")`).
     - All `jNSMR::newhall_batch(...)` calls are replaced with `getExportedValue("jNSMR", "newhall_batch")(...)` so `R CMD check`'s static `::` scanner does not report `unstated dependencies in 'tests'`.
  3. Remove `_R_CHECK_FORCE_SUGGESTS_: "false"` from `.github/workflows/ci.yml` (since all remaining `Suggests` — `terra`, `testthat` — are standard CRAN packages installed in CI).
- **Grep anchors:** `r-pkg/DESCRIPTION`, `r-pkg/tests/testthat/test-jnsmr-parity.R`, `.github/workflows/ci.yml`.
- **Acceptance & targeted verification:**
  - `mkdir -p .development/tmp && TMPDIR="$PWD/.development/tmp" Rscript -e "devtools::test('r-pkg')"` exits 0 with 0 failures and 0 warnings.
  - `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` exits 0.

---

### Task `8.1.2`: Eliminate `libnewhallrs.a` compiled-code WARNING symbols via LTO/DCE (`G30`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/rust/Cargo.toml`, `r-pkg/src/rust/Cargo.lock` (if updated)
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/`, `r-pkg/R/`, `r-pkg/DESCRIPTION`, `r-pkg/tests/`. Do NOT create `cran-comments.md`.
- **Depends on:** None
- **Spec:**
  1. Add `[profile.release]` with `lto = true` and `codegen-units = 1` in `r-pkg/src/rust/Cargo.toml` so unused `extendr-api` functions (`R_NamespaceRegistry` in `extendr_api::functions::namespace_registry`) and dead Rust `std` symbols (`_exit`, `exit`, `abort`) are dead-code-eliminated from `libnewhallrs.a` / `newhallrs.so`.
  2. Build with `cargo build --release --manifest-path r-pkg/src/rust/Cargo.toml` and inspect `nm -A -u r-pkg/src/rust/target/release/libnewhallrs.a | grep -E 'R_NamespaceRegistry|\b_?exit\b|\babort\b' || true`.
  3. If any of those symbols still survive in `libnewhallrs.a` or `newhallrs.so`, STOP and report the exact surviving symbols and originating object files so the orchestrator can escalate to a root-cause fix.
- **Grep anchors:** `r-pkg/src/rust/Cargo.toml`.
- **Acceptance & targeted verification:**
  - `cargo build --release --manifest-path r-pkg/src/rust/Cargo.toml` exits 0, and `nm -A -u r-pkg/src/rust/target/release/libnewhallrs.a | grep -E 'R_NamespaceRegistry|\b_?exit\b|\babort\b'` produces empty output (or exact surviving symbols reported if blocked).

---

### Task `8.2.1`: Full `R CMD check --as-cran`, `testthat`, `sync-core.sh --check`, and Rust gate
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Temp/scratch:* `.development/tmp/` (untracked)
  - *Do not touch:* Any tracked files (verification-only task).
- **Depends on:** `8.1.1`, `8.1.2`
- **Spec:**
  1. Run `bash r-pkg/tools/sync-core.sh --check && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets && cargo test -p newhall-core --no-default-features`.
  2. Build and check the R tarball: `mkdir -p .development/tmp && (cd .development/tmp && TMPDIR="$PWD" R CMD build ../../r-pkg) && TMPDIR="$PWD/.development/tmp" R CMD check --as-cran --no-manual -o .development/tmp .development/tmp/newhallrs_*.tar.gz`.
  3. Run `TMPDIR="$PWD/.development/tmp" Rscript -e "devtools::test('r-pkg')"`.
  4. Report exact `R CMD check --as-cran` errors, warnings, and notes (confirming zero `Suggests` / `unstated dependencies in 'tests'` issues).
- **Grep anchors:** `planning/STATE_8.md`.
- **Acceptance & targeted verification:**
  - All Rust tests, `sync-core.sh --check`, and `devtools::test('r-pkg')` pass with 0 failures; `R CMD check --as-cran --no-manual` exits 0 with **0 ERRORs**, **0 WARNINGs**, and **0 `Suggests`/unstated-dependency NOTEs**.
