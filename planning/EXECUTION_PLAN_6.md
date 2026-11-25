# Execution Plan 6: `G16` Core/R Optimizations, `G5` CLI (`run`/`verify`/`bench`), and `G10` `ARCHITECTURE.md`

## Goal & Scope

### Goal
Complete the remaining project goals in a single multi-wave orchestration:
1. **`G16` (Wave 1):** Replace `powf` with fast integer/sqrt powers in Gardner (1958) capillary rise, eliminate unnecessary `[f64; 360]` copies in `SoilState`, and allocate R output vectors directly (`Doubles`/`Integers`) in `r-pkg/src/rust/src/lib.rs` without intermediate `Vec` copies.
2. **`G5` & `G10` (Wave 2, parallel):** Expand `crates/newhall-cli` with `run` (CSV batch), `verify` (golden-master parity), and `bench` (`>100,000 sims/sec` multi-core `ClassicNewhall` verification), and write `ARCHITECTURE.md` documenting how to implement, test, and register new physics models in `newhall-core`.
3. **Integration & Verification (Wave 3):** Sync `newhall-core` into `r-pkg/src/rust/newhall-core/` via `r-pkg/tools/sync-core.sh`, verify all Rust + R test suites are 100% green, and record updated `newhall-cli bench` and `bench_jnsmr.R` numbers.

### Verbatim Constraints
- "All Phase 1–3 golden-master parity tests (`jNSMR` / Java baseline) MUST remain 100% green and unchanged. Legacy behavior is the default; Phase 4 physics are opt-in trait implementations."
- "Keep `newhall-core` free of R/`extendr` dependencies."

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `6.1.1` | Fast Gardner powers & lean `SoilState` in `newhall-core` (`G16`) | `newhall-rs` | — | Edit: `crates/newhall-core/src/water_table.rs`, `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/hydrology.rs`, `crates/newhall-core/src/engine.rs` (only if `SoilState` fields change) |
| 1 | `6.1.2` | Direct R vector allocation in `r-pkg` `outputs_to_r_list()` (`G16`) | `newhall-rs` | — | Edit: `r-pkg/src/rust/src/lib.rs` |
| 2 | `6.2.1` | Add `newhall-cli` `run`, `verify`, and `bench` commands (`G5`) | `newhall-rs` | `6.1.1` | Edit: `crates/newhall-cli/src/main.rs`, `crates/newhall-cli/Cargo.toml` (only if needed) |
| 2 | `6.2.2` | Write `ARCHITECTURE.md` physics & extension guide (`G10`) | `newhall-rs` | `6.1.1`, `6.1.2` | Create: `ARCHITECTURE.md` |
| 3 | `6.3.1` | Sync `newhall-core` to `r-pkg`, run full suites & refresh benchmarks | `newhall-rs` | `6.1.1`, `6.1.2`, `6.2.1`, `6.2.2` | Edit: `r-pkg/src/rust/newhall-core/` (via `sync-core.sh`), `planning/benchmarks/jnsmr_raster_benchmark.md` |

---

## Task Specifications

### Task `6.1.1`: Fast Gardner powers & lean `SoilState` in `newhall-core` (`G16`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/water_table.rs`, `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/hydrology.rs` (and `crates/newhall-core/src/engine.rs` only if `SoilState` layout changes require it)
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `r-pkg/`, `crates/newhall-cli/`, `crates/newhall-core/tests/`.
- **Depends on:** None
- **Spec:**
  1. In `crates/newhall-core/src/water_table.rs`, replace `powf()` in Gardner (1958) capillary rise with fast integer/sqrt powers (e.g., `x * x`, `x * x * x.sqrt()`, `x * x * x`, etc. matched on `SoilTextureClass` exponents) while keeping numerical equivalence within `1e-12` and preserving all `adversarial_phase4.rs` invariants.
  2. Inspect `SoilState` in `crates/newhall-core/src/types.rs` and its usage in `hydrology.rs` / `water_table.rs` / `engine.rs`: eliminate any unconditional `[f64; 360]` or large-array copies in `ClassicHydrology` / `SoilState` while keeping `SoilState: Copy` and preserving 100% golden-master and Phase 4 parity.
- **Grep anchors:** `crates/newhall-core/src/water_table.rs`, `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/hydrology.rs`.
- **Acceptance & targeted verification:**
  - `cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check` exits 0.

---

### Task `6.1.2`: Direct R vector allocation in `r-pkg` `outputs_to_r_list()` (`G16`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/rust/src/lib.rs`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/R/`, `r-pkg/tests/`.
- **Depends on:** None
- **Spec:**
  1. Refactor `outputs_to_r_list(outputs: &[SiteOutput], include_phase4: bool) -> List` in `r-pkg/src/rust/src/lib.rs` to allocate `extendr_api::Doubles` and `extendr_api::Integers` directly (using `Doubles::new(n)` / `Integers::new(n)` and writing elements in-place via `as_real_slice_mut()` or `set_elt`, with `Rint::na()` for `None` temperature regimes) instead of building 19–21 intermediate Rust `Vec`s and copying them into R vectors.
  2. Verify all R tests pass with identical values, types, and `NA` handling.
