# Java NSM Source & Custom Modifications Audit (`~/workspace/jNSMR/java`)

Audit of the modified Java Newhall Simulation Model (`NSM_VERSION = "1.6.5"`) bundled in `/home/andrew/workspace/jNSMR/java` (compiled as `newhall-1.6.5.jar`, derived from upstream jNSM v1.6.1).

---

## 1. Java File Inventory (`wc -l`)

All 10 `.java` files under `/home/andrew/workspace/jNSMR/java` (7,587 total lines):

| File | Lines | Package / Class | Responsibility |
| --- | ---: | --- | --- |
| `org/psu/newhall/Newhall.java` | 48 | `org.psu.newhall.Newhall` | Defines `NSM_VERSION = "1.6.5"`; CLI `main(String[] args)` accepting `<path-to-CSV-file> <smcsawc> <soilAirOffset> <amplitude>` and printing `NewhallDataset` + `NewhallResults`. (Legacy Swing GUI launch commented out.) |
| `org/psu/newhall/sim/BASICSimulationModel.java` | 6,377 | `org.psu.newhall.sim.BASICSimulationModel` | Core NSM engine. Contains `runBatch`, `runBatch2`, two duplicated `runSimulation` implementations (raw arrays vs `NewhallDataset`), `computeWaterBalance`, and `@Deprecated computeSoilCalendar`. |
| `org/psu/newhall/sim/BASICSimulationModelConstants.java` | 82 | `org.psu.newhall.sim.BASICSimulationModelConstants` | Static lookup tables and constants: `tempRegimes[10]`, `zpe[24]`, `zt[24]`, `inz[12][31]`, `rn[31]`, `fs[12][13]`, `rs[13]`, `fc = 2.5`, `fcd = 0.66`, `cv = 5.0/9.0`, `dp[65]`, `dr[64]`, `lagPhaseSummer = 21`, `lagPhaseWinter = 10`. |
| `org/psu/newhall/sim/NewhallBatchResults.java` | 117 | `org.psu.newhall.sim.NewhallBatchResults` | Columnar/array batch output container (`public` arrays across `n` simulations) populated from `NewhallResults[]` for low-overhead `rJava` field access. Computes `annualPotentialEvapotranspiration` and `int[n][360]` calendars. |
| `org/psu/newhall/sim/NewhallDataset.java` | 159 | `org.psu.newhall.sim.NewhallDataset` | Immutable-ish POJO holding single-station inputs (`name`, `country`, `latitude`, `longitude`, `nsHemisphere`, `ewHemisphere`, `elevation`, 12-elem `precipitation` & `temperature` lists, `startYear`, `endYear`, `isMetric`, `waterholdingCapacity`, optional `metadata`), plus DMS helpers (`getLatitudeDegrees()`, `getLatitudeMinutes()`). |
| `org/psu/newhall/sim/NewhallDatasetMetadata.java` | 290 | `org.psu.newhall.sim.NewhallDatasetMetadata` | Station/contributor metadata POJO including `soilAirOffset` (default `2.5`) and `amplitude` (default `0.66`). |
| `org/psu/newhall/sim/NewhallResults.java` | 276 | `org.psu.newhall.sim.NewhallResults` | Single-run output POJO storing water balance scalars, 12-month `meanPotentialEvapotranspiration`, day counters, 360-day `temperatureCalendar` (`ArrayList<Character>`) and `moistureCalendar` (`ArrayList<Integer>`), regimes, subdivisions, and raw `flxFile` string. |
| `org/psu/newhall/util/CSVFileParser.java` | 102 | `org.psu.newhall.util.CSVFileParser` | Parses legacy 2-row CSV station files into a `NewhallDataset` (hardcodes `waterholdingCapacity = 200.0`, sets `soilAirOffset = 2.5 * 9/5` when `!isMetric`). |
| `org/psu/newhall/util/CSVParser.java` | 81 | `org.psu.newhall.util.CSVParser` | Simple comma-splitting file reader (`String.split(",")`, no quote escaping). |
| `org/psu/newhall/util/CSVResultsExporter.java` | 55 | `org/psu/newhall.util.CSVResultsExporter` | Writes `NewhallResults.getFlxFile()` to a `File` (shows `JOptionPane` dialog on error). |

---

