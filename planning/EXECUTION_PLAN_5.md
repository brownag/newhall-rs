# Execution Plan 5: `R CMD check` Hygiene, Live `jNSMR` Parity, and Raster Speedup Benchmark (`G8`)

## Goal & Scope

### Goal
Bring `r-pkg` (`newhallrs`) to release-quality `R CMD check` cleanliness on a standalone built source tarball (0 errors, 0 warnings), add live end-to-end `testthat` parity tests against `jNSMR` on sample tabular and `terra::SpatRaster` datasets (`prism_issr800_sample.tif`, `daymet_issr800_sample.tif`, `All_PA_jNSM_Example_Batch_Metric.csv`), and produce a reproducible `newhallrs` vs `jNSMR` sample-raster speedup benchmark and report.

### In Scope
- **Wave 1 (Standalone Tarball Build & `R CMD check` Metadata Hygiene):**
  - `r-pkg/.Rbuildignore`, `r-pkg/tools/sync-core.sh`, `r-pkg/src/rust/Cargo.toml`, `r-pkg/src/Makevars`: Exclude `src/rust/target/`, `src/*.o`, `src/*.so` from R source tarballs and stage `crates/newhall-core/` into `r-pkg/src/rust/newhall-core/` (via `sync-core.sh`) so `R CMD build` + `R CMD check` compiles cleanly inside an isolated `/tmp` tarball.
  - `r-pkg/DESCRIPTION`, `r-pkg/LICENSE`, `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd`: Resolve `R CMD check` metadata/documentation issues (`License: MIT + file LICENSE`, remove duplicate `Author`/`Maintainer` in favor of `Authors@R`, ensure `roxygen2` docs/exports are clean).
- **Wave 2 (Live `jNSMR` Parity Suite & Sample-Raster Benchmark):**
  - `r-pkg/tests/testthat/test-jnsmr-parity.R`: Guarded with `skip_if_not_installed("jNSMR")`, verifies `newhallrs::newhall_batch()` against `jNSMR::newhall_batch()` on `prism_issr800_sample.tif`, `daymet_issr800_sample.tif`, and `All_PA_jNSM_Example_Batch_Metric.csv` (`<= 1e-9` float tolerance, 100% exact categorical match and `terra::cats()` levels).
  - `r-pkg/inst/benchmarks/bench_jnsmr.R` & `planning/benchmarks/jnsmr_raster_benchmark.md`: Reproducible benchmark script and recorded results comparing `newhallrs::newhall_batch()` vs `jNSMR::newhall_batch()` on both sample rasters (wall-clock time, cells/sec, speedup factor).
- **Wave 3 (Standalone `R CMD check` Gate):**
  - Run `devtools::check('r-pkg')` on a freshly synced standalone source tarball, fix any remaining `R CMD check` errors/warnings, and confirm `devtools::test('r-pkg')` and all Rust workspace suites are 100% green.

### Out of Scope (Explicitly Deferred)
- `R CMD check --as-cran` 0-note requirement (CRAN-specific `New submission` / `installed size` notes are acceptable).
- `newhall-cli` batch CSV/benchmark commands (`G5`), secondary Rust/R perf optimizations (`G16`), and `ARCHITECTURE.md` (`G10`).
- Modifying `crates/newhall-core/` physics or golden-master tests.

### Verbatim Constraints
- "All Phase 1–3 golden-master parity tests (`jNSMR` / Java baseline) MUST remain 100% green and unchanged. Legacy behavior is the default; Phase 4 physics are opt-in trait implementations."
- "Keep `newhall-core` free of R/`extendr` dependencies."

---

## Decision Log

