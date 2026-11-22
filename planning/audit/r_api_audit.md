# `jNSMR` R API, Categorical Factors & Sample Datasets Audit

Audited repository: `/home/andrew/workspace/jNSMR` (v0.3.1, `DESCRIPTION`, `NAMESPACE`, `R/`, `tests/`, `inst/extdata/`).
All observations are from direct source inspection and read-only `terra`/R metadata inspection.

---

## 1. File Inventory

### 1.1 `/home/andrew/workspace/jNSMR/R/` (12 files, 2,283 lines, 104,013 bytes)

| File | Lines | Bytes | Primary Responsibility |
| --- | ---: | ---: | --- |
| `AAAA.R` | 55 | 1,832 | `.onLoad`/`.onAttach` JVM classpath setup (`newhall-1.6.5.jar` default via `.get_default_suffix()`), `newhall_version()` |
| `BASICSimulationModel.R` | 47 | 1,948 | `BASICSimulationModel()` Java object constructor; `newhall_simulation()` single-run wrapper |
| `jNSMR-package.R` | 8 | 279 | Package-level roxygen stub documenting `jNSMR.JAR_SUFFIX` |
| `NewhallBatch.R` | 964 | 49,488 | `newhall_batch()` generic & 5 S3 methods, `batch1`/`batch2`/`batch3`, `.setCats()`, `.fctDF()`, `.str`/`.smr`/`.smrsub1`/`.smrsub2` |
| `NewhallData.R` | 660 | 27,297 | Data acquisition/formatting helpers (`newhall_data_dir`, `.add_LL`, PRISM, Daymet, ISSR-800, WorldClim, CMIP6) |
| `NewhallDatasetMetadata.R` | 95 | 5,111 | `NewhallDatasetMetadata()` Java wrapper (`org/psu/newhall/sim/NewhallDatasetMetadata`) |
| `NewhallDataset.R` | 176 | 6,440 | `NewhallDataset()`, `NewhallDatasetFromPath()`, `XMLFileParser()`, `CSVFileParser()`, `xml_NewhallDataset()`, `csv_NewhallDataset()` |
| `NewhallGUI.R` | 20 | 891 | `newhall_GUI()` Swing launcher (hardcodes `-1.6.1` JAR suffix) |
| `NewhallInput.R` | 93 | 4,794 | Unexported batch CSV template/reader/writer helpers (`newhall_writeBatchTemplate`, `newhall_readBatchInput`, `newhall_writeBatchOutput`, `.colnamesNewhallBatch`, `.newhall_batch_template`) |
| `NewhallResults.R` | 69 | 2,682 | `XMLResultsExporter`, `XMLStringResultsExporter`, `CSVResultsExporter` and `newhall_*Exporter()` wrappers |
| `rJava_utils.R` | 55 | 1,614 | `.rvec2jarraylist()` helper converting R vectors to `java.util.ArrayList` |
| `writeRasterLayers.R` | 41 | 1,637 | `writeRasterLayers()` splits multiband rasters into single-layer files via `terra::writeRaster()` |

### 1.2 `/home/andrew/workspace/jNSMR/tests/` (9 files, 470 lines, 14,727 bytes)

