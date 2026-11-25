# Execution Plan 7: `G19` CI/CD, `G20` CRAN/Makevars Hygiene, `G21` Root Docs, and `G25` `crates.io` Readiness

## Goal & Scope

### Goal
Prepare `newhall-rs` for public release and automated quality enforcement across both Rust and R in a single 3-wave orchestration:
1. **`G19` (Wave 1):** Add `--check` drift verification to `r-pkg/tools/sync-core.sh` and create `.github/workflows/ci.yml` running Rust workspace checks + `R CMD check`/`testthat` + sync-drift check on `ubuntu-latest`, plus Rust `cargo test` on `macos-latest` and `windows-latest`.
2. **`G21` (Wave 1, parallel):** Create root `README.md`, `LICENSE` (MIT), and `CHANGELOG.md` covering `newhall-core`, `newhall-cli`, and `newhallrs` quickstarts, classic vs. strict vs. Phase 4 modes, and benchmark highlights.
3. **`G20` (Wave 1, parallel + Wave 3 gate):** Fix `r-pkg/src/Makevars` to track Rust source prerequisites (eliminating stale `libnewhallrs.a` rebuilds), add `r-pkg/src/Makevars.win`, and update `r-pkg/DESCRIPTION` and `r-pkg/LICENSE` with real maintainer and repository metadata.
4. **`G25` (Wave 2):** Make `newhall-core` crates.io-ready with complete public rustdoc (`#![warn(missing_docs)]`), `Cargo.toml` metadata (`authors`, `repository`, `homepage`, `readme`, `keywords`, `categories`), and passing `cargo test --no-default-features` + `cargo package --list`.
5. **Integration & Verification (Wave 3):** Sync updated `crates/newhall-core/` into `r-pkg/src/rust/newhall-core/`, verify `sync-core.sh --check`, run all Rust checks, and pass `R CMD build` + `R CMD check --as-cran` and `devtools::test('r-pkg')` cleanly.

### Out of Scope (Deferred)
- CRAN offline `vendor.tar.xz` crate bundling (deferred to actual CRAN submission / `G28`).
- R vignettes (`G22`), `newhall_batch_phase4.character` (`G24`), `newhall-cli` Phase 4 flags (`G23`), Phase 4 analytical reference tests (`G26`), optional `serde` (`G27`), and tag-based release workflow (`G28`).

### Verbatim Constraints
- "All Phase 1–3 golden-master parity tests (`jNSMR` / Java baseline) MUST remain 100% green and unchanged. Legacy behavior is the default; Phase 4 physics are opt-in trait implementations."
- "Keep `newhall-core` free of R/`extendr` dependencies."

### Decision Log (2026-11-24)
- **D1 (Scope):** Bundle `G19 + G20 + G21 + G25` into Orchestration 7.
- **D2 (CI Matrix):** `ubuntu-latest` for Rust + R (`R CMD check` + `testthat` + `sync-core.sh --check`), plus `macos-latest` and `windows-latest` for Rust `cargo test` only.
- **D3 (Package Metadata):** Author `Andrew Gene Brown <brown.andrewg@gmail.com>` and repository/BugReports URL `https://github.com/brownag/newhall-rs` across `r-pkg/DESCRIPTION`, `LICENSE`, and `crates/newhall-core/Cargo.toml`.
- **D4 (CRAN Vendoring):** Defer `vendor.tar.xz` offline crate bundling to `G28`/CRAN submission; `G20` focuses on `Makevars`/`Makevars.win`, metadata, and passing `R CMD check --as-cran` locally.
- **D5 (Workers & Escalation):** `flash` workers, escalate to `hydrogen` (pro) after 2 failed attempts; up to 3 parallel workers per wave on pairwise-disjoint files.

### Recon Hazards
- `/tmp` can be tight on this host; set `TMPDIR=.development/tmp` (after `mkdir -p .development/tmp`) when running `R CMD build`, `R CMD check`, or `devtools::test`.
- `r-pkg/src/Makevars` currently has no prerequisites on `$(STATLIB)` — `7.1.3` fixes this so edits to `r-pkg/src/rust/` always trigger Cargo rebuilds.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `7.1.1` | Add `sync-core.sh --check` and `.github/workflows/ci.yml` (`G19`) | `newhall-rs` | — | Edit: `r-pkg/tools/sync-core.sh`; Create: `.github/workflows/ci.yml` |
| 1 | `7.1.2` | Create root `README.md`, `LICENSE`, and `CHANGELOG.md` (`G21`) | `newhall-rs` | — | Create: `README.md`, `LICENSE`, `CHANGELOG.md` |
| 1 | `7.1.3` | `r-pkg` `Makevars` Rust tracking, `Makevars.win`, and `DESCRIPTION`/`LICENSE` metadata (`G20`) | `newhall-rs` | — | Edit: `r-pkg/src/Makevars`, `r-pkg/DESCRIPTION`, `r-pkg/LICENSE`, `r-pkg/.Rbuildignore` (if needed); Create: `r-pkg/src/Makevars.win` |
| 2 | `7.2.1` | `newhall-core` crates.io metadata, `#![warn(missing_docs)]`, and `--no-default-features` (`G25`) | `newhall-rs` | `7.1.2` | Edit: `crates/newhall-core/Cargo.toml`, `crates/newhall-core/src/*.rs` |
| 3 | `7.3.1` | Sync `newhall-core` to `r-pkg`, run full Rust + `R CMD check --as-cran` + `testthat` gate | `newhall-rs` | `7.1.1`, `7.1.3`, `7.2.1` | Edit: `r-pkg/src/rust/newhall-core/` (via `sync-core.sh`) |

