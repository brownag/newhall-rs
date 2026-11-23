# Execution Plan 3: Strict-Mode Source Defect Remediation (`G18`)

## Goal & Scope

### Goal
Implement opt-in, defect-free `StrictThornthwaiteForcing`, `StrictThermal`, and `StrictNewhall` static-dispatch models in `newhall-core`—remediating all 10 applicable audited `jNSMR` source defects (`#1`–`#10`) while leaving `ClassicNewhall` 100% untouched for golden-master parity—and expose `strict` / `strict_forcing` options through `r-pkg` (`newhall_batch()` and `newhall_batch_phase4()`).

### In Scope
- **Wave 1 (`newhall-core` Physics Remediation):**
  - `crates/newhall-core/src/forcing.rs`: `StrictThornthwaiteForcing` (`ForcingModel`, `Copy`, `Default`) fixing Defect #1 (SH PET `* 60.0` parentheses), Defect #2 (`is_metric = false` °F→°C and in→mm conversion), Defect #7 (`lat_dd.abs()` in SH lookup), and Defect #9 (`NaN` propagation).
  - `crates/newhall-core/src/thermal.rs`: `StrictThermal` (`ThermalModel`, `Copy`, `Default`) fixing Defect #2 (`is_metric = false` unit conversion), Defect #3 (`Pergelic` not overwritten by `Frigid`/`Isofrigid`), Defect #4 (attenuated `dif = (st - wt) * fcd` with `.abs()`), Defect #5 (cold-site `ncpm2 = 0` when all months `< 8°C`), Defect #6 (accumulate `nsd` across all `>5°C` intervals, not just the first), Defect #8 (canonical `regime_subdivision1` = group `div`, `regime_subdivision2` = modifier `q`), Defect #9 (`NaN` in `precip_mm` yields `MoistureRegime::Undefined`, not `Perudic`), and Defect #10 (exact `5.0`/`8.0` threshold handling without `5.01`/`8.01` perturbation).
- **Wave 2 (`newhall-core` Engine & Exports):**
  - `crates/newhall-core/src/engine.rs` & `crates/newhall-core/src/lib.rs`: Wire `StrictNewhall` (`SimEngine<StrictThermal, StrictThornthwaiteForcing, ClassicHydrology>`), metric normalization helper for `StrictNewhall` (`Defect #2` for `awc_mm` and `precip_mm` passed to `ClassicHydrology`), and public re-exports.
- **Wave 3 (R Bindings & Integration Verification):**
  - `r-pkg/`: Add `strict = FALSE` to `newhall_batch()` and `strict_forcing = TRUE` to `newhall_batch_phase4()` (with `SimEngine` dispatch in `r-pkg/src/rust/src/lib.rs`, R wrappers, docs, and `testthat` tests).
  - `crates/newhall-core/tests/strict_mode.rs`: Integration tests verifying remediation of Defects #1–#10 and non-regression of `ClassicNewhall`.

