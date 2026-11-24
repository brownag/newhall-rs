# Execution Plan 4: R User-Facing S3 API (`data.frame`, `character`, `terra::SpatRaster`) (`G7`)

## Goal & Scope

### Goal
Mirror `jNSMR`'s user-facing R API in `r-pkg/R/` by turning `newhall_batch()` and `newhall_batch_phase4()` into S3 generics supporting `default` (vector/matrix), `data.frame`, `character` (CSV paths), and chunked `terra::SpatRaster` inputs (`readStart()`/`readValues()`/`writeValues()` + `terra::set.cats()`), with per-row/per-cell soil parameter support (`soilAirOffset`, `amplitude`, `hasOHorizon`, `isSaturated`) and full `jNSMR`-parity default mode alongside opt-in `strict` and Phase 4 modes.

### In Scope
- **Wave 1 (Rust Per-Site Config FFI & R Categorical/Schema Helpers):**
  - `r-pkg/src/rust/src/lib.rs` & `r-pkg/R/extendr-wrappers.R`: Extend `newhall_batch_rust` and `newhall_batch_phase4_rust` to accept length-`1` or length-`n` vectors for `soil_air_offset`, `amplitude`, `has_o_horizon`, and `is_saturated` (with `is_metric` preserved).
  - `r-pkg/R/categories.R` & `r-pkg/DESCRIPTION`: Add `terra` to `Suggests`, define `jNSMR`-exact categorical factor level tables (`.str_levels()`, `.smr_levels()`, `.smrsub1_levels()`, `.smrsub2_levels()`), `.decode_regimes()`, `.regime_cat_df()`, `unitSystem` normalization, and `.extract_batch_inputs()` validation/extraction helpers.
- **Wave 2 (S3 Generics & Chunked `SpatRaster` Driver):**
  - `r-pkg/R/newhall.R`: Refactor `newhall_batch()` and `newhall_batch_phase4()` into S3 generics (`default`, `data.frame`, `character` for `newhall_batch`; `default`, `data.frame` for `newhall_batch_phase4`) plus `newhall_batch_raw()` / `newhall_batch_phase4_raw()` preserving low-level matrix calls.
  - `r-pkg/R/raster.R`: Implement `newhall_batch.SpatRaster()` and `newhall_batch_phase4.SpatRaster()` using chunked `terra::readStart()`/`writeStart()`/`readValues()`/`writeValues()`/`writeStop()`/`readStop()`, `NA` cell masking/re-insertion, and `terra::set.cats()`.
- **Wave 3 (Docs, Exports & `testthat` Verification):**
  - `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd`, `r-pkg/tests/testthat/test-s3-api.R` (and `test-phase4.R` if needed): Regenerate exports/man pages and add comprehensive `testthat` tests covering `data.frame`, `character` CSV, multi-block `SpatRaster` with `NA`s and `terra::cats()`, per-row/per-cell soil parameters, `strict = TRUE`, and Phase 4 `SpatRaster`.

### Out of Scope (Explicitly Deferred)
- `R CMD check` release polish, `jNSMR` sample-raster parity tests against `/home/andrew/workspace/jNSMR`, and raster speedup benchmarking (`G8`).
- `newhall-cli` batch CSV/benchmark commands (`G5`), secondary Rust/R perf optimizations (`G16`), and `ARCHITECTURE.md` (`G10`).
- Legacy `RasterBrick` / `RasterStack` (`raster` package) S3 methods or `jNSMR` XML/GUI/data-download helpers.
- Modifying `crates/newhall-core/` physics or tests.

### Verbatim Constraints
- "All Phase 1–3 golden-master parity tests (`jNSMR` / Java baseline) MUST remain 100% green and unchanged. Legacy behavior is the default; Phase 4 physics are opt-in trait implementations."
- "Zero heap allocations in the per-site hot loop... All new state structs MUST be `Copy` and stack-allocated."
- "Keep `newhall-core` free of R/`extendr` dependencies."

---

## Decision Log