- **Grep anchors:** `r-pkg/src/rust/src/lib.rs` (`outputs_to_r_list`).
- **Acceptance & targeted verification:**
  - `cargo clippy --manifest-path r-pkg/src/rust/Cargo.toml -- -D warnings && Rscript -e "devtools::test('r-pkg')"` exits 0 with 0 failures and 0 warnings.

---

### Task `6.2.1`: Add `newhall-cli` `run`, `verify`, and `bench` commands (`G5`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-cli/src/main.rs`
  - *Can edit (only if needed):* `crates/newhall-cli/Cargo.toml`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/`.
- **Depends on:** `6.1.1`
- **Spec:**
  1. Add subcommand dispatch in `crates/newhall-cli/src/main.rs` (zero external CLI crate dependencies needed — simple `std::env::args()` dispatch):
     - `run <input.csv> [--output <out.csv>] [--strict]`: reads a jNSM-format batch CSV (reusing the same CSV format as `golden_master.rs`), runs `ClassicNewhall` (or `StrictNewhall` with `--strict`), and writes the 19-column CSV results to `--output` or stdout.
     - `verify <input.csv> <expected.csv>`: runs `ClassicNewhall` on `<input.csv>` and verifies every row against `<expected.csv>` (`<= 1e-9` float tolerance, 100% exact categorical match), exiting non-zero on any mismatch.
     - `bench [N]`: retains the existing multi-stage benchmark (default `N = 100000` or user-specified `N`) and verifies multi-core `ClassicNewhall` throughput.
  2. Add unit tests in `crates/newhall-cli/src/main.rs` covering CSV parsing/formatting and `verify` pass/fail behavior.
- **Grep anchors:** `crates/newhall-cli/src/main.rs`, `crates/newhall-core/tests/golden_master.rs`.
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-cli && cargo clippy -p newhall-cli --all-targets -- -D warnings && cargo run --release -p newhall-cli -- verify crates/newhall-core/tests/fixtures/pa_batch_metric_input.csv crates/newhall-core/tests/fixtures/pa_batch_metric_expected.csv && cargo run --release -p newhall-cli -- bench 100000` exits 0.

---

### Task `6.2.2`: Write `ARCHITECTURE.md` physics & extension guide (`G10`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `ARCHITECTURE.md`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/`, `r-pkg/`.
- **Depends on:** `6.1.1`, `6.1.2`
- **Spec:**
  1. Create `ARCHITECTURE.md` at the repository root covering:
     - Workspace layout (`crates/newhall-core`, `crates/newhall-cli`, `r-pkg`, `planning/`).
     - Static-dispatch physics architecture (`SimEngine<T, F, H>`, `ThermalModel`, `ForcingModel`, `HydrologyStep`) and zero-allocation `Copy` state (`SiteInput`, `SiteConfig`, `SoilState`, `SiteOutput`, `SoaBatchInput`).
     - Model families: `ClassicNewhall` (verbatim `jNSMR` parity), `StrictNewhall` (remediated defects `#1`–`#10`), and Phase 4 (`Thermal1DModel`, `WaterTableHydrology`).
     - R package (`r-pkg`) architecture: `extendr-api` `0.8` direct-allocation bridge, `r-pkg/tools/sync-core.sh` standalone tarball staging, S3 dispatch (`default`, `data.frame`, `character`, `SpatRaster`).
     - Step-by-step developer guide: how to implement, test (unit + `adversarial_phase4` + `proptest`), and register a new physics model without breaking `ClassicNewhall` parity.
- **Grep anchors:** `crates/newhall-core/src/lib.rs`, `crates/newhall-core/src/traits.rs`, `crates/newhall-core/src/engine.rs`, `r-pkg/src/rust/src/lib.rs`.
- **Acceptance & targeted verification:**
  - `test -s ARCHITECTURE.md` and every file path / Rust symbol referenced in `ARCHITECTURE.md` exists in the repo.

---

### Task `6.3.1`: Sync `newhall-core` to `r-pkg`, run full suites & refresh benchmarks
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/rust/newhall-core/` (via `bash r-pkg/tools/sync-core.sh`), `planning/benchmarks/jnsmr_raster_benchmark.md`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/R/`, `r-pkg/tests/`.
- **Depends on:** `6.1.1`, `6.1.2`, `6.2.1`, `6.2.2`
- **Spec:**
  1. Run `bash r-pkg/tools/sync-core.sh` to sync the updated `crates/newhall-core/` into `r-pkg/src/rust/newhall-core/`.
  2. Run the full Rust workspace suite (`cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`) and full R test suite (`Rscript -e "devtools::test('r-pkg')"`).
  3. Re-run `Rscript r-pkg/inst/benchmarks/bench_jnsmr.R` and `cargo run --release -p newhall-cli -- bench 100000`, and append a post-`G16` comparison note to `planning/benchmarks/jnsmr_raster_benchmark.md`.
- **Grep anchors:** `r-pkg/tools/sync-core.sh`, `planning/benchmarks/jnsmr_raster_benchmark.md`.
- **Acceptance & targeted verification:**
  - `bash r-pkg/tools/sync-core.sh && cargo test --workspace --all-targets && Rscript -e "devtools::test('r-pkg')"` exits 0 with 0 failures.
