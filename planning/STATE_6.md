# Orchestration 6 State (`planning/STATE_6.md`)

Plan: `planning/EXECUTION_PLAN_6.md`
Last Updated: 2026-11-24

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs`
- [x] Rust baseline green from `STATE_5.md` (`43` unit + `13` adversarial + `4` golden-master + `8` strict_mode, clippy & fmt clean)
- [x] R package baseline green from `STATE_5.md` + `9618e7a` (`217` PASS, `0` FAIL, `0` WARN, `0` SKIP; `extendr-api 0.8.2`)

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `6.1.1` | Fast Gardner powers & lean `SoilState` in `newhall-core` (`G16`) | done | flash | `25c3c0d` |
| 1 | `6.1.2` | Direct R vector allocation in `r-pkg` `outputs_to_r_list()` (`G16`) | done | flash | `6d9242c` |
| 2 | `6.2.1` | Add `newhall-cli` `run`, `verify`, and `bench` commands (`G5`) | pending | flash | — |
| 2 | `6.2.2` | Write `ARCHITECTURE.md` physics & extension guide (`G10`) | pending | flash | — |
| 3 | `6.3.1` | Sync `newhall-core` to `r-pkg`, run full suites & refresh benchmarks | pending | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

## 6.1.1
- **status:** done
- **files:** edited `crates/newhall-core/src/water_table.rs` (`+62/-10`), `crates/newhall-core/src/types.rs` (`+59/-3`), `crates/newhall-core/src/hydrology.rs` (`+2/-2`)
- **commit:** `25c3c0d`
- **verification:** `cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check` PASS (45 unit + 13 adversarial + 4 golden-master + 8 strict_mode).
- **deviations:** none
- **gotchas:** `SoilState.z_wt_daily` is now `HalfMonthlyWaterTable([f64; 24])` (`Index<usize>` on `day / 15`, `SoilState` shrunk from 3,784 B to 1,096 B). Sync into `r-pkg/src/rust/newhall-core/` in Wave 3 (`6.3.1`).

## 6.1.2
- **status:** done
- **files:** edited `r-pkg/src/rust/src/lib.rs` (`+113/-90`)
- **commit:** `6d9242c`
- **verification:** `cargo clippy --manifest-path r-pkg/src/rust/Cargo.toml -- -D warnings` PASS; `devtools::test('r-pkg')` PASS (`217` PASS, `0` FAIL, `0` WARN).
- **deviations:** none
- **gotchas:** `/tmp` is tight; set `TMPDIR=.development/tmp` for R `devtools::test`/`check` if needed.
