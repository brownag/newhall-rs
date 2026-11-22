# Orchestration 1 State (`planning/STATE_1.md`)

Plan: `planning/EXECUTION_PLAN_1.md`
Last Updated: 2026-11-22

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs`
- [x] Reference repo readable: `/home/andrew/workspace/jNSMR` (v0.3.1) and `/home/andrew/workspace/jNSMR/java`
- [x] R + Java (`rJava`, `terra`, `testthat`, `jNSMR`) availability verified (checked in `1.2.2`)

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `1.1.1` | Audit Java NSM source & custom modifications | done | flash | — |
| 1 | `1.1.2` | Audit `jNSMR` R API, factor levels & sample datasets | done | flash | — |
| 2 | `1.2.1` | Synthesize `planning/AUDIT_JNSM.md` & ambiguity sign-off list | done | flash | — |
| 2 | `1.2.2` | Verify `jNSMR` oracle baseline & write fixture generator script | done | flash | — |
| 3 | `1.3.1` | Generate & verify golden-master fixtures | done | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

## 1.1.2
- **Status:** done
- **Files created/changed:** `planning/audit/r_api_audit.md` (209 lines), `planning/tasks/1.1.2.todo.md` (33 lines)
- **Verification:** `test -s planning/audit/r_api_audit.md` (PASS); `grep -q "newhall_batch" planning/audit/r_api_audit.md && grep -q "All_PA_jNSM_Example_Batch_Metric.csv" planning/audit/r_api_audit.md` (PASS)
- **Deviations:** none (also read `NewhallBatchResults.java` read-only to verify exact Java field types)
- **Gotchas for dependent tasks:**
  - `batch3()` defines `.doUnitsTemp`/`.doUnitsLength` but never calls them; it forwards raw inputs + `isMetric` (`unitSystem == "cm"`) to Java `runBatch2()`, selecting `^p[A-Z][a-z]{2}$` and `^t[A-Z][a-z]{2}$` by regex in column order.
  - `newhall_batch.SpatRaster()` skips cells **only** via `is.na(awc)`; in `prism_issr800_sample.tif` and `daymet_issr800_sample.tif` (`113 x 168`, `18,984` cells, `INT2S`, `EPSG:4269`), `awc` has `194` `NA`s (`18,790` valid cells) and `elev` is `0` everywhere.
  - `terra::set.cats()` uses **0-indexed** codes (`0..k-1`), mapping `" "` in `regimeSubdivision1`/`2` to `"Undefined"`.
  - Empty-block `cores > 1` fallback reverses `annualPotentialEvapotranspiration` & `summerWaterBalance`; `test-NewhallBatch.R:24-25` compares PascalCase `res2$TemperatureRegime` vs `res3$temperatureRegime`.

## 1.1.1
- **Status:** done
- **Files created/changed:** `planning/audit/java_core_audit.md` (266 lines); `planning/tasks/1.1.1.todo.md` (30 lines)
- **Verification:**
  - `test -s planning/audit/java_core_audit.md` -> pass
  - `grep -q "## Ambiguities" planning/audit/java_core_audit.md` -> pass
  - All 10 `.java` files in `/home/andrew/workspace/jNSMR/java` verified present -> `ALL_10_JAVA_FILES_VERIFIED`
- **Deviations:** none
- **Gotchas for dependent tasks (`1.2.1`):**
  - `runBatch2` -> `runSimulation(double[], ...)` (v1.6.5) diverges from `runSimulation(NewhallDataset, ...)`: drops `cs` adjustment on `st`/`wt`, changes `cr[7]` to `>= 6`, adds `hasOHorizon`/`isSaturated` `cryic_ht`, and ignores `isMetric` for temp/precip/WHC/offset.
  - `BASICSimulationModel` swaps `q` and `div` into `NewhallResults(..., div, q)`, putting `q` in `regimeSubdivision1` and `div` in `regimeSubdivision2`.
  - Key quirks for sign-off: SH PET `* 60.0D` precedence bug (`5 <= lat < 50`); `Pergelic` overwritten by `Frigid`/`Isofrigid` when `tma < 0 && st < 0`; cold-site `ncpm[2] = ncpm[1]` when `np8 == 0`; `NaN` precip yields `Perudic`; `hasRunAlready` skips intervals 2+ for `nsd`.

## 1.2.1
- **Status:** done
- **Files created/changed:** `planning/AUDIT_JNSM.md` (185 lines), `planning/tasks/1.2.1.todo.md` (19 lines)
- **Verification:**
  - `test -s planning/AUDIT_JNSM.md` -> PASS (`TEST_S_PASS`)
  - `grep -q "Sign-Off Checklist" planning/AUDIT_JNSM.md` -> PASS (`GREP_PASS`)
- **Deviations:** none
- **Gotchas for dependent tasks (`G3`/`G4`/`G7`):**
  - 11 items in `## Sign-Off Checklist` of `planning/AUDIT_JNSM.md` require explicit user confirmation before deviating from or replicating Java/R quirks (SH PET `* 60.0D` precedence bug, broken `isMetric = false` conversion, `Pergelic` overwrite, unattenuated `dif = st - wt`, cold `ncpm[2]` fallback, `hasRunAlready`, negative `latDD` handling, `q`/`div` swap, `NaN`/`Perudic` trap, threshold `5.0`/`8.0` perturbation, `runBatch2` vs `NewhallDataset` divergence).
  - `NewhallBatchResults` exposes 3 extra `fieldsmatrix` arrays (`meanPotentialEvapotranspiration`, `temperatureCalendar` with `-1`/`5`/`8`, `moistureCalendar` with `1`/`2`/`3`) beyond the 19 `batch3()` columns.