## 2. Exact Simulation Algorithm (`runBatch2` -> `runSimulation` raw overload)

`jNSMR` invokes `BASICSimulationModel.runBatch2(...)` (`BASICSimulationModel.java:34-40`), which loops over `i = 0 .. latitude.length - 1` and delegates each element to the 12-argument `runSimulation(...)` (`BASICSimulationModel.java:42-3162`). Differences in the legacy `NewhallDataset` overload (`lines 3167-6285`) are noted in Sections 3 and 4.

### 2.1 Input Parameters, Units, and Hemisphere Detection
- **1-indexed monthly arrays:** `temperature` and `precip` are `double[13]` where **index `0` is unused (`0.0`)** and indices **`1..12` correspond to Jan..Dec**. (`R/NewhallBatch.R` prepends `0.0` via `cbind(0.0, ...)`.)
- **Parameters:**
  - `temperature[1..12]` (°C)
  - `precip[1..12]` (mm)
  - `latDD` (decimal degrees latitude, expected `>= 0`)
  - `lonDD` (decimal degrees longitude — **unused** in `runSimulation(double[], ...)`)
  - `elevation` (converted via `elevation *= 0.3048D` if `!isMetric`, then **unused**)
  - `northsouth` (`char`: `'N'` for Northern Hemisphere; any other `char`, e.g., `'S'`, selects Southern Hemisphere)
  - `isMetric` (`boolean`: **does NOT convert `temperature`, `precip`, `waterHoldingCapacity`, or `fc`** in `runSimulation(double[], ...)`! It only scales unused `elevation` by `0.3048D`.)
  - `waterHoldingCapacity` (mm; standard default `200.0`)
  - `fc` (`soilAirOffset` in °C; standard default `2.5`)
  - `fcd` (`amplitude` / soil-air temperature ratio; standard default `0.66`)
  - `hasOHorizon` (`boolean`: presence of O horizon)
  - `isSaturated` (`boolean`: saturation at 50 cm depth)
- **Hemisphere detection:** Java performs **no** sign-based hemisphere detection; callers must pass `northsouth = 'N'` or `'S'` and non-negative `latDD`.
- **Annual rainfall:** `arf = sum_{i=1}^{12} precip[i]`.

### 2.2 Monthly Thornthwaite PET Calculation
Lines `65-201`:
1. **Monthly heat index `mwi[1..12]`:**
   - If `temperature[j] > 0.0`: `mwi[j] = Math.pow(temperature[j] / 5.0D, 1.514D)`, else `0.0D`.
2. **Annual heat index `swi` & exponent `a`:**
   - `swi = sum_{b=0}^{12} mwi[b]` (`mwi[0] == 0.0`).
   - `a = 6.75E-7 * swi^3 - 7.71E-5 * swi^2 + 0.01792 * swi + 0.49239`.
3. **Unadjusted PET `upe[1..12]`:**
   - If `temperature[i] <= 0.0`: `upe[i] = 0.0`.
   - Else if `temperature[i] < 26.5`: `upe[i] = 16.0 * Math.pow(10.0 * temperature[i] / swi, a)`.
   - Else if `temperature[i] >= 38.0`: `upe[i] = 185.0`.
   - Else (`26.5 <= temperature[i] < 38.0`): step lookup in `zt[0..23]` (`26.5, 27.0, ..., 38.0` in `0.5`°C steps) and `zpe[0..23]` (`135.0` to `185.0`). Selects `zpe[ki - 1]` where `zt[ki - 1] <= T < zt[ki]`; **no interpolation**, **no rounding**.