- **2026-11-24 — Standalone `newhall-core` Staging (`Option A`):** Add `r-pkg/tools/sync-core.sh` to stage `crates/newhall-core/` into `r-pkg/src/rust/newhall-core/` for standalone `R CMD build`/`check` tarballs (keeping `crates/newhall-core/` as the single source of truth), and add `r-pkg/.Rbuildignore` excluding `src/rust/target/` and compiled artifacts.
- **2026-11-24 — `jNSMR` Parity Test Strategy (`Option A`):** Guard live `jNSMR` parity tests with `skip_if_not_installed("jNSMR")` and load `jNSMR`'s bundled `extdata/` datasets via `system.file("extdata/...", package = "jNSMR")`.
- **2026-11-24 — Benchmark & Execution Bar (`Option A`):** Add `r-pkg/inst/benchmarks/bench_jnsmr.R` and `planning/benchmarks/jnsmr_raster_benchmark.md`; use `flash` workers (max 3 concurrent, escalate to `pro`/frontier after 2 failures); track `planning/*_5.md` via `docs(planning):`; require `devtools::check('r-pkg')` with **0 errors, 0 warnings**, `devtools::test('r-pkg')` with **0 failures**, and Rust workspace tests/clippy/fmt 100% green.

---

## Recon & Hazard Notes

- **Baseline Green Suites:** `43` unit + `13` adversarial + `4` golden-master + `8` strict_mode Rust tests (`cargo test --workspace --all-targets`), `clippy` & `fmt` clean, and `132` R `testthat` assertions (`devtools::test('r-pkg')`). `jNSMR` `0.3.1` and `terra` `1.9.51` are installed locally.
- **Hazard — Missing `.Rbuildignore`:** Without `.Rbuildignore` excluding `^src/rust/target$`, `^src/.*\\.o$`, `^src/.*\\.so$`, `R CMD build` copies `src/rust/target/` into `/tmp` and exhausts `/tmp` disk space (`No space left on device`).
- **Hazard — `newhall-core` Relative Path:** `r-pkg/src/rust/Cargo.toml` currently has `newhall-core = { path = "../../../crates/newhall-core" }`. When staged into `r-pkg/src/rust/newhall-core/`, `Cargo.toml` should point to `path = "newhall-core"` (and `sync-core.sh` should copy only `Cargo.toml` and `src/` from `crates/newhall-core/`, excluding `tests/`/`target/`, so the R tarball stays small and self-contained).
- **Hazard — `jNSMR` `SpatRaster` Output Comparison:** `jNSMR::newhall_batch()` returns a 19-layer `SpatRaster` with `terra::set.cats()` on layers 16–19. Compare numeric layers (`1:15`) with tolerance `1e-9` (handling `NA` positions identically) and categorical layers (`16:19`) for exact value + `terra::cats()` label match.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `5.1.1` | Stage `newhall-core` for standalone R tarballs & add `.Rbuildignore` | `newhall-rs` | — | Create: `r-pkg/.Rbuildignore`, `r-pkg/tools/sync-core.sh`, `r-pkg/src/rust/newhall-core/`; Edit: `r-pkg/src/rust/Cargo.toml`, `r-pkg/src/Makevars` |
| 1 | `5.1.2` | Fix `DESCRIPTION`/`LICENSE` metadata and `R CMD check` doc hygiene | `newhall-rs` | — | Create: `r-pkg/LICENSE`; Edit: `r-pkg/DESCRIPTION`, `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd` |
| 2 | `5.2.1` | Add live `jNSMR` tabular and `SpatRaster` parity `testthat` suite | `newhall-rs` | `5.1.1`, `5.1.2` | Create: `r-pkg/tests/testthat/test-jnsmr-parity.R` |
| 2 | `5.2.2` | Add reproducible `newhallrs` vs `jNSMR` raster benchmark & report | `newhall-rs` | `5.1.1`, `5.1.2` | Create: `r-pkg/inst/benchmarks/bench_jnsmr.R`, `planning/benchmarks/jnsmr_raster_benchmark.md` |
| 3 | `5.3.1` | Run standalone `devtools::check('r-pkg')` gate & fix any remaining findings | `newhall-rs` | `5.2.1`, `5.2.2` | Edit (as needed for check findings): `r-pkg/DESCRIPTION`, `r-pkg/.Rbuildignore`, `r-pkg/man/*.Rd`, `r-pkg/R/*.R` |

