# nsm-rs: Port `jNSMR` and Custom Java Newhall Simulation Model (jNSM) to Rust

## Objective
Port my R package (`jNSMR`) and customized Java Newhall Simulation Model (jNSM) to a high-throughput Rust workspace with R bindings. The primary goals are:
1. **100% numerical parity** with the existing customized Java model as a verified baseline.
2. **Maximum throughput** for large raster (`terra::SpatRaster`) and tabular batch runs (targeting 100,000+ simulations/sec on multi-core CPUs by eliminating JVM/`rJava` overhead).
3. **Modular, zero-cost extensibility** for future soil physics extensions: soil thermal characteristics, alternative atmospheric forcings/PET, and boundary conditions (e.g., fluctuating water tables, snowpack, restricted root zones).

## Local Reference Repositories
Before writing any code, inspect these local repositories to understand the exact Java implementation, custom modifications, R API, and existing test data:
- Existing R package (`jNSMR`): `~/workspace/jNSMR`
- Custom Java NSM source: `~/workspace/jNSMR/java`

---

## Target Architecture
Create a Cargo workspace separating the pure-Rust simulation engine from the R bindings:

```text
newhall-rs/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── newhall-core/           # Pure Rust: classic NSM + modular physics traits (no R dependencies)
│   └── newhall-cli/            # Lightweight CLI for CSV/batch benchmarking & golden-master testing
└── r-pkg/                      # R package (extendr-based)
    ├── DESCRIPTION
    ├── NAMESPACE
    ├── R/                      # terra::SpatRaster & data.frame wrappers
    └── src/rust/               # extendr bridge crate calling newhall-core

Use **`extendr` (`rextendr`)** for the R–Rust bridge so the package remains compatible with R-universe and CRAN vendoring (`rextendr::vendor_pkgs()`).

---

## Performance & Memory Requirements

1. **Zero heap allocations in the inner simulation loop:**
* Represent the 8×8 moisture profile matrix as a stack-allocated fixed-size array (`[f64; 64]`).
* Keep per-site simulation state in `Copy` structs so hot loops stay in L1 cache.


2. **Struct-of-Arrays (SoA) at the R boundary:**
* Pass flat numeric slices (`&[f64]`) directly from R (e.g., `precip`, `temp`, `awc`, `lat`, `elev`, etc.) rather than constructing row-oriented structs across FFI.
* Handle `NA` / `NaN` values cleanly without panicking, returning `NA` outputs for invalid cells.


3. **Parallel batch execution:**
* Use `rayon` (`into_par_iter()`) inside Rust to parallelize across sites/pixels.
* In R, process `terra::SpatRaster` inputs in memory-safe chunks using `terra::readStart()` / `terra::readValues()` / `terra::writeValues()`, passing each chunk's flat vectors to Rust.


4. **Integer enum codes for categorical outputs:**
* Return `u8`/`i32` discriminants for Soil Moisture Regime (SMR), Soil Temperature Regime (STR), and subdivisions instead of allocating strings per pixel.
* Attach factor labels in R via `terra::set.cats()`.



---

## Extensibility Design (Static-Dispatch Traits)

Design `newhall-core` so the classic/custom Java behavior is the default implementation, while new physics can be swapped in via Rust traits with **static dispatch** (generics or `enum_dispatch`—no `dyn Trait` overhead inside the pixel loop):

1. **`ThermalModel`**
* *Baseline:* Static `soilAirOffset` (+2.5°C default) + existing O-horizon/saturation cryic checks from the Java code.
* *Extension hook:* Interface supporting 1D heat conduction / damping-depth models using bulk density, volumetric water content, O-horizon thermal conductivity, and snowpack insulation.


2. **`ForcingModel` (PET & Atmospheric Forcings)**
* *Baseline:* Monthly Thornthwaite PET (temperature + latitude).
* *Extension hook:* Interface supporting Hargreaves-Samani (Tmin/Tmax), Priestley-Taylor, and Penman-Monteith when optional forcing arrays (radiation, VPD, wind speed) are provided.


3. **`HydrologyStep` / `BoundaryCondition**`
* *Baseline:* Classic freely drained 8×8 recharge/discharge profile.
* *Extension hook:* Interface supporting fluctuating water table depth (capillary rise / aquic saturation), restricted infiltration / root-zone depth, and snow accumulation/melt.



Define a generic engine struct (e.g., `SimEngine<T: ThermalModel, F: ForcingModel, H: HydrologyStep>`) so the compiler monomorphizes and inlines the full pipeline.

---

## Phased Execution Plan

Execute in phases. Complete and verify each phase before moving to the next:

### Phase 1: Audit & Golden-Master Fixtures

1. Audit the local Java NSM source and `jNSMR` R code. Document every custom modification relative to standard jNSM v1.6.4 (e.g., O-horizon logic, saturation flags, offsets, edge cases).
2. Generate or extract golden-master test fixtures (inputs + exact Java outputs: monthly water balance, PET, cumulative/consecutive dry & moist days, SMR, STR, subdivisions) from the existing package's sample datasets (e.g., Pennsylvania CSV and PRISM/DAYMET raster samples).
3. Save fixtures in `crates/newhall-core/tests/fixtures/`.

### Phase 2: `newhall-core` Parity Implementation

1. Scaffold the Cargo workspace and implement the trait architecture + data types in `newhall-core`.
2. Port the exact 8×8 recharge/discharge loop, Thornthwaite PET, thermal regime classification, and all custom Java logic.
3. Write integration tests comparing Rust outputs against all golden-master fixtures.
4. **Acceptance criteria:** `cargo test` passes with floating-point outputs within `1e-9` of Java and 100% exact match on all SMR/STR/subdivision classifications.

### Phase 3: R Package (`extendr`) & `terra` Integration

1. Scaffold `r-pkg/` with `extendr` bindings exposing a vectorized `newhall_batch()` function using SoA slices and `rayon`.
2. Implement R wrappers mirroring `jNSMR`'s user-facing API for both `data.frame` and `terra::SpatRaster` inputs (with chunked processing and `terra::set.cats()`).
3. Benchmark the Rust-backed R package against the existing `jNSMR` implementation on the sample raster dataset and report the speedup.
4. **Acceptance criteria:** `R CMD check` passes cleanly and `testthat` verifies parity with `jNSMR`.

### Phase 4: Scaffolding Physics Extensions

1. Add stub/initial implementations and unit tests for the extended traits (`ThermalModel`, `ForcingModel`, `HydrologyStep`) so new physics modules can be plugged in without breaking the legacy API.
2. Document how to add and register new physics models in `ARCHITECTURE.md`.

---

## Working Guidelines

* Start with **Phase 1** and summarize your findings from the Java/R codebase (especially custom modifications) before writing the Rust core.
* Keep commits small, logical, and green (`cargo test` passing).
* Do not silently change scientific assumptions or rounding behavior in the baseline model; if you find ambiguities or bugs in the Java code, flag them and ask before deviating from parity.