4. **Daylength / latitude correction `mpe[1..12]`:**
   - Initialized to `0.0`. Only updated when `upe[i] > 0.0`.
   - **Northern Hemisphere (`northsouth == 'N'`):**
     - `nrow` = count of `rn[0..30]` (`0, 5, 10, 15, 20, 25, 26, ..., 50`) satisfying `latDD >= rn[idx]`. Effectively clamps `latDD >= 50` to row `31` (`50°N`), and selects the **floor** bin (no interpolation).
     - `mpe[i] = upe[i] * inz[i - 1][nrow - 1]`.
   - **Southern Hemisphere (`northsouth != 'N'`):**
     - `nrow` = count of `rs[0..12]` (`5, 10, 15, 20, 25, 30, 35, 40, 42, 44, 46, 48, 50`) satisfying `latDD >= rs[idx]`.
     - If `nrow != 0` (`latDD >= 5.0`):
       - If `nrow >= 13` (`latDD >= 50.0`) or `fs[i - 1][nrow - 1] == fs[i - 1][nrow]`: `cf = fs[i - 1][nrow - 1]`.
       - Else:
         ```java
         cf = (fs[i - 1][nrow] - fs[i - 1][nrow - 1])
            * (((int)latDD - rs[nrow - 1]) * 60.0D + ((latDD - (int)latDD) * 60))
            / (rs[nrow] - rs[nrow - 1]) * 60.0D
            + fs[i - 1][nrow - 1];
         ```
         *(See Ambiguities & Edge Cases below for the missing-parentheses `* 60.0D` bug in this expression.)*
     - Else (`0 <= latDD < 5.0`): interpolates between NH `0°` (`inz[i - 1][0]`) and SH `5°` (`fs[i - 1][0]`):
       ```java
       cf = (fs[i - 1][0] - inz[i - 1][0])
          * (((int)latDD * 60) + ((latDD - (int)latDD) * 60)) / 300.0D
          + inz[i - 1][0];
       ```
     - `mpe[i] = upe[i] * cf`.
5. **Derived Water Balances (`computeWaterBalance`, lines 6287-6311):**
   - `annualWaterBalance (awb) = sum_{i=1}^{12} (precip[i] - mpe[i])`.
   - `summerWaterBalance (swb)`: sum of `(precip[i] - mpe[i])` over `i in {6, 7, 8}` if `northsouth == 'N'`, else `i in {12, 1, 2}`.
   - `annualPotentialEvapotranspiration` (computed in `NewhallBatchResults.java:96-101`): `sum_{j=0}^{11} mpe[j + 1]`.

### 2.3 8×8 Soil Moisture Profile Recharge/Discharge Loop & 360-Day Calendar
Lines `284-1344`:
- **Calendar model:** Fixed **360-day year** (`12` months × `30` days), 1-indexed `iday[1..360]`. No 365-day / leap-year calendar is used anywhere.
- **Soil profile grid:** 64 slots `sl[1..64]` (1-indexed; `sl[0]` unused), each initialized to `0.0D` with capacity `fsl = waterHoldingCapacity / 64.0D`.
- **Per-month 3-step moisture update (for month `m = 1..12`):**
  1. **First 15 days:** `lp = precip[m] / 2.0`, `npe = (lp - mpe[m]) / 2.0`.
     - If `npe <= 0.0` (net deficit; `npe = -npe`): **discharge** slots in `dp[0..63]` order (1-based indices: `8, 7, 16, 6, 15, 24, ..., 57`; note `dp[64] = 160` is never indexed) with depletion resistance `dr[0..63]` (`1.0` to `5.0`). For slot `nr = dp[idx]`, available water weighted by resistance is `rpe = sl[nr] * dr[idx]`. If `npe <= rpe`, `sl[nr] -= npe / dr[idx]` and `npe = 0`; else `sl[nr] = 0` and `npe -= rpe`.
     - If `npe > 0.0` (net surplus): **recharge** slots sequentially `1..64`, filling each up to `fsl`; excess runs off (discarded).
  2. **Mid-month storm (`hp = precip[m] / 2.0`):** instantaneous **recharge** of slots `1..64` up to `fsl`.
  3. **Second 15 days:** identical to Step 1 (`npe = (precip[m]/2.0 - mpe[m])/2.0`).
- **Equilibration loop (up to 10 years, lines 373-554):**
  - Runs Steps 1–3 for all 12 months per year (`n = 1..10`) with `gogr = 0` (no calendar recording).
  - After each year, `tmoi = sum_{it=1}^{64} sl[it]`. Stops early if `Math.abs(tmoi - prmo) < prmo / 100.0D`; otherwise `prmo = tmoi`.
- **Moisture Control Section (MCS) state `k`:**
  - Inspected on slots `sl[9]`, `sl[17]`, `sl[25]` (top of columns 2, 3, 4 in the 8×8 matrix):
    - `k = 1` (**Dry**): `sl[9] <= 0 && sl[17] <= 0 && sl[25] <= 0`
    - `k = 2` (**Moist/Dry**): not all `<= 0`, and at least one `<= 0`
    - `k = 3` (**Moist**): `sl[9] > 0 && sl[17] > 0 && sl[25] > 0`