## 1.2.2
- **Status:** done
- **Files created/changed:** `scripts/generate_golden_fixtures.R` (269 lines), `planning/tasks/1.2.2.todo.md` (24 lines)
- **Verification:**
  - `Rscript -e 'testthat::test_local("/home/andrew/workspace/jNSMR")'` -> PASS (`[ FAIL 0 | WARN 0 | SKIP 4 | PASS 35 ]`, 2.4 s; `jNSMR` 0.3.1, `newhall-1.6.5.jar`, R 4.6.2, OpenJDK 25.0.5.1)
  - `Rscript -e 'parse("scripts/generate_golden_fixtures.R")'` -> PASS (exit 0)
- **Deviations:** none (script avoids `SpatRaster` `FLT4S` GeoTIFF quantization by extracting raster cells to `data.frame`, skipping `is.na(awc)`, and calling `newhall_batch()` / `batch3()` directly)
- **Gotchas for dependent tasks (`1.3.1`):**
  - Run `Rscript scripts/generate_golden_fixtures.R` from `/home/andrew/workspace/newhall-rs` (writes to relative `crates/newhall-core/tests/fixtures/`).
  - Categorical columns (`temperatureRegime`, `moistureRegime`, `regimeSubdivision1`, `regimeSubdivision2`) are double-quoted so Java's single-space `" "` subdivision stays distinct from `NA`; numeric columns are unquoted with `%.17g` precision.

## 1.3.1
- **Status:** done
- **Files created/changed:** `crates/newhall-core/tests/fixtures/pa_batch_metric.csv` (109 lines), `crates/newhall-core/tests/fixtures/prism_sample.csv` (18,985 lines), `crates/newhall-core/tests/fixtures/daymet_sample.csv` (18,985 lines), `crates/newhall-core/tests/fixtures/manifest.json` (61 lines), `planning/tasks/1.3.1.todo.md` (18 lines)
- **Verification:**
  - `Rscript scripts/generate_golden_fixtures.R` -> PASS (exit 0; 20.348s wall clock)
  - `test -s crates/newhall-core/tests/fixtures/pa_batch_metric.csv && test -s crates/newhall-core/tests/fixtures/prism_sample.csv && test -s crates/newhall-core/tests/fixtures/daymet_sample.csv && test -s crates/newhall-core/tests/fixtures/manifest.json` -> PASS
  - Schema/manifest/precision check (`wc -l` & Python CSV/JSON audit) -> PASS (47 columns matching manifest, `%.17g` floats up to 17 sig digits, all 4 categorical columns verified)
- **Counts & baseline `jNSMR` timing (`jNSMR` 0.3.1, `newhall-1.6.5.jar`):**
  - `pa_batch_metric.csv`: `108` rows (`108` valid, `0` `NA` rows, `0` `NA` cells), `0.095s`
  - `prism_sample.csv`: `18,984` rows (`18,790` valid, `194` `NA` rows, `3,880` `NA` cells = `194 * 20` cols; `0` `NA` cells in valid rows), `9.204s`
  - `daymet_sample.csv`: `18,984` rows (`18,790` valid, `194` `NA` rows, `3,880` `NA` cells = `194 * 20` cols; `0` `NA` cells in valid rows), `9.091s`
- **Deviations:** none
- **Gotchas for dependent tasks:**
  - `pa_batch_metric.csv` contains `" "` (single space) in `regimeSubdivision1`/`regimeSubdivision2` for `Perudic` rows (`toString = FALSE` Java raw strings vs `terra::set.cats()` `"Undefined"`).
  - `NA` cells in `prism_sample.csv` / `daymet_sample.csv` occur strictly on the 194 `is.na(awc)` rows (all 28 input cols except `awc` remain numeric; `awc` + 19 output cols are `"NA"`).
