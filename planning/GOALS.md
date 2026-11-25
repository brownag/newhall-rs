# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-24
Source State: STATE_5.md (G8 R CMD check hygiene, live jNSMR parity, and raster benchmark completed)

## Circled Goal (Active)

- **[G16]** `newhall-core` uses fast integer/sqrt powers in Gardner capillary rise and a lean `SoilState` without unconditional `[f64; 360]` copies, and `r-pkg` allocates R output vectors directly without intermediate `Vec` copies, while keeping all Rust and R tests green and improving measured throughput.
  - Why now: Locks in the final `SoilState` layout, Gardner fast-math, and `r-pkg` allocation pattern before `G5` benchmarks multi-core `newhall-cli` throughput and before `G10` documents the final architecture in `ARCHITECTURE.md`.
  - Exit criteria: All Rust tests (`68+`), `clippy`/`fmt`, and R `testthat` (`217+`) pass with 0 failures; `newhall-cli bench` and `bench_jnsmr.R` confirm improved or parity-maintained throughput.
  - First action: Inspect `crates/newhall-core/src/water_table.rs`, `crates/newhall-core/src/types.rs`, and `r-pkg/src/rust/src/lib.rs` to draft `planning/EXECUTION_PLAN_6.md` covering `G16` + `G5` + `G10`.

## Completed Goals

- **[G8]** `r-pkg` standalone tarball staging (`sync-core.sh`, `.Rbuildignore`), CRAN MIT `LICENSE`/`DESCRIPTION` hygiene, live `jNSMR` tabular & `SpatRaster` parity suite (`85` PASS, `217` total R tests), and reproducible raster benchmark (`~395x` speedup over `jNSMR`) completed (`STATE_5.md`, `9618e7a`).

- **[G1]** Local reference repositories (`~/workspace/jNSMR` and `~/workspace/jNSMR/java`) audited (`planning/AUDIT_JNSM.md`).
- **[G2]** Deterministic golden-master test fixtures generated from `jNSMR` (`crates/newhall-core/tests/fixtures/`).
- **[G3]** Cargo workspace (`crates/newhall-core`, `crates/newhall-cli`) compiles cleanly with static-dispatch traits (`ThermalModel`, `ForcingModel`, `HydrologyStep`), `Copy` per-site state, `[f64; 64]` moisture profile, and integer enum discriminants.
- **[G4]** `newhall-core` implements the baseline/custom Java NSM with zero inner-loop heap allocations; `cargo test` passes within `1e-9` on floats and 100% exact match on all categorical outputs across golden-master fixtures.
- **[G9]** Phase 4 `Thermal1DModel` and `WaterTableHydrology` implemented with adversarial validation (`adversarial_phase4.rs`, 10k `proptest`) and `r-pkg` `extendr` bindings (`newhall_batch`, `newhall_batch_phase4`).
- **[G11]** Pure-Rust Phase 4 micro-benchmark suite (`newhall-cli bench`) measures per-stage throughput (`ClassicNewhall`, `WaterTableHydrology`, `Thermal1DModel`, `Full Phase 4`).
- **[G12]** Adversarial & parity regression gate (`adversarial_phase4.rs` + `golden_master.rs` + `strict_mode.rs`) guards all optimizations and physics changes.
- **[G13]** Linear fast-path in `step_backward_euler` outside the freeze–thaw window (`|T - T_f| > 6 K`).
- **[G14]** Precomputed per-site thermal material constants (`PrecomputedMaterial`) eliminate `powf()` calls inside the daily 16-node Newton–Picard loop.
- **[G15]** Coarse-to-fine thermal spin-up equilibrates deep nodes in 36 monthly steps before 1–2 daily refinement years.
- **[G17]** Phase 4 batch throughput reaches `10,161 sims/sec` (`> 10,000 sims/sec` target, `9.0x` speedup over `1,125 sims/sec` baseline).
- **[G18]** Opt-in `StrictThornthwaiteForcing`, `StrictThermal`, `StrictHydrology`, and `StrictNewhall` remediate the applicable audited `jNSMR` defects (`#1`–`#10`) while preserving `ClassicNewhall` parity, with `strict`/`strict_forcing` exposed in `r-pkg` (`STATE_3.md`).
- **[G7]** `r-pkg/R/` mirrors `jNSMR`'s user-facing S3 API (`default`, `data.frame`, `character`, chunked `terra::SpatRaster` with `terra::set.cats()`) and per-site soil config FFI (`STATE_4.md`).

## Sequenced Goals (Dependency DAG)

- **[G5]** `newhall-cli` runs CSV/batch simulations and golden-master verification and benchmarks multi-core throughput against the 100,000+ simulations/sec target.
  - Depends on: G16
