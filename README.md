# newhall-rs

`newhall-rs` is a pure-Rust reimplementation and modular physics extension of the Newhall Simulation Model (`jNSMR` / `newhall-1.6.5.jar`), paired with a native CLI (`newhall-cli`) and a vectorized R package (`newhallrs`) built with `extendr-api` and `rayon`.

- **Pure-Rust core (`crates/newhall-core`):** Stack-allocated `Copy` simulation state, zero-heap-allocation single-site execution, Struct-of-Arrays (`SoaBatchInput`) batch execution, optional `rayon` parallelism (`parallel` feature enabled by default), and zero R/`extendr` dependencies.
- **Native CLI (`crates/newhall-cli`):** Batch CSV runner (`run`), `jNSMR` golden-parity verifier (`verify`), and throughput benchmark harness (`bench`).
- **R package (`r-pkg` / `newhallrs`):** Vectorized S3 interfaces (`newhall_batch`, `newhall_batch_hydrothermal`) over `data.frame`, CSV paths (`character`), raw vectors/matrices, and chunked `terra::SpatRaster` objects with direct R vector allocation and attached categorical tables.

See the [Architecture & Extension Guide](ARCHITECTURE.md) for module responsibilities, trait pipeline details, and instructions for adding new physics models.

---

## Model Modes

Simulation physics compose three static-dispatch traits (`ForcingModel`, `HydrologyStep`, `ThermalModel`) in `SimEngine<T, F, H>`:

1. **`ClassicNewhall` (Default — Verbatim `jNSMR` Parity):**
   `SimEngine<ClassicThermal, ThornthwaiteForcing, ClassicHydrology>` reproduces `newhall-1.6.5.jar` (`BASICSimulationModel.runBatch2`) outputs verbatim, preserving legacy edge-case behaviors for drop-in compatibility with `jNSMR`.
2. **`StrictNewhall` (Opt-in — Defect-Remediated Classic Physics):**
   `SimEngine<StrictThermal, StrictThornthwaiteForcing, StrictHydrology>` remediates audited `jNSMR` Defects `#1`–`#10` (documented in [planning/AUDIT_JNSM.md](planning/AUDIT_JNSM.md)), including Southern Hemisphere latitude/daylength interpolation, English-to-metric unit conversion when `!is_metric`, Pergelic vs. Frigid precedence, attenuated amplitude checks, multi-interval `>5°C` day accumulation, clean `5.0`/`8.0°C` threshold crossings, and `NaN` propagation to `Undefined` / `None`.
3. **Hydrothermal Physics (`Thermal1DModel` & `WaterTableHydrology`):**
   Physically based opt-in extensions specified in [planning/PHASE4_PHYSICS_SPEC.md](planning/PHASE4_PHYSICS_SPEC.md):
   - `Thermal1DModel`: 16-node geometrically stretched 1D vertical heat conduction (`0`–`5.0 m`) solved via implicit Backward Euler + tridiagonal Thomas solver with smoothed latent heat of fusion, Côté & Konrad (2005) thermal conductivity, snowpack and O-horizon series thermal resistance, `T(0.50 m)` regime evaluation, and `aquic_bio_days`.
   - `WaterTableHydrology`: maps the 8×8 soil moisture matrix onto `[0, z_prof_m]`, applies saturation floors and Gardner (1958) capillary rise across `PrescribedMonthly`, `PrescribedHarmonic`, and `Prognostic` water-table modes, and records `sat_days_mcs`.

---

## Rust Quickstart

### Library (`newhall-core`)

Add `newhall-core` from this repository with `cargo add newhall-core --path crates/newhall-core`.