- **Final Year Simulation & 360-Day Calendar (`im = 1..12`, lines 607-1344):**
  - Before month 1, `k` is evaluated from post-equilibration `sl`.
  - During each 15-day half-month (when `npe != 0.0`), whenever modifying a slot changes `k` (`kk != pmc`) while `npe > 0.0`:
    - `rpe = cnpe - npe` (energy/water consumed so far)
    - `dmc[pmc] = (int)(15.0D * rpe / cnpe) - dpmc` (truncating integer cast)
    - Fills `igmc = dmc[pmc]` days in `iday[ib..ie]` with state `pmc`, adds `igmc` to `nd[pmc]`, and advances `dpmc += igmc`, `pmc = kk`.
  - Remaining `15 - dpmc` days of the half-month are assigned to the ending state `k`.
  - Note: in mid-month `hp` recharge, `k` is updated instantaneously (0 days assigned) before the second half-month.
  - `nd[1]` = `numCumulativeDaysDry`, `nd[2]` = `numCumulativeDaysMoistDry`, `nd[3]` = `numCumulativeDaysMoist`.

### 2.4 Day Counters (>5°C, >8°C, Solstice Windows)
Lines `1353-2991`:
- **>5°C Window (`nsd[1..3]`, `lt5c`, `id5c`):**
  - Checks if any `temperature[i] < 5.0` (air temperature!).
  - If **none** `< 5.0`: `tc = 0`, `nsd[1..3] = nd[1..3]`, `lt5c = 360.0`, `id5c = 0.0`.
  - If **any** `< 5.0`:
    - Mutates `temperature[it] = 5.01D` if consecutive months `(it, it+1)` (wrapping `12 -> 1`) both equal `5.0D`.
    - Finds crossings `crr = 5.0` using asymmetric offsets `ia = 36` (rising) and `iz = 25` (falling):
      - Rising (`T[m1] < 5 < T[m2]` or `T[m0] < 5 == T[m1] < T[m2]`): `day = m0*30 + 36 + (int)(30*(5 - T[m1])/(T[m2] - T[m1]))` (wrapped mod 360).
      - Falling (`T[m2] < 5 < T[m1]` or `T[m0] > 5 == T[m1] > T[m2]`): `day = m0*30 + 25 + (int)(30*(T[m1] - 5)/(T[m1] - T[m2]))` (wrapped mod 360).
    - Pairs intervals `[nbd[i22], ned[i22]]` (`np` intervals). If `np == 0` and `wt <= 5.0`: `lt5c = 0`, `nsd[1..3] = 0`.
    - Otherwise sums interval lengths into `lt5c`, and counts `iday` states into `nsd[1..3]` **ONLY for the first interval** due to the `hasRunAlready` guard (lines `1559-1564`).
- **>8°C Window & `ncpm[1..2]`:**
  - `ncpm[1]` (`numConsecutiveDaysMoistInSomeParts`): max consecutive run of `iday[i] != 1` (i.e. Moist or Moist/Dry) across a 720-day doubled ring (`1..720` mod 360), capped at `360`.
  - Checks if any `temperature[i] < 8.0`. If true:
    - Mutates `temperature[it] = 8.01D` for consecutive `8.0` months.
    - Finds crossings of `crr = 8.0` using symmetric offsets `ia = 30`, `iz = 30`.
    - For each `>8°C` interval (`1..np8`), computes max consecutive `iday[i] != 1` (WITHOUT wrapping beyond interval end — wait: it scans `sib .. sib + sir - 1` and checks `iday[n1] == 1` where `n1` wraps mod 360), storing the max across intervals in `ncpm[2]` (`numConsecutiveDaysMoistInSomePartsOver8C`) and sets `sn = -1`.
    - **Quirk:** If `!tempUnder8C` OR `np8 == 0` (all months `< 8°C`), `sn == 0`, so line `2023` sets `ncpm[2] = ncpm[1]`.
