# Orchestration 4 State (`planning/STATE_4.md`)

Plan: `planning/EXECUTION_PLAN_4.md`
Last Updated: 2026-11-24

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs`
- [x] Rust baseline green from `STATE_3.md` (`43` unit + `13` adversarial + `4` golden-master + `8` strict_mode, clippy & fmt clean)
- [x] R package baseline green from `STATE_3.md` (`27` PASS, `0` FAIL) and `terra` `1.9.51` verified installed

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `4.1.1` | Extend Rust FFI for per-site soil config vectors | done | flash | `0af774c` |
| 1 | `4.1.2` | Add categorical level tables, schema helpers, and `terra` Suggests | done | flash | `4fdcb86` |
| 2 | `4.2.1` | Refactor `newhall_batch` & `newhall_batch_phase4` into S3 generics (`default`, `data.frame`, `character`) | done | flash | `7ed6fa0` |
| 2 | `4.2.2` | Implement chunked `SpatRaster` S3 methods with `terra::set.cats()` | done | flash | `dd94e33` |
| 3 | `4.3.1` | Sync `NAMESPACE`/docs and add `testthat` S3 + `SpatRaster` test suite | done | flash | `9162d5b` |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

## 4.1.1
- **status:** done
- **files changed:** `r-pkg/src/rust/src/lib.rs` (431 lines, +139/-28); `r-pkg/R/extendr-wrappers.R` verified unchanged
- **code commit ID:** `0af774c` (`feat(r-pkg): support per-site soil config slices in Rust FFI`)
- **verification:** `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt -- --check` on `r-pkg/src/rust/Cargo.toml` -> PASS (exit 0)
- **deviations:** none
- **gotchas for dependent tasks:** `has_o_horizon` and `is_saturated` are now `&[i32]` (`!= 0` => `true`), so `4.2.1` must pass `as.integer(as.logical(...))` and rebuild `r-pkg/src/newhallrs.so` before running R tests.

## 4.1.2
- **status:** done
- **files created/changed:** created `r-pkg/R/categories.R` (164 lines); edited `r-pkg/DESCRIPTION` (+1/-0)
- **code commit ID:** `4fdcb86` (`feat(r-pkg): add categorical level tables, schema helpers, and terra Suggests`)
- **verification:** `Rscript -e "source('r-pkg/R/categories.R'); stopifnot(length(.str_levels()) == 10L, length(.newhall_required_cols()) == 28L)"` -> PASS (exit 0)
- **deviations:** none
- **gotchas for dependent tasks:** `.normalize_unit_system(unitSystem)` returns scalar logical `TRUE`/`FALSE`. `.extract_df_inputs()` extracts `temp`/`precip` explicitly by `paste0("t", month.abb)`/`paste0("p", month.abb)` and returns `hasOHorizon`/`isSaturated` as logical vectors requiring `as.integer(as.logical(...))` before calling `newhall_batch_rust`.

## 4.2.1
- **status:** done
- **files changed:** `r-pkg/R/newhall.R` (369 lines, +249/-18)
- **code commit ID:** `7ed6fa0` (`feat(r-pkg): refactor batch runners into S3 generics`)
- **verification:** `Rscript -e "parse('r-pkg/R/newhall.R')"` -> PASS (exit 0)
- **deviations:** none
- **gotchas for dependent tasks:** `NAMESPACE` and `man/*.Rd` must be regenerated in `4.3.1` via `roxygen2::roxygenise('r-pkg')` to export `newhall_batch_raw`, `newhall_batch_phase4_raw`, and all S3 methods (`default`, `data.frame`, `character`). `.default` preserves integer regime codes, whereas `.data.frame`/`.character` return decoded character regime names.

## 4.2.2
- **status:** done
- **files created:** `r-pkg/R/raster.R` (411 lines)
- **code commit ID:** `dd94e33` (`feat(r-pkg): add chunked SpatRaster S3 methods with categorical regime tables`)
- **verification:** `Rscript -e "parse('r-pkg/R/raster.R')"` + mocked `terra` `1.9.51` in-memory/GeoTIFF smoke checks -> PASS (exit 0)
- **deviations:** none
- **gotchas for dependent tasks:** `terra::set.cats()` overwrites `names(out)[layer]` to `colnames(value)[2]`, so `.attach_regime_cats()` renames column 2 of `.regime_cat_df()` to each target layer name (`temperatureRegime`, `moistureRegime`, `regimeSubdivision1`, `regimeSubdivision2`). `newhall_batch_phase4.SpatRaster()` errors on non-metric `unitSystem`.

## 4.3.1
- **status:** done
- **files created/changed:** edited `r-pkg/NAMESPACE` (14 lines), `r-pkg/man/newhall_batch.Rd` (97 lines), `r-pkg/man/newhall_batch_phase4.Rd` (154 lines); created `r-pkg/man/newhall_batch.SpatRaster.Rd` (62 lines), `r-pkg/man/newhall_batch_phase4.SpatRaster.Rd` (113 lines), `r-pkg/tests/testthat/test-s3-api.R` (409 lines)
- **code commit ID:** `9162d5b` (`test(r-pkg): sync S3 exports, docs, and SpatRaster test suite`)
- **verification:** `rm -f r-pkg/src/newhallrs.so r-pkg/src/rust/target/release/libnewhallrs.a && Rscript -e "roxygen2::roxygenise('r-pkg')"` -> PASS (exit 0); `rm -f r-pkg/src/newhallrs.so r-pkg/src/rust/target/release/libnewhallrs.a && Rscript -e "devtools::test('r-pkg')"` -> PASS (`[ FAIL 0 | WARN 0 | SKIP 0 | PASS 132 ]`, `phase4`: 27, `s3-api`: 105)
- **deviations:** none (`test-phase4.R` passed unchanged)
- **gotchas for dependent tasks:** `devtools::test('r-pkg')` exits 0 even if testthat reports failures unless `stop_on_failure = TRUE` is passed or output summary is checked; always verify `[ FAIL 0 | WARN 0 ]`.