```rust
use newhall_core::{
    run_batch_soa, simulate_site, SimEngine, SiteConfig, SiteInput, SoaBatchInput,
    StrictNewhall, StrictThornthwaiteForcing, Thermal1DModel, WaterTableHydrology,
};

fn main() {
    let input = SiteInput {
        lat_dd: 40.0,
        lon_dd: -75.0,
        temp_c: [0.0, 1.0, 5.0, 11.0, 17.0, 22.0, 25.0, 24.0, 20.0, 14.0, 8.0, 2.0],
        precip_mm: [80.0; 12],
        awc_mm: 200.0,
        elev_m: 100.0,
    };
    let config = SiteConfig::default();

    // 1. Default ClassicNewhall (verbatim jNSMR parity)
    let classic = simulate_site(&input, &config);
    println!("Classic: {:?} / {:?}", classic.temperature_regime, classic.moisture_regime);

    // 2. Opt-in StrictNewhall (remediated defects #1-#10)
    let strict = StrictNewhall::default().run(&input, &config);
    println!("Strict: {:?} / {:?}", strict.temperature_regime, strict.moisture_regime);

    // 3. Opt-in hydrothermal physics (1D thermal + fluctuating water table)
    let hydrothermal_engine = SimEngine::new(
        Thermal1DModel::default(),
        StrictThornthwaiteForcing,
        WaterTableHydrology::default(),
    );
    let hydrothermal = hydrothermal_engine.run(&input, &config);
    println!("Hydrothermal aquic_bio_days={}, sat_days_mcs={}", hydrothermal.aquic_bio_days, hydrothermal.sat_days_mcs);

    // 4. Parallel Struct-of-Arrays (SoA) batch execution
    let lat = [40.0, 35.0];
    let lon = [-75.0, -105.0];
    let t = [[0.0, 2.0]; 12];
    let p = [[80.0, 30.0]; 12];
    let awc = [200.0, 150.0];
    let elev = [100.0, 1600.0];
    let batch = SoaBatchInput {
        lat_dd: &lat,
        lon_dd: &lon,
        temp_c: std::array::from_fn(|m| &t[m][..]),
        precip_mm: std::array::from_fn(|m| &p[m][..]),
        awc_mm: &awc,
        elev_m: &elev,
    };
    let outputs = run_batch_soa(&batch, &config);
    assert_eq!(outputs.len(), 2);
}
```

To build or test `newhall-core` without `rayon` (sequential execution):

```sh
cargo test -p newhall-core --no-default-features
```

### CLI (`newhall-cli`)

`newhall-cli` provides three subcommands (`crates/newhall-cli/src/main.rs`):

```sh
# Run classic simulation on a jNSMR-schema CSV (writes to stdout or --output)
cargo run --release -p newhall-cli -- run input.csv --output output.csv

# Run with defect-remediated StrictNewhall mode
cargo run --release -p newhall-cli -- run input.csv --output output_strict.csv --strict

# Verify ClassicNewhall outputs against an expected jNSMR CSV within 1e-9 tolerance
cargo run --release -p newhall-cli -- verify \
  crates/newhall-core/tests/fixtures/pa_batch_metric.csv \
  crates/newhall-core/tests/fixtures/pa_batch_metric.csv

# Run multi-stage Rust throughput benchmark (defaults to 100000 sites if N omitted)
cargo run --release -p newhall-cli -- bench 100000
```

---

## R Quickstart (`newhallrs`)

### Installation

Requires R and Cargo (`rustc >= 1.70`):

```sh
R CMD INSTALL r-pkg
```

### `data.frame` and `terra::SpatRaster` Examples

