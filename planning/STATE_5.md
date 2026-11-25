# Orchestration 5 State (`planning/STATE_5.md`)

Plan: `planning/EXECUTION_PLAN_5.md`
Last Updated: 2026-11-24

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs`
- [x] Rust baseline green from `STATE_4.md` (`43` unit + `13` adversarial + `4` golden-master + `8` strict_mode, clippy & fmt clean)
- [x] R package baseline green from `STATE_4.md` (`132` PASS, `0` FAIL, `0` WARN)
- [x] `jNSMR` `0.3.1` and `terra` `1.9.51` verified installed locally

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `5.1.1` | Stage `newhall-core` for standalone R tarballs & add `.Rbuildignore` | done | flash | `6ae3f7b` |
| 1 | `5.1.2` | Fix `DESCRIPTION`/`LICENSE` metadata and `R CMD check` doc hygiene | done | flash | `f7daf60` |
| 2 | `5.2.1` | Add live `jNSMR` tabular and `SpatRaster` parity `testthat` suite | done | flash | `17851bc` |
| 2 | `5.2.2` | Add reproducible `newhallrs` vs `jNSMR` raster benchmark & report | done | flash | `3085270` |
| 3 | `5.3.1` | Run standalone `devtools::check('r-pkg')` gate & fix any remaining findings | blocked | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

## 5.1.1
- **status:** done
- **files:** created `r-pkg/.Rbuildignore` (8 lines), `r-pkg/tools/sync-core.sh` (20 lines, executable), `r-pkg/src/rust/newhall-core/` (10 files, 5629 lines); changed `r-pkg/src/rust/Cargo.toml` (15 lines), `r-pkg/src/Makevars` (18 lines)
- **commit:** `6ae3f7b`
- **verification:** `bash r-pkg/tools/sync-core.sh && cargo check --manifest-path r-pkg/src/rust/Cargo.toml && cargo clippy --manifest-path r-pkg/src/rust/Cargo.toml -- -D warnings` PASS (exit 0); isolated `R CMD build` + `R CMD INSTALL` PASS.
- **deviations:** none
- **gotchas:** `/tmp` (tmpfs) has limited free space (~122M); clean `r-pkg/src/rust/target/` or set `TMPDIR` before `devtools::check('r-pkg')`.

## 5.1.2
- **status:** done
- **files:** created `r-pkg/LICENSE` (2 lines); changed `r-pkg/DESCRIPTION` (`+4/-3`, MIT + file LICENSE, removed Author/Maintainer, added `Imports: utils`, added `jNSMR` to `Suggests:`); inspected `NAMESPACE` and `man/*.Rd` (0 issues).
- **commit:** `f7daf60`
- **verification:** `read.dcf("r-pkg/DESCRIPTION")` check PASS; `tools::checkRd()`, `codoc()`, `undoc()`, `checkDocFiles()`, `checkDocStyle()` PASS (0 findings).
- **deviations:** none (added `Imports: utils` per Q1 approval).
- **gotchas:** none.

## 5.2.1
- **status:** done
- **files:** created `r-pkg/tests/testthat/test-jnsmr-parity.R` (83 lines)
- **commit:** `17851bc`
- **verification:** `Rscript -e "devtools::test('r-pkg', filter = 'jnsmr-parity')"` PASS (`[ FAIL 0 | WARN 0 | SKIP 0 | PASS 85 ]`, 2.0 s).
- **deviations:** none
- **gotchas:** `terra::nlyr()` returns `double` (`19`); `jNSMR` returns `integer` for day-count CSV columns whereas `newhallrs` returns `double`.

## 5.2.2
- **status:** done
- **files:** created `r-pkg/inst/benchmarks/bench_jnsmr.R` (139 lines), `planning/benchmarks/jnsmr_raster_benchmark.md` (70 lines)
- **commit:** `3085270`
- **verification:** `Rscript r-pkg/inst/benchmarks/bench_jnsmr.R` PASS (PRISM: `0.0230s` vs `9.0730s`, `394.5x` speedup; Daymet: `0.0230s` vs `9.1160s`, `396.3x` speedup; `816,957` vs `~2,066` valid cells/s).
- **deviations:** none
- **gotchas:** `jNSMR` requires attaching (`library(jNSMR)`) so `.onAttach()` initializes `options(jNSMR.NSM_VERSION)`. Clean `r-pkg/src/rust/target/` or set `TMPDIR` before `devtools::check('r-pkg')` if `/tmp` is tight.

## 5.3.1
- **status:** blocked (all R/doc/metadata/test checks clean; 2 compiled-code/Rust warnings outside 5.3.1 file scope)
- **files:** none modified (created `planning/tasks/5.3.1.todo.md`)
- **commit:** none
- **verification:**
  - `TMPDIR=.development/tmp Rscript -e "devtools::check('r-pkg', error_on = 'warning')"` -> `0 errors | 2 warnings | 0 notes`
  - `Rscript -e "devtools::test('r-pkg')"` -> PASS (`[ FAIL 0 | WARN 0 | SKIP 0 | PASS 217 ]`)
- **deviations:** stopped at scope boundary (did not edit `r-pkg/src/Makevars` or `r-pkg/src/rust/Cargo.toml`)
- **blocker:** (1) `checking Rust compilation` (`--as-cran`) requires logging `rustc --version` in `r-pkg/src/Makevars`; (2) `checking compiled code` on R 4.6.2 flags `_exit`/`abort`/`exit` in `libnewhallrs.a` and non-API R calls (`BODY`, `CLOENV`, `DATAPTR`, `ENCLOS`, `FORMALS`, `R_NamespaceRegistry`, `Rf_findVarInFrame`) from `extendr-api` `0.7.1`.
