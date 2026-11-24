# Orchestration 3 State (`planning/STATE_3.md`)

Plan: `planning/EXECUTION_PLAN_3.md`
Last Updated: 2026-11-22

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs` (clean git tree at `51b1345`)
- [x] Rust toolchain verified (`cargo test --workspace --all-targets`: 29 unit + 13 adversarial + 4 golden-master green)
- [x] Clippy & fmt verified (`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`)
- [x] R package verified (`devtools::test('r-pkg')`: 15/15 green)

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `3.1.1` | Implement `StrictThornthwaiteForcing` (`#1`, `#2`, `#7`, `#9`) | done | flash | `488ae8b` |
| 1 | `3.1.2` | Implement `StrictThermal` (`#2`, `#3`, `#4`, `#5`, `#6`, `#8`, `#9`, `#10`) | done | flash | `5deb0f3` |
| 2 | `3.2.1` | Wire `StrictHydrology`, `StrictNewhall`, and `lib.rs` exports | done | flash | `eab2740` |
| 3 | `3.3.1` | Expose `strict` and `strict_forcing` in `r-pkg` + `testthat` | done | flash | `59e515e` |
| 3 | `3.3.2` | Add `strict_mode.rs` integration tests verifying Defects #1–#10 | done | flash | `6b2ac04` |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

## 3.1.1
- **Status:** `done`
- **Files created/changed:** `crates/newhall-core/src/forcing.rs` (644 lines; +221/-12); `planning/tasks/3.1.1.todo.md`
- **Code commit ID:** `488ae8b` (cherry-picked from `fb0ad37`)
- **Verification:** `cargo test -p newhall-core --lib forcing` (10 passed, 0 failed)
- **Deviations:** none
- **Gotchas for dependent tasks:** `StrictThornthwaiteForcing` is at `crate::forcing::StrictThornthwaiteForcing` and awaits `lib.rs` re-export in `3.2.1`; `NaN` in `temp_c`, `precip_mm`, or `lat_dd` returns all-`NaN` `WaterBalance` fields.

## 3.1.2
- **Status:** `done`
- **Files created/changed:** `crates/newhall-core/src/thermal.rs` (1641 lines; +641/-10); `planning/tasks/3.1.2.todo.md`
- **Code commit ID:** `5deb0f3` (cherry-picked from `8f7be77`)
- **Verification:** `cargo test -p newhall-core --lib thermal` (11 passed, 0 failed); Wave 1 full suite: 40 unit + 13 adversarial + 4 golden-master green, clippy & fmt clean
- **Deviations:** none
- **Gotchas for dependent tasks:** `StrictThermal::evaluate` expects `water_balance.mpe` already in mm (handled by `StrictThornthwaiteForcing` when `!config.is_metric`) and keeps `RegimeSubdivision1` (`q`) / `RegimeSubdivision2` (`div`) typed enums per `ThermalClassification`.

## 3.2.1
- **Status:** `done`
- **Files created/changed:** `crates/newhall-core/src/engine.rs` (695 lines; +235/-4); `crates/newhall-core/src/lib.rs` (26 lines; +5/-3); `planning/tasks/3.2.1.todo.md`
- **Code commit ID:** `eab2740` (cherry-picked from `4b41123`)
- **Verification:** `cargo test --workspace --all-targets` (43 unit + 13 adversarial + 4 golden-master green), clippy & fmt clean
- **Deviations:** Used `ClassicHydrology.simulate_hydrology(...)` unit-struct syntax instead of `ClassicHydrology::default()` to satisfy `clippy::default-constructed-unit-structs`.
- **Gotchas for dependent tasks:** `StrictNewhall`, `StrictHydrology`, `StrictThornthwaiteForcing`, and `StrictThermal` are now exported at `newhall_core::*`; `SimEngine::run` sets `SiteOutput.water_holding_capacity` to raw `input.awc_mm` (unscaled when `is_metric = false`), while forcing, hydrology, day counters, and regimes use converted metric units.

## 3.3.1
- **Status:** `done`
- **Files created/changed:** `r-pkg/src/rust/src/lib.rs`, `r-pkg/R/newhall.R`, `r-pkg/R/extendr-wrappers.R`, `r-pkg/man/newhall_batch.Rd`, `r-pkg/man/newhall_batch_phase4.Rd`, `r-pkg/tests/testthat/test-phase4.R`; `planning/tasks/3.3.1.todo.md`
- **Code commit ID:** `59e515e` (cherry-picked from `a2aedc6`)
- **Verification:** `devtools::test('r-pkg')` (`27` PASS, `0` FAIL, `0` WARN, `0` SKIP after rebuilding `libnewhallrs.a`)
- **Deviations:** none
- **Gotchas for dependent tasks:** `newhall_batch` defaults `strict = FALSE` for `jNSMR` parity, whereas `newhall_batch_phase4` defaults `strict_forcing = TRUE`; when `is_metric = FALSE` and `strict = TRUE`, callers pass `soil_air_offset` in °F difference and `waterHoldingCapacity` retains raw `awc` input units.

## 3.3.2
- **Status:** `done`
- **Files created/changed:** `crates/newhall-core/tests/strict_mode.rs` (548 lines); `planning/tasks/3.3.2.todo.md`
- **Code commit ID:** `6b2ac04` (cherry-picked from `ac5746a`)
- **Verification:** `cargo test --workspace --all-targets` (`43` unit + `13` adversarial + `4` golden-master + `8` strict_mode green), `clippy` & `fmt` clean
- **Deviations:** none
- **Gotchas for dependent tasks:** none