| File | Lines | Bytes | Coverage |
| --- | ---: | ---: | --- |
| `tests/testthat.R` | 4 | 54 | Calls `test_check("jNSMR")` |
| `tests/testthat/test-BASICSimulationModel.R` | 60 | 2,020 | `NewhallDataset()`, `newhall_simulation()`, `NewhallDatasetMetadata()`, `newhall_CSVResultsExporter()`, v1.6.1 XML exporter messages |
| `tests/testthat/test-NewhallBatch.R` | 64 | 2,195 | `newhall_batch()` on PA CSV, `batch1()`/`batch2()`, `newhall_writeBatchTemplate()`, `SpatRaster` single-core & `cores = 2`, PET balance check |
| `tests/testthat/test-NewhallData.R` | 129 | 2,994 | `newhall_data_dir()`, `.add_LL()`, `newhall_nad83_template()`, offline/CRAN-skipped PRISM/Daymet/ISSR-800 tests |
| `tests/testthat/test-NewhallDatasetMetadata.R` | 30 | 1,070 | `NewhallDatasetMetadata()` construction and `$getMetadata()`/`$setMetadata()` |
| `tests/testthat/test-NewhallDataset.R` | 162 | 5,751 | `csv_NewhallDataset()` with `mead_agronomy.csv`, `checkargs` validation errors, v1.6.1-only XML I/O test |
| `tests/testthat/test-NewhallGUI.R` | 7 | 109 | `newhall_GUI(command_only = TRUE)` |
| `tests/testthat/test-NewhallInput.R` | 7 | 270 | `newhall_writeBatchTemplate()`, `newhall_readBatchInput()`, `newhall_writeBatchOutput()` |
| `tests/testthat/test-NewhallResults.R` | 7 | 264 | Legacy XML exporter/parser messages when `newhall_version() != "1.6.1"` |

### 1.3 `/home/andrew/workspace/jNSMR/inst/extdata/` (6 files, 475,203 bytes)

| File | Lines | Bytes | Format |
| --- | ---: | ---: | --- |
| `All_PA_jNSM_Example_Batch_Metric.csv` | 109 (1 header + 108 rows) | 23,534 | Metric batch CSV (38 columns) |
| `Williamsport_PA_jNSM_Example_Batch_English.csv` | 116 (1 header + 115 rows) | 27,747 | English batch CSV (38 columns) |
| `mead_agronomy.csv` | 5 | 353 | Legacy 2-station positional CSV |
| `WILLIAMSPORT_1930_1930_input.xml` | 83 | 2,244 | Legacy jNSM v1.5.0/1.6.1 XML input |
| `prism_issr800_sample.tif` | — (binary) | 221,129 | 28-layer GeoTIFF (`113 x 168`, `INT2S`) |
| `daymet_issr800_sample.tif` | — (binary) | 200,196 | 28-layer GeoTIFF (`113 x 168`, `INT2S`) |

### 1.4 `NAMESPACE` Summary
Exports:
- 5 S3 methods for `newhall_batch`: `RasterBrick`, `RasterStack`, `SpatRaster`, `character`, `default`
- 34 exported functions: `BASICSimulationModel`, `CSVFileParser`, `CSVResultsExporter`, `NewhallDataset`, `NewhallDatasetFromPath`, `NewhallDatasetMetadata`, `XMLFileParser`, `XMLResultsExporter`, `XMLStringResultsExporter`, `csv_NewhallDataset`, `newhall_CSVResultsExporter`, `newhall_GUI`, `newhall_XMLResultsExporter`, `newhall_XMLStringResultsExporter`, `newhall_batch`, `newhall_cmip6_cache`, `newhall_cmip6_rast`, `newhall_cmip6_subset`, `newhall_data_dir`, `newhall_daymet_subset`, `newhall_issr800_cache`, `newhall_issr800_rast`, `newhall_issr800_subset`, `newhall_nad83_template`, `newhall_prism_cache`, `newhall_prism_rast`, `newhall_prism_subset`, `newhall_simulation`, `newhall_version`, `newhall_worldclim_cache`, `newhall_worldclim_rast`, `newhall_worldclim_subset`, `writeRasterLayers`, `xml_NewhallDataset`.

---

## 2. `newhall_batch()` S3 Dispatch, Signatures & Schemas

### 2.1 Generic and S3 Method Signatures

All methods live in `/home/andrew/workspace/jNSMR/R/NewhallBatch.R`.

