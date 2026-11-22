# `jNSMR` & Custom Java NSM (`v1.6.5`) Parity Specification

Synthesized from [java_core_audit.md](file:///home/andrew/workspace/newhall-rs/planning/audit/java_core_audit.md) and [r_api_audit.md](file:///home/andrew/workspace/newhall-rs/planning/audit/r_api_audit.md).
Reference target: `/home/andrew/workspace/jNSMR` (`v0.3.1`) bundling `newhall-1.6.5.jar` (`org.psu.newhall.sim.BASICSimulationModel.runBatch2`, derived from upstream jNSM `v1.6.1`).
Serves as the authoritative parity contract for `newhall-core` (`G3`/`G4`) and `extendr`/R bindings (`G7`).

---

## 1. Executive Summary: Custom `v1.6.1` → `v1.6.5` Modifications

Relative to upstream jNSM `v1.6.1`, the bundled `newhall-1.6.5.jar` introduces the following modifications (tracked across `v1.6.2` → `v1.6.5`):

1. **JDOM / XML / Swing GUI Removal (`v1.6.2`):** Removed `org.jdom.*`, `org.psu.newhall.ui.*`, `org.psu.newhall.flerry.*`, and XML importer/exporter classes; fixed decompiler-artifact infinite loops (`timesi > cc.length` guards) in `BASICSimulationModel.java` and fixed `NewhallResults.toString()` monthly PET formatting.
2. **CLI Entry Point (`v1.6.2`):** Replaced Swing GUI launch in `Newhall.main()` with `<path-to-CSV-file> <smcsawc> <soilAirOffset> <amplitude>`.
3. **Zero-Boxing Batch Interface (`v1.6.3`):** Added `BASICSimulationModel.runBatch(...)` and `runBatch2(...)` returning `NewhallBatchResults` (flat `public` primitive/`String` arrays for `rJava`), plus a second `runSimulation(double[] temperature, double[] precip, ...)` overload bypassing `NewhallDataset` / `ArrayList<Double>` boxing. Made `computeWaterBalance` `public`.
4. **Annual PET & Calendar Matrices (`v1.6.4`):** Added `annualPotentialEvapotranspiration` (`sum` of 12 monthly `mpe` values) to `NewhallBatchResults`, and populated `temperatureCalendar` and `moistureCalendar` (`int[n][360]`, previously stubbed as zeros in `v1.6.3`).
5. **Cryic / Frigid / Iso-Threshold Revisions (`v1.6.4` → `v1.6.5`):** In `runSimulation(double[], ...)` **only**:
   - Removed the `cs = dif * (1 - fcd) / 2` attenuation adjustment from `st` and `wt` (leaving `st = at_summer`, `wt = at_winter`, and `dif = st - wt`).
   - Changed iso-regime criterion `cr[7]` from `(dif * fcd) > 5.0` to `(dif * fcd) >= 6.0`.
   - Changed Cryic summer-temp criterion `cr[3]` from `(st - cs) < 15.0` to `st >= 0.0 && st < cryic_ht`.
6. **Spatially Explicit Soil Parameters (`v1.6.5` / `jNSMR` `0.3.0`):** Added per-row/per-cell `soilAirOffset` (`fc`), `amplitude` (`fcd`), `hasOHorizon`, and `isSaturated` arrays to `runBatch2` and `runSimulation(double[], ...)`, setting `cryic_ht` dynamically (`15`, `13`, `8`, or `6` °C).

> [!IMPORTANT]
> `runSimulation(NewhallDataset, ...)` (used by `Newhall.main()` and legacy `runBatch()`) was **not** updated to the `v1.6.4`/`v1.6.5` thermal regime rules or O-horizon/saturation inputs. `jNSMR::newhall_batch()` (`batch3()`) exclusively invokes `runBatch2()` → `runSimulation(double[], ...)`.

---

## 2. Exact Input & Output Schema

### 2.1 Inputs (`batch3()` → `BASICSimulationModel.runBatch2()` → `newhall-core`)

`R/NewhallBatch.R` requires 28 mandatory columns/layers (`.colnamesNewhallBatch()`), plus 4 optional columns/layers (broadcast from scalar function arguments if absent).

| Java `runBatch2` / `runSimulation` Arg | Java Type | R Column / Layer Name | R Default / Extraction | Target Rust Type (`newhall-core`) | Notes & Units |
| --- | --- | --- | --- | --- | --- |
| `precip[i][0..12]` | `double[][]` | `pJan`..`pDec` | Regex `^p[A-Z][a-z]{2}$` + `cbind(0.0, ...)` | `[f64; 12]` | mm (or in; see Sign-Off #2). Java index `0` unused (`0.0`); `1..12` = Jan..Dec; column order in R matters. |
| `temperature[i][0..12]` | `double[][]` | `tJan`..`tDec` | Regex `^t[A-Z][a-z]{2}$` + `cbind(0.0, ...)` | `[f64; 12]` | °C (or °F; see Sign-Off #2). Java index `0` unused (`0.0`); `1..12` = Jan..Dec; column order in R matters. |
| `latitude[i]` (`latDD`) | `double[]` | `latDD` | Required (`as.double`) | `f64` | Decimal degrees. Java expects `>= 0` when `northsouth == 'N'`; see Sign-Off #7 for negative `latDD` in R/Java. |
| `longitude[i]` (`lonDD`) | `double[]` | `lonDD` | Required (`as.double`) | `f64` (metadata/unused) | Decimal degrees; **unused** inside `runSimulation(double[], ...)`. |
| `northsouth[i]` | `char[]` | *(derived from `latDD`)* | `'N'` if `latDD >= 0`, `'S'` if `latDD < 0` | `Hemisphere` (`North` \| `South`) | Derived in `batch3()` (note `batch2()` hardcoded `'N'`). |
| `elevation[i]` | `double[]` | `elev` | Required (`as.double`) | `f64` (unused) | Multiplied by `0.3048` in Java if `!isMetric`, then **unused**. |
| `isMetric[i]` | `boolean[]` | *(arg `unitSystem`)* | `unitSystem == "cm"` (`"metric"`/`"mm"`/`"cm"` → `true`; `"in"`/`"english"` → `false`) | `bool` / unit enum | **Ignored** in `runSimulation(double[], ...)` except for unused `elevation`! See Sign-Off #2. |
| `waterHoldingCapacity[i]` | `double[]` | `awc` | Required (`as.double`) | `f64` | Available water capacity (mm; standard `200.0`). In `SpatRaster`, **only** `is.na(awc)` skips cells. |
| `soilAirOffset[i]` (`fc`) | `double[]` | `soilAirOffset` *(opt.)* | Scalar arg `2.5` (`4.5` if `"in"`/`"english"`) | `f64` | Soil-air temperature offset (°C; default `2.5`). |
| `amplitude[i]` (`fcd`) | `double[]` | `amplitude` *(opt.)* | Scalar arg `0.66` | `f64` | Soil/air temperature amplitude ratio (default `0.66`). |
| `hasOHorizon[i]` | `boolean[]` | `hasOHorizon` *(opt.)* | Scalar arg `FALSE` | `bool` | Presence of organic O-horizon (`false`). |
| `isSaturated[i]` | `boolean[]` | `isSaturated` *(opt.)* | Scalar arg `FALSE` | `bool` | Saturation at 50 cm depth (`false`). |

### 2.2 Outputs (`NewhallBatchResults` → `batch3()` / `SpatRaster` → `newhall-core`)

`batch3()` and `newhall_batch.SpatRaster()` extract the 19 fields below (`fields` / `cnm`). Three additional fields (`fieldsmatrix`) exist on `NewhallBatchResults` in Java and are included here for `newhall-core` completeness.

| Idx | Java `NewhallBatchResults` Field | Java Type | R `data.frame` Type (`batch3`) | `SpatRaster` Encoding | Target Rust Type (`newhall-core`) |
| ---: | --- | --- | --- | --- | --- |
| 1 | `annualRainfall` | `double[]` | `numeric` (`double`) | `double` | `f64` |
| 2 | `waterHoldingCapacity` | `double[]` | `numeric` (`double`) | `double` | `f64` |
| 3 | `annualWaterBalance` | `double[]` | `numeric` (`double`) | `double` | `f64` |
| 4 | `annualPotentialEvapotranspiration` | `double[]` | `numeric` (`double`) | `double` | `f64` |
| 5 | `summerWaterBalance` | `double[]` | `numeric` (`double`) | `double` | `f64` |
| 6 | `dryDaysAfterSummerSolstice` | `int[]` | `integer` | `numeric` (via `as.matrix`) | `i32` / `u16` |
| 7 | `moistDaysAfterWinterSolstice` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 8 | `numCumulativeDaysDry` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 9 | `numCumulativeDaysMoistDry` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 10 | `numCumulativeDaysMoist` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 11 | `numCumulativeDaysDryOver5C` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 12 | `numCumulativeDaysMoistDryOver5C` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 13 | `numCumulativeDaysMoistOver5C` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 14 | `numConsecutiveDaysMoistInSomeParts` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 15 | `numConsecutiveDaysMoistInSomePartsOver8C` | `int[]` | `integer` | `numeric` | `i32` / `u16` |
| 16 | `temperatureRegime` | `String[]` | `character` | Categorical (`0..9`) | `TemperatureRegime` (`Option<...>` / `Unknown` if `""`) |
| 17 | `moistureRegime` | `String[]` | `character` | Categorical (`0..5`) | `MoistureRegime` |
| 18 | `regimeSubdivision1` | `String[]` | `character` | Categorical (`0..8`) | `RegimeSubdivision1` (holds `q`) |
| 19 | `regimeSubdivision2` | `String[]` | `character` | Categorical (`0..7`) | `RegimeSubdivision2` (holds `div`) |
| — | `meanPotentialEvapotranspiration` | `double[][]` | *(not in `batch3` df)* | *(not in `SpatRaster`)* | `[f64; 12]` |
| — | `temperatureCalendar` | `int[][]` | *(not in `batch3` df)* | *(not in `SpatRaster`)* | `[i8; 360]` (`-1` = `<5°C`, `5` = `5..8°C`, `8` = `>8°C`) |
| — | `moistureCalendar` | `int[][]` | *(not in `batch3` df)* | *(not in `SpatRaster`)* | `[u8; 360]` (`1` = Dry, `2` = Moist/Dry, `3` = Moist) |

---

## 3. Exact Enum Discriminant & Categorical Factor Tables

In `SpatRaster` outputs (`R/NewhallBatch.R:641-664`), `.setCats()` converts string results to **0-indexed** numeric codes (`as.numeric(factor(...)) - 1`) and attaches `terra::set.cats()` tables (`value = 0..k-1`, column `category`). Before factor encoding, `.setCats()` maps `" "` (single space returned by Java for Perudic/Undefined subdivisions) to `"Undefined"` via `gsub(" ", "Undefined", ...)`.

| Field / Layer | `0` | `1` | `2` | `3` | `4` | `5` | `6` | `7` | `8` | `9` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `temperatureRegime` (`.str()`) | `Pergelic` | `Cryic` | `Frigid` | `Mesic` | `Thermic` | `Hyperthermic` | `Isofrigid` | `Isomesic` | `Isothermic` | `Isohyperthermic` |
| `moistureRegime` (`.smr()`) | `Aridic` | `Ustic` | `Xeric` | `Udic` | `Perudic` | `Undefined` | — | — | — | — |
| `regimeSubdivision1` (`.smrsub1()`) | `Typic` | `Weak` | `Wet` | `Dry` | `Extreme` | `Xeric` | `Udic` | `Aridic` | `Undefined` | — |
| `regimeSubdivision2` (`.smrsub2()`) | `Aridic` | `Tempustic` | `Tropustic` | `Tempudic` | `Xeric` | `Udic` | `Tropudic` | `Undefined` | — | — |

**Java `q` vs `div` Parameter Swap:**
`NewhallResults` constructor declares `(..., String div, String q, ...)` assigning `regimeSubdivision1 = div` and `regimeSubdivision2 = q`, **but both `runSimulation` methods pass `(..., q, div, ...)`**. Therefore, in Java `NewhallBatchResults` and R `jNSMR`:
- `regimeSubdivision1` contains `q` (`Typic`, `Weak`, `Wet`, `Dry`, `Extreme`, `Xeric`, `Udic`, `Aridic`, `" "`/`Undefined`).
- `regimeSubdivision2` contains `div` (`Aridic`, `Tempustic`, `Tropustic`, `Tempudic`, `Xeric`, `Udic`, `Tropudic`, `Undefined`, `" "`/`Undefined`).

**Java `temperatureCalendar` Encoding:**
Java populates `char` `'-'` (`<5°C`), `'5'` (`5..8°C`), `'8'` (`>8°C`), and `NewhallBatchResults` converts via `Character.getNumericValue()`, producing **`-1` (`'-'`), `5` (`'5'`), `8` (`'8'`)**.

---

## 4. Simulation Algorithm & Parity Rules (`runSimulation(double[], ...)`)

All arithmetic is unrounded IEEE-754 `f64` (`double`), with summation strictly in `1..12` order (`0..12` for `swi`) and truncating `(int)` casts where noted.

### 4.1 Thornthwaite PET & Water Balances
1. **Monthly heat index (`j = 1..12`):** `mwi[j] = (T[j] > 0.0) ? (T[j] / 5.0)^1.514 : 0.0`; `swi = sum(mwi[1..12])`.
2. **Thornthwaite exponent:** `a = 6.75e-7 * swi^3 - 7.71e-5 * swi^2 + 0.01792 * swi + 0.49239`.
3. **Unadjusted PET `upe[i]` (`i = 1..12`):**
   - `T[i] <= 0.0` → `0.0`
   - `0.0 < T[i] < 26.5` → `16.0 * (10.0 * T[i] / swi)^a`
   - `26.5 <= T[i] < 38.0` → step lookup `zpe[ki - 1]` for `zt[ki - 1] <= T[i] < zt[ki]` (`26.5..38.0` by `0.5°C`; no interpolation)
   - `T[i] >= 38.0` → `185.0`
4. **Latitude / daylength correction `mpe[i]` (only when `upe[i] > 0.0`):**
   - **Northern (`'N'`):** `nrow` = count of `rn[0..30]` (`0, 5, 10, 15, 20, 25, 26..50`) with `latDD >= rn[idx]`; `mpe[i] = upe[i] * inz[i - 1][nrow - 1]` (floor bin, clamped at `50°N`; `latDD < 0` throws `ArrayIndexOutOfBoundsException`).
   - **Southern (`!= 'N'`):** `nrow` = count of `rs[0..12]` (`5, 10..40, 42..50`) with `latDD >= rs[idx]`. Clamps `>= 50°S`; for `0 <= latDD < 5`, linearly interpolates `inz[i - 1][0]`..`fs[i - 1][0]`; for `5 <= latDD < 50`, contains the `* 60.0D` precedence bug (Sign-Off #1).
5. **Balances:** `annualRainfall = sum(P[1..12])`, `annualPotentialEvapotranspiration = sum(mpe[1..12])`, `annualWaterBalance = sum(P[i] - mpe[i])`, `summerWaterBalance = sum` over `{6, 7, 8}` (NH) or `{12, 1, 2}` (SH).

### 4.2 8×8 Soil Moisture Loop & 360-Day Calendar
- **Grid:** 64 slots `sl[1..64]` initialized to `0.0`, capacity `fsl = awc / 64.0`. Fixed **360-day** year (`12 × 30`).
- **Monthly 3-step cycle (`m = 1..12`):**
  1. **First 15 days:** `npe = (P[m]/2.0 - mpe[m]) / 2.0`. If `npe <= 0`, discharge `sl` in `dp[0..63]` order weighted by `dr[0..63]` (`1.0..5.0`). If `npe > 0`, recharge `sl[1..64]` sequentially up to `fsl` (excess discarded).
  2. **Mid-month storm:** `hp = P[m] / 2.0` instantaneously recharges `sl[1..64]` up to `fsl` (0 calendar days).
  3. **Second 15 days:** identical to step 1.
- **Equilibration:** Up to 10 years (`n = 1..10`); stops early if `abs(tmoi - prmo) < prmo / 100.0`.
- **MCS State `k`:** Inspected on `sl[9], sl[17], sl[25]`: `1` (Dry) if all `<= 0`; `3` (Moist) if all `> 0`; `2` (Moist/Dry) otherwise.
- **Final Year Day Allocation:** On every `k` transition during step 1 or 3 while `npe > 0`, assigns `(int)(15.0 * (cnpe - npe) / cnpe) - dpmc` days to `pmc` before updating `pmc = kk`, with remaining `15 - dpmc` days assigned to ending `k`.

### 4.3 Day Counters (`>5°C`, `>8°C`, Solstice Windows)
- **>5°C Window (`nsd[1..3]`, `lt5c`):** If no `T[i] < 5.0`, `nsd = nd` and `lt5c = 360`. Otherwise, mutates consecutive `T == 5.0` to `5.01`, finds crossings using `ia = 36` (rising) / `iz = 25` (falling) with `(int)(30 * ...)` truncation, sums interval lengths into `lt5c`, and counts `iday` into `nsd[1..3]` for the **first interval only** (`hasRunAlready` guard; Sign-Off #6).
- **>8°C Window (`ncpm[1]`, `ncpm[2]`):** `ncpm[1]` = max consecutive `iday != 1` on a doubled 720-day ring (capped at `360`). For `>8°C`, mutates consecutive `T == 8.0` to `8.01`, finds crossings with symmetric `ia = 30, iz = 30`, and sets `ncpm[2]` to max consecutive `iday != 1` inside each `>8°C` interval (`sn = -1`). If `!tempUnder8C` or `np8 == 0`, `sn == 0` sets `ncpm[2] = ncpm[1]` (Sign-Off #5).
- **Solstice 120-day windows:** NH summer = `181..300`, winter = `1..120`; SH summer = `1..120`, winter = `181..300` (mod 360). `nccd` = max consecutive `iday == 1` in summer window; `nccm` = max consecutive `iday == 3` in winter window.

### 4.4 STR & SMR Classification Trees
- **STR (`tma = mean(T) + fc`, `st = mean(T_summer) + fc`, `wt = mean(T_winter) + fc`, `dif = abs(at1 - at2)`):**
  - `cryic_ht` = `6` (`hasOHorizon && isSaturated`), `8` (`hasOHorizon && !isSaturated`), `13` (`!hasOHorizon && isSaturated`), `15` (`!hasOHorizon && !isSaturated`).
  - Evaluates `reg[1..10]` in order (**last `true` wins**): `Pergelic` (`tma < 0`), `Cryic` (`0 <= tma < 8 && 0 <= st < cryic_ht`), `Frigid` (`tma < 8 && !(0 <= st < cryic_ht) && dif*fcd >= 6`), `Mesic` (`8 <= tma < 15 && dif*fcd >= 6`), `Thermic` (`15 <= tma < 22 && dif*fcd >= 6`), `Hyperthermic` (`tma >= 22 && dif*fcd >= 6`), `Isofrigid` (`tma < 8 && !(0 <= st < cryic_ht) && dif*fcd < 6`), `Isomesic`, `Isothermic`, `Isohyperthermic`.
- **SMR (`if / else if`, using unattenuated `dif = st - wt`):**
  1. All 12 months `mpe[i] <= P[i]` (`swt != 0`) → `Perudic` (`q = " "`, `div = " "`).
  2. `nsd[1] > lt5c / 2.0 && ncpm[2] < 90` → `Aridic` (`div = "Aridic"`, `q = Extreme | Typic | Weak`).
  3. `tma < 22.0 && dif >= 5.0 && nccd >= 45 && nccm >= 45` → `Xeric` (`div = "Xeric"`, `q = Dry | Typic`).
  4. `nd[1] + nd[2] < 90.0` → `Udic` (`Typic/Udic`, `Dry/Tropudic`, `Dry/Tempudic`).
  5. `trr` not `Pergelic`/`Cryic` → `Ustic` (`Tempustic` with `Typic | Xeric | Wet`, or `Tropustic` with `Aridic | Typic | Udic`).
  6. Else → `Undefined` (`div = "Undefined"`, `q = " "`).

### 4.5 R `NA`/`NaN` & Raster Chunking Rules
- `batch3()` does **no** `NA`/`NaN` filtering; `NA`/`NaN` values are forwarded as `NaN` to Java.
- `newhall_batch.SpatRaster()` skips cells **only** where `is.na(awc)`, runs `batch3()` on remaining rows, re-inserts `NA` rows, and calls `.setCats()`. When `cores > 1` and `nrow(blockcomplete) == 0`, an existing R bug reverses `annualPotentialEvapotranspiration` and `summerWaterBalance` column positions in the empty placeholder frame.

---

## Sign-Off Checklist

Per project constraints (*"do not silently change scientific assumptions or rounding behavior in the baseline model; if you find ambiguities or bugs in the Java code, flag them and ask before deviating from parity"*), each item below requires explicit user confirmation before `G3`/`G4`/`G7` implementation:

1. **Southern Hemisphere PET `* 60.0D` Precedence Bug (`BASICSimulationModel.java:167`):**
   - For SH sites with `5.0 <= latDD < 50.0` off exact `rs[]` steps, Java lacks parentheses around `(rs[nrow] - rs[nrow - 1]) * 60.0D`, multiplying the interpolation delta by `3600` and producing wildly erroneous (even negative) `mpe`.
   - **Decision needed:** Replicate bug verbatim for strict parity, fix parentheses, or gate behind a compatibility flag?
2. **Broken `unitSystem = "english"` / `isMetric = false` Conversion (`NewhallBatch.R:437-440`, `BASICSimulationModel.java:42-3162`):**
   - `batch3()` defines `.doUnitsTemp`/`.doUnitsLength` but never calls them; `runSimulation(double[], ...)` ignores `isMetric` for `temperature`, `precip`, `waterHoldingCapacity`, and `soilAirOffset` (scaling only unused `elevation` by `0.3048`).
   - **Decision needed:** Keep `newhall-core` strictly metric (deg C, mm) and perform unit conversion in the R/`newhall-rs` wrapper when `unitSystem` is `"in"`/`"english"`, or preserve Java's no-op behavior for `isMetric = false`?
3. **`Pergelic` Overwritten by `Frigid` / `Isofrigid` (`BASICSimulationModel.java:254-281`):**
   - Because `cr[11]` is `tma < 8` and `cr[3]` requires `st >= 0 && st < cryic_ht`, any `tma < 0` site with `st < 0` (or `st >= cryic_ht`) has `reg[1]` (`Pergelic`) overwritten by `reg[3]` (`Frigid`) or `reg[7]` (`Isofrigid`).
   - **Decision needed:** Preserve last-match-wins Java behavior (`Frigid`/`Isofrigid`) or guard `Frigid`/`Isofrigid` with `tma >= 0` (`cr[2]`)?
4. **Unattenuated `dif = st - wt` in SMR Tree (`BASICSimulationModel.java:275, 3025, 3046, 3060`):**
   - Commenting out `st -= cs; wt += cs;` in `v1.6.4`/`v1.6.5` left `dif = st - wt` unattenuated (raw air-temperature range instead of soil range `dif * fcd`) and without `Math.abs()` in the Xeric/Udic/Ustic branches.
   - **Decision needed:** Preserve unattenuated `st - wt` for `v1.6.5` parity, or use soil-attenuated `dif * fcd`?
5. **Cold-Site `ncpm[2]` Fallback Bug (`BASICSimulationModel.java:1736, 2023`):**
   - When all 12 months are `< 8°C` (`tempUnder8C == true && np8 == 0`), `sn` stays `0`, so line `2023` sets `ncpm[2] = ncpm[1]` (assigning full-year moist-in-some-parts days to the `>8°C` counter instead of `0`).
   - **Decision needed:** Replicate `ncpm[2] = ncpm[1]` fallback for strict parity, or fix `np8 == 0` to `ncpm[2] = 0`?
6. **Multi-Interval `>5°C` `hasRunAlready` Guard (`BASICSimulationModel.java:58, 1559-1564`):**
   - For bimodal `>5°C` temperature curves (`np >= 2`), `nsd[1..3]` counts days from the **first interval only** while `lt5c` sums all intervals.
   - **Decision needed:** Preserve `hasRunAlready` single-interval behavior or accumulate all `>5°C` intervals into `nsd[1..3]`?
7. **Negative `latDD` + Hemisphere Handling (`NewhallBatch.R:447-448`, `BASICSimulationModel.java:155`):**
   - `batch3()` passes raw negative `latDD` alongside `northsouth = 'S'` (`latDD < 0`). In Java, SH `nrow` stays `0`, hitting the `0 <= latDD < 5` branch with negative `latDD`! Meanwhile, passing `northsouth = 'N'` with negative `latDD` directly to Java throws `ArrayIndexOutOfBoundsException`.
   - **Decision needed:** Should `newhall-core` take `lat_dd.abs()` when hemisphere is derived from sign (matching standard geographic expectation), or pass signed `latDD` into the SH `0..5°` formula to match `batch3()`?
8. **`regimeSubdivision1` / `regimeSubdivision2` Swapped Order (`BASICSimulationModel.java:3161` vs `NewhallResults.java:35`):**
   - Java passes `(q, div)` into `(div, q)`, placing modifier `q` (`"Typic"`, `"Dry"`, etc.) in `regimeSubdivision1` and group `div` (`"Udic"`, `"Tempustic"`, etc.) in `regimeSubdivision2`. R `.smrsub1()`/`.smrsub2()` match this swapped assignment.
   - **Decision needed:** Confirm `newhall-core` and R wrappers should keep `regimeSubdivision1 = q` and `regimeSubdivision2 = div` for `jNSMR` parity.
9. **`NaN` / `NA` Propagation & `Perudic` Trap (`BASICSimulationModel.java:3003`):**
   - In Java, `NaN` in `precip` makes `mpe[i] > precip[i]` evaluate `false`, yielding `moistureRegime = "Perudic"`, and `NaN` in `temperature` yields `temperatureRegime = ""` (`NA` after `.setCats()`). In `SpatRaster`, only `is.na(awc)` skips cells.
   - **Decision needed:** For any `NaN`/`NA` in required inputs (`t*`, `p*`, `latDD`, `awc`), should `newhall-core` / R return `NA` across all outputs (clean missing-data semantics) or match Java's `Perudic`/partial-`NaN` behavior when `awc` is non-`NA`?
10. **Exact-`5.0` / `8.0` In-Place Mutation & `is.na(awc)` / Multicore R Bugs (`BASICSimulationModel.java:1380,1613`, `NewhallBatch.R:789-799`):**
    - Confirm whether `newhall-core` should replicate the `5.0 -> 5.01` / `8.0 -> 8.01` consecutive-month threshold perturbation without mutating caller inputs, and confirm that R wrapper (`G7`) should **not** replicate the `cores > 1` empty-block `annualPotentialEvapotranspiration`/`summerWaterBalance` column swap bug.
11. **`runBatch()` / `NewhallDataset` Legacy Divergence:**
    - Confirm `G3`/`G4`/`G7` target parity strictly with `runBatch2()` (`runSimulation(double[], ...)` / `v1.6.5`), and do **not** need to support the older `NewhallDataset` (`v1.6.1`/`v1.6.3`) `cs`-attenuated thermal regime path.
