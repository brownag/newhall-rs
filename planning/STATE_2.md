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
| 1 | `2.1.1` | Scaffold Cargo workspace, `types.rs`, `traits.rs`, and `newhall-cli` stub | pending | flash | — |
| 2 | `2.2.1` | Implement Thornthwaite PET & latitude forcing (`forcing.rs`) | pending | flash | — |
| 2 | `2.2.2` | Implement 8×8 soil moisture loop & 360-day calendar (`hydrology.rs`) | pending | flash | — |
| 2 | `2.2.3` | Implement thermal model, day counters & STR/SMR classifiers (`thermal.rs`) | pending | flash | — |
| 3 | `2.3.1` | Wire `SimEngine`, SoA batch runner (`engine.rs`), and `lib.rs` exports | pending | flash | — |
| 4 | `2.4.1` | Add golden-master integration tests (`golden_master.rs`) & verify parity | pending | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->
