//! Thermal model, `>5°C` / `>8°C` day counters, and STR/SMR classifiers
//! matching `jNSMR` `v1.6.5` (`BASICSimulationModel.runSimulation(double[], ...)`).

use crate::traits::ThermalModel;
use crate::types::{
    DayCounters, Hemisphere, MoistureRegime, RegimeSubdivision1, RegimeSubdivision2, SiteConfig,
    SiteInput, SoilState, TemperatureRegime, ThermalClassification, WaterBalance,
};

/// Classic Newhall thermal model and STR/SMR classifier (`jNSMR` `v1.6.5` parity).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClassicThermal;

impl ThermalModel for ClassicThermal {
    fn evaluate(
        &self,
        input: &SiteInput,
        config: &SiteConfig,
        water_balance: &WaterBalance,
        soil_state: &SoilState,
    ) -> ThermalClassification {
        let _ = config.is_metric; // Defect #2: ignored in newhall-core
        let hemisphere = input.hemisphere();

        let (temperature_regime, tma, wt, dif) = classify_str(&input.temp_c, hemisphere, config);

        // Local mutable copy for consecutive 5.0 -> 5.01 and 8.0 -> 8.01 perturbations (Defect #10).
        let mut temp = input.temp_c;

        // >5°C crossing solver (BASICSimulationModel.java:1353-1592).
        let mut nsd = [0.0_f64; 3];
        let mut lt5c = 0.0_f64;
        let mut skip_to_890 = false;
        let mut has_run_already = false;

        if temp.iter().any(|&t| t < 5.0) {
            let (np, intervals) = find_threshold_intervals(&mut temp, 5.0, 5.01, 36, 25);
            if np == 0 {
                if wt <= 5.0 {
                    skip_to_890 = true;
                }
            } else {
                for &(beg, end) in &intervals[..np] {
                    let ir = interval_len(beg, end);
                    let mut nzd = [0.0_f64; 3];
                    for l in beg..=(beg + ir - 1) {
                        let day = wrap_day(l);
                        let ik = soil_state.iday[(day - 1) as usize];
                        if (1..=3).contains(&ik) {
                            nzd[(ik - 1) as usize] += 1.0;
                        }
                    }
                    // Defect #6: accumulate nsd from the first >5°C interval only.
                    if !has_run_already {
                        for i in 0..3 {
                            nsd[i] += nzd[i];
                        }
                        has_run_already = true;
                    }
                    lt5c += ir as f64;
                }
                skip_to_890 = true;
            }
        }

        if !skip_to_890 {
            nsd = soil_state.nd;
            lt5c = 360.0;
        }

        // >8°C crossing solver & ncpm[1..2] (BASICSimulationModel.java:1596-2026).
        let mut ncpm2 = 0_i32;
        let mut sn = 0_i32;

        if temp.iter().any(|&t| t < 8.0) {
            let (np8, intervals8) = find_threshold_intervals(&mut temp, 8.0, 8.01, 30, 30);
            if np8 != 0 {
                for &(beg, end) in &intervals8[..np8] {
                    let ir = interval_len(beg, end);
                    let icon = consecutive_iday_run(&soil_state.iday, beg, ir, 1, false);
                    if ncpm2 <= icon {
                        ncpm2 = icon;
                    }
                }
                sn = -1;
            }
        }

        let ncpm1 = consecutive_iday_run(&soil_state.iday, 1, 720, 1, false).min(360);
        // Defect #5: when all months < 8°C (sn == 0), fallback assigns ncpm[1] to ncpm[2].
        if sn == 0 {
            ncpm2 = ncpm1;
        }

        // 120-day solstice windows (BASICSimulationModel.java:2030-2277).
        let (summer_start, winter_start) = match hemisphere {
            Hemisphere::North => (181, 1),
            Hemisphere::South => (1, 181),
        };
        let nccd = consecutive_iday_run(&soil_state.iday, summer_start, 120, 1, true);
        let nccm = consecutive_iday_run(&soil_state.iday, winter_start, 120, 3, true);

        // SMR + q/div classification (BASICSimulationModel.java:359-369, 2999-3087).
        let swt_nonzero = !(0..12).any(|i| water_balance.mpe[i] > input.precip_mm[i]);

        let (moisture_regime, regime_subdivision1, regime_subdivision2) = classify_smr(
            swt_nonzero,
            nsd[0],
            lt5c,
            ncpm2,
            soil_state.nd,
            tma,
            dif,
            nccd,
            nccm,
            temperature_regime,
        );

        let day_counters = DayCounters {
            dry_days_after_summer_solstice: nccd,
            moist_days_after_winter_solstice: nccm,
            num_cumulative_days_dry: soil_state.nd[0] as i32,
            num_cumulative_days_moist_dry: soil_state.nd[1] as i32,
            num_cumulative_days_moist: soil_state.nd[2] as i32,
            num_cumulative_days_dry_over_5c: nsd[0] as i32,
            num_cumulative_days_moist_dry_over_5c: nsd[1] as i32,
            num_cumulative_days_moist_over_5c: nsd[2] as i32,
            num_consecutive_days_moist_in_some_parts: ncpm1,
            num_consecutive_days_moist_in_some_parts_over_8c: ncpm2,
        };

        ThermalClassification {
            day_counters,
            temperature_regime,
            moisture_regime,
            regime_subdivision1,
            regime_subdivision2,
            aquic_bio_days: 0,
        }
    }
}

/// Strict Newhall thermal model and STR/SMR classifier.
///
/// Remediates audited `jNSMR` defects:
/// - `#2`: converts `temp_c` (°F→°C), `precip_mm` (in→mm), and `soil_air_offset` (`* 5/9`) when `!config.is_metric`
/// - `#3`: `tma < 0.0` always yields `TemperatureRegime::Pergelic` (never overwritten by `Frigid`/`Isofrigid`)
/// - `#4`: uses soil-attenuated `dif = |(st - wt) * config.amplitude|` in SMR branches
/// - `#5`: cold sites with any month `< 8.0°C` and `np8 == 0` keep `ncpm2 = 0`
/// - `#6`: accumulates `nsd` across all `>5°C` intervals
/// - `#8`: preserves Rust type-safe [`RegimeSubdivision1`] (`q` modifier) and [`RegimeSubdivision2`] (`div` group)
/// - `#9`: `NaN` in precipitation, PET, temperature, or thermal config yields `MoistureRegime::Undefined`
/// - `#10`: handles exact `5.0`/`8.0` °C plateaus cleanly without `5.01`/`8.01` perturbation
#[derive(Debug, Clone, Copy, Default)]
pub struct StrictThermal;