- **Solstice Windows (120-day & 180-day):**
  - If `northsouth == 'N'`: Summer solstice day `ib = 181` (Jul 1); Winter solstice day `i15 = 1` (Jan 1).
  - If `northsouth != 'N'`: Summer solstice day `ib = 1`; Winter solstice day `i15 = 181`.
  - `nccd` (`dryDaysAfterSummerSolstice`): max consecutive days with `iday == 1` in `[ib, ib + 119]` (mod 360).
  - `nccm` (`moistDaysAfterWinterSolstice`): max consecutive days with `iday == 3` in `[i15, i15 + 119]` (mod 360).
  - (Also computes 180-day counters `ncsm`, `ncwm`, `ncsp`, `ncwp`, `ntsu[1..3]`, `ntwi[1..3]` for `flxFile` only; not exposed in `NewhallBatchResults`.)
- **Temperature Calendar `ntd[1..360]`:**
  - Populated with ASCII `'-'` (`<5°C`), `'5'` (`5..8°C`), or `'8'` (`>8°C`).
  - `NewhallBatchResults` converts each `char` via `Character.getNumericValue()`, yielding **`-1` for `'-'`, `5` for `'5'`, `8` for `'8'`**.

### 2.5 Classification Trees (STR, SMR, Subdivisions)

#### Soil Temperature Regime (`trr`, lines 205-282)
- `tma = (sum_{i=1}^{12} temperature[i]) / 12.0 + fc`
- `at1 = (temperature[6] + temperature[7] + temperature[8]) / 3.0 + fc`
- `at2 = (temperature[1] + temperature[2] + temperature[12]) / 3.0 + fc`
- `st = (northsouth == 'N') ? at1 : at2`; `wt = (northsouth == 'N') ? at2 : at1`
- `dif = Math.abs(at1 - at2)` (note: `cs` is commented out in v1.6.5, so `st`/`wt` are NOT attenuated by `cs`, and later `dif = st - wt` at line `275`)
- `cryic_ht`:
  - `hasOHorizon == true && isSaturated == true` -> `6`
  - `hasOHorizon == true && isSaturated == false` -> `8`
  - `hasOHorizon == false && isSaturated == true` -> `13`
  - `hasOHorizon == false && isSaturated == false` -> `15`
- Criteria:
  - `cr[1] = tma < 0`
  - `cr[2] = 0 <= tma && tma < 8`
  - `cr[3] = st >= 0 && st < cryic_ht`
  - `cr[7] = (dif * fcd) >= 6`
  - `cr[8] = tma >= 8 && tma < 15`
  - `cr[9] = tma >= 15 && tma < 22`
  - `cr[10] = tma >= 22`
  - `cr[11] = tma < 8`
- Regimes `reg[1..10]` evaluated in ascending order (**last `true` wins**):
  1. `Pergelic`: `cr[1]` (`tma < 0`)
  2. `Cryic`: `cr[2] && cr[3]`
  3. `Frigid`: `cr[11] && !cr[3] && cr[7]`
  4. `Mesic`: `cr[8] && cr[7]`
  5. `Thermic`: `cr[9] && cr[7]`
  6. `Hyperthermic`: `cr[10] && cr[7]`
  7. `Isofrigid`: `cr[11] && !cr[7] && !cr[3]`
  8. `Isomesic`: `cr[8] && !cr[7]`
  9. `Isothermic`: `cr[9] && !cr[7]`
  10. `Isohyperthermic`: `cr[10] && !cr[7]`

#### Soil Moisture Regime (`ans`) & Subdivisions (lines 2999-3088)
Evaluated as an `if / else if` chain using `dif = st - wt` (unattenuated!):
1. **`swt != 0`** (all 12 months `mpe[i] <= precip[i]`):
   - `ans = "Perudic"`, `q = " "`, `div = " "` *(also sets `ncsm=ncwm=ncsp=ncwp=ntsu[3]=ntwi[3]=180`)*.
2. **Else if `nsd[1] > lt5c / 2.0 && ncpm[2] < 90`**:
   - `ans = "Aridic"`, `div = "Aridic"`.
   - `q`: `"Extreme"` if `nd[1] == 360.0`; else `"Typic"` if `ncpm[2] <= 45`; else `"Weak"`.
3. **Else if `tma < 22.0 && dif >= 5.0 && nccd >= 45 && nccm >= 45`**:
   - `ans = "Xeric"`, `div = "Xeric"`.
   - `q`: `"Dry"` if `nccd > 90`; else `"Typic"`.