- **2026-11-23 — S3 API Shape (`Option A`):** Make `newhall_batch(.data, ...)` an S3 generic matching `jNSMR` (`data.frame`, `SpatRaster`, `character`), and preserve the low-level vectorized matrix interface via `newhall_batch_raw()` and `newhall_batch.default()`.
- **2026-11-24 — Per-Site Soil Parameters (`Option A`):** Extend `newhall_batch_rust` (and `newhall_batch_phase4_rust`) to accept length-`1` or length-`n` vectors for `soil_air_offset`, `amplitude`, `has_o_horizon`, and `is_saturated` so optional `data.frame` columns and `SpatRaster` layers match `jNSMR` `v1.6.5`.
- **2026-11-24 — `terra` Dependency & Phase 4 S3 (`Option A`):** Declare `terra` in `Suggests` (guarded with `requireNamespace("terra", quietly = TRUE)` in `SpatRaster` methods), support `strict = FALSE` on `newhall_batch()` S3 methods, and make `newhall_batch_phase4()` an S3 generic (`default`, `data.frame`, `SpatRaster`).
- **2026-11-24 — Execution & Acceptance (`Option A`):** Use `flash` workers, max **3 concurrent workers** per wave with worktree isolation, escalating to `pro`/frontier after 2 failed attempts; track `planning/*_4.md` and `planning/tasks/4.*.todo.md` in Git via `docs(planning):` commits; require `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `Rscript -e "devtools::test('r-pkg')"` green.

---

## Recon & Hazard Notes

- **Baseline Green Suites:** `43` unit + `13` adversarial + `4` golden-master + `8` strict_mode Rust tests (`cargo test --workspace --all-targets`), `clippy` & `fmt` clean, and `27` R `testthat` assertions (`devtools::test('r-pkg')`). `terra` `1.9.51` is installed in the local R environment.
- **File Sizes & Anchors:**
  - `r-pkg/src/rust/src/lib.rs` (320 lines): `newhall_batch_rust` (L121), `newhall_batch_phase4_rust` (L198).
  - `r-pkg/R/newhall.R` (138 lines): `newhall_batch` (L18), `newhall_batch_phase4` (L80).
  - `r-pkg/R/extendr-wrappers.R` (33 lines).
  - `r-pkg/DESCRIPTION` (17 lines), `r-pkg/NAMESPACE` (5 lines).
  - `planning/audit/r_api_audit.md`: exact 28 required columns/layers, 4 optional columns/layers, 19 output columns/layers, and 0-indexed categorical factor tables.
- **Hazard — `extendr` Logical Slice Passing:** Passing `has_o_horizon` / `is_saturated` as `&[i32]` (converted from `as.integer(as.logical(...))` in R, where `!= 0` is `true`) avoids `extendr_api::Rbool` / `Logicals` edge cases and accepts length-`1` or length-`n` slices cleanly.
- **Hazard — Shared Library Rebuild:** When `r-pkg/src/rust/src/lib.rs` changes, `r-pkg/src/newhallrs.so` and `r-pkg/src/rust/target/` staticlib must be rebuilt (e.g., `rm -f r-pkg/src/newhallrs.so r-pkg/src/rust/target/release/libnewhallrs.a && Rscript -e "devtools::test('r-pkg')"`).
- **Hazard — `NA` Row Re-Insertion when `length(skip_idx) == 0`:** Do NOT copy `jNSMR`'s `1:length(skip.idx)` bug; handle `length(skip_idx) == 0` and `length(skip_idx) == nrow(block)` (all-`NA` block) explicitly with `seq_len()`.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `4.1.1` | Extend Rust FFI for per-site soil config vectors | `newhall-rs` | — | Edit: `r-pkg/src/rust/src/lib.rs`, `r-pkg/R/extendr-wrappers.R` |
| 1 | `4.1.2` | Add categorical level tables, schema helpers, and `terra` Suggests | `newhall-rs` | — | Create: `r-pkg/R/categories.R`; Edit: `r-pkg/DESCRIPTION` |
| 2 | `4.2.1` | Refactor `newhall_batch` & `newhall_batch_phase4` into S3 generics (`default`, `data.frame`, `character`) | `newhall-rs` | `4.1.1`, `4.1.2` | Edit: `r-pkg/R/newhall.R` |
| 2 | `4.2.2` | Implement chunked `SpatRaster` S3 methods with `terra::set.cats()` | `newhall-rs` | `4.1.1`, `4.1.2` | Create: `r-pkg/R/raster.R` |
| 3 | `4.3.1` | Sync `NAMESPACE`/docs and add `testthat` S3 + `SpatRaster` test suite | `newhall-rs` | `4.2.1`, `4.2.2` | Edit: `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd`, `r-pkg/tests/testthat/test-phase4.R`; Create: `r-pkg/tests/testthat/test-s3-api.R` |

---

## Task Specifications

### Task `4.1.1`: Extend Rust FFI for per-site soil config vectors
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/rust/src/lib.rs`, `r-pkg/R/extendr-wrappers.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/R/newhall.R`, `r-pkg/DESCRIPTION`.
- **Depends on:** None
- **Spec:**
  1. In `r-pkg/src/rust/src/lib.rs`, update `newhall_batch_rust` and `newhall_batch_phase4_rust` so `soil_air_offset: &[f64]`, `amplitude: &[f64]`, `has_o_horizon: &[i32]`, and `is_saturated: &[i32]` each accept length `1` (broadcast) or length `n` (per-site); return `Err(Error::Other(...))` if any has another length (when `n > 0`).
  2. When all 4 config slices have length `1` in `newhall_batch_rust`, keep calling `run_batch_soa(&batch, &cfg)` for maximum speed; when any has length `n > 1`, run `(0..n).into_par_iter().map(|i| ...)` constructing per-site `SiteInput` and `SiteConfig` with `StrictNewhall::default().run(...)` or `ClassicNewhall::default().run(...)`.
  3. In `newhall_batch_phase4_rust`, index each config slice by `if slice.len() == 1 { 0 } else { i }` inside the existing `rayon` parallel loop.
  4. Update `r-pkg/R/extendr-wrappers.R` to match the updated function signatures if needed (parameter names remain identical).
