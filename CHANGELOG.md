# Changelog

All notable changes to `newhall-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-24

### Added
- **Pure-Rust Simulation Core (`newhall-core`):**
  - Static-dispatch 3-stage pipeline (`ForcingModel`, `HydrologyStep`, `ThermalModel`, `SimEngine<T, F, H>`) with stack-allocated `Copy` state types and Struct-of-Arrays (`SoaBatchInput`) batch execution.
  - `ClassicNewhall` engine achieving golden-master parity against `jNSMR` / `newhall-1.6.5.jar` (`BASICSimulationModel.runBatch2`) across `pa_batch_metric.csv`, `daymet_sample.csv`, and `prism_sample.csv`.
  - `StrictNewhall` opt-in engine remediating audited `jNSMR` Defects `#1`–`#10` (Southern Hemisphere daylength/latitude handling, non-metric unit conversion, Pergelic/Frigid precedence, soil-attenuated amplitude checks, multi-interval `>5°C` accumulation, clean threshold crossings, and `NaN` propagation).
  - Hydrothermal physics modules: `Thermal1DModel` (16-node Backward Euler + Thomas tridiagonal heat conduction with smoothed latent heat of fusion, Côté & Konrad 2005 thermal conductivity, snow/O-horizon series resistance, and `aquic_bio_days`) and `WaterTableHydrology` (prescribed monthly/harmonic and prognostic water-table modes with Gardner 1958 capillary rise and `sat_days_mcs`).
  - Optional `rayon`-backed `parallel` feature (enabled by default; sequential fallback with `--no-default-features`).
  - Integration and property test suites (`golden_master`, `strict_mode`, and `adversarial_hydrothermal` covering conservation laws, Carslaw–Jaeger analytical solutions, monotonicity, and `proptest` invariants).
- **Native CLI & Benchmark Harness (`newhall-cli`):**
  - `run` subcommand supporting CSV batch execution and `--strict` mode.
  - `verify` subcommand checking `ClassicNewhall` outputs against reference `jNSMR` CSV files.
  - `bench` subcommand reporting parallel batch throughput and single-thread per-stage latency breakdowns across Classic and hydrothermal configurations.
- **R Package (`newhallrs` under `r-pkg/`):**
  - `extendr-api` `0.8` native bridge writing `SiteOutput` fields directly into R `Doubles`/`Integers` vectors without intermediate `Vec` allocations.
  - `newhall_batch()` S3 generic with `default` (`newhall_batch_raw`), `data.frame`, `character` (CSV), and chunked `terra::SpatRaster` methods with `terra::set.cats()` categorical tables on layers 16–19.
  - `newhall_batch_hydrothermal()` S3 generic with `default` (`newhall_batch_hydrothermal_raw`), `data.frame`, and chunked `terra::SpatRaster` methods returning 21 outputs (`satDaysMcs`, `aquicBioDays`).
  - `r-pkg/tools/sync-core.sh` standalone core staging script and `r-pkg/inst/benchmarks/bench_jnsmr.R` benchmark script.
- **Documentation & Licensing:**
  - Root `README.md`, `LICENSE` (MIT), `CHANGELOG.md`, `ARCHITECTURE.md`, and specifications under `planning/`.
