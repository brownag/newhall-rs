# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-22
Source State: INITIAL_PROMPT.md

## Circled Goal (Active)

- **[G3]** Cargo workspace (`crates/newhall-core`, `crates/newhall-cli`, `r-pkg/`) compiles cleanly with static-dispatch traits (`ThermalModel`, `ForcingModel`, `HydrologyStep`), `Copy` per-site state, `[f64; 64]` moisture profile, and integer enum discriminants.
  - Depends on: G1

## Completed Goals

- **[G1]** Local reference repositories (`~/workspace/jNSMR` and `~/workspace/jNSMR/java`) audited (`planning/AUDIT_JNSM.md`).
- **[G2]** Deterministic golden-master test fixtures generated from `jNSMR` (`crates/newhall-core/tests/fixtures/`).

## Sequenced Goals (Dependency DAG)

- **[G4]** `newhall-core` implements the baseline/custom Java NSM with zero inner-loop heap allocations; `cargo test` passes within `1e-9` on floats and 100% exact match on all categorical outputs across golden-master fixtures.
  - Depends on: G1, G2, G3
- **[G5]** `newhall-cli` runs CSV/batch simulations and golden-master verification and benchmarks multi-core throughput against the 100,000+ simulations/sec target.
  - Depends on: G2, G3, G4
- **[G6]** `r-pkg/src/rust/` exposes a vectorized `newhall_batch()` via `extendr` using SoA `&[f64]` slices and `rayon`, handling `NA`/`NaN` safely and supporting CRAN vendoring (`rextendr::vendor_pkgs()`).
  - Depends on: G3, G4
- **[G7]** `r-pkg/R/` mirrors `jNSMR`'s user-facing API for `data.frame` and `terra::SpatRaster` inputs using chunked `readStart()`/`readValues()`/`writeValues()` and `terra::set.cats()`.
  - Depends on: G1, G6
- **[G8]** `R CMD check` passes cleanly, `testthat` verifies parity with `jNSMR`, and a benchmark reports speedup over `jNSMR` on the sample raster dataset.
  - Depends on: G2, G4, G6, G7
- **[G9]** Stub/initial implementations and unit tests exist for extended `ThermalModel`, `ForcingModel`, and `HydrologyStep` traits without breaking the baseline API.
  - Depends on: G3
- **[G10]** `ARCHITECTURE.md` documents how to implement, test, and register new physics models in `newhall-core`.
  - Depends on: G3, G9

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