---

## Task Specifications

### Task `5.1.1`: Stage `newhall-core` for standalone R tarballs & add `.Rbuildignore`
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `r-pkg/.Rbuildignore`, `r-pkg/tools/sync-core.sh`, `r-pkg/src/rust/newhall-core/`
  - *Must edit:* `r-pkg/src/rust/Cargo.toml`, `r-pkg/src/Makevars`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/DESCRIPTION`, `r-pkg/R/`, `r-pkg/tests/`.
- **Depends on:** None
- **Spec:**
  1. Create `r-pkg/.Rbuildignore` excluding `^src/rust/target$`, `^src/\.cargo$`, `^src/.*\.o$`, `^src/.*\.so$`, `^src/.*\.dll$`, `^tools$`, `^\.Rproj\.user$`, and `^.*\.Rproj$`.
  2. Create executable `r-pkg/tools/sync-core.sh` that cleanly copies `crates/newhall-core/Cargo.toml` and `crates/newhall-core/src/` (excluding `tests/` and `target/`) into `r-pkg/src/rust/newhall-core/`. Run it to populate `r-pkg/src/rust/newhall-core/`.
  3. Update `r-pkg/src/rust/Cargo.toml` to depend on `newhall-core = { path = "newhall-core" }` (and add `[workspace]` table if needed so Cargo treats `r-pkg/src/rust` as standalone inside tarballs).
  4. Verify `r-pkg/src/Makevars` builds cleanly with the staged `newhall-core` and does not leave temporary artifacts inside `src/` that break `R CMD check`.
- **Grep anchors:** `r-pkg/src/rust/Cargo.toml`, `r-pkg/src/Makevars`.
- **Acceptance & targeted verification:**
  - `bash r-pkg/tools/sync-core.sh && cargo check --manifest-path r-pkg/src/rust/Cargo.toml && cargo clippy --manifest-path r-pkg/src/rust/Cargo.toml -- -D warnings` exits 0.

---

### Task `5.1.2`: Fix `DESCRIPTION`/`LICENSE` metadata and `R CMD check` doc hygiene
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `r-pkg/LICENSE`
  - *Must edit:* `r-pkg/DESCRIPTION`, `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/src/`, `r-pkg/R/`, `r-pkg/tests/`.
- **Depends on:** None
- **Spec:**
  1. Create `r-pkg/LICENSE` in standard CRAN MIT template (`YEAR: 2026`, `COPYRIGHT HOLDER: Newhall Contributors`).
  2. Update `r-pkg/DESCRIPTION`:
     - Change `License: MIT` to `License: MIT + file LICENSE`.
     - Remove duplicate `Author:` and `Maintainer:` fields (keep `Authors@R:`).
     - Add `jNSMR` to `Suggests:` (since `test-jnsmr-parity.R` uses `skip_if_not_installed("jNSMR")` and `R CMD check` checks that suggested packages used in tests are declared in `Suggests:`).
  3. Inspect `r-pkg/NAMESPACE` and `r-pkg/man/*.Rd` for any `R CMD check` documentation warnings (e.g., undocumented arguments, mismatched `\usage{}`) and note any findings.
- **Grep anchors:** `r-pkg/DESCRIPTION`, `r-pkg/NAMESPACE`, `r-pkg/man/newhall_batch.Rd`.
- **Acceptance & targeted verification:**
  - `Rscript -e "desc <- read.dcf('r-pkg/DESCRIPTION'); stopifnot(grepl('MIT \\+ file LICENSE', desc[,'License']), !('Author' %in% colnames(desc)), grepl('jNSMR', desc[,'Suggests']))"` exits 0.

---

### Task `5.2.1`: Add live `jNSMR` tabular and `SpatRaster` parity `testthat` suite
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `r-pkg/tests/testthat/test-jnsmr-parity.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/src/`, `r-pkg/R/`.
- **Depends on:** `5.1.1`, `5.1.2`
- **Spec:**
  1. Create `r-pkg/tests/testthat/test-jnsmr-parity.R` with `skip_if_not_installed("jNSMR")` and `skip_if_not_installed("terra")`.
  2. Test tabular CSV parity on `system.file("extdata/All_PA_jNSM_Example_Batch_Metric.csv", package = "jNSMR")`: compare `newhallrs::newhall_batch(csv_path)` against `jNSMR::newhall_batch(csv_path)` across all 19 columns (`tolerance = 1e-9` on numeric columns, exact string match on categorical columns).
  3. Test `SpatRaster` parity on `system.file("extdata/prism_issr800_sample.tif", package = "jNSMR")` and `system.file("extdata/daymet_issr800_sample.tif", package = "jNSMR")`: compare `newhallrs::newhall_batch(r)` against `jNSMR::newhall_batch(r)` on all 19 layers (exact `NA` mask, `tolerance = 1e-9` on numeric layers `1:15`, exact categorical values and `terra::cats()` labels on layers `16:19`).
- **Grep anchors:** `r-pkg/tests/testthat/test-s3-api.R`, `r-pkg/R/raster.R`.
- **Acceptance & targeted verification:**
  - `Rscript -e "devtools::test('r-pkg', filter = 'jnsmr-parity')"` exits 0 with 0 failures and 0 skips (locally where `jNSMR` is installed).

---

### Task `5.2.2`: Add reproducible `newhallrs` vs `jNSMR` raster benchmark & report
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `r-pkg/inst/benchmarks/bench_jnsmr.R`, `planning/benchmarks/jnsmr_raster_benchmark.md`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/src/`, `r-pkg/R/`, `r-pkg/tests/`.
- **Depends on:** `5.1.1`, `5.1.2`
- **Spec:**
  1. Create `r-pkg/inst/benchmarks/bench_jnsmr.R` that loads `newhallrs` (via `devtools::load_all('r-pkg')` or installed package) and `jNSMR`, runs warm-up + timed runs (e.g., 3 replicates, median wall-clock seconds) of `newhallrs::newhall_batch()` vs `jNSMR::newhall_batch()` on `prism_issr800_sample.tif` and `daymet_issr800_sample.tif` (reporting total cells, valid non-`NA` cells, median wall-clock time, cells/sec, and speedup ratio).
  2. Run `Rscript r-pkg/inst/benchmarks/bench_jnsmr.R` and record the measured results, hardware/environment metadata, and reproduction command in `planning/benchmarks/jnsmr_raster_benchmark.md`.
- **Grep anchors:** `r-pkg/R/raster.R`.
- **Acceptance & targeted verification:**
  - `Rscript r-pkg/inst/benchmarks/bench_jnsmr.R` exits 0 and prints a clean summary table for both sample rasters.

---

### Task `5.3.1`: Run standalone `devtools::check('r-pkg')` gate & fix any remaining findings
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit (only as needed to resolve `R CMD check` errors/warnings):* `r-pkg/DESCRIPTION`, `r-pkg/.Rbuildignore`, `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd`, `r-pkg/R/*.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/src/rust/src/lib.rs`.
- **Depends on:** `5.2.1`, `5.2.2`
- **Spec:**
  1. Run `Rscript -e "devtools::check('r-pkg', error_on = 'warning')"` (which builds a standalone source tarball and runs `R CMD check`).
  2. Fix any `R CMD check` errors or warnings (e.g., `codoc`, `NAMESPACE`, `R/utils::read.csv` imports, undeclared globals, `.Rbuildignore` patterns) and re-run until `0 errors | 0 warnings`.
  3. Confirm `Rscript -e "devtools::test('r-pkg')"` passes with 0 failures and 0 warnings.
- **Grep anchors:** `r-pkg/DESCRIPTION`, `r-pkg/NAMESPACE`, `r-pkg/.Rbuildignore`.
- **Acceptance & targeted verification:**
  - `Rscript -e "devtools::check('r-pkg', error_on = 'warning')"` exits 0 with `0 errors | 0 warnings`.