---

## Task Specifications

### Task `7.1.1`: Add `sync-core.sh --check` and `.github/workflows/ci.yml` (`G19`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/tools/sync-core.sh`
  - *Must create:* `.github/workflows/ci.yml`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/`, `r-pkg/R/`, `r-pkg/src/`, `r-pkg/DESCRIPTION`.
- **Depends on:** None
- **Spec:**
  1. Update `r-pkg/tools/sync-core.sh` to support an optional `--check` flag: when invoked as `bash r-pkg/tools/sync-core.sh --check`, compare `crates/newhall-core/Cargo.toml` vs `r-pkg/src/rust/newhall-core/Cargo.toml` and `diff -rq crates/newhall-core/src r-pkg/src/rust/newhall-core/src`, exiting 0 if identical and non-zero with a clear message if drifted. Default (no args) behavior remains unchanged.
  2. Create `.github/workflows/ci.yml` triggered on `push` and `pull_request` to `master` with:
     - `rust-check` job on `ubuntu-latest`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`, `cargo test -p newhall-core --no-default-features`, and `bash r-pkg/tools/sync-core.sh --check`.
     - `rust-cross` job on `macos-latest` and `windows-latest`: `cargo test --workspace --all-targets`.
     - `r-check` job on `ubuntu-latest`: install R + system deps (`terra` GDAL/GEOS/PROJ), install `testthat`/`terra`, run `R CMD build r-pkg`, `R CMD check --no-manual newhallrs_*.tar.gz`, and `Rscript -e "devtools::test('r-pkg')"`.
- **Grep anchors:** `r-pkg/tools/sync-core.sh`.
- **Acceptance & targeted verification:**
  - `bash -n r-pkg/tools/sync-core.sh && bash r-pkg/tools/sync-core.sh --check` exits 0.
  - `.github/workflows/ci.yml` parses cleanly with Python/Ruby YAML or `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`.

---

### Task `7.1.2`: Create root `README.md`, `LICENSE`, and `CHANGELOG.md` (`G21`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must create:* `README.md`, `LICENSE`, `CHANGELOG.md`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/`, `r-pkg/`, `ARCHITECTURE.md`.
- **Depends on:** None
- **Spec:**
  1. Create root `LICENSE` with standard MIT license text (`Copyright (c) 2026 Andrew Gene Brown`).
  2. Create `README.md` at the repository root covering:
     - Project overview (pure-Rust Newhall Simulation Model engine + CLI + `extendr` R package `newhallrs`).
     - Quickstart for Rust (`newhall-core`, `newhall-cli run/verify/bench`) and R (`newhall_batch`, `newhall_batch_phase4` on `data.frame` and `terra::SpatRaster`).
     - Summary of model modes (`ClassicNewhall` for verbatim `jNSMR` parity, `StrictNewhall` for remediated defects, and Phase 4 1D thermal + fluctuating water-table physics).
     - Benchmark highlights (citing `planning/benchmarks/jnsmr_raster_benchmark.md` and `newhall-cli bench` numbers) and link to `ARCHITECTURE.md`.
  3. Create `CHANGELOG.md` (Keep a Changelog format) summarizing `0.1.0` capabilities.
- **Grep anchors:** `ARCHITECTURE.md`, `planning/benchmarks/jnsmr_raster_benchmark.md`, `r-pkg/NAMESPACE`.
- **Acceptance & targeted verification:**
  - `test -s README.md && test -s LICENSE && test -s CHANGELOG.md` exits 0.

---

