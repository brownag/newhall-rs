# Execution Plan 1: Audit `jNSMR` / Custom Java NSM (G1) & Generate Golden-Master Fixtures (G2)

## Goal & Scope

### Goal
Audit the local `jNSMR` R package (`~/workspace/jNSMR`) and its customized Java Newhall Simulation Model source (`~/workspace/jNSMR/java`) to document every custom modification relative to standard jNSM, exact numerical/rounding behavior, and the R API contract (`G1`), then run `jNSMR` as a verified oracle to generate deterministic, full-precision golden-master test fixtures for `newhall-core` (`G2`).

### In Scope
- **Wave 1 (G1 — Parallel Read-Only Audit):**
  - Full inventory and diff/audit of `~/workspace/jNSMR/java` (all classes, custom v1.6.1 → v1.6.5 modifications, 8×8 moisture loop, Thornthwaite PET, thermal regime, O-horizon/saturation/offset logic, rounding, edge cases, ambiguities).
  - Full inventory of `~/workspace/jNSMR` (`R/`, `tests/`, `inst/extdata/`): `newhall_batch()` S3 API, input/output schemas, categorical factor levels (SMR, STR, subdivisions), `NA`/`NaN` handling, and sample datasets.
- **Wave 2 (G1 Synthesis + G2 Oracle Prep):**
  - Consolidate findings into `planning/AUDIT_JNSM.md` with explicit sign-off list for any scientific/rounding ambiguities.
  - Verify `jNSMR` `testthat` baseline and author `scripts/generate_golden_fixtures.R`.
- **Wave 3 (G2 — Golden-Master Generation):**
  - Run `scripts/generate_golden_fixtures.R` to produce `pa_batch_metric.csv`, `prism_sample.csv`, `daymet_sample.csv`, and `manifest.json` in `crates/newhall-core/tests/fixtures/` with `%.17g` IEEE-754 double precision.

### Out of Scope (Deferred to Later Orchestrations)
- Scaffolding the Cargo workspace, Rust traits, or `newhall-core` implementation (`G3`, `G4`).
- `newhall-cli` benchmarking (`G5`).
- `extendr` R–Rust bridge and R wrappers (`G6`, `G7`, `G8`).
- Physics extension stubs and `ARCHITECTURE.md` (`G9`, `G10`).
- Modifying any file inside `~/workspace/jNSMR` (strictly read-only reference repo).

### Verbatim Constraints
- "Start with **Phase 1** and summarize your findings from the Java/R codebase (especially custom modifications) before writing the Rust core."
- "Keep commits small, logical, and green (`cargo test` passing)."
- "Do not silently change scientific assumptions or rounding behavior in the baseline model; if you find ambiguities or bugs in the Java code, flag them and ask before deviating from parity."

---

## Decision Log

- **2026-11-22 — Target workspace path:** `~/workspace/newhall-rs` (with reference repos at `~/workspace/jNSMR` and `~/workspace/jNSMR/java`).
- **2026-11-22 — Wave split (G1 vs G2):** Keep read-only codebase audit (`G1`) and `jNSMR` oracle execution / golden-master generation (`G2`) in separate waves within Orchestration 1.
- **2026-11-22 — Workers & concurrency:** `flash` or Haiku-tier workers, escalate to `pro`/frontier on failure, max **3 concurrent workers** per wave.
- **2026-11-22 — Golden-master fixture format:** CSV with `%.17g` double precision (inputs + exact Java outputs) for PA CSV, PRISM raster sample, and DAYMET raster sample, plus `manifest.json` recording `jNSMR` version, JAR filename, and row/cell counts.

---

## Recon & Hazard Notes

- **Reference repo (`~/workspace/jNSMR`):** v0.3.1, BSD-3-Clause, `Depends: R (>= 4.1.0)`, `Imports: utils, parallel, data.table, rJava, terra`, `Suggests: testthat (>= 3.0.0)`. Bundles modified `newhall-1.6.5.jar` (derived from jNSM v1.6.1).
- **Known custom modifications (`NEWS.md` & DeepWiki `https://deepwiki.com/ncss-tech/jNSMR`):**
  - `v1.6.2`: Deprecated JDOM API; added CLI input support.
  - `v1.6.3`: Optimized batching interface with simpler Java data structures.
  - `v1.6.4`: Added `annualPotentialEvapotranspiration` (sum of mean monthly PET) to `NewhallBatchResults`.
  - `v1.6.5` (`0.3.0`): Added spatially explicit O-horizon, Saturation, and `soilAirOffset` inputs; updated Cryic STR criteria.