```r
library(newhallrs)

# 1. Classic jNSMR-compatible batch simulation on a data.frame
df <- data.frame(
  latDD = 40.79, lonDD = -77.86,
  tJan = -2.8, tFeb = -1.6, tMar = 3.2, tApr = 9.4, tMay = 15.1, tJun = 20.0,
  tJul = 22.4, tAug = 21.5, tSep = 17.2, tOct = 11.0, tNov = 5.2, tDec = -0.3,
  pJan = 70, pFeb = 64, pMar = 84, pApr = 86, pMay = 96, pJun = 104,
  pJul = 94, pAug = 86, pSep = 88, pOct = 79, pNov = 82, pDec = 75,
  awc = 200, elev = 350
)

res_classic <- newhall_batch(df)
res_strict  <- newhall_batch(df, strict = TRUE)

# CSV file paths are also accepted by newhall_batch()
# res_csv <- newhall_batch("crates/newhall-core/tests/fixtures/pa_batch_metric.csv")

# 2. Hydrothermal simulation (1D thermal + fluctuating water table) on a data.frame
res_hydrothermal <- newhall_batch_hydrothermal(
  df,
  oHorizonThickness = 0.05,
  bulkDensity = 1350,
  quartzFraction = 0.4,
  porosity = 0.48,
  zWt = 0.75,
  texture = "medium"
)
res_hydrothermal[, c("temperatureRegime", "moistureRegime", "satDaysMcs", "aquicBioDays")]

# 3. Chunked terra::SpatRaster processing (28 required layers; outputs 19 or 21 layers
#    with 0-indexed categorical tables attached to layers 16-19 via terra::set.cats())
if (requireNamespace("terra", quietly = TRUE) && requireNamespace("jNSMR", quietly = TRUE)) {
  r <- terra::rast(system.file("extdata", "daymet_issr800_sample.tif", package = "jNSMR"))
  out_classic <- newhall_batch(r, file = "")
  out_hydrothermal  <- newhall_batch_hydrothermal(r, z_wt = 1.0, file = "")
}
```

---

## Benchmark Highlights

Measured on an AMD Ryzen 9 9950X (`16` cores / `32` threads) and documented in [planning/benchmarks/jnsmr_raster_benchmark.md](planning/benchmarks/jnsmr_raster_benchmark.md):

### `terra::SpatRaster`: `newhallrs` vs `jNSMR` (`r-pkg/inst/benchmarks/bench_jnsmr.R`)

Sample GeoTIFFs (`113 x 168 x 28`, `18,984` total cells / `18,790` valid cells; `jNSMR` `0.3.1` / Java `1.6.5` vs `newhallrs` `0.1.0`):

| Raster | `newhallrs` Median (s) | `jNSMR` Median (s) | `newhallrs` Valid Cells/s | Speedup |
| --- | ---: | ---: | ---: | ---: |
| `prism_issr800_sample.tif` | `0.0240` | `9.1120` | `782,917` | `379.7x` |
| `daymet_issr800_sample.tif` | `0.0220` | `9.0940` | `854,091` | `413.4x` |

### Rust Core Throughput (`cargo run --release -p newhall-cli -- bench 100000`)

| Configuration | Throughput (sims/sec) | Time / Site | Total (100k sites) |
| --- | ---: | ---: | ---: |
| `ClassicNewhall` (`rayon`) | `4,705,436` | `0.21 µs` | `0.0213 s` |
| `WaterTable + ClassicThermal` | `3,075,353` | `0.33 µs` | `0.0325 s` |
| `Thermal1D + ClassicHydro` | `10,146` | `98.56 µs` | `9.8564 s` |
| Full Hydrothermal (`rayon`) | `10,365` | `96.48 µs` | `9.6480 s` |

---

## Documentation & Licensing

- [Architecture & Extension Guide](ARCHITECTURE.md)
- [jNSMR Parity Audit](planning/AUDIT_JNSM.md)
- [Hydrothermal Physics Specification](planning/PHASE4_PHYSICS_SPEC.md)
- [Adversarial Validation Specification](planning/ADVERSARIAL_VALIDATION.md)
- [Benchmark Report](planning/benchmarks/jnsmr_raster_benchmark.md)
- [Changelog](CHANGELOG.md)
- [MIT License](LICENSE)
- [Notice & Acknowledgements](NOTICE.md)

**Acknowledgements:** newhall-rs builds on Franklin Newhall's soil moisture regime model, the Van Wambeke BASIC implementation, and the Java Newhall Simulation Model ([jNSM](https://github.com/drww/newhall)) by Drew Waltman, as extended in [jNSMR](https://github.com/ncss-tech/jNSMR). No jNSM or jNSMR source code is included; see [NOTICE.md](NOTICE.md) for details.