### Task `7.1.3`: `r-pkg` `Makevars` Rust tracking, `Makevars.win`, and `DESCRIPTION`/`LICENSE` metadata (`G20`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/Makevars`, `r-pkg/DESCRIPTION`, `r-pkg/LICENSE`
  - *Must create:* `r-pkg/src/Makevars.win`
  - *Can edit (if needed):* `r-pkg/.Rbuildignore`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/`, `r-pkg/R/`, `r-pkg/tests/`, `r-pkg/src/rust/src/lib.rs`.
- **Depends on:** None
- **Spec:**
  1. Update `r-pkg/src/Makevars` so `$(STATLIB)` depends on Rust source files (e.g., `RUST_SRCS = $(wildcard ./rust/src/*.rs ./rust/newhall-core/src/*.rs ./rust/Cargo.toml ./rust/newhall-core/Cargo.toml)` or `.PHONY` cargo invocation) so editing Rust files always triggers `cargo build --release` without manually deleting `libnewhallrs.a`.
  2. Create `r-pkg/src/Makevars.win` mirroring `Makevars` for Windows Rtools + Cargo (`--target x86_64-pc-windows-gnu` or standard extendr Windows pattern).
  3. Update `r-pkg/DESCRIPTION`: set `Authors@R: person("Andrew Gene", "Brown", email = "brown.andrewg@gmail.com", role = c("aut", "cre"))`, add `URL: https://github.com/brownag/newhall-rs` and `BugReports: https://github.com/brownag/newhall-rs/issues`.
  4. Update `r-pkg/LICENSE` to `YEAR: 2026` and `COPYRIGHT HOLDER: Andrew Gene Brown`.
- **Grep anchors:** `r-pkg/src/Makevars`, `r-pkg/DESCRIPTION`, `r-pkg/LICENSE`, `r-pkg/.Rbuildignore`.
- **Acceptance & targeted verification:**
  - `mkdir -p .development/tmp && TMPDIR=.development/tmp Rscript -e "devtools::test('r-pkg')"` exits 0 with 0 failures and 0 warnings.

---

### Task `7.2.1`: `newhall-core` crates.io metadata, `#![warn(missing_docs)]`, and `--no-default-features` (`G25`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/Cargo.toml`, `crates/newhall-core/src/lib.rs`, and any `crates/newhall-core/src/*.rs` files needing public rustdoc comments.
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `r-pkg/`, `crates/newhall-cli/`, `crates/newhall-core/tests/`.
- **Depends on:** `7.1.2`
- **Spec:**
  1. Update `crates/newhall-core/Cargo.toml` with crates.io metadata: `authors = ["Andrew Gene Brown <brown.andrewg@gmail.com>"]`, `repository = "https://github.com/brownag/newhall-rs"`, `homepage = "https://github.com/brownag/newhall-rs"`, `readme = "../../README.md"`, `keywords = ["soil", "hydrology", "climate", "newhall", "simulation"]`, `categories = ["science", "simulation"]`.
  2. Add `#![warn(missing_docs)]` to `crates/newhall-core/src/lib.rs` and add concise, accurate rustdoc comments to all public items so `cargo doc --no-deps` and `cargo clippy --workspace --all-targets -- -D warnings` pass with zero warnings. Do NOT change any runtime logic or public signatures.
  3. Verify `cargo test -p newhall-core --no-default-features` (without `rayon`) and `cargo package -p newhall-core --list --allow-dirty` succeed cleanly.
- **Grep anchors:** `crates/newhall-core/Cargo.toml`, `crates/newhall-core/src/lib.rs`.
- **Acceptance & targeted verification:**
  - `cargo test --workspace --all-targets && cargo test -p newhall-core --no-default-features && cargo clippy --workspace --all-targets -- -D warnings && cargo doc -p newhall-core --no-deps && cargo fmt --all -- --check` exits 0.

---

### Task `7.3.1`: Sync `newhall-core` to `r-pkg`, run full Rust + `R CMD check --as-cran` + `testthat` gate
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/rust/newhall-core/` (via `bash r-pkg/tools/sync-core.sh`)
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/R/`, `r-pkg/tests/`.
- **Depends on:** `7.1.1`, `7.1.3`, `7.2.1`
- **Spec:**
  1. Run `bash r-pkg/tools/sync-core.sh` and verify `bash r-pkg/tools/sync-core.sh --check` exits 0.
  2. Run the full Rust verification suite (`cargo test --workspace --all-targets`, `cargo test -p newhall-core --no-default-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`).
  3. Build and check the R source tarball with `mkdir -p .development/tmp && TMPDIR=.development/tmp R CMD build r-pkg -o .development/tmp && TMPDIR=.development/tmp R CMD check --as-cran --no-manual -o .development/tmp .development/tmp/newhallrs_*.tar.gz`, plus `TMPDIR=.development/tmp Rscript -e "devtools::test('r-pkg')"`.
  4. Report exact check results (errors, warnings, notes) in `STATE_7.md`.
- **Grep anchors:** `r-pkg/tools/sync-core.sh`.
- **Acceptance & targeted verification:**
  - `bash r-pkg/tools/sync-core.sh --check && cargo test --workspace --all-targets && cargo test -p newhall-core --no-default-features` exits 0, and `R CMD check --as-cran --no-manual` reports 0 errors and 0 warnings (document any unavoidable CRAN-online notes if present).