4. **Else if `nd[1] + nd[2] < 90.0`**:
   - `ans = "Udic"`.
   - If `nd[1] + nd[2] < 30.0`: `q = "Typic"`, `div = "Udic"`.
   - Else if `dif < 5.0`: `q = "Dry"`, `div = "Tropudic"`.
   - Else: `q = "Dry"`, `div = "Tempudic"`.
5. **Else if `!trr.equalsIgnoreCase("pergelic") && !trr.equalsIgnoreCase("cryic")`**:
   - `ans = "Ustic"`.
   - If `dif >= 5.0`: `div = "Tempustic"`; `q =` `"Typic"` if `nccm <= 45`, else `"Xeric"` if `nccd > 45`, else `"Wet"`.
   - Else (`dif < 5.0`): `div = "Tropustic"`; `q =` `"Aridic"` if `ncpm[2] < 180`, else `"Typic"` if `ncpm[2] < 270`, else `"Udic"`.
6. **Else** (`trr` is `"Pergelic"` or `"Cryic"`, or `""`):
   - `ans = "Undefined"`, `div = "Undefined"`, `q = " "`.

**Note on `regimeSubdivision1` vs `regimeSubdivision2`:**
`NewhallResults` constructor parameter order is `(..., String div, String q, ...)` assigning `regimeSubdivision1 = div` and `regimeSubdivision2 = q` (`NewhallResults.java:35,44-45`), **but both `runSimulation` methods pass `(..., q, div, ...)`** (`BASICSimulationModel.java:3161, 6284`). Consequently:
- **`regimeSubdivision1` holds `q`** (`"Typic"`, `"Dry"`, `"Weak"`, `"Extreme"`, `"Xeric"`, `"Wet"`, `"Aridic"`, `"Udic"`, `" "`).
- **`regimeSubdivision2` holds `div`** (`"Aridic"`, `"Xeric"`, `"Udic"`, `"Tropudic"`, `"Tempudic"`, `"Tempustic"`, `"Tropustic"`, `"Undefined"`, `" "`).

---

## 3. Custom Modifications Relative to Standard jNSM v1.6.1

Tracked from `jNSMR` git history (`d992732` -> `cb4e4d6`):

1. **JDOM & GUI/XML Removal (`v1.6.2`, commit `2d0a4a2`):**
   - Removed all `org.jdom.*`, `org.psu.newhall.ui.*`, `org.psu.newhall.flerry.*`, and XML importer/exporter classes; removed JDOM 1.1 from `LICENSE`.
   - Fixed decompiler-induced infinite loops in `BASICSimulationModel.java` (`timesi > cc.length` guards on `cc` loops) and fixed `NewhallResults.toString()` monthly PET formatting.
2. **CLI Entry Point (`v1.6.2`, commit `4984d0b`):**
   - Replaced Swing GUI startup in `Newhall.main()` with CLI: `java -jar newhall-1.6.5.jar <path-to-CSV-file> <smcsawc> <soilAirOffset> <amplitude>`.
3. **Batching Interface & `NewhallBatchResults` (`v1.6.3`, commits `8af47ee`, `db92c2f`, `42d5816`):**
   - Added `BASICSimulationModel.runBatch(...)` returning `NewhallBatchResults` (flat public primitive/String arrays instead of `NewhallResults[]` getters).
   - Added `BASICSimulationModel.runBatch2(...)` and `runSimulation(double[] temperature, double[] precip, ...)` bypassing `NewhallDataset` / `ArrayList<Double>` boxing.
   - Added `BASICSimulationModel.computeWaterBalance` visibility `public`.
4. **`annualPotentialEvapotranspiration` (`v1.6.4`, commit `3d24cb0`):**
   - Added `public double[] annualPotentialEvapotranspiration` to `NewhallBatchResults`, computed as the sum of the 12 `meanPotentialEvapotranspiration` elements.
   - Populated `temperatureCalendar` and `moistureCalendar` arrays in `NewhallBatchResults` (previously stubbed as zeros in `v1.6.3`).
