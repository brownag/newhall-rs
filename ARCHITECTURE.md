# Newhall Simulation Model (`newhall-rs`) Architecture & Extension Guide

`newhall-rs` is a pure-Rust reimplementation and modular physics extension of the Newhall Simulation Model (`jNSMR` / `newhall-1.6.5.jar`), paired with a native R package (`newhallrs` under `r-pkg/`) powered by `extendr-api` `0.8` and `rayon`.

The codebase enforces two core architectural invariants:
1. **Default `jNSMR` parity:** `ClassicNewhall` is the default simulation engine and reproduces `newhall-1.6.5.jar` (`BASICSimulationModel.runBatch2`) outputs verbatim.
2. **Pure-Rust core:** `crates/newhall-core` has no R or `extendr` dependencies (`rayon` is the only optional runtime dependency, enabled via the `parallel` feature).

---

## 1. Workspace Layout

The root `Cargo.toml` defines a two-crate Rust workspace and explicitly excludes `r-pkg/src/rust` so R builds remain standalone:

```text
newhall-rs/
├── Cargo.toml                     # Workspace root (members: newhall-core, newhall-cli; excludes r-pkg/src/rust)
├── crates/
│   ├── newhall-core/              # Pure-Rust physics engine, traits, Copy state types, and parity/adversarial tests
│   │   ├── Cargo.toml             # feature `parallel = ["dep:rayon"]` (on by default); dev-dep `proptest`
│   │   ├── src/
│   │   │   ├── lib.rs             # Public API re-exports
│   │   │   ├── traits.rs          # ForcingModel, HydrologyStep, ThermalModel, SimEngine<T, F, H>
│   │   │   ├── types.rs           # Copy input/state/output types and #[repr(u8)] regime enums
│   │   │   ├── engine.rs          # SimEngine::run, SoaBatchInput, ClassicNewhall, StrictNewhall, StrictHydrology
│   │   │   ├── forcing.rs         # ThornthwaiteForcing & StrictThornthwaiteForcing
│   │   │   ├── hydrology.rs       # ClassicHydrology (8x8 soil moisture matrix & 360-day calendar)
│   │   │   ├── thermal.rs         # ClassicThermal & StrictThermal (>5C/>8C solvers, STR/SMR classifiers)
│   │   │   ├── thermal_1d.rs      # Hydrothermal Thermal1DModel (16-node Backward Euler + Thomas solver)
│   │   │   └── water_table.rs     # Hydrothermal WaterTableHydrology (prescribed/prognostic WT + Gardner capillary)
│   │   └── tests/
│   │       ├── golden_master.rs   # Bit/1e-9 parity checks against jNSMR CSV fixtures
│   │       ├── strict_mode.rs     # Defect #1-#10 remediation tests across ClassicNewhall/StrictNewhall/hydrothermal
│   │       ├── adversarial_hydrothermal.rs # Conservation, Carslaw-Jaeger analytical, monotonicity, and proptest fuzzing
│   │       └── fixtures/          # manifest.json, pa_batch_metric.csv, daymet_sample.csv, prism_sample.csv
│   └── newhall-cli/               # Native Rust CLI & multi-stage throughput benchmark harness
│       ├── Cargo.toml
│       └── src/main.rs
├── r-pkg/                         # Standalone R package (`newhallrs`)
│   ├── DESCRIPTION                # SystemRequirements: Cargo; Suggests: jNSMR, terra, testthat
│   ├── NAMESPACE                  # Exported functions and S3 method registrations
│   ├── R/
│   │   ├── newhall.R              # newhall_batch / newhall_batch_hydrothermal S3 generics & raw/default/df/char methods
│   │   ├── raster.R               # Chunked terra::SpatRaster S3 methods and categorical table attachment
│   │   ├── categories.R           # .str_levels/.smr_levels/.smrsub1_levels/.smrsub2_levels & schema helpers
│   │   └── extendr-wrappers.R     # Auto-generated .Call() wrappers for wrap__newhall_batch*_rust
│   ├── src/
│   │   ├── Makevars               # R native build integration
│   │   ├── entrypoint.c           # C symbol registration entrypoint
│   │   └── rust/
│   │       ├── Cargo.toml         # Standalone Cargo crate (`staticlib`, `extendr-api = "0.8"`, `rayon = "1"`)
│   │       ├── src/lib.rs         # Zero-intermediate-Vec R bridge (`outputs_to_r_list`)
│   │       └── newhall-core/      # Vendored copy of crates/newhall-core (managed via sync-core.sh)
│   ├── tools/sync-core.sh         # Copies crates/newhall-core/{Cargo.toml,src} into r-pkg/src/rust/newhall-core/
│   ├── tests/testthat/            # test-jnsmr-parity.R, test-hydrothermal.R, test-s3-api.R
│   └── inst/benchmarks/bench_jnsmr.R
├── planning/                      # Audits, physics specs, execution plans, state logs, benchmarks
│   ├── AUDIT_JNSM.md              # jNSMR v0.3.1 / newhall-1.6.5.jar parity & Defect #1-#11 audit
│   ├── PHASE4_PHYSICS_SPEC.md     # 1D thermal & water-table mathematical specification
│   └── ADVERSARIAL_VALIDATION.md  # Invariant & analytical validation specification
└── scripts/                       # Helper R scripts (benchmark_hydrothermal.R, generate_golden_fixtures.R)
```