| Function | Line | Notable Default Differences | Dispatch Path |
| --- | ---: | --- | --- |
| `newhall_batch()` | 583 | `toString = TRUE`, `cores = 1`, `core_thresh = 25000L`, `file = paste0(tempfile(), ".tif")`, `nrows = nrow(.data)`, `overwrite = TRUE` | `UseMethod("newhall_batch", .data)` |
| `newhall_batch.default()` | 85 | `.data = NULL`, `toString = TRUE`, `cores = NULL`, `core_thresh = NULL`, `file = NULL`, `nrows = NULL`, `overwrite = NULL` | Calls `batch3()` if `newhall_version() >= "1.6.5"`, `batch2()` if `1.6.3 <= v < 1.6.5`, else `batch1()` |
| `newhall_batch.character()` | 602 | Same defaults as generic | Reads each existing path via `newhall_readBatchInput(x)` (`[, .colnamesNewhallBatch()]`), binds with `data.table::rbindlist(..., fill = TRUE)`, calls `newhall_batch.default()` |
| `newhall_batch.SpatRaster()` | 675 | `toString = FALSE`, `nrows = nrow(.data) / (terra::ncell(.data) / core_thresh)` | Chunked `terra::readValues()` -> `batch3()` (if `cores > 1`) or `newhall_batch.default()` (if `cores == 1`) -> `.setCats()` -> `terra::writeValues()` -> `terra::set.cats()` |
| `newhall_batch.RasterBrick()` | 898 | `toString = TRUE`, `nrows = ifelse(ncol(.data)*nrow(.data) < core_thresh, nrow(.data), floor(ncol(.data)*nrow(.data) / (core_thresh * cores)))` | Delegates to `newhall_batch.RasterStack()` |
| `newhall_batch.RasterStack()` | 933 | Same defaults as `RasterBrick` | Converts via `terra::rast(.data)` and delegates to `newhall_batch.SpatRaster()` |

Shared simulation parameter defaults across all 6 functions:
- `unitSystem = "metric"` (accepted case-insensitively: `"metric"`, `"mm"`, `"cm"` -> internal `"cm"`; `"in"`, `"english"` -> internal `"in"`)
- `soilAirOffset = ifelse(unitSystem %in% c("in", "english"), 4.5, 2.5)`
- `amplitude = 0.66`
- `hasOHorizon = FALSE`
- `isSaturated = FALSE`
- `verbose = TRUE`
- `checkargs = TRUE`

### 2.2 Input Requirements

Required columns/layers enforced by `.colnamesNewhallBatch()` (`NewhallInput.R:47-50`):
- Coordinates: `"latDD"`, `"lonDD"`
- Monthly air temperature (deg C or F): `"tJan"`, `"tFeb"`, `"tMar"`, `"tApr"`, `"tMay"`, `"tJun"`, `"tJul"`, `"tAug"`, `"tSep"`, `"tOct"`, `"tNov"`, `"tDec"`
- Monthly precipitation (mm or in): `"pJan"`, `"pFeb"`, `"pMar"`, `"pApr"`, `"pMay"`, `"pJun"`, `"pJul"`, `"pAug"`, `"pSep"`, `"pOct"`, `"pNov"`, `"pDec"`
- Soil AWC & elevation: `"awc"`, `"elev"` (total: **28 required columns/layers**)

Optional per-row / per-cell input columns/layers read by `batch3()` (`NewhallBatch.R:456-474`):
- `"soilAirOffset"` (`as.double`), `"amplitude"` (`as.double`), `"hasOHorizon"` (`as.logical`), `"isSaturated"` (`as.logical`). If absent (`length(...) == 0`), `batch3()` broadcasts the corresponding scalar argument across `nrow(.data)`.

