# Orchestration 2 State (`planning/STATE_2.md`)

Plan: `planning/EXECUTION_PLAN_2.md`
Last Updated: 2026-11-22

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs`
- [x] Rust toolchain verified (`rustc 1.99.1`, `cargo 1.99.1`)
- [x] Initial git baseline commit recorded (`planning/` + `scripts/` + `fixtures/`)

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `2.1.1` | Scaffold Cargo workspace, `types.rs`, `traits.rs`, and `newhall-cli` stub | done | flash | `103b349` |
| 2 | `2.2.1` | Implement Thornthwaite PET & latitude forcing (`forcing.rs`) | done | flash | `61ff37a` |
| 2 | `2.2.2` | Implement 8×8 soil moisture loop & 360-day calendar (`hydrology.rs`) | done | flash | `a8d5f8b` |
| 2 | `2.2.3` | Implement thermal model, day counters & STR/SMR classifiers (`thermal.rs`) | done | flash | `2da24ae` |
| 3 | `2.3.1` | Wire `SimEngine`, SoA batch runner (`engine.rs`), and `lib.rs` exports | pending | flash | — |
| 4 | `2.4.1` | Add golden-master integration tests (`golden_master.rs`) & verify parity | pending | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

## 2.1.1
- **status:** done
- **files created:** `.gitignore` (3), `Cargo.toml` (4), `Cargo.lock` (68), `crates/newhall-cli/Cargo.toml` (9), `crates/newhall-cli/src/main.rs` (6), `crates/newhall-core/Cargo.toml` (13), `crates/newhall-core/src/lib.rs` (14), `crates/newhall-core/src/types.rs` (550), `crates/newhall-core/src/traits.rs` (51), `crates/newhall-core/src/{forcing,hydrology,thermal,engine}.rs` (1 each)
- **commit:** `103b349` (`feat(core): scaffold Cargo workspace, Copy types, and physics traits`)
- **verification:** `cargo check --workspace` (exit 0), `cargo test --workspace` (5 passed, 0 failed), `cargo test --workspace --no-default-features` (5 passed, 0 failed)
- **deviations:** none (`Cargo.lock` generated and committed with workspace binary)
- **gotchas for dependent tasks:**
  - `RegimeSubdivision2::Undefined.as_java_str()` returns `"Undefined"`, whereas `Perudic` emits `" "` in Java; use `SiteOutput::regime_subdivision2_java_str()` for full-output Java CSV string parity.
  - `ForcingModel::compute_water_balance`, `HydrologyStep::simulate_hydrology`, and `ThermalModel::evaluate` signatures are fixed in `traits.rs`; implement `Default`/`Copy` structs (`ThornthwaiteForcing`, `ClassicHydrology`, `ClassicThermal`) against them.
  - `SiteConfig.is_metric` is stored for parity only and must remain a no-op inside `newhall-core` (Defect #2).

## 2.2.1
- **status:** done
- **files changed:** `crates/newhall-core/src/forcing.rs` (434 lines)
- **commit:** `61ff37a` (cherry-picked from `c99f421`, `feat(core): implement Thornthwaite PET and latitude forcing model`)
- **verification:** `cargo test -p newhall-core --lib forcing` (exit 0; 6 passed, 0 failed)
- **deviations:** none
- **gotchas for dependent tasks:**
  - `ForcingModel::compute_water_balance` derives hemisphere via `input.hemisphere()` (`lat_dd >= 0.0` -> North), so negative SH latitudes enter the SH `0..5` deg branch with signed `lat_dd` (Defect #7), while `5..50` SH interpolation (`* 60.0` precedence bug, Defect #1) is preserved in the internal `compute_with_hemisphere` helper.
  - `ThornthwaiteForcing` lives in `crate::forcing::ThornthwaiteForcing` and is not yet re-exported from `lib.rs` (left for Task `2.3.1`).

## 2.2.2
- **status:** done
- **files changed:** `crates/newhall-core/src/hydrology.rs` (499 lines)
- **commit:** `a8d5f8b` (cherry-picked from `b3d69dc`, `feat(core): implement ClassicHydrology soil moisture loop and 360-day calendar`)
- **verification:** `cargo test -p newhall-core --lib hydrology` (exit 0; 9 passed, 0 failed)
- **deviations:** none
- **gotchas for dependent tasks:**
  - `SoilState.iday` stores `1_u8` (Dry), `2_u8` (Moist/Dry), `3_u8` (Moist) in 0-indexed `[0..360]`; `SoilState.nd` stores `[dry, moist_dry, moist]` summing to `360.0`.
  - When `npe` hits `0.0` on the exact slot triggering an MCS transition in the final year, Java's early `break` leaves `k` reverted to `pmc` for the remaining half-month days (`ClassicHydrology` preserves this verbatim).

## 2.2.3
- **status:** done
- **files changed:** `crates/newhall-core/src/thermal.rs` (987 lines; +987/-1)
- **commit:** `2da24ae` (cherry-picked from `1d6bee8`, `feat(core): implement ClassicThermal day counters and STR/SMR classifiers`)
- **verification:** `cargo test -p newhall-core --lib thermal` (exit 0; 4 passed, 0 failed)
- **deviations:** none
- **gotchas for dependent tasks:**
  - `ClassicThermal` is a zero-sized `Copy` + `Default` struct implementing `ThermalModel` with zero heap allocations; wire it directly into `SimEngine` in `2.3.1`.
  - `consecutive_iday_run` replicates Java's exact lookahead `iday[n1]` boundary behavior (an isolated 1-day run yields `0` when `sw == 0`; runs continuing past window end count via `ifin`).
  - `soil_state.iday` must hold `1..=3`; invalid `0` values are safely ignored in `nzd` accumulation rather than panicking.