---

## 2. Static-Dispatch Physics Architecture & Zero-Allocation State

### 2.1 Composable 3-Stage Pipeline (`crates/newhall-core/src/traits.rs`, `engine.rs`)

Simulation physics are decomposed into three independent traits combined by `SimEngine<T, F, H>` via static dispatch (monomorphization, zero `dyn` vtable overhead):

```text
  SiteInput + SiteConfig
           │
           ▼
  F: ForcingModel::compute_water_balance(&self, input, config) ──▶ WaterBalance
           │
           ▼
  H: HydrologyStep::simulate_hydrology(&self, input, config, &wb) ──▶ SoilState
           │
           ▼
  T: ThermalModel::evaluate(&self, input, config, &wb, &soil) ──▶ ThermalClassification
           │
           ▼
       SiteOutput
```

- `ForcingModel`: computes monthly adjusted PET (`WaterBalance::mpe`), `annual_rainfall`, `annual_pet`, `annual_water_balance`, and `summer_water_balance`.
- `HydrologyStep`: equilibrates the 8×8 soil moisture matrix and emits `SoilState` (final 64-slot profile, 360-day moisture calendar `iday`, cumulative day totals `nd`, MCS saturation days `sat_days_mcs`, and half-monthly water-table depth `z_wt_daily`).
- `ThermalModel`: evaluates Soil Temperature Regime (STR), `>5°C` and `>8°C` growing-season day counters (`DayCounters`), Soil Moisture Regime (SMR) and subdivisions, plus hydrothermal `aquic_bio_days` into `ThermalClassification`.
- `SimEngine<T, F, H>`:
  - `SimEngine::new(thermal, forcing, hydrology)` constructs any static combination.
  - `SimEngine::run(&self, input: &SiteInput, config: &SiteConfig) -> SiteOutput` executes all three stages strictly on the stack with zero heap allocations.
  - `SimEngine::run_batch_soa(&self, batch: &SoaBatchInput<'_>, config: &SiteConfig) -> Vec<SiteOutput>` and `SimEngine::run_batch_soa_into(&self, batch, config, out: &mut [SiteOutput])` iterate across sites in parallel via `rayon` when `T: ThermalModel + Sync`, `F: ForcingModel + Sync`, `H: HydrologyStep + Sync` (falling back to sequential iteration when the `parallel` feature is disabled).
  - Top-level convenience functions `simulate_site` and `run_batch_soa` delegate to `ClassicNewhall::default()`.

### 2.2 Zero-Allocation `Copy` State & SoA Input (`crates/newhall-core/src/types.rs`, `engine.rs`)

Every per-site struct and enum in `newhall-core` implements `Copy` (enforced by `all_core_types_are_copy` in `types.rs`):

