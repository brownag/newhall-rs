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
| 2 | `4.2.1` | Refactor `newhall_batch` & `newhall_batch_phase4` into S3 generics (`default`, `data.frame`, `character`) | pending | flash | — |
| 2 | `4.2.2` | Implement chunked `SpatRaster` S3 methods with `terra::set.cats()` | pending | flash | — |
| 3 | `4.3.1` | Sync `NAMESPACE`/docs and add `testthat` S3 + `SpatRaster` test suite | pending | flash | — |

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