### Out of Scope (Explicitly Deferred)
- Modifying `ClassicNewhall`, `ThornthwaiteForcing`, `ClassicHydrology`, or `ClassicThermal` behavior (must remain 100% bit-identical to `jNSMR` `v1.6.5`).
- Legacy `runBatch()` / `NewhallDataset` (`v1.6.1`/`v1.6.3`) compatibility (Defect #11 — superseded by `v1.6.5` / `StrictNewhall`).
- Modifying `crates/newhall-core/tests/golden_master.rs`, `crates/newhall-core/tests/adversarial_phase4.rs`, or `crates/newhall-core/tests/fixtures/`.
- `terra::SpatRaster` chunked R API (`G7`, `G8`) and `ARCHITECTURE.md` (`G10`).

### Verbatim Constraints
- "All Phase 1–3 golden-master parity tests (`jNSMR` / Java baseline) MUST remain 100% green and unchanged. Legacy behavior is the default; Phase 4 physics are opt-in trait implementations."
- "Zero heap allocations in the per-site hot loop... All new state structs MUST be `Copy` and stack-allocated."
- "Static dispatch only... No `dyn Trait` inside the pixel loop."
- "Keep `newhall-core` free of R/`extendr` dependencies."

---

## Decision Log

- **2026-11-22 — API Shape (`Option A`):** Expose defect-free physics as dedicated zero-sized `Copy` trait implementations (`StrictThornthwaiteForcing`, `StrictThermal`, `StrictNewhall`) rather than branching on `SiteConfig.strict_mode`, preserving zero overhead in `ClassicNewhall` and direct composition with `Thermal1DModel`.
- **2026-11-22 — Scope (`Option A`):** Include `r-pkg` exposure (`newhall_batch(..., strict = FALSE)` and `newhall_batch_phase4(..., strict_forcing = TRUE)`) and remediate all applicable core defects (`#1`–`#10`), including Defect #2 (`is_metric = false` °F/in conversion) and Defect #8 (canonical `div`/`q` order).
- **2026-11-22 — Execution & Escalation (`Option A`):** Use `flash` workers, max **3 concurrent workers** per wave with worktree isolation for parallel waves, escalating to `pro`/frontier after 2 failed attempts.
- **2026-11-22 — Tracking & Acceptance (`Option A`):** Track `planning/*_3.md` and `planning/tasks/3.*.todo.md` in Git via separate `docs(planning):` commits; require `cargo test --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `Rscript -e "devtools::test('r-pkg')"` green.

---

## Recon & Hazard Notes

- **Baseline Green Suites:** `29` unit tests + `13` adversarial/proptest tests + `4` golden-master tests (`cargo test --workspace --all-targets`), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `15` R `testthat` assertions (`devtools::test('r-pkg')`).
- **Defect Catalog:** `planning/GOALS.md` (`## Source Defects`, items 1–11) and `planning/AUDIT_JNSM.md`.
- **File Sizes & Anchors:**
  - `crates/newhall-core/src/forcing.rs` (435 lines): `compute_with_hemisphere` (L75), `apply_daylength_correction` (L143).
  - `crates/newhall-core/src/thermal.rs` (1,010 lines): `evaluate` (L15), `classify_str` (L144), `find_threshold_intervals` (L219), `classify_smr` (L367).
  - `crates/newhall-core/src/engine.rs` (466 lines), `crates/newhall-core/src/lib.rs` (24 lines).
  - `r-pkg/src/rust/src/lib.rs` (288 lines).
- **Hazard — `StrictNewhall` with `is_metric = false`:** `ClassicHydrology` ignores `config.is_metric` (`Defect #2`). In `StrictNewhall`, `SimEngine<StrictThermal, StrictThornthwaiteForcing, ClassicHydrology>` does not automatically convert `awc_mm` and `precip_mm` before `ClassicHydrology` unless `StrictNewhall` wraps or provides a `StrictNewhall::run(&input, &config)` / `StrictHydrology` helper. Implementing `StrictHydrology` (a zero-sized `Copy` wrapper in `engine.rs` or `hydrology.rs` that converts `precip_mm` and `awc_mm` from inches to mm when `!config.is_metric` and delegates to `ClassicHydrology`) ensures `SimEngine::run` and `run_batch_soa` both handle `is_metric = false` seamlessly.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Repo | Depends On | Files in Scope (Create/Edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | `3.1.1` | Implement `StrictThornthwaiteForcing` (`#1`, `#2`, `#7`, `#9`) | `newhall-rs` | — | Edit: `crates/newhall-core/src/forcing.rs` |
| 1 | `3.1.2` | Implement `StrictThermal` (`#2`, `#3`, `#4`, `#5`, `#6`, `#8`, `#9`, `#10`) | `newhall-rs` | — | Edit: `crates/newhall-core/src/thermal.rs` |
| 2 | `3.2.1` | Wire `StrictHydrology`, `StrictNewhall`, and `lib.rs` exports | `newhall-rs` | `3.1.1`, `3.1.2` | Edit: `crates/newhall-core/src/engine.rs`, `crates/newhall-core/src/lib.rs` |
| 3 | `3.3.1` | Expose `strict` and `strict_forcing` in `r-pkg` + `testthat` | `newhall-rs` | `3.2.1` | Edit: `r-pkg/src/rust/src/lib.rs`, `r-pkg/R/newhall.R`, `r-pkg/R/extendr-wrappers.R`, `r-pkg/man/newhall_batch.Rd`, `r-pkg/man/newhall_batch_phase4.Rd`, `r-pkg/tests/testthat/test-phase4.R` |
| 3 | `3.3.2` | Add `strict_mode.rs` integration tests verifying Defects #1–#10 | `newhall-rs` | `3.2.1` | Create: `crates/newhall-core/tests/strict_mode.rs` |

---

## Task Specifications

### Task `3.1.1`: Implement `StrictThornthwaiteForcing` (`#1`, `#2`, `#7`, `#9`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/forcing.rs`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* Any other file in `newhall-rs` (especially `ThornthwaiteForcing` behavior).
- **Depends on:** None
- **Spec:**
  1. Add `#[derive(Debug, Clone, Copy, Default)] pub struct StrictThornthwaiteForcing;` implementing `ForcingModel`.
  2. Fix **Defect #2**: when `!config.is_metric`, convert `temp_c` from °F to °C (`(t - 32.0) * 5.0 / 9.0`) and `precip_mm` from inches to mm (`p * 25.4`) on local stack copies.
  3. Fix **Defect #7**: use `input.lat_dd.abs()` when indexing `rs` / `fs` tables for Southern Hemisphere (`lat_dd < 0.0`).
  4. Fix **Defect #1**: parenthesize `(rs[nrow] - rs[nrow - 1]) * 60.0` in the `5.0 <= lat < 50.0` SH interpolation branch so `mpe` is positive and monotonic.
  5. Fix **Defect #9**: if any `temp_c` or `precip_mm` is `NaN`, return `NaN` balances cleanly without panicking.
  6. Add unit tests inside `forcing.rs` verifying `StrictThornthwaiteForcing` fixes #1, #2, #7, and #9 while `ThornthwaiteForcing` tests remain unchanged.
- **Grep anchors:** `crates/newhall-core/src/forcing.rs` (`compute_with_hemisphere` L75, `apply_daylength_correction` L143), `planning/GOALS.md` (`## Source Defects`).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --lib forcing` exits 0.

---

### Task `3.1.2`: Implement `StrictThermal` (`#2`, `#3`, `#4`, `#5`, `#6`, `#8`, `#9`, `#10`)
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/thermal.rs`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* Any other file in `newhall-rs` (especially `ClassicThermal` behavior).
- **Depends on:** None
- **Spec:**
  1. Add `#[derive(Debug, Clone, Copy, Default)] pub struct StrictThermal;` implementing `ThermalModel`.
  2. Fix **Defect #2**: when `!config.is_metric`, convert `temp_c` (°F→°C), `precip_mm` (in→mm), and `soil_air_offset` (`* 5.0 / 9.0`) on local stack copies.
  3. Fix **Defect #3**: in strict STR classification, `tma < 0.0` always yields `TemperatureRegime::Pergelic` (never overwritten by `Frigid` / `Isofrigid`).
  4. Fix **Defect #4**: use attenuated `dif = (st - wt) * config.amplitude` (with `.abs()` for magnitude checks where appropriate) in SMR branches.
  5. Fix **Defect #5**: when all 12 months are `< 8.0 °C` (`sn == 0`), `ncpm2` stays `0` instead of falling back to `ncpm1`.
  6. Fix **Defect #6**: accumulate `nsd` across *all* `>5°C` intervals (remove `has_run_already` guard).
  7. Fix **Defect #8**: return canonical order (`regime_subdivision1` = group `div` mapped cleanly or documented, wait — check `RegimeSubdivision1` vs `RegimeSubdivision2` enum types! Since `ThermalClassification` has typed fields `regime_subdivision1: RegimeSubdivision1` and `regime_subdivision2: RegimeSubdivision2`, keep the typed enums accurate: document/preserve type safety without breaking `RegimeSubdivision1`/`2` types; fix `NaN` in `precip_mm` -> `MoistureRegime::Undefined` per **Defect #9**).
  8. Fix **Defect #10**: handle exact `5.0`/`8.0` plateaus cleanly without mutating to `5.01`/`8.01`.
  9. Add unit tests inside `thermal.rs` verifying `StrictThermal` fixes #2, #3, #4, #5, #6, #9, #10 while `ClassicThermal` tests remain unchanged.
- **Grep anchors:** `crates/newhall-core/src/thermal.rs` (`evaluate` L15, `classify_str` L144, `find_threshold_intervals` L219, `classify_smr` L367), `crates/newhall-core/src/types.rs` (`RegimeSubdivision1`, `RegimeSubdivision2`).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --lib thermal` exits 0.

---

### Task `3.2.1`: Wire `StrictHydrology`, `StrictNewhall`, and `lib.rs` exports
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `crates/newhall-core/src/engine.rs`, `crates/newhall-core/src/lib.rs`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `forcing.rs`, `thermal.rs`, `hydrology.rs`, `types.rs`, `ClassicNewhall`.
- **Depends on:** `3.1.1`, `3.1.2`
- **Spec:**
  1. Add `#[derive(Debug, Clone, Copy, Default)] pub struct StrictHydrology;` in `engine.rs` implementing `HydrologyStep`: when `!config.is_metric`, convert `input.precip_mm` and `input.awc_mm` from inches to mm (`* 25.4`) on a stack copy of `SiteInput`, then delegate to `ClassicHydrology::default().simulate_hydrology(...)`.
  2. Define `pub type StrictNewhall = SimEngine<StrictThermal, StrictThornthwaiteForcing, StrictHydrology>;` in `engine.rs`.
  3. Re-export `StrictHydrology`, `StrictNewhall`, `StrictThornthwaiteForcing`, and `StrictThermal` in `crates/newhall-core/src/lib.rs`.
  4. Add unit tests in `engine.rs` verifying `StrictNewhall` and `is_metric = false` unit conversion.
- **Grep anchors:** `crates/newhall-core/src/engine.rs` (L1–L30, L95–L140), `crates/newhall-core/src/lib.rs` (L1–L24).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --lib` exits 0.

---

### Task `3.3.1`: Expose `strict` and `strict_forcing` in `r-pkg` + `testthat`
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Must edit:* `r-pkg/src/rust/src/lib.rs`, `r-pkg/R/newhall.R`, `r-pkg/R/extendr-wrappers.R`, `r-pkg/man/newhall_batch.Rd`, `r-pkg/man/newhall_batch_phase4.Rd`, `r-pkg/tests/testthat/test-phase4.R`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/`.
- **Depends on:** `3.2.1`
- **Spec:**
  1. In `r-pkg/src/rust/src/lib.rs`, add `strict: bool` to `newhall_batch_rust` (dispatching `StrictNewhall::default()` when `true`, `ClassicNewhall::default()` when `false`) and `strict_forcing: bool` to `newhall_batch_phase4_rust` (dispatching `StrictThornthwaiteForcing` when `true`, `ThornthwaiteForcing` when `false`).
  2. Update `r-pkg/R/newhall.R` (`newhall_batch(..., strict = FALSE)`, `newhall_batch_phase4(..., strict_forcing = TRUE)`), `r-pkg/R/extendr-wrappers.R`, and `.Rd` man pages.
  3. Add `testthat` tests in `r-pkg/tests/testthat/test-phase4.R` covering `strict = TRUE` (SH PET, `is_metric = FALSE`) and `strict_forcing = TRUE` in Phase 4.
- **Grep anchors:** `r-pkg/src/rust/src/lib.rs` (`newhall_batch_rust`, `newhall_batch_phase4_rust`), `r-pkg/R/newhall.R`, `r-pkg/tests/testthat/test-phase4.R`.
- **Acceptance & targeted verification:**
  - `rm -f r-pkg/src/newhallrs.so && Rscript -e "devtools::test('r-pkg')"` exits 0 with 0 failures.

---

### Task `3.3.2`: Add `strict_mode.rs` integration tests verifying Defects #1–#10
- **Repo:** `/home/andrew/workspace/newhall-rs`
- **Files in scope:**
  - *Can create:* `crates/newhall-core/tests/strict_mode.rs`
  - *Temp/scratch:* `.development/` (untracked)
  - *Do not touch:* `crates/newhall-core/src/`, `golden_master.rs`, `adversarial_phase4.rs`.
- **Depends on:** `3.2.1`
- **Spec:**
  1. Create `crates/newhall-core/tests/strict_mode.rs` with explicit regression tests comparing `ClassicNewhall` vs. `StrictNewhall` and `SimEngine<Thermal1DModel, StrictThornthwaiteForcing, WaterTableHydrology>` across Defects #1, #2, #3, #4, #5, #6, #7, #9, and #10.
  2. Verify `StrictNewhall` handles `NaN` precipitation as `MoistureRegime::Undefined` (not `Perudic`), negative SH latitudes without huge/negative `mpe`, `Pergelic` without `Frigid` overwrite, and `is_metric = false` equivalence to metric inputs.
- **Grep anchors:** `crates/newhall-core/src/lib.rs`, `planning/GOALS.md` (`## Source Defects`).
- **Acceptance & targeted verification:**
  - `cargo test -p newhall-core --test strict_mode` exits 0.