| Type | Purpose & Layout Highlights |
| --- | --- |
| `SiteInput` | Per-site climate & soil inputs: `lat_dd`, `lon_dd`, `temp_c: [f64; 12]`, `precip_mm: [f64; 12]`, `awc_mm`, `elev_m`; `hemisphere()` maps `lat_dd >= 0.0` to `Hemisphere::North`. |
| `SiteConfig` | Per-site thermal/unit flags: `soil_air_offset` (`2.5`), `amplitude` (`0.66`), `has_o_horizon` (`false`), `is_saturated` (`false`), `is_metric` (`true`). |
| `WaterBalance` | `mpe: [f64; 12]` plus annual rainfall, PET, annual water balance, and summer water balance (`f64`). |
| `HalfMonthlyWaterTable` | Compact `HalfMonthlyWaterTable(pub [f64; 24])` storing 24 half-monthly water-table depths (m) and implementing `Index<usize>` by `day / 15`. Replaces a 360-element `f64` array so `SoilState` remains `<= 1120` bytes on the stack with no large-array copy overhead in classic mode. |
| `SoilState` | `profile: [f64; 64]`, `iday: [u8; 360]` (`1`=Dry, `2`=Moist/Dry, `3`=Moist), `nd: [f64; 3]`, `sat_days_mcs: i32`, and `z_wt_daily: HalfMonthlyWaterTable`. |
| `DayCounters` | All 10 integer day-counter fields (`dry_days_after_summer_solstice` .. `num_consecutive_days_moist_in_some_parts_over_8c`). |
| `ThermalClassification` | `day_counters`, `temperature_regime: Option<TemperatureRegime>`, `moisture_regime: MoistureRegime`, `regime_subdivision1: RegimeSubdivision1`, `regime_subdivision2: RegimeSubdivision2`, `aquic_bio_days: i32`. |
| `SiteOutput` | Complete 21-field `Copy` output (19 classic fields + `sat_days_mcs` + `aquic_bio_days`), plus `*_java_str()` formatters matching Java blank-space rules. |
| `SoaBatchInput<'a>` | Struct-of-Arrays borrowed slice view (`lat_dd`, `lon_dd`, `temp_c: [&'a [f64]; 12]`, `precip_mm: [&'a [f64]; 12]`, `awc_mm`, `elev_m`). `validate()` asserts equal slice lengths; `site_input(idx)` materializes a stack-allocated `SiteInput` on demand. |

All categorical outputs use `#[repr(u8)]` enums with explicit discriminants matching `jNSMR` `terra::set.cats()` factor tables (`0`-indexed), `from_u8()`, `as_java_str()`, and `as_r_label()`:
- `TemperatureRegime` (`0..=9`): `Pergelic` .. `Isohyperthermic` (`Option<TemperatureRegime>` maps `None` to Java `""` / R `NA`).
- `MoistureRegime` (`0..=5`): `Aridic`, `Ustic`, `Xeric`, `Udic`, `Perudic`, `Undefined`.
- `RegimeSubdivision1` (`0..=8`, holds Java modifier `q`): `Typic` .. `Undefined` (`" "` in Java vs `"Undefined"` in R).
- `RegimeSubdivision2` (`0..=7`, holds Java group `div`): `Aridic` .. `Undefined` (`" "` in Java when `MoistureRegime::Perudic`).

---

## 3. Model Families

### 3.1 `ClassicNewhall` (Verbatim `jNSMR` Parity)

Type alias in `crates/newhall-core/src/engine.rs`:
```rust
pub type ClassicNewhall = SimEngine<ClassicThermal, ThornthwaiteForcing, ClassicHydrology>;
```

- `ThornthwaiteForcing` (`crates/newhall-core/src/forcing.rs`): reproduces `BASICSimulationModel` Thornthwaite heat index, exponent, `[26.5, 38.0]` step lookup (`ZPE`/`ZT`), Northern/Southern daylength tables (`INZ`/`RN`, `FS`/`RS`), and Southern Hemisphere `* 60.0` precedence and signed-latitude behavior.
- `ClassicHydrology` (`crates/newhall-core/src/hydrology.rs`): runs up to 10 annual equilibration cycles (`|tmoi - prmo| < prmo / 100.0` early exit) over the 64-slot 8×8 soil moisture matrix (`DP`, `DR`) with half-monthly infiltration/evaporation + mid-month storm recharge, then records the 360-day calendar (`iday`) and `nd` using `inspect_mcs` on `sl[8]`, `sl[16]`, `sl[24]`.
- `ClassicThermal` (`crates/newhall-core/src/thermal.rs`): reproduces `v1.6.5` STR/SMR classification, `5.0 -> 5.01` / `8.0 -> 8.01` threshold perturbations, `has_run_already` first-interval `nsd` accumulation, cold-site `ncpm2 = ncpm1` fallback, and 120-day solstice windows.