- **Known sample datasets (`~/workspace/jNSMR/inst/extdata/`):** `All_PA_jNSM_Example_Batch_Metric.csv`, `prism_issr800_sample.tif` (18,790 cells), `daymet_issr800_sample.tif` (18,790 cells).
- **Hazard — Java runtime & `rJava`:** Wave 2/3 tasks that invoke `jNSMR` require a working JRE and installed `jNSMR` + `terra` + `testthat` R packages. Run R/Java tasks serially (never concurrently against shared JVM/temp state).
- **Hazard — Read-only reference repo:** Workers must NEVER modify files in `~/workspace/jNSMR`.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `1.1.1` | Audit Java NSM source & custom modifications | `newhall-rs` (reads `jNSMR/java`) | — | Create: `planning/audit/java_core_audit.md` |
| 1 | `1.1.2` | Audit `jNSMR` R API, factor levels & sample datasets | `newhall-rs` (reads `jNSMR`) | — | Create: `planning/audit/r_api_audit.md` |
| 2 | `1.2.1` | Synthesize `planning/AUDIT_JNSM.md` & ambiguity sign-off list | `newhall-rs` | `1.1.1`, `1.1.2` | Create: `planning/AUDIT_JNSM.md` |
| 2 | `1.2.2` | Verify `jNSMR` oracle baseline & write fixture generator script | `newhall-rs` (reads `jNSMR`) | `1.1.2` | Create: `scripts/generate_golden_fixtures.R` |
| 3 | `1.3.1` | Generate & verify golden-master fixtures | `newhall-rs` | `1.2.2` | Create: `crates/newhall-core/tests/fixtures/pa_batch_metric.csv`, `prism_sample.csv`, `daymet_sample.csv`, `manifest.json` |

---

## Task Specifications

### Task `1.1.1`: Audit Java NSM source & custom modifications
- **Repo:** `/home/andrew/workspace/newhall-rs` (read-only access to `/home/andrew/workspace/jNSMR/java`)
- **Files in scope:**
  - *Can create:* `planning/audit/java_core_audit.md`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* Anything in `/home/andrew/workspace/jNSMR` or any other file in `newhall-rs`.
- **Depends on:** None
- **Spec:**
  1. Inventory all `.java` files in `/home/andrew/workspace/jNSMR/java` with line counts (`wc -l`) and class responsibilities.
  2. Trace and document the exact simulation algorithm:
     - Input parameters, units, and hemisphere detection.
     - Monthly Thornthwaite PET calculation (including any clamping/rounding).
     - 8×8 soil moisture profile recharge/discharge loop and calendar (360-day vs 365-day assumptions).
     - Cumulative/consecutive dry, moist/dry, and moist day counters (>5°C, >8°C, solstice windows).
     - Soil Temperature Regime (STR), Soil Moisture Regime (SMR), and subdivision classification trees.
  3. Document every custom modification relative to standard jNSM v1.6.1 (batching interface, `annualPotentialEvapotranspiration`, O-horizon, saturation, `soilAirOffset`, Cryic STR thresholds, JDOM deprecation).
  4. Explicitly list any rounding quirks, floating-point order-of-operations sensitivities, edge cases (`NA`/`NaN`/zero AWC/extreme latitudes), or suspected bugs in a dedicated `## Ambiguities & Edge Cases` section.
- **Grep anchors:** `~/workspace/jNSMR/java` (`BASICSimulationModel`, `NewhallBatchResults`, `soilAirOffset`, `Cryic`, `annualPotentialEvapotranspiration`, `O_horizon` / `oHorizon`, `saturation`).
- **Acceptance & targeted verification:**
  - `test -s planning/audit/java_core_audit.md`
  - Verify all `.java` files in `~/workspace/jNSMR/java` are listed and `## Ambiguities & Edge Cases` is present (`grep -q "## Ambiguities" planning/audit/java_core_audit.md`).

---

### Task `1.1.2`: Audit `jNSMR` R API, factor levels & sample datasets
- **Repo:** `/home/andrew/workspace/newhall-rs` (read-only access to `/home/andrew/workspace/jNSMR`)
- **Files in scope:**
  - *Can create:* `planning/audit/r_api_audit.md`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* Anything in `/home/andrew/workspace/jNSMR` or any other file in `newhall-rs`.
- **Depends on:** None
- **Spec:**
  1. Inventory all files in `/home/andrew/workspace/jNSMR/R/`, `tests/`, and `inst/extdata/` with line counts / file sizes.
  2. Document the `newhall_batch()` S3 dispatch (`SpatRaster`, `RasterBrick`, `RasterStack`, `character`, `default`/`data.frame`), exact arguments, defaults, and return column/layer names and types.
  3. Extract the exact categorical factor levels and integer codes (if any) used by `terra::set.cats()` for `temperatureRegime`, `moistureRegime`, `regimeSubdivision1`, and `regimeSubdivision2`.
  4. Document `NA`/`NaN` handling and chunking behavior (`readStart`/`readValues`/`writeValues`).
  5. Document all sample datasets in `inst/extdata/` (`All_PA_jNSM_Example_Batch_Metric.csv`, `prism_issr800_sample.tif`, `daymet_issr800_sample.tif`, and any others), including column/layer names and how `jNSMR` tests invoke them.
- **Grep anchors:** `~/workspace/jNSMR/NAMESPACE`, `~/workspace/jNSMR/R/` (`newhall_batch`, `set.cats`, `readValues`, `soilAirOffset`), `~/workspace/jNSMR/tests/testthat/`.
- **Acceptance & targeted verification:**
  - `test -s planning/audit/r_api_audit.md`
  - `grep -q "newhall_batch" planning/audit/r_api_audit.md && grep -q "All_PA_jNSM_Example_Batch_Metric.csv" planning/audit/r_api_audit.md`