- **Grep anchors:** `r-pkg/src/rust/src/lib.rs` (`newhall_batch_rust` L121, `newhall_batch_phase4_rust` L198), `r-pkg/R/extendr-wrappers.R`.
- **Acceptance & targeted verification:**
  - `cargo check --manifest-path r-pkg/src/rust/Cargo.toml` and `cargo clippy --manifest-path r-pkg/src/rust/Cargo.toml -- -D warnings` exit 0.

---

### Task `4.1.2`: Add categorical level tables, schema helpers, and `terra` Suggests
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `r-pkg/R/categories.R`
  - *Must edit:* `r-pkg/DESCRIPTION`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `r-pkg/src/`, `r-pkg/R/newhall.R`.
- **Depends on:** None
- **Spec:**
  1. Add `terra` to `Suggests:` in `r-pkg/DESCRIPTION`.
  2. Create `r-pkg/R/categories.R` with internal helpers matching `planning/audit/r_api_audit.md` Section 3:
     - `.str_levels()` (`Pergelic`, `Cryic`, `Frigid`, `Mesic`, `Thermic`, `Hyperthermic`, `Isofrigid`, `Isomesic`, `Isothermic`, `Isohyperthermic`)
     - `.smr_levels()` (`Aridic`, `Ustic`, `Xeric`, `Udic`, `Perudic`, `Undefined`)
     - `.smrsub1_levels()` (`Typic`, `Weak`, `Wet`, `Dry`, `Extreme`, `Xeric`, `Udic`, `Aridic`, `Undefined`)
     - `.smrsub2_levels()` (`Aridic`, `Tempustic`, `Tropustic`, `Tempudic`, `Xeric`, `Udic`, `Tropudic`, `Undefined`)
     - `.regime_cat_df(levels)`: returns `data.frame(value = seq_along(levels) - 1L, category = levels, stringsAsFactors = FALSE)` for `terra::set.cats()`.
     - `.decode_regimes(df)`: converts integer 0-indexed `temperatureRegime`, `moistureRegime`, `regimeSubdivision1`, `regimeSubdivision2` columns in a result `data.frame` into character strings (preserving `NA_character_` for `NA` codes).
     - `.normalize_unit_system(unitSystem)`: maps `"metric"`, `"mm"`, `"cm"` (case-insensitive) to `is_metric = TRUE` and `"in"`, `"english"` to `is_metric = FALSE` (erroring on unknown values).
     - `.newhall_required_cols()` (28 required names: `latDD`, `lonDD`, `tJan`..`tDec`, `pJan`..`pDec`, `awc`, `elev`) and `.extract_df_inputs(.data, soilAirOffset, amplitude, hasOHorizon, isSaturated)` that validates required columns and extracts `lat`, `lon`, `temp` (`n x 12` matrix in Jan..Dec order), `precip` (`n x 12` matrix in Jan..Dec order), `awc`, `elev`, and optional per-row `soilAirOffset`, `amplitude`, `hasOHorizon`, `isSaturated`.