### 3.2 `StrictNewhall` (Remediated Defects `#1`–`#10`)

Type alias in `crates/newhall-core/src/engine.rs`:
```rust
pub type StrictNewhall = SimEngine<StrictThermal, StrictThornthwaiteForcing, StrictHydrology>;
```

Opt-in model family remediating the audited `jNSMR` defects documented in `planning/AUDIT_JNSM.md` and verified in `crates/newhall-core/tests/strict_mode.rs`:
- **Defects `#1` & `#7` (`StrictThornthwaiteForcing`):** uses `|lat_dd|` (clamped to `50°`) and parenthesized `(RS[nrow] - RS[nrow - 1]) * 60.0` in Southern Hemisphere daylength interpolation.
- **Defect `#2` (`StrictThornthwaiteForcing`, `StrictHydrology`, `StrictThermal`):** when `!config.is_metric`, converts `temp_c` (°F→°C), `precip_mm` (in→mm), `awc_mm` (in→mm), and `soil_air_offset` (`* 5.0 / 9.0`).
- **Defect `#3` (`StrictThermal`):** `tma < 0.0` always classifies as `TemperatureRegime::Pergelic` without being overwritten by `Frigid` or `Isofrigid`.
- **Defect `#4` (`StrictThermal`):** evaluates SMR transitions using soil-attenuated `|(st - wt) * config.amplitude|` rather than unattenuated `st - wt`.
- **Defect `#5` (`StrictThermal`):** sites with any month `< 8.0°C` and zero `>8°C` intervals yield `ncpm2 = 0` instead of copying `ncpm1`.
- **Defect `#6` (`StrictThermal`):** accumulates `nsd` across all `>5°C` intervals rather than stopping after the first interval.
- **Defect `#8` (`types.rs` / `StrictThermal`):** enforces typed `RegimeSubdivision1` (`q`) and `RegimeSubdivision2` (`div`) fields.
- **Defect `#9` (`StrictThornthwaiteForcing`, `StrictThermal`):** propagates `NaN` in climate/thermal inputs to `MoistureRegime::Undefined` and `temperature_regime = None` instead of `Perudic`.
- **Defect `#10` (`StrictThermal`):** detects threshold crossings cleanly without `5.0 -> 5.01` or `8.0 -> 8.01` perturbations.

### 3.3 Hydrothermal Physics (`Thermal1DModel` & `WaterTableHydrology`)

Hydrothermal traits implement physically based vertical heat conduction and fluctuating water-table coupling (`planning/PHASE4_PHYSICS_SPEC.md`) and compose cleanly with either `ThornthwaiteForcing` or `StrictThornthwaiteForcing`:

- `Thermal1DModel` (`crates/newhall-core/src/thermal_1d.rs`):
  - Configured by `Thermal1DConfig` (`porosity`, `bulk_density_kg_m3`, `quartz_fraction`, `organic_fraction`, `o_horizon_thickness_m`, `o_horizon_conductivity`, `snow_depth_m`, `snow_conductivity`, `convective_hc`, `texture: SoilTextureClass`, `fixed_theta_tot`, `max_spinup_years`).
  - Solves 1D heat conduction with smoothed latent heat of fusion ($\Delta T_f = 0.5\text{ K}$) and Côté & Konrad (2005) thermal conductivity on a `THERMAL_NODES` (`16`) geometrically stretched grid down to `Z_MAX_M` (`5.0` m) via `stretched_grid()`.
  - Models snowpack and surface O-horizon series thermal resistance (`surface_resistance`), advances with unconditionally stable implicit Backward Euler + `O(N)` tridiagonal `thomas_solve` (`step_backward_euler`) on stack-allocated `SoilThermalState` (`[f64; 16]`), interpolates daily $T(0.50\text{ m})$ (`interp_at_depth`) for MAST/MSST/MWST and `>5°C`/`>8°C` counters, and tracks `aquic_bio_days` (consecutive days with `soil_state.z_wt_daily[day] <= 0.50` and $T(0.50\text{ m}) > 5.0^\circ\text{C}$).