impl ThermalModel for StrictThermal {
    fn evaluate(
        &self,
        input: &SiteInput,
        config: &SiteConfig,
        water_balance: &WaterBalance,
        soil_state: &SoilState,
    ) -> ThermalClassification {
        let hemisphere = input.hemisphere();
        let (temp, precip_mm, soil_air_offset) = normalize_metric_thermal(input, config);

        let temp_has_nan = temp.iter().any(|t| t.is_nan())
            || soil_air_offset.is_nan()
            || config.amplitude.is_nan();
        let precip_or_pet_has_nan =
            precip_mm.iter().any(|p| p.is_nan()) || water_balance.mpe.iter().any(|e| e.is_nan());

        let (temperature_regime, tma, wt, dif) = classify_str_strict(
            &temp,
            hemisphere,
            soil_air_offset,
            config.amplitude,
            config.has_o_horizon,
            config.is_saturated,
        );

        let (nsd, lt5c, ncpm1, ncpm2) = if temp_has_nan {
            (
                [0.0_f64; 3],
                0.0_f64,
                consecutive_iday_run(&soil_state.iday, 1, 720, 1, false).min(360),
                0_i32,
            )
        } else {
            // >5°C crossing solver without 5.01 perturbation (Defect #10) or has_run_already guard (Defect #6).
            let mut nsd = [0.0_f64; 3];
            let mut lt5c = 0.0_f64;
            let mut skip_to_890 = false;

            if temp.iter().any(|&t| t < 5.0) {
                let (np, intervals) = find_threshold_intervals_strict(&temp, 5.0, 36, 25);
                if np == 0 {
                    if wt <= 5.0 {
                        skip_to_890 = true;
                    }
                } else {
                    for &(beg, end) in &intervals[..np] {
                        let ir = interval_len(beg, end);
                        for l in beg..=(beg + ir - 1) {
                            let day = wrap_day(l);
                            let ik = soil_state.iday[(day - 1) as usize];
                            if (1..=3).contains(&ik) {
                                nsd[(ik - 1) as usize] += 1.0;
                            }
                        }
                        lt5c += ir as f64;
                    }
                    skip_to_890 = true;
                }
            }

            if !skip_to_890 {
                nsd = soil_state.nd;
                lt5c = 360.0;
            }

            // >8°C crossing solver without 8.01 perturbation (Defect #10) or cold-site fallback (Defect #5).
            let ncpm1 = consecutive_iday_run(&soil_state.iday, 1, 720, 1, false).min(360);
            let ncpm2 = if temp.iter().any(|&t| t < 8.0) {
                let (np8, intervals8) = find_threshold_intervals_strict(&temp, 8.0, 30, 30);
                let mut max_run = 0_i32;
                for &(beg, end) in &intervals8[..np8] {
                    let ir = interval_len(beg, end);
                    let icon = consecutive_iday_run(&soil_state.iday, beg, ir, 1, false);
                    if max_run <= icon {
                        max_run = icon;
                    }
                }
                max_run
            } else {
                ncpm1
            };

            (nsd, lt5c, ncpm1, ncpm2)
        };

        // 120-day solstice windows.
        let (summer_start, winter_start) = match hemisphere {
            Hemisphere::North => (181, 1),
            Hemisphere::South => (1, 181),
        };
        let nccd = consecutive_iday_run(&soil_state.iday, summer_start, 120, 1, true);
        let nccm = consecutive_iday_run(&soil_state.iday, winter_start, 120, 3, true);

        // Defect #9: any NaN in temperature, precipitation, PET, or thermal config yields Undefined SMR.
        // Defect #8: ThermalClassification enforces typed RegimeSubdivision1 (q) & RegimeSubdivision2 (div).
        let (moisture_regime, regime_subdivision1, regime_subdivision2) =
            if temp_has_nan || precip_or_pet_has_nan {
                (
                    MoistureRegime::Undefined,
                    RegimeSubdivision1::Undefined,
                    RegimeSubdivision2::Undefined,
                )
            } else {
                let swt_nonzero = !(0..12).any(|i| water_balance.mpe[i] > precip_mm[i]);
                classify_smr(
                    swt_nonzero,
                    nsd[0],
                    lt5c,
                    ncpm2,
                    soil_state.nd,
                    tma,
                    dif,
                    nccd,
                    nccm,
                    temperature_regime,
                )
            };

        let day_counters = DayCounters {
            dry_days_after_summer_solstice: nccd,
            moist_days_after_winter_solstice: nccm,
            num_cumulative_days_dry: soil_state.nd[0] as i32,
            num_cumulative_days_moist_dry: soil_state.nd[1] as i32,
            num_cumulative_days_moist: soil_state.nd[2] as i32,
            num_cumulative_days_dry_over_5c: nsd[0] as i32,
            num_cumulative_days_moist_dry_over_5c: nsd[1] as i32,
            num_cumulative_days_moist_over_5c: nsd[2] as i32,
            num_consecutive_days_moist_in_some_parts: ncpm1,
            num_consecutive_days_moist_in_some_parts_over_8c: ncpm2,
        };

        ThermalClassification {
            day_counters,
            temperature_regime,
            moisture_regime,
            regime_subdivision1,
            regime_subdivision2,
            aquic_bio_days: 0,
        }
    }
}

#[inline]
fn normalize_metric_thermal(input: &SiteInput, config: &SiteConfig) -> ([f64; 12], [f64; 12], f64) {
    if config.is_metric {
        (input.temp_c, input.precip_mm, config.soil_air_offset)
    } else {
        let mut temp_c = [0.0_f64; 12];
        let mut precip_mm = [0.0_f64; 12];
        for i in 0..12 {
            temp_c[i] = (input.temp_c[i] - 32.0) * 5.0 / 9.0;
            precip_mm[i] = input.precip_mm[i] * 25.4;
        }
        (temp_c, precip_mm, config.soil_air_offset * 5.0 / 9.0)
    }
}

#[inline]
fn compute_str_metrics(
    temp_c: &[f64; 12],
    hemisphere: Hemisphere,
    soil_air_offset: f64,
    has_o_horizon: bool,
    is_saturated: bool,
) -> (f64, f64, f64, f64, f64) {
    let mut sumt = 0.0_f64;
    for &t in temp_c {
        sumt += t;
    }

    let tma = sumt / 12.0 + soil_air_offset;
    let at1 = (temp_c[5] + temp_c[6] + temp_c[7]) / 3.0 + soil_air_offset;
    let at2 = (temp_c[0] + temp_c[1] + temp_c[11]) / 3.0 + soil_air_offset;

    let (st, wt) = match hemisphere {
        Hemisphere::North => (at1, at2),
        Hemisphere::South => (at2, at1),
    };
    let dif_abs = (at1 - at2).abs();

    let cryic_ht = match (has_o_horizon, is_saturated) {
        (true, true) => 6.0,
        (true, false) => 8.0,
        (false, true) => 13.0,
        (false, false) => 15.0,
    };

    (tma, st, wt, dif_abs, cryic_ht)
}

