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
| 2 | `6.2.1` | Add `newhall-cli` `run`, `verify`, and `bench` commands (`G5`) | done | flash | `763dd5e` |
| 2 | `6.2.2` | Write `ARCHITECTURE.md` physics & extension guide (`G10`) | done | flash | `837cd16` |
| 3 | `6.3.1` | Sync `newhall-core` to `r-pkg`, run full suites & refresh benchmarks | done | flash | `1297b50` |

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

## 6.2.1
- **status:** done
- **files:** edited `crates/newhall-cli/src/main.rs` (`+900/-13`)
- **commit:** `763dd5e`
- **verification:** `cargo test -p newhall-cli` PASS (6 passed); `verify` PASS on `pa_batch_metric.csv` (108 rows), `prism_sample.csv` (18,984 rows), `daymet_sample.csv` (18,984 rows); `bench 100000` PASS (`ClassicNewhall (rayon)`: `5,025,564 sims/sec`, `0.20 us/site`).
- **deviations:** none
- **gotchas:** `run` and `verify` accept both 28/19-col and 47-col jNSM CSVs.

## 6.2.2
- **status:** done
- **files:** created `ARCHITECTURE.md` (235 lines)
- **commit:** `837cd16`
- **verification:** `test -s ARCHITECTURE.md` PASS; all 27 backticked paths, 41 tree paths, and 177 symbols verified existing (0 missing).
- **deviations:** none
- **gotchas:** none.

## 6.3.1
- **status:** done
- **files:** synced `r-pkg/src/rust/newhall-core/src/{hydrology.rs,types.rs,water_table.rs}` and updated `planning/benchmarks/jnsmr_raster_benchmark.md` (`+32/-0`)
- **commit:** `1297b50`
- **verification:** `cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check` PASS (`6` CLI + `45` core unit + `13` adversarial + `4` golden_master + `8` strict_mode); `devtools::test('r-pkg')` PASS (`217` PASS, `0` FAIL, `0` WARN, `0` SKIP); `bench_jnsmr.R` PASS (`prism`: `0.0240s`, `379.7x`; `daymet`: `0.0220s`, `413.4x`); `newhall-cli bench 100000` PASS (`ClassicNewhall (rayon)`: `4,705,436 sims/sec`).
- **deviations:** none
- **gotchas:** `Makevars` has no prerequisites on `$(STATLIB)`; clean `libnewhallrs.a`/`newhallrs.so` after `sync-core.sh` to force rebuild.
