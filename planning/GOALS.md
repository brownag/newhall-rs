# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-24
Source State: STATE_7.md (Orchestration 7 complete — CI/CD, CRAN hygiene, root docs, crates.io readiness)

## Circled Goal (Active)

- **[G29]** `r-pkg` removes `jNSMR` from `DESCRIPTION` `Suggests` and gates live `jNSMR` parity checks cleanly so `R CMD check --as-cran` has zero non-CRAN `Suggests` or unstated-dependency notes while local `jNSMR` parity remains runnable.
  - Why now: Removes the `Suggests not in mainstream repositories: jNSMR` NOTE and eliminates the `_R_CHECK_FORCE_SUGGESTS_=false` CI workaround before any downstream R package work (`G22`, `G24`, `G28`).
  - Exit criteria: `jNSMR` is absent from `r-pkg/DESCRIPTION`; `R CMD check --as-cran --no-manual` reports no `Suggests` or `unstated dependencies in 'tests'` notes; local `jNSMR` parity verification still passes when `jNSMR` is installed.
  - First action: Inspect `r-pkg/tests/testthat/test-jnsmr-parity.R` and decide between gated dynamic lookup vs. moving to `scripts/`.
- **[G30]** `r-pkg` investigates and remediates the `R CMD check --as-cran` `checking compiled code` WARNING (`R_NamespaceRegistry` non-API call and `_exit`/`abort`/`exit` symbols in `libnewhallrs.a`).
  - Why now: Sole remaining WARNING blocking a 0-ERROR / 0-WARNING `R CMD check --as-cran` gate before public release (`G28`).
  - Exit criteria: `R CMD check --as-cran --no-manual` reports 0 WARNINGs on `checking compiled code` (or documents exact upstream `extendr-api` / Rust `std` blocker with remediation path).
  - First action: Inspect `nm -u` on `libnewhallrs.a` and check `extendr-api` version/features for `R_NamespaceRegistry` and `panic`/`exit` symbols.

## Completed Goals

- **[G19]** Automated CI/CD pipeline (`.github/workflows/ci.yml`) and `r-pkg/tools/sync-core.sh --check` drift verification (`STATE_7.md`, `95c0d50`).
- **[G20]** `r-pkg` `Makevars` `.PHONY: $(STATLIB)` Rust tracking, `Makevars.win`, `DESCRIPTION`/`LICENSE` metadata, and `R CMD check --as-cran` / `testthat` verification (`STATE_7.md`, `e3cb07b`, `a95f33e`; upstream `extendr-api 0.8.2` / R 4.6.2 compiled-code warning noted).
- **[G21]** Root `README.md`, MIT `LICENSE`, and `CHANGELOG.md` covering Rust/R quickstarts, model modes, and benchmark highlights (`STATE_7.md`, `91dd2a8`).
- **[G25]** `newhall-core` crates.io metadata, `#![warn(missing_docs)]` with full rustdoc coverage, and passing `--no-default-features` + `cargo package --list` (`STATE_7.md`, `09c0f79`).

- **[G16]** Fast integer/sqrt powers in Gardner capillary rise (`water_table.rs`), lean `SoilState` via `HalfMonthlyWaterTable([f64; 24])` (`types.rs`, shrunk from 3,784 B to 1,096 B), and direct R vector allocation (`Doubles`/`Integers`) in `r-pkg` `outputs_to_r_list()` (`STATE_6.md`, `25c3c0d`, `6d9242c`, `1297b50`).
- **[G5]** `newhall-cli` implements `run` (CSV batch), `verify` (golden-master parity on PA/PRISM/Daymet), and `bench` (`4.7M–5.0M sims/sec` multi-core `ClassicNewhall`, >47× above the 100k target) (`STATE_6.md`, `763dd5e`).
- **[G10]** `ARCHITECTURE.md` documents workspace layout, static-dispatch traits, `Copy` state, `r-pkg` `extendr` bridge, and step-by-step physics extension guide (`STATE_6.md`, `837cd16`).
- **[G8]** `r-pkg` standalone tarball staging (`sync-core.sh`, `.Rbuildignore`), CRAN MIT `LICENSE`/`DESCRIPTION` hygiene, live `jNSMR` tabular & `SpatRaster` parity suite (`85` PASS, `217` total R tests), and reproducible raster benchmark (`~380x–413x` speedup over `jNSMR`) completed (`STATE_5.md`, `9618e7a`).

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

- **[G22]** User-facing R vignette(s) in `r-pkg/vignettes/` demonstrate tabular, `terra::SpatRaster`, strict-mode, and Phase 4 workflows and build cleanly under `R CMD build`.
  - Depends on: G20
- **[G24]** `newhall_batch_phase4.character` (CSV/path method) completes full S3 parity with `newhall_batch` (`default`, `data.frame`, `character`, `SpatRaster`), with testthat coverage.
  - Depends on: G20
- **[G28]** Automated release workflow (`.github/workflows/release.yml`) builds and attaches `newhall-cli` binaries and the `newhallrs` R source tarball on `v*` tags with synchronized versions.
  - Depends on: G19, G20, G21, G25

## Independent Backlog

- **[G23]** `newhall-cli run` supports `--strict` and Phase 4 execution from CSV with unit tests.
- **[G26]** Phase 4 physics has deterministic analytical and soil-profile reference validation tests (harmonic damping depth, Stefan freeze-front depth, wetland/permafrost profiles).

## Deferred

- **[G27]** `newhall-core` supports optional `serde` serialization behind a `serde` Cargo feature.
  - Trigger: When JSON/Parquet/Arrow pipelines or `--format json` CLI output are requested.

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