/// Strict STR classification: `tma < 0` always yields `Pergelic` (Defect #3),
/// `NaN` yields `None` (Defect #9), and returned `dif` is `|(st - wt) * fcd|` (Defect #4).
fn classify_str_strict(
    temp_c: &[f64; 12],
    hemisphere: Hemisphere,
    soil_air_offset: f64,
    amplitude: f64,
    has_o_horizon: bool,
    is_saturated: bool,
) -> (Option<TemperatureRegime>, f64, f64, f64) {
    let (tma, st, wt, dif_abs, cryic_ht) = compute_str_metrics(
        temp_c,
        hemisphere,
        soil_air_offset,
        has_o_horizon,
        is_saturated,
    );
    let dif_attenuated = dif_abs * amplitude;

    if tma.is_nan() || st.is_nan() || wt.is_nan() || amplitude.is_nan() {
        return (None, tma, wt, dif_attenuated);
    }

    let is_iso = dif_attenuated < 6.0;
    let trr = if tma < 0.0 {
        Some(TemperatureRegime::Pergelic)
    } else if tma < 8.0 {
        if st >= 0.0 && st < cryic_ht {
            Some(TemperatureRegime::Cryic)
        } else if is_iso {
            Some(TemperatureRegime::Isofrigid)
        } else {
            Some(TemperatureRegime::Frigid)
        }
    } else if tma < 15.0 {
        if is_iso {
            Some(TemperatureRegime::Isomesic)
        } else {
            Some(TemperatureRegime::Mesic)
        }
    } else if tma < 22.0 {
        if is_iso {
            Some(TemperatureRegime::Isothermic)
        } else {
            Some(TemperatureRegime::Thermic)
        }
    } else if is_iso {
        Some(TemperatureRegime::Isohyperthermic)
    } else {
        Some(TemperatureRegime::Hyperthermic)
    };

    (trr, tma, wt, dif_attenuated)
}

/// Computes `tma`, `wt`, unattenuated `dif = st - wt` (Defect #4), and STR (Defects #3, #9).
fn classify_str(
    temp_c: &[f64; 12],
    hemisphere: Hemisphere,
    config: &SiteConfig,
) -> (Option<TemperatureRegime>, f64, f64, f64) {
    let fcd = config.amplitude;
    let (tma, st, wt, dif_abs, cryic_ht) = compute_str_metrics(
        temp_c,
        hemisphere,
        config.soil_air_offset,
        config.has_o_horizon,
        config.is_saturated,
    );

    let cr1 = tma < 0.0;
    let cr2 = (0.0..8.0).contains(&tma);
    let cr3 = st >= 0.0 && st < cryic_ht;
    let cr7 = (dif_abs * fcd) >= 6.0;
    let cr8 = (8.0..15.0).contains(&tma);
    let cr9 = (15.0..22.0).contains(&tma);
    let cr10 = tma >= 22.0;
    let cr11 = tma < 8.0;

    // Last-match-wins across reg[1..10] (Defect #3: Frigid/Isofrigid overwrite Pergelic).
    let mut trr = None;
    if cr1 {
        trr = Some(TemperatureRegime::Pergelic);
    }
    if cr2 && cr3 {
        trr = Some(TemperatureRegime::Cryic);
    }
    if cr11 && !cr3 && cr7 {
        trr = Some(TemperatureRegime::Frigid);
    }
    if cr8 && cr7 {
        trr = Some(TemperatureRegime::Mesic);
    }
    if cr9 && cr7 {
        trr = Some(TemperatureRegime::Thermic);
    }
    if cr10 && cr7 {
        trr = Some(TemperatureRegime::Hyperthermic);
    }
    if cr11 && !cr7 && !cr3 {
        trr = Some(TemperatureRegime::Isofrigid);
    }
    if cr8 && !cr7 {
        trr = Some(TemperatureRegime::Isomesic);
    }
    if cr9 && !cr7 {
        trr = Some(TemperatureRegime::Isothermic);
    }
    if cr10 && !cr7 {
        trr = Some(TemperatureRegime::Isohyperthermic);
    }

    (trr, tma, wt, st - wt)
}

/// Finds crossing intervals above `crr` (`5.0` or `8.0`) without mutating `temp` (Defect #10).
///
/// Consecutive `== crr` months form a plateau bounded by `t_before != crr` and `t_after != crr`
/// (guaranteed to exist when at least one month is `< crr`). A rising crossing is emitted once
/// at the end of a `< crr -> crr..crr -> > crr` plateau; a falling crossing is emitted once at
/// the start of a `> crr -> crr..crr -> < crr` plateau; non-crossing touches (`< crr -> crr..crr -> < crr`
/// or `> crr -> crr..crr -> > crr`) emit no crossing.
fn find_threshold_intervals_strict(
    temp: &[f64; 12],
    crr: f64,
    ia: i32,
    iz: i32,
) -> (usize, [(i32, i32); 6]) {
    let mut nj = [0_i32; 12];
    let mut kj = 0_usize;
    let mut zwt = false;

    for i19 in 1..=12_i32 {
        let m0 = if i19 == 1 { 12 } else { i19 - 1 };
        let m1 = i19;
        let m2 = if i19 == 12 { 1 } else { i19 + 1 };

        let t0 = temp[(m0 - 1) as usize];
        let t1 = temp[(m1 - 1) as usize];
        let t2 = temp[(m2 - 1) as usize];

        if t1 < crr && t2 > crr {
            let day = wrap_day(m0 * 30 + ia + (30.0 * (crr - t1) / (t2 - t1)) as i32);
            nj[kj] = day;
            kj += 1;
            zwt = true;
        } else if t1 > crr && t2 < crr {
            let day = wrap_day(m0 * 30 + iz + (30.0 * (t1 - crr) / (t1 - t2)) as i32);
            nj[kj] = day;
            kj += 1;
            zwt = false;
        } else if t1 == crr && (t0 != crr || t2 != crr) {
            let mut t_before = t0;
            if t_before == crr {
                for step in 1..12 {
                    let idx = ((m1 - 1 - step).rem_euclid(12)) as usize;
                    if temp[idx] != crr {
                        t_before = temp[idx];
                        break;
                    }
                }
            }
            let mut t_after = t2;
            if t_after == crr {
                for step in 1..12 {
                    let idx = ((m1 - 1 + step).rem_euclid(12)) as usize;
                    if temp[idx] != crr {
                        t_after = temp[idx];
                        break;
                    }
                }
            }

            if t_before < crr && t_after > crr && t2 > crr {
                nj[kj] = wrap_day(m0 * 30 + ia);
                kj += 1;
                zwt = true;
            } else if t_before > crr && t_after < crr && t0 > crr {
                nj[kj] = wrap_day(m0 * 30 + iz);
                kj += 1;
                zwt = false;
            }
        }
    }

    if zwt && kj > 0 {
        nj[..kj].rotate_left(1);
    }

    let np = kj / 2;
    let mut intervals = [(0_i32, 0_i32); 6];
    for i in 0..np {
        intervals[i] = (nj[2 * i], nj[2 * i + 1]);
    }
    (np, intervals)
}

/// Finds crossing intervals above `crr` (`5.0` or `8.0`) on a local `temp` copy.
fn find_threshold_intervals(
    temp: &mut [f64; 12],
    crr: f64,
    perturb: f64,
    ia: i32,
    iz: i32,
) -> (usize, [(i32, i32); 6]) {
    let t_jan = temp[0];
    for it in 0..11 {
        if temp[it] == crr && temp[it + 1] == crr {
            temp[it] = perturb;
        }
    }
    if temp[11] == crr && t_jan == crr {
        temp[11] = perturb;
    }

    let mut nj = [0_i32; 12];
    let mut kj = 0_usize;
    let mut zwt = false;

    for i19 in 1..=12_i32 {
        let m0 = if i19 == 1 { 12 } else { i19 - 1 };
        let m1 = i19;
        let m2 = if i19 == 12 { 1 } else { i19 + 1 };

        let t0 = temp[(m0 - 1) as usize];
        let t1 = temp[(m1 - 1) as usize];
        let t2 = temp[(m2 - 1) as usize];

        let rising = (t1 < crr && t2 > crr) || (t0 < crr && t1 == crr && t2 > crr);
        let falling = (t2 < crr && t1 > crr) || (t0 > crr && t2 < crr && t1 == crr);

        if rising {
            let mut day = m0 * 30 + ia + (30.0 * (crr - t1) / (t2 - t1)) as i32;
            if day > 360 {
                day -= 360;
            }
            nj[kj] = day;
            kj += 1;
            zwt = true;
        } else if falling {
            let mut day = m0 * 30 + iz + (30.0 * (t1 - crr) / (t1 - t2)) as i32;
            if day > 360 {
                day -= 360;
            }
            nj[kj] = day;
            kj += 1;
            zwt = false;
        }
    }

    if zwt && kj > 0 {
        nj[..kj].rotate_left(1);
    }

    let np = kj / 2;
    let mut intervals = [(0_i32, 0_i32); 6];
    for i in 0..np {
        intervals[i] = (nj[2 * i], nj[2 * i + 1]);
    }
    (np, intervals)
}