**Critical implementation details in `batch3()` (`NewhallBatch.R:400-533`):**
1. **Regex column selection:** `batch3()` selects monthly precipitation and temperature columns using `grep("^p[A-Z][a-z]{2}$", cnd)` and `grep("^t[A-Z][a-z]{2}$", cnd)` and prepends a dummy `0.0` column (`cbind(0.0, as.matrix(...))`). Column order in `.data` among the 12 months matters (must be Jan..Dec) if columns are selected by regex!
2. **Dead unit-conversion helpers:** `batch3()` (and `batch2()`) define `.doUnitsTemp` and `.doUnitsLength` (lines 437–440) **but never invoke them**; instead they pass raw values and `rep(unitSystem == "cm", nrow(.data))` (`isMetric`) to Java `BASICSimulationModel.runBatch2(...)`.
3. **Hemisphere handling:** `batch3()` computes `hem` as `'N'` (`latDD >= 0`) or `'S'` (`latDD < 0`) (lines 447–448), whereas `batch2()` hardcoded `'N'` for all rows (line 344).
4. **Unused `toString` / `checkargs` in `batch2()`/`batch3()`:** Neither `batch2()` nor `batch3()` uses `toString` or `checkargs` (unlike `batch1()`, which creates `dataset`, `results`, `output` list/character columns).

### 2.3 Return Columns / Layers & Types (`newhall-1.6.5.jar` / `batch3()`)

From `NewhallBatch.R:497-532, 706-712` and `NewhallBatchResults.java:6-27`:

| Index | Name | Java `NewhallBatchResults` Type | R `data.frame` Type (`batch3` after `type.convert(..., as.is = TRUE)`) | `SpatRaster` Layer Encoding |
| ---: | --- | --- | --- | --- |
| 1 | `annualRainfall` | `double[]` | `numeric` (double) | `numeric` (double) |
| 2 | `waterHoldingCapacity` | `double[]` | `numeric` (double) | `numeric` (double) |
| 3 | `annualWaterBalance` | `double[]` | `numeric` (double) | `numeric` (double) |
| 4 | `annualPotentialEvapotranspiration` | `double[]` | `numeric` (double) | `numeric` (double) |
| 5 | `summerWaterBalance` | `double[]` | `numeric` (double) | `numeric` (double) |
| 6 | `dryDaysAfterSummerSolstice` | `int[]` | `integer` | `numeric` (written via `as.matrix`) |
| 7 | `moistDaysAfterWinterSolstice` | `int[]` | `integer` | `numeric` |
| 8 | `numCumulativeDaysDry` | `int[]` | `integer` | `numeric` |
| 9 | `numCumulativeDaysMoistDry` | `int[]` | `integer` | `numeric` |
| 10 | `numCumulativeDaysMoist` | `int[]` | `integer` | `numeric` |
| 11 | `numCumulativeDaysDryOver5C` | `int[]` | `integer` | `numeric` |
| 12 | `numCumulativeDaysMoistDryOver5C` | `int[]` | `integer` | `numeric` |
| 13 | `numCumulativeDaysMoistOver5C` | `int[]` | `integer` | `numeric` |
| 14 | `numConsecutiveDaysMoistInSomeParts` | `int[]` | `integer` | `numeric` |
| 15 | `numConsecutiveDaysMoistInSomePartsOver8C` | `int[]` | `integer` | `numeric` |
| 16 | `temperatureRegime` | `String[]` | `character` | Categorical (`0..9` via `terra::set.cats`) |
| 17 | `moistureRegime` | `String[]` | `character` | Categorical (`0..5` via `terra::set.cats`) |
| 18 | `regimeSubdivision1` | `String[]` | `character` | Categorical (`0..8` via `terra::set.cats`) |
| 19 | `regimeSubdivision2` | `String[]` | `character` | Categorical (`0..7` via `terra::set.cats`) |