5. **Cryic / Frigid / Iso-threshold Updates (`v1.6.4` -> `v1.6.5`, commits `bc50806`, `d25ee3d`):**
   - In `runSimulation(double[], ...)` ONLY:
     - Removed `cs = dif * (1 - fcd) / 2` adjustment from `st` and `wt` (kept `st` and `wt` as `at_summer`/`at_winter` directly).
     - Changed `cr[7]` threshold from `(dif * fcd) > 5.0` to `(dif * fcd) >= 6`.
     - Changed `cr[3]` from `(st - cs) < 15.0` to `st >= 0 && st < cryic_ht`.
6. **Spatially Explicit `soilAirOffset`, `amplitude`, `hasOHorizon`, `isSaturated` (`v1.6.5`, commit `d25ee3d`):**
   - Added `boolean[] hasOHorizon` and `boolean[] isSaturated` to `runBatch2` and `runSimulation(double[], ...)`.
   - Set `cryic_ht` upper summer temperature threshold based on `hasOHorizon` and `isSaturated`: `15` (neither), `13` (saturated mineral), `8` (O-horizon unsaturated), `6` (O-horizon + saturated).

---

## Ambiguities & Edge Cases

1. **Southern Hemisphere PET Interpolation `* 60.0D` Precedence Bug (`BASICSimulationModel.java:167` & `3299`):**
   - Expression `... / (rs[nrow] - rs[nrow - 1]) * 60.0D` lacks parentheses around `(rs[nrow] - rs[nrow - 1]) * 60.0D`. Because `/` and `*` associate left-to-right, any Southern Hemisphere site with `5.0 <= latDD < 50.0` not lying on an exact `rs[]` latitude step has its correction-factor interpolation delta multiplied by `3600` instead of `1`, producing wildly erroneous `mpe` (positive or negative!). (The `0 <= latDD < 5.0` branch divides by `300.0D` correctly.)
2. **`Pergelic` Overwritten by `Frigid` / `Isofrigid` When `st < 0` or `st >= cryic_ht` (`BASICSimulationModel.java:254-281`):**
   - `cr[11]` is `tma < 8` (true when `tma < 0`), and `cr[3]` is `st >= 0 && st < cryic_ht`. For cold sites where `tma < 0` AND `st < 0` (summer mean soil temp below 0°C) — or theoretically `tma < 0` with `st >= cryic_ht` — `cr[3]` is `false`, so `reg[3]` (`Frigid`) or `reg[7]` (`Isofrigid`) evaluates to `true` after `reg[1]` (`Pergelic`), overwriting `trr` to `"Frigid"` or `"Isofrigid"`.
3. **Divergence Between `runSimulation(double[], ...)` and `runSimulation(NewhallDataset, ...)`:**
   - The `NewhallDataset` overload (used by `Newhall.main()` CLI and `runBatch()`) was **not** updated to v1.6.4/v1.6.5: it still computes `cs = dif * (1 - fcd) / 2`, mutates `st -= cs; wt += cs;`, checks `cr[3] = (st * (1 - fcd) < 15.0)` (even in `bc50806` it changed `st - cs` to `st * (1 - fcd)`), checks `cr[7] = (dif * fcd > 5.0)`, and lacks `hasOHorizon` / `isSaturated`.
   - `runSimulation(double[], ...)` (used by `runBatch2`) **ignores `isMetric` for `temperature`, `precip`, `waterHoldingCapacity`, and `fc`** (only scaling unused `elevation` by `0.3048` instead of `0.305`).
4. **Unattenuated `dif = st - wt` Used in SMR Tree (`BASICSimulationModel.java:275, 3025, 3046, 3060`):**
   - Because `st -= cs; wt += cs;` were commented out in v1.6.4/v1.6.5, `dif` entering the Xeric (`dif >= 5.0`), Udic (`dif < 5.0`), and Ustic (`dif >= 5.0`) branches is now the raw air-temperature seasonal difference `st - wt` rather than the soil-attenuated difference (`(at_summer - at_winter) * fcd`). Also, `st - wt` is **not** wrapped in `Math.abs(...)`, so if `wt > st` then `dif < 0`.
5. **Cold-Site `ncpm[2]` Fallback Bug (`BASICSimulationModel.java:1736, 2023`):**
   - When `tempUnder8C` is true and `np8 == 0` (all 12 months `< 8°C`), `sn` remains `0`, causing line `2023` (`if (sn == 0) ncpm[2] = ncpm[1];`) to assign the full-year consecutive moist-in-some-parts count (`ncpm[1]`) to `ncpm[2]` (`>8°C`) instead of `0`. This directly affects Aridic (`ncpm[2] < 90`) and Tropustic criteria!
