# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-22
Source State: INITIAL_PROMPT.md

## Circled Goal (Active)

- **[G1]** Local reference repositories (`~/workspace/jNSMR` and `~/workspace/jNSMR/java`) are audited, with every custom modification relative to standard jNSM, rounding/edge-case behavior, and R API contract documented.
  - Why now: Root bottleneck that transitively unblocks all 9 downstream goals (`G2`–`G10`) and eliminates design guesswork before any Rust or R code is written.
  - Exit criteria: Every Java source file in `~/workspace/jNSMR/java` and R wrapper/test/data file in `~/workspace/jNSMR` is accounted for in `planning/AUDIT_JNSM.md`, all custom modifications and ambiguities are explicitly documented, and exact golden-master sample datasets and Java entry points are identified.
  - First action: Inventory `~/workspace/jNSMR` and `~/workspace/jNSMR/java` file structure, sizes, and test suites (via `plan-wave` Phase 2 recon).

## Sequenced Goals (Dependency DAG)

- **[G2]** Deterministic golden-master test fixtures (inputs + exact Java outputs) are generated from `jNSMR` sample datasets and stored in `crates/newhall-core/tests/fixtures/`.
  - Depends on: G1
- **[G3]** Cargo workspace (`crates/newhall-core`, `crates/newhall-cli`, `r-pkg/`) compiles cleanly with static-dispatch traits (`ThermalModel`, `ForcingModel`, `HydrologyStep`), `Copy` per-site state, `[f64; 64]` moisture profile, and integer enum discriminants.
  - Depends on: G1
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