- `WaterTableHydrology` (`crates/newhall-core/src/water_table.rs`):
  - Configured by `WaterTableConfig` (`mode: WaterTableMode`, `z_prof_m`, `k_sat_mm_per_day`, `texture: SoilTextureClass`, `enable_capillary`).
  - Supports three `WaterTableMode` variants: `PrescribedMonthly([f64; 12])`, `PrescribedHarmonic { mean_depth_m, amplitude_m, peak_day }`, and `Prognostic { z_imp_m, sy, k_lat_per_day }` (backed by `WaterTableState` and `StepMassBalance` diagnostics in `step_prognostic`).
  - Maps the 8 rows of the 8×8 soil matrix onto `[0, z_prof_m]`, applies saturation floors below `z_wt` (`apply_water_table`), computes upward capillary flux using fast integer/sqrt powers matched on `SoilTextureClass` in `gardner_q_cap_max` (replacing `powf` from Task `6.1.1`), records `sat_days_mcs`, and populates `SoilState::z_wt_daily` (`HalfMonthlyWaterTable`).

---

## 4. R Package (`r-pkg`) Architecture

### 4.1 `extendr-api` `0.8` Direct-Allocation Bridge (`r-pkg/src/rust/src/lib.rs`)

`r-pkg/src/rust/src/lib.rs` exports two `#[extendr]` entrypoints registered via `extendr_module! { mod newhallrs; ... }`:
1. `newhall_batch_rust`: constructs a zero-copy `SoaBatchInput` view over R's column-major numeric vectors (`temp_flat`, `precip_flat` of length `12 * n`) and dispatches to `ClassicNewhall` or `StrictNewhall` (using `run_batch_soa` when soil configs are scalar, or `rayon` `run_per_site_soa` when per-site vectors are supplied).
2. `newhall_batch_hydrothermal_rust`: runs `rayon` parallel per-site simulations via `run_hydrothermal_with_forcing`, statically monomorphizing the 4 combinations of `(use_thermal_1d, use_water_table)` across `ThornthwaiteForcing` and `StrictThornthwaiteForcing`.

To eliminate intermediate Rust `Vec` heap allocations on the R boundary, `outputs_to_r_list(outputs: &[SiteOutput], include_hydrothermal: bool) -> List` allocates `extendr_api::Doubles::new(n)` and `extendr_api::Integers::new(n)` directly in R memory, acquires `&mut [f64]` / `&mut [i32]` slices via `real_slice_mut` / `int_slice_mut`, writes all 19 (or 21) output columns in a single pass (encoding `None` temperature regimes with `Rint::na().inner()`), and returns a named `List`.

### 4.2 Standalone Tarball Staging (`r-pkg/tools/sync-core.sh`)

R package source tarballs (`R CMD build` / `R CMD check`) cannot depend on workspace-relative paths outside `r-pkg/`. `r-pkg/src/rust/Cargo.toml` therefore declares `[workspace]` (making it an independent Rust crate) and depends on `newhall-core = { path = "newhall-core" }`. Running `bash r-pkg/tools/sync-core.sh` refreshes `r-pkg/src/rust/newhall-core/` by copying `crates/newhall-core/Cargo.toml` and `crates/newhall-core/src/` while stripping `tests/` and `target/`.

### 4.3 R S3 Dispatch & Categorical Encoding (`r-pkg/R/`)

- **`newhall_batch` (`r-pkg/R/newhall.R`, `r-pkg/R/raster.R`):**
  - `newhall_batch_raw` & `newhall_batch.default`: direct vector/matrix interface returning integer `0`-indexed regime codes from `newhall_batch_rust`.
  - `newhall_batch.data.frame`: validates required `jNSMR` columns via `.extract_df_inputs`, normalizes `unitSystem` via `.normalize_unit_system`, calls `newhall_batch_raw`, and decodes regime codes to strings via `.decode_regimes` (`r-pkg/R/categories.R`).
  - `newhall_batch.character`: reads one or more CSV paths with `utils::read.csv` and forwards to `newhall_batch.data.frame`.
  - `newhall_batch.SpatRaster`: streams chunked raster blocks via `.run_spatraster_chunks` (`terra::readValues` / `terra::writeValues`), skips `NA` cells (`is.na(awc)` in classic mode; any required `NA` when `strict = TRUE`), and attaches `terra::set.cats()` tables to layers 16–19 via `.attach_regime_cats`.
