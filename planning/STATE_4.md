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
| 1 | `4.1.1` | Extend Rust FFI for per-site soil config vectors | pending | flash | — |
| 1 | `4.1.2` | Add categorical level tables, schema helpers, and `terra` Suggests | pending | flash | — |
| 2 | `4.2.1` | Refactor `newhall_batch` & `newhall_batch_phase4` into S3 generics (`default`, `data.frame`, `character`) | pending | flash | — |
| 2 | `4.2.2` | Implement chunked `SpatRaster` S3 methods with `terra::set.cats()` | pending | flash | — |
| 3 | `4.3.1` | Sync `NAMESPACE`/docs and add `testthat` S3 + `SpatRaster` test suite | pending | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->
