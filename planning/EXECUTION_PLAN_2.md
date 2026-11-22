# Execution Plan 2: Cargo Workspace & Traits (`G3`) + `newhall-core` Parity Implementation (`G4`)

## Goal & Scope

### Goal
Scaffold the pure-Rust Cargo workspace (`crates/newhall-core` and `crates/newhall-cli`) with static-dispatch physics traits (`ThermalModel`, `ForcingModel`, `HydrologyStep`), `Copy` per-site state, stack-allocated `[f64; 64]` moisture profile, and integer enum discriminants (`G3`), then implement the exact `jNSMR` `v1.6.5` (`runBatch2` / `runSimulation(double[], ...)`) Newhall Simulation Model in `newhall-core`—replicating all 11 documented source defects verbatim—and verify 100% categorical parity and `<= 1e-9` floating-point parity against all three golden-master fixtures (`G4`).

### In Scope
- **Wave 1 (`G3` — Workspace, Types & Traits):**
  - `.gitignore`, workspace `Cargo.toml` (`crates/newhall-core`, `crates/newhall-cli`; `r-pkg/src/rust` excluded until `G6`).
  - `crates/newhall-cli` minimal compilable binary stub.
  - `crates/newhall-core/Cargo.toml` (`edition = "2021"`, optional `parallel` feature with `rayon` enabled by default, zero other required runtime dependencies).
  - `crates/newhall-core/src/types.rs` (`SiteInput`, `SiteConfig`, `SiteOutput`, `SoilState` with `[f64; 64]` and `Copy`, `u8` enum discriminants for `TemperatureRegime`, `MoistureRegime`, `RegimeSubdivision1`, `RegimeSubdivision2` matching `jNSMR` `0..k-1` codes and exact Java/R strings, with `Option<TemperatureRegime>` for `NaN` temperature `""`).
  - `crates/newhall-core/src/traits.rs` (`ThermalModel`, `ForcingModel`, `HydrologyStep`, `SimEngine<T, F, H>`), plus empty placeholder modules (`forcing.rs`, `hydrology.rs`, `thermal.rs`, `engine.rs`) wired in `lib.rs` so Wave 2 modules compile and test independently.
- **Wave 2 (`G4` Part 1 — Parallel Core Physics Modules):**
  - `crates/newhall-core/src/forcing.rs`: `ThornthwaiteForcing` (`ForcingModel`), `rn`/`inz`, `rs`/`fs`, `zt`/`zpe` tables, `mwi`/`swi`/`a`/`upe`/`mpe`, balances (including verbatim SH `* 60.0D` precedence bug and signed `latDD`).
  - `crates/newhall-core/src/hydrology.rs`: `ClassicHydrology` (`HydrologyStep`), `dp[64]`/`dr[64]`, 8×8 `[f64; 64]` recharge/discharge loop, 10-year equilibration, 360-day calendar (`nd[1..3]`, `iday[1..360]`).
  - `crates/newhall-core/src/thermal.rs`: `ClassicThermal` (`ThermalModel`), `>5°C` and `>8°C` crossing day counters, 120-day solstice windows (`nccd`, `nccm`), STR classification (`reg[1..10]`, `cryic_ht`), and SMR + `q`/`div` classification tree (including verbatim `Pergelic` overwrite, unattenuated `dif = st - wt`, `ncpm[2]` fallback, `hasRunAlready`, `q`/`div` swap, `NaN`/`Perudic`, and `5.01`/`8.01` perturbation).
- **Wave 3 (`G4` Part 2 — Engine & SoA Batch Integration):**
  - `crates/newhall-core/src/engine.rs` + `src/lib.rs`: `SimEngine<T, F, H>::run()`, `ClassicNewhall` default engine, and SoA batch runner (`run_batch_soa`) with `#[cfg(feature = "parallel")]` `rayon` parallelism.
- **Wave 4 (`G4` Acceptance — Golden-Master Parity Verification):**
  - `crates/newhall-core/tests/golden_master.rs`: Integration tests against `pa_batch_metric.csv` (108 rows), `prism_sample.csv` (18,984 rows / 18,790 valid), and `daymet_sample.csv` (18,984 rows / 18,790 valid) verifying `<= 1e-9` float error, exact integer day counts, and 100% exact categorical match.