- **`newhall_batch_hydrothermal` (`r-pkg/R/newhall.R`, `r-pkg/R/raster.R`):**
  - Dispatches `default` (`newhall_batch_hydrothermal_raw`), `data.frame` (supporting optional `oHorizonThickness`, `bulkDensity`, `quartzFraction`, `porosity`, `zWt`, and `snowJan`..`snowDec` columns), and `SpatRaster` (21-layer output raster including `satDaysMcs` and `aquicBioDays`).

---

## 5. Developer Guide: Implementing, Testing, and Registering a New Physics Model

Follow these steps to add a new forcing, hydrology, or thermal model without disturbing `ClassicNewhall` parity or introducing R dependencies into `newhall-core`.

### Step 1: Implement the Physics Trait in `crates/newhall-core/src/`

1. Create a new module (e.g., `crates/newhall-core/src/<model_name>.rs`) or extend an existing physics module, and declare `pub mod <model_name>;` in `crates/newhall-core/src/lib.rs`.
2. Define a `#[derive(Debug, Clone, Copy, Default)]` config struct (if needed) and model struct implementing one of `ForcingModel`, `HydrologyStep`, or `ThermalModel` from `crates/newhall-core/src/traits.rs`:
   - Keep all per-site state stack-allocated and `Copy` (no `Vec`, `Box`, or heap allocations inside `compute_water_balance`, `simulate_hydrology`, or `evaluate`).
   - Ensure the struct is `Sync` (automatic for `Copy` value types without interior mutability) so `SimEngine::run_batch_soa` works with `rayon`.
   - Do **not** modify `ThornthwaiteForcing`, `ClassicHydrology`, or `ClassicThermal`, and do **not** add `extendr-api` or R crates to `crates/newhall-core/Cargo.toml`.
   - If your model adds new diagnostic fields, evaluate whether existing `SiteOutput` / `SoilState` / `ThermalClassification` fields suffice before altering shared `Copy` struct layouts (which affect stack size across all models).

### Step 2: Write Unit, Adversarial, and Parity Tests

1. **Module unit tests (`#[cfg(test)]`):** test edge cases (`f64::NAN`, `awc_mm == 0.0`, `is_metric = false`, extreme latitudes) inside your module.
2. **Adversarial & property tests (`crates/newhall-core/tests/adversarial_hydrothermal.rs` or a new integration test):**
   - Verify physical conservation laws (e.g., mass or energy balance closure), analytical limits, and monotonicity under parameter sweeps.
   - Add `proptest!` cases verifying finite outputs, `num_cumulative_days_dry + num_cumulative_days_moist_dry + num_cumulative_days_moist == 360`, and deterministic `run` vs `run_batch_soa` equivalence.
3. **Verify zero regression on `ClassicNewhall` and `StrictNewhall`:**
   ```bash
   cargo test --workspace --all-targets
   cargo clippy --workspace --all-targets -- -D warnings
   cargo fmt --all -- --check
   ```
   Confirm `crates/newhall-core/tests/golden_master.rs` and `crates/newhall-core/tests/strict_mode.rs` remain 100% green without relaxing assertions.

### Step 3: Register and Expose the Model

1. **In `crates/newhall-core`:**
   - Re-export the new model and config types from `crates/newhall-core/src/lib.rs`.
   - Optionally define a `pub type <PresetName> = SimEngine<..., ..., ...>;` alias in `crates/newhall-core/src/engine.rs` (keeping `ClassicNewhall` as the default used by `simulate_site` and `run_batch_soa`).
2. **In `r-pkg` (if exposing to R):**
   - Sync the updated core into the standalone R crate:
     ```bash
     bash r-pkg/tools/sync-core.sh
     ```
   - Add or extend an `#[extendr]` wrapper in `r-pkg/src/rust/src/lib.rs` using `SoaBatchInput` / `outputs_to_r_list`, and register it in `extendr_module!`.
   - Regenerate wrappers and expose the corresponding S3 methods in `r-pkg/R/newhall.R` / `r-pkg/R/raster.R` (reusing `.decode_regimes`, `.extract_df_inputs`, and `.run_spatraster_chunks`), then verify:
     ```bash
     cargo clippy --manifest-path r-pkg/src/rust/Cargo.toml -- -D warnings
     Rscript -e "devtools::test('r-pkg')"
     ```