- **[G10]** `ARCHITECTURE.md` documents how to implement, test, and register new physics models in `newhall-core`.
  - Depends on: G16

## Independent Backlog

- None.

## Deferred

- None.

## Blocked / Pruned

- None.

## Source Defects

**Policy:** Replicate all source defects **verbatim** in the initial `newhall-core` / `r-pkg` parity implementation (`G3`–`G8`) to demonstrate exact golden-master parity against `jNSMR` (`newhall-1.6.5.jar`). Once parity is verified, systematically remediate each defect (e.g., behind explicit compatibility/strict-mode flags or in a clean post-parity release). See `planning/AUDIT_JNSM.md` for full details.

1. **Southern Hemisphere PET `* 60.0D` precedence bug** (`BASICSimulationModel.java:167`): Missing parentheses around `(rs[nrow] - rs[nrow - 1]) * 60.0D` for `5.0 <= latDD < 50.0` off exact `rs[]` steps multiplies the interpolation delta by `3600`, producing erroneous/negative `mpe`.
2. **Broken `unitSystem = "english"` / `isMetric = false` conversion** (`NewhallBatch.R:437-440`, `BASICSimulationModel.java:42-3162`): `batch3()` defines `.doUnitsTemp`/`.doUnitsLength` but never calls them, and `runSimulation(double[], ...)` ignores `isMetric` for `temperature`, `precip`, `waterHoldingCapacity`, and `soilAirOffset` (scaling only unused `elevation` by `0.3048`).
3. **`Pergelic` overwritten by `Frigid` / `Isofrigid`** (`BASICSimulationModel.java:254-281`): `cr[11]` checks `tma < 8` without `tma >= 0`, so any `tma < 0` site with `st < 0` or `st >= cryic_ht` has `reg[1]` (`Pergelic`) overwritten by `Frigid` or `Isofrigid`.
4. **Unattenuated `dif = st - wt` in SMR tree** (`BASICSimulationModel.java:275, 3025, 3046, 3060`): Commenting out `st -= cs; wt += cs;` left `dif = st - wt` using raw air-temperature range without `fcd` attenuation or `Math.abs()` in Xeric/Udic/Ustic branches.
5. **Cold-site `ncpm[2]` fallback bug** (`BASICSimulationModel.java:1736, 2023`): When all 12 months are `< 8°C` (`tempUnder8C && np8 == 0`), `sn == 0` triggers `ncpm[2] = ncpm[1]`, assigning full-year moist-in-some-parts days instead of `0`.
6. **Multi-interval `>5°C` `hasRunAlready` guard** (`BASICSimulationModel.java:58, 1559-1564`): For bimodal `>5°C` curves (`np >= 2`), `nsd[1..3]` counts days from the first interval only while `lt5c` sums all intervals.
7. **Negative `latDD` + hemisphere handling** (`NewhallBatch.R:447-448`, `BASICSimulationModel.java:155`): `batch3()` passes raw negative `latDD` with `northsouth = 'S'`, hitting the `0 <= latDD < 5` SH interpolation branch with negative `latDD` (while `northsouth = 'N'` + negative `latDD` throws `ArrayIndexOutOfBoundsException`).
8. **`regimeSubdivision1` / `regimeSubdivision2` swapped order** (`BASICSimulationModel.java:3161` vs `NewhallResults.java:35`): Java passes `(q, div)` into `(div, q)`, placing modifier `q` in `regimeSubdivision1` and group `div` in `regimeSubdivision2` (matched by R `.smrsub1()`/`.smrsub2()`).
9. **`NaN` / `NA` propagation & `Perudic` trap** (`BASICSimulationModel.java:3003`, `NewhallBatch.R`): `NaN` in `precip` makes `mpe[i] > precip[i]` false, yielding `moistureRegime = "Perudic"`, and `NaN` in `temperature` yields `""` (`NA` after `.setCats()`). In `SpatRaster`, only `is.na(awc)` skips cells.
10. **Exact-`5.0` / `8.0` perturbation & multicore R column-swap bug** (`BASICSimulationModel.java:1380,1613`, `NewhallBatch.R:789-799`): Consecutive `5.0`/`8.0` temperatures are perturbed to `5.01`/`8.01` (replicate without mutating caller inputs); `cores > 1` empty-block fallback in R reverses `annualPotentialEvapotranspiration` and `summerWaterBalance` columns.
11. **`runBatch()` / `NewhallDataset` legacy divergence** (`BASICSimulationModel.java`): `runBatch2()` (`v1.6.5`) diverges from `NewhallDataset` (`v1.6.1`/`v1.6.3`) on `cs` attenuation and `cr[7]` (`>= 6`); parity targets `runBatch2()` (`v1.6.5`).