- **Grep anchors:** `planning/audit/r_api_audit.md` (Section 2.2, Section 3), `r-pkg/DESCRIPTION`.
- **Acceptance & targeted verification:**
  - `Rscript -e "source('r-pkg/R/categories.R'); stopifnot(length(.str_levels()) == 10L, length(.newhall_required_cols()) == 28L)"` exits 0.

---

### Task `4.2.1`: Refactor `newhall_batch` & `newhall_batch_phase4` into S3 generics (`default`, `data.frame`, `character`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/R/newhall.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `r-pkg/src/`, `r-pkg/R/categories.R`, `r-pkg/R/raster.R`.
- **Depends on:** `4.1.1`, `4.1.2`
- **Spec:**
  1. Add `newhall_batch_raw(lat, lon, temp, precip, awc, elev = ..., soil_air_offset = 2.5, amplitude = 0.66, has_o_horizon = FALSE, is_saturated = FALSE, is_metric = TRUE, strict = FALSE)` calling `newhall_batch_rust(...)` with `as.double(soil_air_offset)`, `as.double(amplitude)`, `as.integer(as.logical(has_o_horizon))`, `as.integer(as.logical(is_saturated))`.
  2. Make `newhall_batch(.data, ...)` an S3 generic (`UseMethod("newhall_batch")`):
     - `newhall_batch.default(.data = NULL, ...)` delegates to `newhall_batch_raw(...)` (supporting existing positional/named vector+matrix calls `newhall_batch(lat = ..., lon = ..., temp = ..., precip = ..., awc = ...)` cleanly).
     - `newhall_batch.data.frame(.data, unitSystem = "metric", soilAirOffset = ifelse(tolower(unitSystem) %in% c("in", "english"), 4.5, 2.5), amplitude = 0.66, hasOHorizon = FALSE, isSaturated = FALSE, strict = FALSE, ...)` uses `.normalize_unit_system()`, `.extract_df_inputs()`, calls `newhall_batch_raw()`, and applies `.decode_regimes()` so character regime names match `jNSMR::batch3()`.
     - `newhall_batch.character(.data, ...)` reads CSV file path(s) via `utils::read.csv()` (combining multiple files with `do.call(rbind, ...)`) and delegates to `newhall_batch.data.frame()`.
  3. Similarly add `newhall_batch_phase4_raw(...)`, make `newhall_batch_phase4(.data, ...)` an S3 generic with `newhall_batch_phase4.default(...)` (preserving existing vector/matrix calls) and `newhall_batch_phase4.data.frame(...)` (extracting the 28 base columns plus optional Phase 4 columns `oHorizonThickness`, `bulkDensity`, `quartzFraction`, `porosity`, `zWt`, `snowJan`..`snowDec`, returning decoded character regimes + `satDaysMcs` + `aquicBioDays`).
  4. Add complete roxygen2 tags (`@export`, `@rdname`, `@method`) for all new generics, methods, and `_raw` helpers.
- **Grep anchors:** `r-pkg/R/newhall.R` (L1–L138), `r-pkg/R/categories.R`, `r-pkg/R/extendr-wrappers.R`.
- **Acceptance & targeted verification:**
  - `Rscript -e "parse('r-pkg/R/newhall.R')"` exits 0.

---