Notes:
- **Layer-order inconsistency in empty-block fallback (`NewhallBatch.R:789-799`):** When `cores > 1` and `nrow(blockcomplete) == 0`, the placeholder `data.frame` places `summerWaterBalance` before `annualPotentialEvapotranspiration` (index 4 vs 5) — reversed relative to `fields` (lines 501–502) and `cnm` (lines 706–707).
- **Pre-1.6.4 `SpatRaster` branch (`NewhallBatch.R:714-720`):** Lists 15 layers starting with `"nrow"` and omitting `annualRainfall`, `waterHoldingCapacity`, `annualWaterBalance`, `annualPotentialEvapotranspiration`, `summerWaterBalance`.
- **Legacy `batch1()` return (`NewhallBatch.R:216-235`):** Uses PascalCase column names (`NumCumulativeDaysMoist`, `TemperatureRegime`, `MoistureRegime`, `RegimeSubdivision1`, `RegimeSubdivision2`, etc.) plus `dataset`, `results`, `output` columns. (Note `test-NewhallBatch.R:24-25` asserts `res2$TemperatureRegime == res3$temperatureRegime`, which only executes when `newhall_version() >= "1.6.3"` and would fail if `res2` came from `batch3()` because `batch3()` returns lowercase `temperatureRegime` — `test-NewhallBatch.R` works only if `batch1` isn't used or wait: `res2 <- newhall_batch(.data, toString = FALSE)` on v1.6.5 calls `batch3()`, so `res2$TemperatureRegime` is `NULL`! See `1.2.2` for baseline test run.)

---

## 3. Categorical Factor Levels & Integer Codes (`terra::set.cats()`)

Defined in `/home/andrew/workspace/jNSMR/R/NewhallBatch.R:641-663` and `885-892`:
- `.fctDF(levels, name = "category", offset = -1)` constructs `data.frame(value = seq_along(levels) + offset, <name> = levels)`.
- `.setCats(x)` converts character outputs to **0-indexed** integer codes (`as.numeric(factor(..., levels = ...)) - 1`), mapping `" "` (single space from Java) in `regimeSubdivision1` and `regimeSubdivision2` to `"Undefined"` via `gsub(" ", "Undefined", ...)`.

| Layer Name | Helper | Code `0` | `1` | `2` | `3` | `4` | `5` | `6` | `7` | `8` | `9` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `temperatureRegime` | `.str()` | `Pergelic` | `Cryic` | `Frigid` | `Mesic` | `Thermic` | `Hyperthermic` | `Isofrigid` | `Isomesic` | `Isothermic` | `Isohyperthermic` |
| `moistureRegime` | `.smr()` | `Aridic` | `Ustic` | `Xeric` | `Udic` | `Perudic` | `Undefined` | — | — | — | — |
| `regimeSubdivision1` | `.smrsub1()` | `Typic` | `Weak` | `Wet` | `Dry` | `Extreme` | `Xeric` | `Udic` | `Aridic` | `Undefined` | — |
| `regimeSubdivision2` | `.smrsub2()` | `Aridic` | `Tempustic` | `Tropustic` | `Tempudic` | `Xeric` | `Udic` | `Tropudic` | `Undefined` | — | — |

---

## 4. `NA` / `NaN` Handling & Raster Chunking Behavior

### 4.1 `NA` / `NaN` Handling
- **`character` / `default` (`batch3()`):** No `NA`/`NaN` filtering is performed. `NA`/`NaN` values in numeric vectors are converted to `NaN` / `NA_real_` by `as.double()` and passed directly to Java `runBatch2()`.
- **`SpatRaster` (`NewhallBatch.R:746-750, 803-804, 835-839, 859-860`):**
  - Before calling Java, `newhall_batch.SpatRaster` identifies skipped cells **exclusively via `is.na(awc)`**:
    `skip.idx <- which(is.na(blockdata$awc))` (for `cores > 1`) and `skip.idx <- which(is.na(dataall$awc))` (for `cores == 1`).
  - Both `NA` and `NaN` in `awc` evaluate `TRUE` under `is.na()`. Cells with `NA`/`NaN` in `t*`, `p*`, `latDD`, `lonDD`, or `elev` (where `awc` is non-`NA`) are **not** filtered out and are sent to Java.
  - After simulation, `r.na <- r[0, , drop = FALSE][1:length(skip.idx), , drop = FALSE]` creates `NA` rows for skipped indices and interleaves them back into cell order via `rbind(r, r.na)[order(c(ids[!ids %in% skip.idx], skip.idx)), ]`.
  - **Edge case when `length(skip.idx) == 0`:** `1:length(skip.idx)` evaluates to `c(1L, 0L)` in R, creating 1 extra `NA` row in `r.na` that is dropped by `order(1:nrow(r))` (which has length `nrow(r)`), leaving output row count intact (`nrow(blockdata)`).
  - **Multicore `NA` fill (`NewhallBatch.R:785-787`):** When `cores > 1`, `data.table::rbindlist(..., use.names = TRUE, fill = TRUE)` can introduce `NA` in `regimeSubdivision1`/`2` if some worker chunks have empty/type-mismatched columns; lines 786–787 explicitly coerce `NA` -> `"Undefined"` in those two columns before `.setCats()`.

### 4.2 Raster Chunking & Parallel Processing (`NewhallBatch.R:690-893`)
1. Opens input raster with `suppressWarnings(terra::readStart(.data))` and creates output template `out <- terra::rast(.data)` with `nlyr = 19` (`newhall_version() >= "1.6.4"`).
2. Forces `cores <- 1` when `cores > 1 && terra::ncell(.data) < core_thresh` (default `core_thresh = 25000L`).
3. Floors `nrows <- floor(nrows)`. Default `nrows` in `SpatRaster` is `nrow(.data) / (terra::ncell(.data) / core_thresh)`, whereas `RasterBrick`/`RasterStack` compute `ifelse(ncol(.data) * nrow(.data) < core_thresh, nrow(.data), floor(ncol(.data) * nrow(.data) / (core_thresh * cores)))`.
4. Calls `out_info <- terra::writeStart(out, filename = file, overwrite = overwrite, progress = 0)` to obtain `terra`'s block boundaries (`out_info$row`, `out_info$nrows`, `out_info$n`), then subdivides each `terra` write block into sub-chunks of at most `nrows` rows (`start_row`, `n_row`).
5. Loops over blocks `(i, j)`:
   - Reads chunk with `terra::readValues(.data, row = start_row[[i]][j], nrows = n_row[[i]][j], dataframe = TRUE)`.
   - If `cores > 1`: splits non-skipped rows (`blockcomplete`) evenly across `cores` PSOCK workers (`parallel::makeCluster(cores)`) calling `batch3()` directly (each worker loads `jNSMR`/JVM via `clusterApply`).
   - If `cores == 1`: calls `newhall_batch.default()` on `datacomplete`, drops `dataset`/`results`/`output` if present.
   - Re-inserts `skip.idx` `NA` rows, runs `.setCats()`, writes chunk with `terra::writeValues(out, as.matrix(r), start_row[[i]][j], nrows = n_row[[i]][j])`.
6. Closes handles with `out <- terra::writeStop(out)` and `terra::readStop(.data)`, attaches categorical tables with 4 `terra::set.cats()` calls, and returns `out`.

---

## 5. Sample Datasets in `inst/extdata/` & Test Usage

### 5.1 `All_PA_jNSM_Example_Batch_Metric.csv`
- **Size:** 109 lines (header + **108 rows**), 23,534 bytes.
- **Columns (38):** `stationName`, `netType`, `latDD`, `lonDD`, `elev`, `tJan`..`tDec`, `pJan`..`pDec`, `pdType`, `pdStartYr`, `pdEndYr`, `awc`, `maatmast`, `cntryCode`, `stProvCode`, `notes`, `stationID`.
- **NAs:** Only `notes` is `NA` (all 108 rows); all numeric/input columns are non-`NA`. All stations are in Pennsylvania (`stProvCode = PA`, `cntryCode = US`, `awc = 200`, `maatmast = 1.2`, metric units: deg C, mm, m).
- **Test invocation (`tests/testthat/test-NewhallBatch.R`):**
  - Test 1 (`"newhall_batch() works"`, lines 3–26): `read.csv(pathname)` -> `newhall_batch(.data = .data, toString = FALSE)`, `batch1(.data)`, and `batch2(.data = .data)`.
  - Test 2 (`"newhall_batch() raster interfaces"`, lines 30–62): selects `latDD`, `lonDD`, `tJan..tDec`, `pJan..pDec`, builds a `9 x 12` (108-cell) `SpatRaster`, adds constant/sequence layers (`stationName`, `awc = 200`, `maatmast = 1.2`, `pdStartYr = 1981`, `pdEndYr = 2010`, `cntryCode = "US"`, `netType = ""`, `elev = -9999`, `stProvCode = ""`, `notes = ""`, `stationID`), and runs `newhall_batch(r)` and `newhall_batch(r, cores = 2)`.

### 5.2 `Williamsport_PA_jNSM_Example_Batch_English.csv`
- **Size:** 116 lines (header + **115 rows**), 27,747 bytes.
- **Columns (38):** Identical schema to `All_PA_jNSM_Example_Batch_Metric.csv`. All `stationID` values are `NA` (115); all other columns non-`NA`. Values are English units (deg F, inches, feet, `awc = 7.874`, `maatmast = 2.16` for 1895–2009 annual/actual rows).
- **Test invocation:** Not invoked in `tests/testthat/`.

### 5.3 `prism_issr800_sample.tif`
- **Size:** 221,129 bytes.
- **Raster geometry:** `SpatRaster` with `113` rows x `168` cols x `28` layers (`18,984` total cells), CRS `NAD83 (EPSG:4269)`, resolution `0.008333333` deg (~800 m), extent `-120.9958, -119.5958, 37.6375, 38.57917` (California Sierra foothills/valley).
- **Layers (28):** `pJan`, `pFeb`, `pMar`, `pApr`, `pMay`, `pJun`, `pJul`, `pAug`, `pSep`, `pOct`, `pNov`, `pDec`, `tJan`, `tFeb`, `tMar`, `tApr`, `tMay`, `tJun`, `tJul`, `tAug`, `tSep`, `tOct`, `tNov`, `tDec`, `lonDD`, `latDD`, `awc`, `elev`.
- **Storage & NAs:** All 28 layers stored as `INT2S` (`scale = 1, offset = 0`); `elev` is `0` everywhere; `awc` has `194` `NA` cells (`18,790` non-`NA` cells), while the other 27 layers have `0` `NA` cells.
- **Test invocation:** Not invoked in `tests/testthat/` (used in `R/NewhallBatch.R` roxygen `@examples`, `README.md`, and `vignettes/newhall-prism.Rmd`).

### 5.4 `daymet_issr800_sample.tif`
- **Size:** 200,196 bytes.
- **Raster geometry:** Identical grid, CRS (`EPSG:4269`), extent, `113 x 168 x 28` dimensions, `INT2S` storage (`scale = 1, offset = 0`), layer names, `elev = 0`, and `194` `NA` cells in `awc` (`18,790` valid cells) as `prism_issr800_sample.tif`.
- **Test invocation:** Not invoked in `tests/testthat/` (used in `README.md`).

### 5.5 `mead_agronomy.csv`
- **Size:** 5 lines, 353 bytes.
- **Content:** Headerless legacy 2-station positional format (2 lines per station, with a blank line in station 2; `Mead Agronomy Lab` and `Dickinson Expt Station ND`, DMS coordinates, English `"E"` unit flag).
- **Test invocation:** `tests/testthat/test-NewhallDataset.R:2-7` via `csv_NewhallDataset(pathname = oldcsv)` and `newhall_simulation(dataset)`.

### 5.6 `WILLIAMSPORT_1930_1930_input.xml`
- **Size:** 83 lines, 2,244 bytes.
- **Content:** Single-station jNSM XML input (`WILLIAMSPORT`, `PA`, `US`, `41.24`, `-76.92`, `158.0`, 1930, `smcsawc = 200.0`, `ampltd = 0.66`, `maatmast = 1.2`).
- **Test invocation:** Not directly loaded in `tests/testthat/` (an identical inline XML string is tested in `test-NewhallDataset.R:54-141` when `newhall_version() == "1.6.1"`, skipped on `1.6.5`).