### Out of Scope (Deferred to Later Orchestrations)
- `newhall-cli` full CLI flags, CSV runner, and throughput benchmarking (`G5`).
- `r-pkg/` `extendr` bindings, `R/` wrappers, `terra::set.cats()`, and `R CMD check` (`G6`, `G7`, `G8`).
- Extended physics trait implementations and `ARCHITECTURE.md` (`G9`, `G10`).
- Fixing any of the 11 documented Source Defects (deferred until after parity is demonstrated).
- Legacy `runBatch()` / `NewhallDataset` (`v1.6.1`/`v1.6.3`) `cs`-attenuated thermal path (Defect #11) and `temperatureCalendar` (`[i8; 360]`) diagnostic array (not part of the 19 `batch3()` outputs).
- Modifying anything in `/home/andrew/workspace/jNSMR`, `scripts/`, or `crates/newhall-core/tests/fixtures/`.

### Verbatim Constraints
- "100% numerical parity with the existing customized Java model as a verified baseline."
- "Zero heap allocations in the inner simulation loop: Represent the 8×8 moisture profile matrix as a stack-allocated fixed-size array (`[f64; 64]`). Keep per-site simulation state in `Copy` structs so hot loops stay in L1 cache."
- "Acceptance criteria: `cargo test` passes with floating-point outputs within `1e-9` of Java and 100% exact match on all SMR/STR/subdivision classifications."
- "Keep commits small, logical, and green (`cargo test` passing)."
- "for now replicate all errors verbatim, and write them up in GOALS.md 'source defects' section. we will want to be able to demonstrate adequate parity and then will systematically fix all defects."

---

## Decision Log

- **2026-11-22 — Pure-Rust Cargo workspace (`G3`):** Root `Cargo.toml` members are `crates/newhall-core` and `crates/newhall-cli` (excluding `r-pkg/src/rust` until `G6`), keeping `cargo test --workspace` pure-Rust and free of `libR` linking requirements.
- **2026-11-22 — Rust edition & `newhall-core` dependencies:** Use `edition = "2021"` for broad MSRV/CRAN/R-universe compatibility, put `rayon` behind an optional `parallel` feature (enabled by default), and keep zero other required runtime dependencies in `newhall-core`.
- **2026-11-22 — Git tracking & baseline commit:** Track `planning/` in git using separate `docs(planning):` commits, ignore `target/` and `.development/`, and create an initial baseline commit (`planning/` + `scripts/` + `fixtures/`) before Wave 1 starts.
- **2026-11-22 — Source defects parity policy:** Replicate all 11 defects documented in `planning/GOALS.md` (`## Source Defects`) and `planning/AUDIT_JNSM.md` verbatim in `newhall-core`.
- **2026-11-22 — Workers & concurrency:** `flash` workers, escalate to `pro`/frontier after 2 failed attempts, max **3 concurrent workers** per wave.

---

## Recon & Hazard Notes

- **Toolchain:** `rustc 1.99.1`, `cargo 1.99.1` installed and ready.
- **Parity spec & defect catalog:** `planning/AUDIT_JNSM.md` (185 lines), `planning/audit/java_core_audit.md` (266 lines), `planning/audit/r_api_audit.md` (209 lines), `planning/GOALS.md` (`## Source Defects`).
- **Golden-master fixtures:** `crates/newhall-core/tests/fixtures/{pa_batch_metric.csv, prism_sample.csv, daymet_sample.csv, manifest.json}`. Note: categorical columns are double-quoted; `" "` (single space) in `regimeSubdivision1`/`2` represents blank subdivision (`Perudic` / `Undefined` rows); `is.na(awc)` rows have `"NA"` in `awc` and all 19 output columns.
- **Hazard — Java `0.0` vs `-0.0` & 1-indexed arrays:** Java `BASICSimulationModel` uses 1-indexed arrays (`double[13]`, `sl[1..64]`, `nd[1..3]`, `iday[1..360]`). Port indices carefully (`0..12`, `0..63`, `0..2`, `0..359`) without off-by-one errors.
- **Hazard — Exact lookup tables:** `rn`, `inz`, `rs`, `fs`, `zt`, `zpe`, `dp`, `dr` must be transcribed verbatim from `/home/andrew/workspace/jNSMR/java/org/psu/newhall/sim/BASICSimulationModelConstants.java` (82 lines, read-only).

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `2.1.1` | Scaffold Cargo workspace, `types.rs`, `traits.rs`, and `newhall-cli` stub | `newhall-rs` | — | Create: `.gitignore`, `Cargo.toml`, `crates/newhall-cli/Cargo.toml`, `crates/newhall-cli/src/main.rs`, `crates/newhall-core/Cargo.toml`, `crates/newhall-core/src/lib.rs`, `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/traits.rs`, `crates/newhall-core/src/{forcing,hydrology,thermal,engine}.rs` (empty stubs) |
| 2 | `2.2.1` | Implement Thornthwaite PET & latitude forcing (`forcing.rs`) | `newhall-rs` | `2.1.1` | Edit: `crates/newhall-core/src/forcing.rs` |
| 2 | `2.2.2` | Implement 8×8 soil moisture loop & 360-day calendar (`hydrology.rs`) | `newhall-rs` | `2.1.1` | Edit: `crates/newhall-core/src/hydrology.rs` |
| 2 | `2.2.3` | Implement thermal model, day counters & STR/SMR classifiers (`thermal.rs`) | `newhall-rs` | `2.1.1` | Edit: `crates/newhall-core/src/thermal.rs` |
| 3 | `2.3.1` | Wire `SimEngine`, SoA batch runner (`engine.rs`), and `lib.rs` exports | `newhall-rs` | `2.2.1`, `2.2.2`, `2.2.3` | Edit: `crates/newhall-core/src/engine.rs`, `crates/newhall-core/src/lib.rs` |
| 4 | `2.4.1` | Add golden-master integration tests (`golden_master.rs`) & verify parity | `newhall-rs` | `2.3.1` | Create: `crates/newhall-core/tests/golden_master.rs` |

---

## Task Specifications

### Task `2.1.1`: Scaffold Cargo workspace, `types.rs`, `traits.rs`, and `newhall-cli` stub
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `.gitignore`, `Cargo.toml`, `crates/newhall-cli/Cargo.toml`, `crates/newhall-cli/src/main.rs`, `crates/newhall-core/Cargo.toml`, `crates/newhall-core/src/lib.rs`, `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/traits.rs`, `crates/newhall-core/src/forcing.rs`, `crates/newhall-core/src/hydrology.rs`, `crates/newhall-core/src/thermal.rs`, `crates/newhall-core/src/engine.rs`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* Anything in `planning/`, `scripts/`, `crates/newhall-core/tests/fixtures/`, or `/home/andrew/workspace/jNSMR`.
- **Depends on:** None
- **Spec:**
  1. Create `.gitignore` ignoring `target/`, `.development/`, and `*.rs.bk`.
  2. Create root `Cargo.toml` with `[workspace]` `members = ["crates/newhall-core", "crates/newhall-cli"]`, `exclude = ["r-pkg/src/rust"]`, `resolver = "2"`.
  3. Create `crates/newhall-cli/Cargo.toml` (`edition = "2021"`, depends on `newhall-core`) and minimal `crates/newhall-cli/src/main.rs`.
  4. Create `crates/newhall-core/Cargo.toml` (`edition = "2021"`, `[features] default = ["parallel"]; parallel = ["dep:rayon"]`, optional `rayon = "1"`).
  5. Implement `crates/newhall-core/src/types.rs`:
     - `#[repr(u8)]` `Copy` enums: `TemperatureRegime` (`0..=9`), `MoistureRegime` (`0..=5`), `RegimeSubdivision1` (`0..=8`), `RegimeSubdivision2` (`0..=7`) matching `planning/AUDIT_JNSM.md` Section 3 discriminants and `.as_java_str()` / `.as_r_label()` methods (including `" "` vs `"Undefined"`).
     - `Copy` structs: `SiteInput` (`lat_dd: f64`, `lon_dd: f64`, `temp_c: [f64; 12]`, `precip_mm: [f64; 12]`, `awc_mm: f64`, `elev_m: f64`), `SiteConfig` (`soil_air_offset: f64`, `amplitude: f64`, `has_o_horizon: bool`, `is_saturated: bool`, `is_metric: bool` — note Defect #2: `is_metric` is stored for parity but must NOT trigger unit conversions in `newhall-core`), `SoilState` (`profile: [f64; 64]`, `iday: [u8; 360]`, `nd: [f64; 3]`), `WaterBalance` (`mpe: [f64; 12]`, `annual_rainfall: f64`, `annual_pet: f64`, `annual_water_balance: f64`, `summer_water_balance: f64`), `DayCounters`, and `SiteOutput` (matching all 19 `batch3()` output fields, with `temperature_regime: Option<TemperatureRegime>` where `None` represents Java's `""` / `NA` on `NaN` temperature).
  6. Implement `crates/newhall-core/src/traits.rs`:
     - Static-dispatch traits: `ForcingModel`, `HydrologyStep`, `ThermalModel`, plus generic `SimEngine<T, F, H>` struct definition.
  7. Create empty placeholder module files (`forcing.rs`, `hydrology.rs`, `thermal.rs`, `engine.rs`), wire `pub mod` declarations and re-exports in `crates/newhall-core/src/lib.rs`, and add unit tests in `types.rs` verifying `#[repr(u8)]` discriminants and `Copy` bounds.
- **Grep anchors:** `planning/AUDIT_JNSM.md` (Sections 2, 3), `planning/INITIAL_PROMPT.md` (Extensibility Design).
- **Acceptance & targeted verification:**
  - `cargo check --workspace` exits 0.
  - `cargo test --workspace` exits 0.

---

### Task `2.2.1`: Implement Thornthwaite PET & latitude forcing (`forcing.rs`)
- **Repo:** `/home/andrew/workspace/newhall-rs` (read-only access to `/home/andrew/workspace/jNSMR/java`)
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/forcing.rs`
  - *Do not touch:* Any other file in `newhall-rs` or anything in `/home/andrew/workspace/jNSMR`.
- **Depends on:** `2.1.1`
- **Spec:**
  1. Transcribe exact `rn[31]`, `inz[12][31]`, `rs[13]`, `fs[12][13]`, `zt[24]`, `zpe[24]` lookup tables from `BASICSimulationModelConstants.java`.
  2. Implement `ThornthwaiteForcing` (`Copy`, `Default`) implementing `ForcingModel`:
     - `mwi`, `swi`, `a`, `upe[12]`, and `mpe[12]` using exact `1..=12` summation order (ignoring `config.is_metric` per Defect #2).
     - Hemisphere detection (`lat_dd >= 0.0` -> `'N'`, else `'S'`), preserving verbatim Defect #1 (`* 60.0D` precedence bug for SH `5.0 <= latDD < 50.0`) and Defect #7 (signed `latDD` passed to SH branch).
     - Compute `annual_rainfall`, `annual_pet`, `annual_water_balance`, and `summer_water_balance` (JJA for NH, DJF for SH).
  3. Add unit tests inside `forcing.rs` (`#[cfg(test)]`) covering NH, SH `0..5`, SH `5..50`, high-temp `>= 26.5`/`>= 38.0`, and `T <= 0.0` cases.
- **Grep anchors:** `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/traits.rs`, `planning/audit/java_core_audit.md`, `/home/andrew/workspace/jNSMR/java/org/psu/newhall/sim/BASICSimulationModelConstants.java` (`rn`, `inz`, `rs`, `fs`, `zt`, `zpe`), `/home/andrew/workspace/jNSMR/java/org/psu/newhall/sim/BASICSimulationModel.java` (lines `65-201` PET, `6287-6311` `computeWaterBalance`).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --lib forcing` exits 0.

---

### Task `2.2.2`: Implement 8×8 soil moisture loop & 360-day calendar (`hydrology.rs`)
- **Repo:** `/home/andrew/workspace/newhall-rs` (read-only access to `/home/andrew/workspace/jNSMR/java`)
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/hydrology.rs`
  - *Do not touch:* Any other file in `newhall-rs` or anything in `/home/andrew/workspace/jNSMR`.
- **Depends on:** `2.1.1`
- **Spec:**
  1. Transcribe exact `dp[64]` and `dr[64]` arrays from `BASICSimulationModelConstants.java` (converting 1-based slot indices `1..=64` to `0..64` cleanly).
  2. Implement `ClassicHydrology` (`Copy`, `Default`) implementing `HydrologyStep`:
     - Zero heap allocations: operates on `[f64; 64]` stack array (`sl`) and `[u8; 360]` (`iday`), ignoring `config.is_metric` per Defect #2.
     - 3-step monthly cycle (15d -> mid-month storm `P[m]/2` -> 15d) for 12 months × up to 10 years, with early equilibration exit `(tmoi - prmo).abs() < prmo / 100.0`.
     - Exact MCS `k` (`1`=Dry, `2`=Moist/Dry, `3`=Moist) inspection on slots `9, 17, 25` (`[8], [16], [24]` 0-indexed).
     - Exact final-year day allocation (`pmc`, `dpmc`, `(15.0 * (cnpe - npe) / cnpe) as i32 - dpmc`) into `nd[0..3]` and `iday[0..360]`.
  3. Add unit tests inside `hydrology.rs` (`#[cfg(test)]`) verifying `dp`/`dr` invariants, recharge/discharge conservation, and 360-day calendar totals (`sum(nd) == 360.0`).
- **Grep anchors:** `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/traits.rs`, `planning/audit/java_core_audit.md`, `/home/andrew/workspace/jNSMR/java/org/psu/newhall/sim/BASICSimulationModelConstants.java` (`dp`, `dr`), `/home/andrew/workspace/jNSMR/java/org/psu/newhall/sim/BASICSimulationModel.java` (lines `284-1344` Hydrology, `6315-6377` `computeSoilCalendar`).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --lib hydrology` exits 0.

---

### Task `2.2.3`: Implement thermal model, day counters & STR/SMR classifiers (`thermal.rs`)
- **Repo:** `/home/andrew/workspace/newhall-rs` (read-only access to `/home/andrew/workspace/jNSMR/java`)
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/thermal.rs`
  - *Do not touch:* Any other file in `newhall-rs` or anything in `/home/andrew/workspace/jNSMR`.
- **Depends on:** `2.1.1`
- **Spec:**
  1. Implement `ClassicThermal` (`Copy`, `Default`) implementing `ThermalModel` (ignoring `config.is_metric` per Defect #2):
     - `tma`, `st`, `wt`, `dif = (at1 - at2).abs()`, `cryic_ht` (`6`/`8`/`13`/`15`), and STR `reg[1..10]` last-match-wins classification (including Defect #3 `Pergelic` overwrite by `Frigid`/`Isofrigid`, and `None` for `NaN` temperature per Defect #9).
     - `>5°C` crossing solver (`ia = 36`, `iz = 25`, `5.0 -> 5.01` local copy perturbation without mutating caller input, `hasRunAlready` single-interval `nsd` guard — Defect #6, #10).
     - `>8°C` crossing solver (`ia = 30`, `iz = 30`, `8.0 -> 8.01` local copy perturbation, `ncpm[1]` on 720-day ring, `ncpm[2]` with `sn == 0` fallback `ncpm[2] = ncpm[1]` — Defect #5).
     - 120-day solstice windows (`nccd`, `nccm`, `dryDaysAfterSummerSolstice`, `moistDaysAfterWinterSolstice`).
     - SMR + `q`/`div` classification tree using unattenuated `dif = st - wt` (Defect #4), `q`/`div` swap (Defect #8), and `NaN`/`Perudic` behavior (Defect #9).
  2. Add unit tests inside `thermal.rs` (`#[cfg(test)]`) covering STR/SMR branches, `5.0`/`8.0` perturbation, and cold-site `ncpm[2]` fallback.
- **Grep anchors:** `crates/newhall-core/src/types.rs`, `crates/newhall-core/src/traits.rs`, `planning/audit/java_core_audit.md`, `/home/andrew/workspace/jNSMR/java/org/psu/newhall/sim/BASICSimulationModel.java` (lines `205-282` STR, `1353-2991` Day counters, `2999-3088` SMR, `3155-3165` `NewhallResults`).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --lib thermal` exits 0.

---

### Task `2.3.1`: Wire `SimEngine`, SoA batch runner (`engine.rs`), and `lib.rs` exports
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/engine.rs`, `crates/newhall-core/src/lib.rs`
  - *Do not touch:* `forcing.rs`, `hydrology.rs`, `thermal.rs`, `crates/newhall-core/tests/fixtures/`, or `/home/andrew/workspace/jNSMR`.
- **Depends on:** `2.2.1`, `2.2.2`, `2.2.3`
- **Spec:**
  1. Implement `SimEngine<T: ThermalModel, F: ForcingModel, H: HydrologyStep>::run(&self, input: &SiteInput, config: &SiteConfig) -> SiteOutput` in `crates/newhall-core/src/engine.rs` with zero heap allocations (leaving `is_metric` unconverted per Defect #2).
  2. Provide `ClassicNewhall` type alias / helper `simulate_site(...)` and SoA batch runner `run_batch_soa(...)` operating on flat `&[f64]` slices (with `#[cfg(feature = "parallel")]` `rayon::prelude::*` parallelization and sequential fallback).
  3. Update `crates/newhall-core/src/lib.rs` public re-exports for `forcing`, `hydrology`, `thermal`, and `engine`.
  4. Run `cargo test --workspace` and verify all unit tests in `types`, `forcing`, `hydrology`, `thermal`, and `engine` compile and pass.
- **Grep anchors:** `crates/newhall-core/src/lib.rs`, `crates/newhall-core/src/traits.rs`.
- **Acceptance & targeted verification:**
  - `cargo test --workspace` exits 0.
  - `cargo test --workspace --no-default-features` (without `parallel` feature) exits 0.

---

### Task `2.4.1`: Add golden-master integration tests (`golden_master.rs`) & verify parity
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `crates/newhall-core/tests/golden_master.rs`
  - *Do not touch:* `crates/newhall-core/tests/fixtures/*`, `scripts/*`, `/home/andrew/workspace/jNSMR`.
- **Depends on:** `2.3.1`
- **Spec:**
  1. Implement `crates/newhall-core/tests/golden_master.rs` parsing `pa_batch_metric.csv` (108 rows), `prism_sample.csv` (18,984 rows), and `daymet_sample.csv` (18,984 rows) using std-only CSV line/quote parsing (handling double-quoted categorical columns including `" "` and `"NA"`).
  2. For every valid row (`awc` != `"NA"`):
     - Assert all 5 `f64` outputs (`annualRainfall`, `waterHoldingCapacity`, `annualWaterBalance`, `annualPotentialEvapotranspiration`, `summerWaterBalance`) match within `1e-9`.
     - Assert all 10 `i32` day-counter outputs match exactly.
     - Assert all 4 categorical outputs (`temperatureRegime`, `moistureRegime`, `regimeSubdivision1`, `regimeSubdivision2`) match 100% exactly via `.as_java_str()` (using `""` when `temperature_regime` is `None`).
  3. For the 194 `awc == "NA"` rows in `prism_sample.csv` and `daymet_sample.csv`, assert all 19 fixture output columns are `"NA"`, and verify `simulate_site` / `run_batch_soa` handle `f64::NAN` `awc` without panicking. Also test `run_batch_soa` against `pa_batch_metric.csv`.
- **Grep anchors:** `crates/newhall-core/tests/fixtures/manifest.json`, `crates/newhall-core/src/engine.rs`, `crates/newhall-core/src/types.rs`.
- **Acceptance & targeted verification:**
  - `cargo test --workspace --release` exits 0 with all golden-master tests passing.