### Task `4.2.2`: Implement chunked `SpatRaster` S3 methods with `terra::set.cats()`
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `r-pkg/R/raster.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `r-pkg/src/`, `r-pkg/R/newhall.R`, `r-pkg/R/categories.R`.
- **Depends on:** `4.1.1`, `4.1.2`
- **Spec:**
  1. Create `r-pkg/R/raster.R` implementing `newhall_batch.SpatRaster` and `newhall_batch_phase4.SpatRaster` with roxygen2 `@export` tags.
  2. Guard `terra` with `if (!requireNamespace("terra", quietly = TRUE)) stop("Package 'terra' is required for SpatRaster inputs.", call. = FALSE)`.
  3. Accept `jNSMR`-compatible arguments: `unitSystem = "metric"`, `soilAirOffset = ...`, `amplitude = 0.66`, `hasOHorizon = FALSE`, `isSaturated = FALSE`, `strict = FALSE`, `file = ""`, `nrows = NULL`, `overwrite = TRUE`, `...` (plus Phase 4 parameters on `newhall_batch_phase4.SpatRaster`).
  4. Validate required 28 layer names in `names(.data)` before reading.
  5. Create output template `out <- terra::rast(.data, nlyrs = 19L)` (or `21L` for Phase 4) with exact `jNSMR` layer names (`annualRainfall` .. `regimeSubdivision2`, plus `satDaysMcs`, `aquicBioDays` for Phase 4).
  6. Use chunked I/O: `terra::readStart(.data)` + `out_info <- terra::writeStart(out, filename = file, overwrite = overwrite)` (with `on.exit()` cleanup for `readStop`/`writeStop` on error). Subdivide `terra` blocks if `nrows` is provided and smaller than block size.
  7. For each chunk:
     - Read via `terra::readValues(.data, row = r_start, nrows = r_n, dataframe = TRUE)`.
     - Identify skipped cells: `is.na(chunk$awc)` when `!strict` (classic `jNSMR` parity), or any `NA` in required columns when `strict = TRUE` / Phase 4.
     - Handle all-`NA` chunks cleanly by writing an `NA_real_` matrix of shape `nrow(chunk) x nlyrs` without calling Rust.
     - Run `newhall_batch_raw(...)` (or `newhall_batch_phase4_raw(...)`) on valid rows, scatter results back into a pre-allocated `NA_real_` matrix of shape `nrow(chunk) x nlyrs`, and write via `terra::writeValues(out, mat, r_start, nrows = r_n)`.
  8. Finish with `out <- terra::writeStop(out)` and `terra::readStop(.data)`, then attach 0-indexed categorical tables via `terra::set.cats()` on layers `16` (`temperatureRegime`), `17` (`moistureRegime`), `18` (`regimeSubdivision1`), and `19` (`regimeSubdivision2`) using `.regime_cat_df()`.
- **Grep anchors:** `planning/audit/r_api_audit.md` (Section 3, Section 4.1–4.2), `r-pkg/R/categories.R`.
- **Acceptance & targeted verification:**
  - `Rscript -e "parse('r-pkg/R/raster.R')"` exits 0.

---

### Task `4.3.1`: Sync `NAMESPACE`/docs and add `testthat` S3 + `SpatRaster` test suite
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/NAMESPACE`, `r-pkg/man/*.Rd`, `r-pkg/tests/testthat/test-phase4.R` (if needed)
  - *Can create:* `r-pkg/tests/testthat/test-s3-api.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`, `r-pkg/src/rust/`.
- **Depends on:** `4.2.1`, `4.2.2`
- **Spec:**
  1. Run `Rscript -e "roxygen2::roxygenise('r-pkg')"` (or update `NAMESPACE` and `man/*.Rd` cleanly if `roxygen2` is unavailable) so all S3 generics, S3 methods (`S3method(newhall_batch, ...)`), and `_raw` helpers are exported and documented.
  2. Create `r-pkg/tests/testthat/test-s3-api.R` testing:
     - `newhall_batch()` on a `data.frame` (returns 19 columns, character regime names, matches `newhall_batch_raw()`).
     - Per-row `soilAirOffset`, `amplitude`, `hasOHorizon`, `isSaturated` columns in `data.frame` change results relative to scalar defaults.
     - `unitSystem = "english"` with `strict = TRUE` matches metric `strict = TRUE`.
     - `newhall_batch()` on a `character` CSV file path (`tempfile(fileext = ".csv")`).
     - `newhall_batch()` on a `terra::SpatRaster` (`skip_if_not_installed("terra")`) with `NA` cells in `awc`, forced multi-block chunking (`nrows = 2L`), `terra::cats()` level verification on all 4 categorical layers, and `strict = TRUE` vs `FALSE`.
     - `newhall_batch_phase4()` on both `data.frame` and `terra::SpatRaster` (21 layers including `satDaysMcs` and `aquicBioDays`, `terra::cats()` present).
  3. Rebuild the shared library (`rm -f r-pkg/src/newhallrs.so r-pkg/src/rust/target/release/libnewhallrs.a`) and run `Rscript -e "devtools::test('r-pkg')"`.
- **Grep anchors:** `r-pkg/R/newhall.R`, `r-pkg/R/raster.R`, `r-pkg/tests/testthat/test-phase4.R`.
- **Acceptance & targeted verification:**
  - `rm -f r-pkg/src/newhallrs.so r-pkg/src/rust/target/release/libnewhallrs.a && Rscript -e "devtools::test('r-pkg')"` exits 0 with 0 failures and 0 warnings.