#[inline]
fn interval_len(beg: i32, end: i32) -> i32 {
    if beg < end {
        end - beg + 1
    } else {
        361 - beg + end
    }
}

#[inline]
fn wrap_day(day: i32) -> i32 {
    if day > 360 {
        day - 360
    } else {
        day
    }
}

/// Replicates Java's exact consecutive-day scan (`iday[si]` + lookahead `iday[n1]`).
fn consecutive_iday_run(
    iday: &[u8; 360],
    start_1based: i32,
    len: i32,
    target: u8,
    match_eq: bool,
) -> i32 {
    let mut count = 0_i32;
    let mut ifin = 0_i32;
    let mut sw = false;
    let mut max = 0_i32;
    let end = start_1based + len - 1;

    for i in start_1based..=end {
        let mut n1 = i + 1;
        while n1 > 360 {
            n1 -= 360;
        }
        let si = wrap_day(i);
        let cur = iday[(si - 1) as usize];
        let next = iday[(n1 - 1) as usize];

        if match_eq {
            if cur == target {
                if cur != next {
                    if sw {
                        count += 1;
                        if count > max {
                            max = count;
                        }
                        count = 0;
                        sw = false;
                    }
                } else {
                    count += 1;
                    sw = true;
                }
            }
        } else if cur != target {
            if next == target {
                if sw {
                    count += 1;
                    if count > max {
                        max = count;
                    }
                    count = 0;
                    sw = false;
                }
            } else {
                count += 1;
                sw = true;
            }
        }
    }

    if sw {
        ifin = count;
    }
    if ifin > max {
        max = ifin;
    }
    max
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn classify_smr(
    swt_nonzero: bool,
    nsd_dry: f64,
    lt5c: f64,
    ncpm2: i32,
    nd: [f64; 3],
    tma: f64,
    dif: f64,
    nccd: i32,
    nccm: i32,
    trr: Option<TemperatureRegime>,
) -> (MoistureRegime, RegimeSubdivision1, RegimeSubdivision2) {
    // Defect #8: regime_subdivision1 holds Java `q`; regime_subdivision2 holds Java `div`.
    if swt_nonzero {
        (
            MoistureRegime::Perudic,
            RegimeSubdivision1::Undefined,
            RegimeSubdivision2::Undefined,
        )
    } else if nsd_dry > lt5c / 2.0 && ncpm2 < 90 {
        let q = if nd[0] == 360.0 {
            RegimeSubdivision1::Extreme
        } else if ncpm2 <= 45 {
            RegimeSubdivision1::Typic
        } else {
            RegimeSubdivision1::Weak
        };
        (MoistureRegime::Aridic, q, RegimeSubdivision2::Aridic)
    } else if tma < 22.0 && dif >= 5.0 && nccd >= 45 && nccm >= 45 {
        let q = if nccd > 90 {
            RegimeSubdivision1::Dry
        } else {
            RegimeSubdivision1::Typic
        };
        (MoistureRegime::Xeric, q, RegimeSubdivision2::Xeric)
    } else if nd[0] + nd[1] < 90.0 {
        if nd[0] + nd[1] < 30.0 {
            (
                MoistureRegime::Udic,
                RegimeSubdivision1::Typic,
                RegimeSubdivision2::Udic,
            )
        } else if dif < 5.0 {
            (
                MoistureRegime::Udic,
                RegimeSubdivision1::Dry,
                RegimeSubdivision2::Tropudic,
            )
        } else {
            (
                MoistureRegime::Udic,
                RegimeSubdivision1::Dry,
                RegimeSubdivision2::Tempudic,
            )
        }
    } else if !matches!(
        trr,
        Some(TemperatureRegime::Pergelic | TemperatureRegime::Cryic)
    ) {
        if dif >= 5.0 {
            let q = if nccm <= 45 {
                RegimeSubdivision1::Typic
            } else if nccd > 45 {
                RegimeSubdivision1::Xeric
            } else {
                RegimeSubdivision1::Wet
            };
            (MoistureRegime::Ustic, q, RegimeSubdivision2::Tempustic)
        } else {
            let q = if ncpm2 < 180 {
                RegimeSubdivision1::Aridic
            } else if ncpm2 < 270 {
                RegimeSubdivision1::Typic
            } else {
                RegimeSubdivision1::Udic
            };
            (MoistureRegime::Ustic, q, RegimeSubdivision2::Tropustic)
        }
    } else {
        (
            MoistureRegime::Undefined,
            RegimeSubdivision1::Undefined,
            RegimeSubdivision2::Undefined,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> SiteInput {
        SiteInput {
            lat_dd: 40.0,
            lon_dd: -77.0,
            temp_c: [
                0.0, 1.0, 5.0, 10.0, 15.0, 20.0, 22.0, 21.0, 16.0, 10.0, 4.0, 1.0,
            ],
            precip_mm: [80.0; 12],
            awc_mm: 200.0,
            elev_m: 300.0,
        }
    }

    fn dry_water_balance() -> WaterBalance {
        WaterBalance {
            mpe: [100.0; 12],
            annual_rainfall: 960.0,
            annual_pet: 1200.0,
            annual_water_balance: -240.0,
            summer_water_balance: -60.0,
        }
    }

    fn uniform_soil(state: u8) -> SoilState {
        let mut nd = [0.0; 3];
        nd[(state - 1) as usize] = 360.0;
        SoilState {
            profile: [0.0; 64],
            iday: [state; 360],
            nd,
            ..SoilState::default()
        }
    }

    #[test]
    fn str_all_regimes_cryic_ht_and_pergelic_overwrite_and_nan() {
        let cfg = SiteConfig::default(); // fc=2.5, fcd=0.66, O=false, sat=false => cryic_ht=15

        // Pergelic survives ONLY when tma < 0 AND 0 <= st < cryic_ht
        let mut inp = base_input();
        // tma = -1.0 (air mean -3.5 + 2.5); summer mean air = 0.0 => st = 2.5 in [0, 15)
        inp.temp_c = [
            -8.0, -8.0, -5.0, -3.0, -1.0, 0.0, 0.0, 0.0, -2.0, -4.0, -5.0, -6.0,
        ];
        let (trr, _, _, _) = classify_str(&inp.temp_c, Hemisphere::North, &cfg);
        assert_eq!(trr, Some(TemperatureRegime::Pergelic));

        // Defect #3: tma < 0 and st < 0 => Frigid overwrites Pergelic when dif*fcd >= 6
        inp.temp_c = [
            -20.0, -20.0, -15.0, -10.0, -8.0, -5.0, -5.0, -5.0, -10.0, -12.0, -15.0, -20.0,
        ];
        // dif = 15 * 0.66 = 9.9 >= 6, st = -2.5 < 0
        let (trr, _, _, _) = classify_str(&inp.temp_c, Hemisphere::North, &cfg);
        assert_eq!(trr, Some(TemperatureRegime::Frigid));

        // Defect #3: tma < 0 and st < 0 and dif*fcd < 6 => Isofrigid overwrites Pergelic
        inp.temp_c = [-5.0; 12]; // st = -2.5 < 0, dif = 0
        let (trr, _, _, _) = classify_str(&inp.temp_c, Hemisphere::North, &cfg);
        assert_eq!(trr, Some(TemperatureRegime::Isofrigid));

        // Cryic thresholds: cryic_ht = 15, 13, 8, 6
        inp.temp_c = [
            0.0, 0.0, 2.0, 3.0, 5.0, 10.0, 10.0, 10.0, 5.0, 3.0, 2.0, 0.0,
        ];
        // tma = 5.833, st = 12.5, dif = 10 => dif*0.66 = 6.6 >= 6
        assert_eq!(
            classify_str(&inp.temp_c, Hemisphere::North, &cfg).0,
            Some(TemperatureRegime::Cryic)
        );
        let cfg_sat = SiteConfig {
            is_saturated: true,
            ..cfg
        };
        assert_eq!(
            classify_str(&inp.temp_c, Hemisphere::North, &cfg_sat).0,
            Some(TemperatureRegime::Cryic) // st=12.5 < 13
        );
        let cfg_o = SiteConfig {
            has_o_horizon: true,
            ..cfg
        };
        assert_eq!(
            classify_str(&inp.temp_c, Hemisphere::North, &cfg_o).0,
            Some(TemperatureRegime::Frigid) // st=12.5 >= 8
        );
        let cfg_o_sat = SiteConfig {
            has_o_horizon: true,
            is_saturated: true,
            ..cfg
        };
        inp.temp_c = [
            -1.0, -1.0, 0.0, 1.0, 2.0, 5.0, 5.0, 5.0, 2.0, 1.0, 0.0, -1.0,
        ]; // st=7.5 >= 6
        assert_eq!(
            classify_str(&inp.temp_c, Hemisphere::North, &cfg_o_sat).0,
            Some(TemperatureRegime::Isofrigid) // dif=6*0.66=3.96 < 6, st=7.5 >= 6
        );
        inp.temp_c = [
            -1.0, -1.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0, 2.0, 1.0, 0.0, -1.0,
        ]; // st=5.5 < 6
        assert_eq!(
            classify_str(&inp.temp_c, Hemisphere::North, &cfg_o_sat).0,
            Some(TemperatureRegime::Cryic)
        );

        // Mesic / Thermic / Hyperthermic and Iso- counterparts
        let seasonal_offset = [
            -6.0, -6.0, 0.0, 0.0, 0.0, 6.0, 6.0, 6.0, 0.0, 0.0, 0.0, -6.0,
        ];
        let iso_offset = [0.0; 12];
        for (base_air, expected_reg, expected_iso) in [
            (8.0, TemperatureRegime::Mesic, TemperatureRegime::Isomesic),
            (
                15.0,
                TemperatureRegime::Thermic,
                TemperatureRegime::Isothermic,
            ),
            (
                22.0,
                TemperatureRegime::Hyperthermic,
                TemperatureRegime::Isohyperthermic,
            ),
        ] {
            let mut t_reg = [0.0; 12];
            let mut t_iso = [0.0; 12];
            for i in 0..12 {
                t_reg[i] = base_air + seasonal_offset[i];
                t_iso[i] = base_air + iso_offset[i];
            }
            assert_eq!(
                classify_str(&t_reg, Hemisphere::North, &cfg).0,
                Some(expected_reg)
            );
            assert_eq!(
                classify_str(&t_iso, Hemisphere::North, &cfg).0,
                Some(expected_iso)
            );
        }

        // Defect #9: NaN in temperature yields None ("")
        let mut nan_temp = [10.0; 12];
        nan_temp[3] = f64::NAN;
        assert_eq!(classify_str(&nan_temp, Hemisphere::North, &cfg).0, None);
    }

    #[test]
    fn smr_branches_and_subdivisions_and_nan_perudic() {
        // Perudic when all mpe <= precip, and also when precip contains NaN (Defect #9)
        let mut inp = base_input();
        inp.precip_mm = [200.0; 12];
        let wb = dry_water_balance(); // mpe = 100
        let res = ClassicThermal.evaluate(&inp, &SiteConfig::default(), &wb, &uniform_soil(3));
        assert_eq!(res.moisture_regime, MoistureRegime::Perudic);
        assert_eq!(res.regime_subdivision1, RegimeSubdivision1::Undefined);
        assert_eq!(res.regime_subdivision2, RegimeSubdivision2::Undefined);

        let mut nan_precip = base_input();
        nan_precip.precip_mm = [f64::NAN; 12];
        let res_nan =
            ClassicThermal.evaluate(&nan_precip, &SiteConfig::default(), &wb, &uniform_soil(1));
        assert_eq!(res_nan.moisture_regime, MoistureRegime::Perudic);

        // Aridic: Extreme (nd[0]==360), Typic (ncpm2<=45), Weak (>45)
        assert_eq!(
            classify_smr(
                false,
                200.0,
                300.0,
                0,
                [360.0, 0.0, 0.0],
                15.0,
                10.0,
                100,
                0,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Aridic,
                RegimeSubdivision1::Extreme,
                RegimeSubdivision2::Aridic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                200.0,
                300.0,
                45,
                [250.0, 50.0, 60.0],
                15.0,
                10.0,
                80,
                20,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Aridic,
                RegimeSubdivision1::Typic,
                RegimeSubdivision2::Aridic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                200.0,
                300.0,
                89,
                [250.0, 50.0, 60.0],
                15.0,
                10.0,
                80,
                20,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Aridic,
                RegimeSubdivision1::Weak,
                RegimeSubdivision2::Aridic
            )
        );

        // Xeric uses unattenuated dif >= 5.0 (Defect #4) and q/div swap (Defect #8)
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                100,
                [100.0, 20.0, 240.0],
                15.0,
                5.0,
                91,
                50,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Xeric,
                RegimeSubdivision1::Dry,
                RegimeSubdivision2::Xeric
            )
        );
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                100,
                [100.0, 20.0, 240.0],
                15.0,
                5.0,
                45,
                45,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Xeric,
                RegimeSubdivision1::Typic,
                RegimeSubdivision2::Xeric
            )
        );

        // Udic: Typic/Udic, Dry/Tropudic, Dry/Tempudic
        assert_eq!(
            classify_smr(
                false,
                0.0,
                300.0,
                200,
                [10.0, 10.0, 340.0],
                15.0,
                10.0,
                10,
                100,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Udic,
                RegimeSubdivision1::Typic,
                RegimeSubdivision2::Udic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                0.0,
                300.0,
                200,
                [50.0, 20.0, 290.0],
                25.0,
                3.0,
                20,
                60,
                Some(TemperatureRegime::Isohyperthermic)
            ),
            (
                MoistureRegime::Udic,
                RegimeSubdivision1::Dry,
                RegimeSubdivision2::Tropudic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                0.0,
                300.0,
                200,
                [50.0, 20.0, 290.0],
                15.0,
                6.0,
                40,
                50,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Udic,
                RegimeSubdivision1::Dry,
                RegimeSubdivision2::Tempudic
            )
        );

        // Ustic: Tempustic (Typic/Xeric/Wet) and Tropustic (Aridic/Typic/Udic)
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                120,
                [100.0, 40.0, 220.0],
                15.0,
                8.0,
                50,
                30,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Ustic,
                RegimeSubdivision1::Typic,
                RegimeSubdivision2::Tempustic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                120,
                [100.0, 40.0, 220.0],
                23.0,
                8.0,
                50,
                60,
                Some(TemperatureRegime::Hyperthermic)
            ),
            (
                MoistureRegime::Ustic,
                RegimeSubdivision1::Xeric,
                RegimeSubdivision2::Tempustic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                120,
                [100.0, 40.0, 220.0],
                15.0,
                8.0,
                30,
                60,
                Some(TemperatureRegime::Mesic)
            ),
            (
                MoistureRegime::Ustic,
                RegimeSubdivision1::Wet,
                RegimeSubdivision2::Tempustic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                150,
                [100.0, 40.0, 220.0],
                24.0,
                2.0,
                50,
                60,
                Some(TemperatureRegime::Isohyperthermic)
            ),
            (
                MoistureRegime::Ustic,
                RegimeSubdivision1::Aridic,
                RegimeSubdivision2::Tropustic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                200,
                [100.0, 40.0, 220.0],
                24.0,
                2.0,
                50,
                60,
                Some(TemperatureRegime::Isohyperthermic)
            ),
            (
                MoistureRegime::Ustic,
                RegimeSubdivision1::Typic,
                RegimeSubdivision2::Tropustic
            )
        );
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                270,
                [100.0, 40.0, 220.0],
                24.0,
                2.0,
                50,
                60,
                Some(TemperatureRegime::Isohyperthermic)
            ),
            (
                MoistureRegime::Ustic,
                RegimeSubdivision1::Udic,
                RegimeSubdivision2::Tropustic
            )
        );

        // Undefined for Cryic / Pergelic
        assert_eq!(
            classify_smr(
                false,
                100.0,
                300.0,
                120,
                [100.0, 40.0, 220.0],
                4.0,
                8.0,
                30,
                60,
                Some(TemperatureRegime::Cryic)
            ),
            (
                MoistureRegime::Undefined,
                RegimeSubdivision1::Undefined,
                RegimeSubdivision2::Undefined
            )
        );
    }

    #[test]
    fn perturbation_5_and_8_and_bimodal_has_run_already() {
        let cfg = SiteConfig::default();
        let wb = dry_water_balance();
        let mut soil = SoilState::default();
        for d in 0..360 {
            soil.iday[d] = if d < 180 { 3 } else { 1 };
        }
        soil.nd = [180.0, 0.0, 180.0];

        // Consecutive 5.0 and 8.0 months (including Dec->Jan wrap on 8.0)
        let mut inp = base_input();
        inp.temp_c = [
            8.0, 4.0, 5.0, 5.0, 10.0, 12.0, 14.0, 12.0, 8.0, 8.0, 4.0, 8.0,
        ];
        let snapshot = inp.temp_c;
        let out = ClassicThermal.evaluate(&inp, &cfg, &wb, &soil);
        // Caller input must remain unmutated (Defect #10)
        assert_eq!(inp.temp_c, snapshot);

        // Must match explicitly pre-perturbed 5.01 / 8.01 months
        let mut perturbed = inp;
        perturbed.temp_c = [
            8.0, 4.0, 5.01, 5.0, 10.0, 12.0, 14.0, 12.0, 8.01, 8.0, 4.0, 8.01,
        ];
        let out_perturbed = ClassicThermal.evaluate(&perturbed, &cfg, &wb, &soil);
        assert_eq!(out.day_counters, out_perturbed.day_counters);

        // Bimodal >5°C curve: has_run_already counts nsd from first interval only (Defect #6)
        let mut bimodal = base_input();
        bimodal.temp_c = [2.0, 8.0, 8.0, 2.0, 2.0, 2.0, 8.0, 8.0, 2.0, 2.0, 2.0, 2.0];
        let res_bi = ClassicThermal.evaluate(&bimodal, &cfg, &wb, &soil);
        let sum_nsd = res_bi.day_counters.num_cumulative_days_dry_over_5c
            + res_bi.day_counters.num_cumulative_days_moist_dry_over_5c
            + res_bi.day_counters.num_cumulative_days_moist_over_5c;
        // First interval is spring (days ~51..110, all iday=3), second is summer (iday=1/3).
        assert!(sum_nsd > 0 && sum_nsd < 120);
        assert_eq!(res_bi.day_counters.num_cumulative_days_dry_over_5c, 0);
    }

    #[test]
    fn cold_site_ncpm2_fallback_and_solstice_and_is_metric_noop() {
        let wb = dry_water_balance();
        let mut soil = SoilState::default();
        // Moist days 1..100, Dry days 101..360 (consecutive moist run = 100)
        for d in 0..360 {
            soil.iday[d] = if d < 100 { 3 } else { 1 };
        }
        soil.nd = [260.0, 0.0, 100.0];

        // All months < 8.0°C (np8 == 0) => Defect #5 sets ncpm[2] = ncpm[1] = 100
        let mut cold = base_input();
        cold.temp_c = [
            -5.0, -3.0, 0.0, 2.0, 4.0, 6.0, 7.0, 6.0, 4.0, 2.0, 0.0, -3.0,
        ];
        let cfg_metric = SiteConfig::default();
        let cfg_non_metric = SiteConfig {
            is_metric: false,
            ..cfg_metric
        };

        let out_m = ClassicThermal.evaluate(&cold, &cfg_metric, &wb, &soil);
        let out_nm = ClassicThermal.evaluate(&cold, &cfg_non_metric, &wb, &soil);
        assert_eq!(out_m, out_nm); // Defect #2: is_metric ignored
        assert_eq!(
            out_m.day_counters.num_consecutive_days_moist_in_some_parts,
            100
        );
        assert_eq!(
            out_m
                .day_counters
                .num_consecutive_days_moist_in_some_parts_over_8c,
            100
        );

        // NH solstice windows: summer = 181..300 (all dry=1 -> nccd=120), winter = 1..120 (100 moist -> nccm=100)
        assert_eq!(out_m.day_counters.dry_days_after_summer_solstice, 120);
        assert_eq!(out_m.day_counters.moist_days_after_winter_solstice, 100);

        // SH solstice windows flip summer/winter starts (summer=1..120, winter=181..300)
        cold.lat_dd = -35.0;
        let out_sh = ClassicThermal.evaluate(&cold, &cfg_metric, &wb, &soil);
        // Days 1..100 are moist (3), days 101..120 are dry (1) (20 days ending at 120, next day 121 is dry too -> 20)
        assert_eq!(out_sh.day_counters.dry_days_after_summer_solstice, 20);
        // Days 181..300 are all dry (1) -> nccm = 0
        assert_eq!(out_sh.day_counters.moist_days_after_winter_solstice, 0);

        // Verify temp_under_five && np == 0 && wt > 5.0 falls through to nsd = nd
        let mut mild_cold = cold;
        mild_cold.temp_c = [4.0; 12]; // temp < 5.0, np == 0, wt = 4.0 + 2.5 = 6.5 > 5.0
        let out_mild = ClassicThermal.evaluate(&mild_cold, &cfg_metric, &wb, &soil);
        assert_eq!(out_mild.day_counters.num_cumulative_days_dry_over_5c, 260);
        assert_eq!(out_mild.day_counters.num_cumulative_days_moist_over_5c, 100);
    }

    #[test]
    fn strict_thermal_defect_2_unit_conversion() {
        let wb = dry_water_balance();
        let mut soil = SoilState::default();
        for d in 0..360 {
            soil.iday[d] = if d < 180 { 3 } else { 1 };
        }
        soil.nd = [180.0, 0.0, 180.0];

        let metric_inp = base_input();
        let metric_cfg = SiteConfig::default();

        let mut imperial_inp = metric_inp;
        for i in 0..12 {
            imperial_inp.temp_c[i] = metric_inp.temp_c[i] * 9.0 / 5.0 + 32.0;
            imperial_inp.precip_mm[i] = metric_inp.precip_mm[i] / 25.4;
        }
        let imperial_cfg = SiteConfig {
            soil_air_offset: metric_cfg.soil_air_offset * 9.0 / 5.0,
            is_metric: false,
            ..metric_cfg
        };

        let out_metric = StrictThermal.evaluate(&metric_inp, &metric_cfg, &wb, &soil);
        let out_imperial = StrictThermal.evaluate(&imperial_inp, &imperial_cfg, &wb, &soil);
        assert_eq!(out_metric, out_imperial);
    }

    #[test]
    fn strict_thermal_defect_3_pergelic_not_overwritten() {
        let cfg = SiteConfig::default();
        let wb = dry_water_balance();
        let soil = uniform_soil(3);

        // tma < 0 and st < 0 with dif*fcd >= 6 => Classic gives Frigid; Strict gives Pergelic.
        let mut inp_frigid = base_input();
        inp_frigid.temp_c = [
            -20.0, -20.0, -15.0, -10.0, -8.0, -5.0, -5.0, -5.0, -10.0, -12.0, -15.0, -20.0,
        ];
        assert_eq!(
            ClassicThermal
                .evaluate(&inp_frigid, &cfg, &wb, &soil)
                .temperature_regime,
            Some(TemperatureRegime::Frigid)
        );
        assert_eq!(
            StrictThermal
                .evaluate(&inp_frigid, &cfg, &wb, &soil)
                .temperature_regime,
            Some(TemperatureRegime::Pergelic)
        );

        // tma < 0 and st < 0 with dif*fcd < 6 => Classic gives Isofrigid; Strict gives Pergelic.
        let mut inp_iso = base_input();
        inp_iso.temp_c = [-5.0; 12];
        assert_eq!(
            ClassicThermal
                .evaluate(&inp_iso, &cfg, &wb, &soil)
                .temperature_regime,
            Some(TemperatureRegime::Isofrigid)
        );
        assert_eq!(
            StrictThermal
                .evaluate(&inp_iso, &cfg, &wb, &soil)
                .temperature_regime,
            Some(TemperatureRegime::Pergelic)
        );

        // tma < 0 and st >= cryic_ht (e.g. O-horizon + saturated => cryic_ht = 6.0, st = 6.5).
        let cfg_o_sat = SiteConfig {
            has_o_horizon: true,
            is_saturated: true,
            ..cfg
        };
        let mut inp_high_summer = base_input();
        inp_high_summer.temp_c = [
            -10.0, -10.0, -6.0, -4.0, -2.0, 4.0, 4.0, 4.0, -2.0, -4.0, -6.0, -10.0,
        ];
        assert_eq!(
            ClassicThermal
                .evaluate(&inp_high_summer, &cfg_o_sat, &wb, &soil)
                .temperature_regime,
            Some(TemperatureRegime::Frigid)
        );
        assert_eq!(
            StrictThermal
                .evaluate(&inp_high_summer, &cfg_o_sat, &wb, &soil)
                .temperature_regime,
            Some(TemperatureRegime::Pergelic)
        );
    }

    #[test]
    fn strict_thermal_defect_4_attenuated_abs_dif_in_smr() {
        let cfg = SiteConfig::default(); // amplitude = 0.66
        let wb = dry_water_balance();

        // Udic with 30 <= nd[0]+nd[1] < 90: st - wt = 6.0 (>= 5.0 unattenuated, 3.96 attenuated).
        let mut soil_udic = SoilState::default();
        for d in 0..360 {
            soil_udic.iday[d] = if d < 50 { 1 } else { 3 };
        }
        soil_udic.nd = [50.0, 0.0, 310.0];

        let mut inp = base_input();
        inp.temp_c = [
            10.0, 10.0, 13.0, 13.0, 13.0, 16.0, 16.0, 16.0, 13.0, 13.0, 13.0, 10.0,
        ];
        let classic_udic = ClassicThermal.evaluate(&inp, &cfg, &wb, &soil_udic);
        let strict_udic = StrictThermal.evaluate(&inp, &cfg, &wb, &soil_udic);
        assert_eq!(classic_udic.moisture_regime, MoistureRegime::Udic);
        assert_eq!(
            classic_udic.regime_subdivision2,
            RegimeSubdivision2::Tempudic
        );
        assert_eq!(strict_udic.moisture_regime, MoistureRegime::Udic);
        assert_eq!(
            strict_udic.regime_subdivision2,
            RegimeSubdivision2::Tropudic
        );
        assert_eq!(strict_udic.regime_subdivision1, RegimeSubdivision1::Dry);

        // Defect #4 + .abs(): when wt > st (st - wt = -10.0, |st - wt| * 0.66 = 6.6 >= 5.0).
        let mut inp_inverted = base_input();
        inp_inverted.temp_c = [
            22.0, 22.0, 17.0, 17.0, 17.0, 12.0, 12.0, 12.0, 17.0, 17.0, 17.0, 22.0,
        ];
        let classic_inv = ClassicThermal.evaluate(&inp_inverted, &cfg, &wb, &soil_udic);
        let strict_inv = StrictThermal.evaluate(&inp_inverted, &cfg, &wb, &soil_udic);
        assert_eq!(
            classic_inv.regime_subdivision2,
            RegimeSubdivision2::Tropudic
        );
        assert_eq!(strict_inv.regime_subdivision2, RegimeSubdivision2::Tempudic);
    }

    #[test]
    fn strict_thermal_defect_5_cold_site_ncpm2_zero() {
        let cfg = SiteConfig::default();
        let wb = dry_water_balance();
        let mut soil = SoilState::default();
        for d in 0..360 {
            soil.iday[d] = if d < 100 { 3 } else { 1 };
        }
        soil.nd = [260.0, 0.0, 100.0];

        // All months < 8.0°C (np8 == 0): Classic sets ncpm2 = ncpm1 = 100; Strict keeps ncpm2 = 0.
        let mut cold = base_input();
        cold.temp_c = [
            -5.0, -3.0, 0.0, 2.0, 4.0, 6.0, 7.0, 6.0, 4.0, 2.0, 0.0, -3.0,
        ];
        let classic_cold = ClassicThermal.evaluate(&cold, &cfg, &wb, &soil);
        let strict_cold = StrictThermal.evaluate(&cold, &cfg, &wb, &soil);
        assert_eq!(
            classic_cold
                .day_counters
                .num_consecutive_days_moist_in_some_parts_over_8c,
            100
        );
        assert_eq!(
            strict_cold
                .day_counters
                .num_consecutive_days_moist_in_some_parts,
            100
        );
        assert_eq!(
            strict_cold
                .day_counters
                .num_consecutive_days_moist_in_some_parts_over_8c,
            0
        );

        // Warm site (all months >= 8.0°C): Strict still sets ncpm2 = ncpm1.
        let mut warm = base_input();
        warm.temp_c = [15.0; 12];
        let strict_warm = StrictThermal.evaluate(&warm, &cfg, &wb, &soil);
        assert_eq!(
            strict_warm
                .day_counters
                .num_consecutive_days_moist_in_some_parts_over_8c,
            100
        );
    }

    #[test]
    fn strict_thermal_defect_6_bimodal_nsd_all_intervals() {
        let cfg = SiteConfig::default();
        let wb = dry_water_balance();
        let mut soil = SoilState::default();
        // First 180 days moist (3), second 180 days dry (1).
        for d in 0..360 {
            soil.iday[d] = if d < 180 { 3 } else { 1 };
        }
        soil.nd = [180.0, 0.0, 180.0];

        let mut bimodal = base_input();
        bimodal.temp_c = [2.0, 8.0, 8.0, 2.0, 2.0, 2.0, 8.0, 8.0, 2.0, 2.0, 2.0, 2.0];

        let classic_bi = ClassicThermal.evaluate(&bimodal, &cfg, &wb, &soil);
        let strict_bi = StrictThermal.evaluate(&bimodal, &cfg, &wb, &soil);

        // Classic ignores the second >5°C interval (in dry summer days 181..360).
        assert_eq!(classic_bi.day_counters.num_cumulative_days_dry_over_5c, 0);
        // Strict counts both intervals (spring moist + summer dry).
        assert!(strict_bi.day_counters.num_cumulative_days_dry_over_5c > 0);
        assert_eq!(
            strict_bi.day_counters.num_cumulative_days_moist_over_5c,
            classic_bi.day_counters.num_cumulative_days_moist_over_5c
        );
        let strict_sum = strict_bi.day_counters.num_cumulative_days_dry_over_5c
            + strict_bi.day_counters.num_cumulative_days_moist_over_5c;
        assert!(strict_sum > classic_bi.day_counters.num_cumulative_days_moist_over_5c);
    }

    #[test]
    fn strict_thermal_defect_9_nan_yields_undefined() {
        let cfg = SiteConfig::default();
        let wb = dry_water_balance();
        let soil = uniform_soil(1);

        // Single NaN in precip_mm on an otherwise wet site: Classic yields Perudic; Strict yields Undefined.
        let mut nan_precip = base_input();
        nan_precip.precip_mm = [200.0; 12];
        nan_precip.precip_mm[6] = f64::NAN;
        assert_eq!(
            ClassicThermal
                .evaluate(&nan_precip, &cfg, &wb, &soil)
                .moisture_regime,
            MoistureRegime::Perudic
        );
        let strict_p = StrictThermal.evaluate(&nan_precip, &cfg, &wb, &soil);
        assert_eq!(strict_p.moisture_regime, MoistureRegime::Undefined);
        assert_eq!(strict_p.regime_subdivision1, RegimeSubdivision1::Undefined);
        assert_eq!(strict_p.regime_subdivision2, RegimeSubdivision2::Undefined);

        // Single NaN in precip_mm on a dry site also yields Undefined in Strict.
        let mut nan_precip_dry = base_input();
        nan_precip_dry.precip_mm[6] = f64::NAN;
        let strict_p_dry = StrictThermal.evaluate(&nan_precip_dry, &cfg, &wb, &soil);
        assert_eq!(strict_p_dry.moisture_regime, MoistureRegime::Undefined);

        // NaN in temp_c: Strict yields temperature_regime=None and MoistureRegime::Undefined.
        let mut nan_temp = base_input();
        nan_temp.temp_c[2] = f64::NAN;
        let strict_t = StrictThermal.evaluate(&nan_temp, &cfg, &wb, &soil);
        assert_eq!(strict_t.temperature_regime, None);
        assert_eq!(strict_t.moisture_regime, MoistureRegime::Undefined);
        assert_eq!(strict_t.regime_subdivision1, RegimeSubdivision1::Undefined);
        assert_eq!(strict_t.regime_subdivision2, RegimeSubdivision2::Undefined);
    }

    #[test]
    fn strict_thermal_defect_10_exact_5_and_8_plateaus() {
        let cfg = SiteConfig {
            soil_air_offset: 0.0,
            ..SiteConfig::default()
        };
        let wb = dry_water_balance();
        let soil = uniform_soil(3);

        // [0, 5, 5, 0, ...]: never exceeds 5.0°C. Classic creates fake interval via 5.01; Strict yields 0.
        let mut touch_5 = base_input();
        touch_5.temp_c = [0.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let classic_touch = ClassicThermal.evaluate(&touch_5, &cfg, &wb, &soil);
        let strict_touch = StrictThermal.evaluate(&touch_5, &cfg, &wb, &soil);
        assert!(classic_touch.day_counters.num_cumulative_days_moist_over_5c > 0);
        assert_eq!(
            strict_touch.day_counters.num_cumulative_days_moist_over_5c,
            0
        );

        // [0, 8, 8, 0, ...]: never exceeds 8.0°C. Classic sets ncpm2 > 0 via 8.01; Strict yields 0.
        let mut touch_8 = base_input();
        touch_8.temp_c = [0.0, 8.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let classic_touch8 = ClassicThermal.evaluate(&touch_8, &cfg, &wb, &soil);
        let strict_touch8 = StrictThermal.evaluate(&touch_8, &cfg, &wb, &soil);
        assert!(
            classic_touch8
                .day_counters
                .num_consecutive_days_moist_in_some_parts_over_8c
                > 0
        );
        assert_eq!(
            strict_touch8
                .day_counters
                .num_consecutive_days_moist_in_some_parts_over_8c,
            0
        );

        // True crossing across multi-month 5.0 / 8.0 plateaus without mutation.
        let crossing = [0.0, 5.0, 5.0, 10.0, 10.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let (np, intervals) = find_threshold_intervals_strict(&crossing, 5.0, 36, 25);
        assert_eq!(np, 1);
        assert_eq!(intervals[0], (96, 175));

        // Dip touching 5.0 from above without crossing < 5.0, plus one true < 5.0 winter dip.
        let dip_touch = [
            0.0, 10.0, 5.0, 5.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 0.0,
        ];
        let (np_dip, intervals_dip) = find_threshold_intervals_strict(&dip_touch, 5.0, 36, 25);
        assert_eq!(np_dip, 1);
        assert!(interval_len(intervals_dip[0].0, intervals_dip[0].1) > 250);
    }
}
