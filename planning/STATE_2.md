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
| 2 | `2.2.1` | Implement Thornthwaite PET & latitude forcing (`forcing.rs`) | pending | flash | — |
| 2 | `2.2.2` | Implement 8×8 soil moisture loop & 360-day calendar (`hydrology.rs`) | pending | flash | — |
| 2 | `2.2.3` | Implement thermal model, day counters & STR/SMR classifiers (`thermal.rs`) | pending | flash | — |
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
