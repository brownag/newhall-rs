# `newhallrs` vs `jNSMR` `SpatRaster` Benchmark Report

## Reproduction Command

Run from the repository root:

```bash
Rscript r-pkg/inst/benchmarks/bench_jnsmr.R
```

Script: `r-pkg/inst/benchmarks/bench_jnsmr.R`

## Environment & Hardware Metadata

- **Timestamp (UTC):** `2026-11-25T01:18:11Z`
- **OS / Kernel:** Linux `7.0.0-43-generic #43~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC` (`x86_64`)
- **CPU:** AMD Ryzen 9 9950X 16-Core Processor (`16` cores / `32` hardware threads)
- **Memory:** `109 GiB` total system RAM
- **R Version:** `R version 4.6.2 (2026-10-29)` (`x86_64-pc-linux-gnu`)
- **Rust Toolchain:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`
- **Java Runtime:** OpenJDK `25.0.5.1` (`OpenJDK 64-Bit Server VM build 25.0.5.1+2-1-24.04.1-Ubuntu`)
- **Packages:**
  - `newhallrs` `0.1.0` (loaded via `devtools::load_all('r-pkg')`, compiled with `--release` and `rayon` parallel batch execution)
  - `jNSMR` `0.3.1` (Java Newhall `1.6.5` via `rJava`, `cores = 1`; note `jNSMR` also forces `cores = 1` when `ncell < core_thresh = 25000`)
  - `terra` `1.9.51`

## Methodology

- **Datasets:** Bundled sample GeoTIFFs from `jNSMR` (`system.file("extdata", ..., package = "jNSMR")`), each `113 x 168 x 28` (`18,984` total cells; `18,790` valid non-`NA` `awc` cells; `194` `NA` `awc` cells).
- **Calls Compared:**
  - `newhallrs::newhall_batch(r, file = "")`
  - `jNSMR::newhall_batch(r, verbose = FALSE, file = "")` (with `library(jNSMR)` attached so `jNSMR:::.onAttach()` initializes `options(jNSMR.NSM_VERSION)` and selects `batch3()` / `runBatch2`)
- **Protocol:** `1` warm-up run (excluded from timing, also verifying identical `dim()` and layer `names()` across both 19-layer `SpatRaster` outputs) followed by `3` timed replicates per raster/package using `proc.time()[["elapsed"]]` with `gc(FALSE)` before each call. Throughput (`cells/s`) is computed over valid non-`NA` `awc` cells (`18,790 / median_s`), and speedup is `jNSMR_median_s / newhallrs_median_s`.

## Measured Results

### Summary Table

| Raster | Total Cells | Valid Cells | `newhallrs` Median (s) | `jNSMR` Median (s) | `newhallrs` Valid Cells/s | `jNSMR` Valid Cells/s | Speedup |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `prism_issr800_sample.tif` | 18,984 | 18,790 | 0.0230 | 9.0730 | 816,957 | 2,071 | 394.5x |
| `daymet_issr800_sample.tif` | 18,984 | 18,790 | 0.0230 | 9.1160 | 816,957 | 2,061 | 396.3x |

### Per-Replicate Wall-Clock Timings (seconds)

| Raster | `newhallrs` Replicates (s) | `jNSMR` Replicates (s) |
| --- | --- | --- |
| `prism_issr800_sample.tif` | `0.0450`, `0.0230`, `0.0230` | `9.1270`, `9.0730`, `9.0550` |
| `daymet_issr800_sample.tif` | `0.0260`, `0.0230`, `0.0230` | `9.1350`, `9.1160`, `9.0460` |

### Raw Script Output

```text
=== newhallrs vs jNSMR SpatRaster Benchmark ===
Timestamp (UTC): 2026-11-25T01:18:11Z
R version:       R version 4.6.2 (2026-10-29)
Platform:        x86_64-pc-linux-gnu
newhallrs:       0.1.0 (devtools::load_all('r-pkg'))
jNSMR:           0.3.1 (Java Newhall 1.6.5, cores = 1)
terra:           1.9.51
Protocol:        1 warm-up + 3 timed replicates (file = "", median wall-clock)

Per-replicate elapsed times (seconds):
  prism_issr800_sample.tif | newhallrs: [0.0450, 0.0230, 0.0230] | jNSMR: [9.1270, 9.0730, 9.0550]
  daymet_issr800_sample.tif | newhallrs: [0.0260, 0.0230, 0.0230] | jNSMR: [9.1350, 9.1160, 9.0460]

 Raster                    TotalCells ValidCells newhallrs_s jNSMR_s newhallrs_cells_s jNSMR_cells_s Speedup
 prism_issr800_sample.tif  18984      18790      0.0230      9.0730  816957            2071          394.5x 
 daymet_issr800_sample.tif 18984      18790      0.0230      9.1160  816957            2061          396.3x 
```