6. **Multi-Interval >5°C `hasRunAlready` Hack (`BASICSimulationModel.java:58, 1559-1564`):**
   - When there are `>= 2` disjoint periods in a year with `temperature > 5°C`, `nsd[1..3]` only accumulates days from the **first** period (`hasRunAlready = true`), whereas `lt5c` sums all periods. Thus `nsd[1] + nsd[2] + nsd[3] < lt5c` for bimodal >5°C temperature curves.
7. **`regimeSubdivision1` / `regimeSubdivision2` Swapped Argument Order (`BASICSimulationModel.java:3161, 6284` vs `NewhallResults.java:35`):**
   - `runSimulation` passes `(..., q, div, ...)` into `NewhallResults(..., div, q, ...)`, placing the modifier (`q`, e.g., `"Typic"`) in `regimeSubdivision1` and the regime group (`div`, e.g., `"Udic"`) in `regimeSubdivision2`.
8. **In-Place Mutation of `temperature[]` at Exact `5.0` / `8.0` (`BASICSimulationModel.java:1380-1386, 1613-1622`):**
   - Consecutive months equal to `5.0` or `8.0` are mutated in place to `5.01` or `8.01`. This mutates the caller's array and affects the `flxFile` string, though `tma`/`st`/`wt`/`upe`/`mpe` are computed prior to this mutation.
9. **Truncation Casts vs Rounding:**
   - Half-month transition day allocation uses truncating `(int)(15.0D * rpe / cnpe)` (`lines 728, 860, 1130, 1260`).
   - Crossing days use truncating `(int)(30.0D * ...)` (`lines 1439, 1450, 1674, 1685`).
   - All PET, heat index, water balance, and slot water values remain unrounded IEEE-754 `double`s; `sum += x` order is fixed `1..12` (or `0..12` for `swi`).
10. **Edge Cases (`NaN`, Zero/Negative AWC, Extreme/Negative Latitudes, `temperature <= 0`):**
    - **`NaN` inputs:** `NaN` in `temperature` makes `mwi = upe = mpe = 0.0`, `tma/st/wt/dif = NaN`, all `cr[] = false` -> `trr = ""` (`""` is not `"Pergelic"`/`"Cryic"`, falling through to `Perudic`/`Ustic`). `NaN` in `precip` causes `mpe[i] > precip[i]` to evaluate `false` for all 12 months -> `swt = -1` -> **`moistureRegime = "Perudic"`**!
    - **Zero or negative `waterHoldingCapacity`:** `fsl <= 0`, `sl[1..64]` remain `0.0`, `sl[9,17,25] <= 0.0` is always true -> `k = 1` (Dry) for all 360 days; 10-year equilibration runs all 10 iterations because `Math.abs(0 - 0) < 0` is `false`.
    - **Extreme latitudes:** `latDD >= 50.0` clamps daylength correction to `50°` (`nrow = 31` in NH, `nrow = 13` in SH). Negative `latDD` (`< 0`) in NH causes `ArrayIndexOutOfBoundsException` at `inz[i19 - 1][-1]` (`nrow = 0`).
    - **All months `temperature <= 0`:** `swi == 0.0`, `upe[1..12] = 0.0`, `mpe[1..12] = 0.0` (`aBase` division by `swi` is guarded by `temperature[i1] > 0.0`).
11. **Unused / Dead State & Unreachable Guard:**
    - `lonDD`, `elevation` (after unused `0.3048` conversion), `aev` (in `runSimulation(double[], ...)`), `swst`, `swfi`, `cc2`, `c2`, `gogr` (no-op `if (gogr != 0);`), `zsw != 0;` no-ops, `NewhallResults.nsd`/`ncpm` `ArrayList<Integer>` fields (never initialized), and `dp[64] = 160` (`i20` only loops `1..64`, indexing `dp[0..63]`).
    - `BASICSimulationModelConstants.zt` loop (`ki = 1..24`, `kl = ki + 1`): `zt[24]` would throw `ArrayIndexOutOfBoundsException` if reached, prevented only by the prior `else if (temperature[i1] >= 38.0D)` branch (`zt[23] == 38.0`).