---

### Task `1.2.1`: Synthesize `planning/AUDIT_JNSM.md` & ambiguity sign-off list
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `planning/AUDIT_JNSM.md`
  - *Do not touch:* `planning/audit/java_core_audit.md`, `planning/audit/r_api_audit.md`, or anything in `~/workspace/jNSMR`.
- **Depends on:** `1.1.1`, `1.1.2`
- **Spec:**
  1. Merge `planning/audit/java_core_audit.md` and `planning/audit/r_api_audit.md` into a single, concise reference document `planning/AUDIT_JNSM.md` suitable as the parity specification for `G3`/`G4`/`G7`.
  2. Include:
     - Executive summary of custom v1.6.1 → v1.6.5 modifications.
     - Exact input/output schema table (Java field ↔ R column/layer ↔ target Rust type).
     - Exact enum discriminant table for SMR, STR, and subdivisions.
     - Numbered **Sign-Off Checklist** of any ambiguities, edge cases, or bugs found in Java/R that require user confirmation before Rust implementation.
- **Grep anchors:** `planning/audit/java_core_audit.md`, `planning/audit/r_api_audit.md`.
- **Acceptance & targeted verification:**
  - `test -s planning/AUDIT_JNSM.md`
  - `grep -q "Sign-Off Checklist" planning/AUDIT_JNSM.md`

---

### Task `1.2.2`: Verify `jNSMR` oracle baseline & write fixture generator script
- **Repo:** `/home/andrew/workspace/newhall-rs` (read-only access to `/home/andrew/workspace/jNSMR`)
- **Files in scope:**
  - *Can create:* `scripts/generate_golden_fixtures.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* Anything in `/home/andrew/workspace/jNSMR`.
- **Depends on:** `1.1.2`
- **Spec:**
  1. Verify `jNSMR` is loadable and its existing `testthat` suite passes (record exact command + pass/fail counts in `STATE_1.md`; if `jNSMR` or Java is missing, stop and report).
  2. Write `scripts/generate_golden_fixtures.R` that:
     - Runs `jNSMR::newhall_batch()` on:
       1. `All_PA_jNSM_Example_Batch_Metric.csv` → `crates/newhall-core/tests/fixtures/pa_batch_metric.csv`
       2. `prism_issr800_sample.tif` → `crates/newhall-core/tests/fixtures/prism_sample.csv`
       3. `daymet_issr800_sample.tif` → `crates/newhall-core/tests/fixtures/daymet_sample.csv`
     - Joins inputs + exact Java outputs per row/cell (preserving `NA` cells or documenting valid-cell filtering explicitly) and formats all floating-point columns with `sprintf("%.17g", x)`.
     - Writes `crates/newhall-core/tests/fixtures/manifest.json` with `jNSMR` version, JAR filename (`newhall_version()`), generation timestamp, and row/cell counts per fixture.
- **Grep anchors:** `planning/audit/r_api_audit.md`, `~/workspace/jNSMR/README.md` (`newhall_batch`, `prism_issr800_sample.tif`, `daymet_issr800_sample.tif`).
- **Acceptance & targeted verification:**
  - `Rscript -e 'parse("scripts/generate_golden_fixtures.R")'` exits 0.
  - `Rscript -e 'testthat::test_local("~/workspace/jNSMR")'` (or `R CMD check`) baseline recorded.

---

### Task `1.3.1`: Generate & verify golden-master fixtures
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:*
    - `crates/newhall-core/tests/fixtures/pa_batch_metric.csv`
    - `crates/newhall-core/tests/fixtures/prism_sample.csv`
    - `crates/newhall-core/tests/fixtures/daymet_sample.csv`
    - `crates/newhall-core/tests/fixtures/manifest.json`
  - *Do not touch:* `scripts/generate_golden_fixtures.R` (if a bug is found, stop and report), or anything in `~/workspace/jNSMR`.
- **Depends on:** `1.2.2`
- **Spec:**
  1. Run `Rscript scripts/generate_golden_fixtures.R` from `/home/andrew/workspace/newhall-rs`.
  2. Verify all 3 CSV fixtures and `manifest.json` exist, are non-empty, have matching row counts against `manifest.json`, and contain full-precision floats and expected categorical columns (`temperatureRegime`, `moistureRegime`, `regimeSubdivision1`, `regimeSubdivision2`).
  3. Record exact row counts, runtime (baseline `jNSMR` timing for future speedup comparison), and any `NA` cell counts in `STATE_1.md`.
- **Grep anchors:** `scripts/generate_golden_fixtures.R`.
- **Acceptance & targeted verification:**
  - `Rscript scripts/generate_golden_fixtures.R` exits 0.
  - `test -s crates/newhall-core/tests/fixtures/pa_batch_metric.csv && test -s crates/newhall-core/tests/fixtures/prism_sample.csv && test -s crates/newhall-core/tests/fixtures/daymet_sample.csv && test -s crates/newhall-core/tests/fixtures/manifest.json`
