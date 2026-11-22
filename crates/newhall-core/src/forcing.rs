//! Thornthwaite PET and atmospheric forcing (`jNSMR` / `newhall-1.6.5.jar` parity).

use crate::traits::ForcingModel;
use crate::types::{Hemisphere, SiteConfig, SiteInput, WaterBalance};

/// Unadjusted PET lookup values (mm) for temperatures in `[26.5, 38.0]` °C (`0.5` °C steps).
const ZPE: [f64; 24] = [
    135.0, 139.5, 143.7, 147.8, 151.7, 155.4, 158.9, 162.1, 165.2, 168.0, 170.7, 173.1, 175.3,
    177.2, 179.0, 180.5, 181.8, 182.9, 183.7, 184.3, 184.7, 184.9, 185.0, 185.0,
];

/// Temperature bin boundaries (°C) paired with [`ZPE`].
const ZT: [f64; 24] = [
    26.5, 27.0, 27.5, 28.0, 28.5, 29.0, 29.5, 30.0, 30.5, 31.0, 31.5, 32.0, 32.5, 33.0, 33.5, 34.0,
    34.5, 35.0, 35.5, 36.0, 36.5, 37.0, 37.5, 38.0,
];

/// Northern Hemisphere monthly daylength correction factors (`[month][lat_bin]`).
#[rustfmt::skip]
const INZ: [[f64; 31]; 12] = [
    [1.04, 1.02, 1.0, 0.97, 0.95, 0.93, 0.92, 0.92, 0.91, 0.91, 0.9, 0.9, 0.89, 0.88, 0.88, 0.87, 0.87, 0.86, 0.85, 0.85, 0.84, 0.83, 0.82, 0.81, 0.81, 0.8, 0.79, 0.77, 0.76, 0.75, 0.74],
    [0.94, 0.93, 0.91, 0.91, 0.9, 0.89, 0.88, 0.88, 0.88, 0.87, 0.87, 0.87, 0.86, 0.86, 0.85, 0.85, 0.85, 0.84, 0.84, 0.84, 0.83, 0.83, 0.83, 0.82, 0.82, 0.81, 0.81, 0.8, 0.8, 0.79, 0.78],
    [1.04, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.02, 1.02, 1.02, 1.02, 1.02, 1.02, 1.02, 1.02],
    [1.01, 1.02, 1.03, 1.04, 1.05, 1.06, 1.06, 1.07, 1.07, 1.07, 1.08, 1.08, 1.08, 1.09, 1.09, 1.09, 1.1, 1.1, 1.1, 1.11, 1.11, 1.11, 1.12, 1.12, 1.13, 1.13, 1.13, 1.14, 1.14, 1.14, 1.15],
    [1.04, 1.06, 1.08, 1.11, 1.13, 1.15, 1.15, 1.16, 1.16, 1.17, 1.18, 1.18, 1.19, 1.19, 1.2, 1.21, 1.21, 1.22, 1.23, 1.23, 1.24, 1.25, 1.26, 1.26, 1.27, 1.28, 1.29, 1.3, 1.31, 1.32, 1.33],
    [1.01, 1.03, 1.06, 1.08, 1.11, 1.14, 1.15, 1.15, 1.16, 1.16, 1.17, 1.18, 1.19, 1.2, 1.2, 1.21, 1.22, 1.23, 1.24, 1.24, 1.25, 1.26, 1.27, 1.28, 1.29, 1.29, 1.31, 1.32, 1.33, 1.34, 1.36],
    [1.04, 1.06, 1.08, 1.12, 1.14, 1.17, 1.17, 1.18, 1.18, 1.19, 1.2, 1.2, 1.21, 1.22, 1.22, 1.23, 1.24, 1.25, 1.25, 1.26, 1.27, 1.27, 1.28, 1.29, 1.3, 1.31, 1.32, 1.33, 1.34, 1.35, 1.37],
    [1.04, 1.05, 1.07, 1.08, 1.11, 1.12, 1.12, 1.13, 1.13, 1.13, 1.14, 1.14, 1.15, 1.15, 1.16, 1.16, 1.16, 1.17, 1.17, 1.18, 1.18, 1.19, 1.19, 1.2, 1.2, 1.21, 1.22, 1.22, 1.23, 1.24, 1.25],
    [1.01, 1.01, 1.02, 1.02, 1.02, 1.02, 1.02, 1.02, 1.02, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.03, 1.04, 1.04, 1.04, 1.04, 1.04, 1.04, 1.04, 1.04, 1.04, 1.04, 1.05, 1.05, 1.06],
    [1.04, 1.03, 1.02, 1.01, 1.0, 0.99, 0.99, 0.99, 0.98, 0.98, 0.98, 0.98, 0.98, 0.97, 0.97, 0.97, 0.97, 0.97, 0.96, 0.96, 0.96, 0.96, 0.95, 0.95, 0.95, 0.94, 0.94, 0.93, 0.93, 0.93, 0.92],
    [1.01, 0.99, 0.98, 0.95, 0.93, 0.91, 0.91, 0.9, 0.9, 0.9, 0.89, 0.89, 0.88, 0.88, 0.87, 0.86, 0.86, 0.85, 0.84, 0.84, 0.83, 0.82, 0.82, 0.81, 0.8, 0.79, 0.79, 0.78, 0.77, 0.76, 0.76],
    [1.04, 1.02, 0.99, 0.97, 0.94, 0.91, 0.91, 0.9, 0.9, 0.89, 0.88, 0.88, 0.87, 0.86, 0.86, 0.85, 0.84, 0.83, 0.83, 0.82, 0.81, 0.8, 0.79, 0.77, 0.76, 0.75, 0.74, 0.73, 0.72, 0.71, 0.7],
];

/// Northern Hemisphere latitude bin thresholds (degrees) paired with [`INZ`].
const RN: [f64; 31] = [
    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0,
    36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 48.0, 49.0, 50.0,
];

/// Southern Hemisphere monthly daylength correction factors (`[month][lat_bin]`).
#[rustfmt::skip]
const FS: [[f64; 13]; 12] = [
    [1.06, 1.08, 1.12, 1.14, 1.17, 1.2, 1.23, 1.27, 1.28, 1.3, 1.32, 1.34, 1.37],
    [0.95, 0.97, 0.98, 1.0, 1.01, 1.03, 1.04, 1.06, 1.07, 1.08, 1.1, 1.11, 1.12],
    [1.04, 1.05, 1.05, 1.05, 1.05, 1.06, 1.06, 1.07, 1.07, 1.07, 1.07, 1.08, 1.08],
    [1.0, 0.99, 0.98, 0.97, 0.96, 0.95, 0.94, 0.93, 0.92, 0.92, 0.91, 0.9, 0.89],
    [1.02, 1.01, 0.98, 0.96, 0.94, 0.92, 0.89, 0.86, 0.85, 0.83, 0.82, 0.8, 0.77],
    [0.99, 0.96, 0.94, 0.91, 0.88, 0.85, 0.82, 0.78, 0.76, 0.74, 0.72, 0.7, 0.67],
    [1.02, 1.0, 0.97, 0.95, 0.93, 0.9, 0.87, 0.84, 0.82, 0.81, 0.79, 0.76, 0.74],
    [1.03, 1.01, 1.0, 0.99, 0.98, 0.96, 0.94, 0.92, 0.92, 0.91, 0.9, 0.89, 0.88],
    [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.99, 0.99, 0.99, 0.99],
    [1.05, 1.06, 1.07, 1.08, 1.1, 1.12, 1.13, 1.15, 1.16, 1.17, 1.17, 1.18, 1.19],
    [1.03, 1.05, 1.07, 1.09, 1.11, 1.14, 1.17, 1.2, 1.22, 1.23, 1.25, 1.27, 1.29],
    [1.06, 1.1, 1.12, 1.15, 1.18, 1.21, 1.25, 1.29, 1.31, 1.33, 1.35, 1.37, 1.41],
];

/// Southern Hemisphere latitude bin thresholds (degrees) paired with [`FS`].
const RS: [f64; 13] = [
    5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 42.0, 44.0, 46.0, 48.0, 50.0,
];

/// Classic Thornthwaite PET and daylength forcing model matching `BASICSimulationModel`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThornthwaiteForcing;

impl ForcingModel for ThornthwaiteForcing {
    fn compute_water_balance(&self, input: &SiteInput, _config: &SiteConfig) -> WaterBalance {
        // Defect #2: `config.is_metric` is intentionally ignored in `runSimulation(double[], ...)`.
        compute_with_hemisphere(input, input.hemisphere())
    }
}

/// Computes Thornthwaite PET and seasonal/annual water balances for an explicit hemisphere.
fn compute_with_hemisphere(input: &SiteInput, hemisphere: Hemisphere) -> WaterBalance {
    let upe = compute_unadjusted_pet(&input.temp_c);
    let mpe = apply_daylength_correction(&upe, input.lat_dd, hemisphere);

    let mut annual_rainfall = 0.0_f64;
    let mut annual_pet = 0.0_f64;
    for (p, e) in input.precip_mm.iter().zip(mpe.iter()) {
        annual_rainfall += p;
        annual_pet += e;
    }

    let mut annual_water_balance = 0.0_f64;
    for (p, e) in input.precip_mm.iter().zip(mpe.iter()) {
        annual_water_balance += p;
        annual_water_balance -= e;
    }

    let summer_water_balance = compute_summer_water_balance(&input.precip_mm, &mpe, hemisphere);

    WaterBalance {
        mpe,
        annual_rainfall,
        annual_pet,
        annual_water_balance,
        summer_water_balance,
    }
}

/// Computes unadjusted monthly PET `upe[0..12]` from monthly air temperature (°C).
fn compute_unadjusted_pet(temp_c: &[f64; 12]) -> [f64; 12] {
    let mut mwi = [0.0_f64; 12];
    for j in 0..12 {
        if temp_c[j] > 0.0 {
            mwi[j] = (temp_c[j] / 5.0).powf(1.514);
        }
    }

    let mut swi = 0.0_f64;
    for &elem in &mwi {
        swi += elem;
    }

    let a = 6.75e-7 * swi * swi * swi - 7.71e-5 * swi * swi + 0.01792 * swi + 0.49239;

    let mut upe = [0.0_f64; 12];
    for i in 0..12 {
        let t = temp_c[i];
        if t > 0.0 {
            if t < 26.5 {
                let a_base = 10.0 * t / swi;
                upe[i] = 16.0 * a_base.powf(a);
            } else if t >= 38.0 {
                upe[i] = 185.0;
            } else {
                for ki in 0..23 {
                    if t >= ZT[ki] && t < ZT[ki + 1] {
                        upe[i] = ZPE[ki];
                        break;
                    }
                }
            }
        }
    }

    upe
}

/// Applies NH (`INZ`/`RN`) or SH (`FS`/`RS`) latitude daylength correction to `upe`.
fn apply_daylength_correction(upe: &[f64; 12], lat_dd: f64, hemisphere: Hemisphere) -> [f64; 12] {
    let mut mpe = [0.0_f64; 12];

    match hemisphere {
        Hemisphere::North => {
            let mut nrow = 0_usize;
            for &r in &RN {
                if lat_dd >= r {
                    nrow += 1;
                } else {
                    break;
                }
            }
            if nrow > 0 {
                let col = nrow - 1;
                for i in 0..12 {
                    if upe[i] > 0.0 {
                        mpe[i] = upe[i] * INZ[i][col];
                    }
                }
            }
        }
        Hemisphere::South => {
            let mut nrow = 0_usize;
            for &r in &RS {
                if lat_dd >= r {
                    nrow += 1;
                } else {
                    break;
                }
            }

            let lat_int_i32 = lat_dd as i32;
            let lat_int = lat_int_i32 as f64;

            if nrow != 0 {
                for i in 0..12 {
                    if upe[i] > 0.0 {
                        if nrow >= 13 || FS[i][nrow - 1] == FS[i][nrow] {
                            mpe[i] = upe[i] * FS[i][nrow - 1];
                        } else {
                            // Defect #1: missing parentheses around `(RS[nrow] - RS[nrow - 1]) * 60.0`
                            // replicates Java's left-to-right `/` then `* 60.0` precedence verbatim.
                            let mut cf = (FS[i][nrow] - FS[i][nrow - 1])
                                * ((lat_int - RS[nrow - 1]) * 60.0 + ((lat_dd - lat_int) * 60.0))
                                / (RS[nrow] - RS[nrow - 1])
                                * 60.0;
                            cf += FS[i][nrow - 1];
                            mpe[i] = upe[i] * cf;
                        }
                    }
                }
            } else {
                // Defect #7: signed `lat_dd` (when `< 0.0`) enters the 0..5 deg SH formula directly.
                let lat_minutes =
                    (lat_int_i32.wrapping_mul(60) as f64) + ((lat_dd - lat_int) * 60.0);
                for i in 0..12 {
                    if upe[i] > 0.0 {
                        let mut cf = (FS[i][0] - INZ[i][0]) * lat_minutes / 300.0;
                        cf += INZ[i][0];
                        mpe[i] = upe[i] * cf;
                    }
                }
            }
        }
    }

    mpe
}

/// Computes summer water balance (JJA for NH, DJF for SH) matching Java's summation order.
fn compute_summer_water_balance(
    precip_mm: &[f64; 12],
    mpe: &[f64; 12],
    hemisphere: Hemisphere,
) -> f64 {
    let mut running_balance = 0.0_f64;
    match hemisphere {
        Hemisphere::North => {
            for i in 5..=7 {
                running_balance += precip_mm[i];
                running_balance -= mpe[i];
            }
        }
        Hemisphere::South => {
            for i in [11_usize, 0, 1] {
                running_balance += precip_mm[i];
                running_balance -= mpe[i];
            }
        }
    }
    running_balance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_site(lat_dd: f64, temp_c: [f64; 12], precip_mm: [f64; 12]) -> SiteInput {
        SiteInput {
            lat_dd,
            lon_dd: 0.0,
            temp_c,
            precip_mm,
            awc_mm: 200.0,
            elev_m: 100.0,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-12,
            "expected {expected:.17}, got {actual:.17} (diff {diff:.3e})"
        );
    }

    #[test]
    fn thornthwaite_forcing_is_copy_and_default() {
        fn assert_copy_default<T: Copy + Default>() {}
        assert_copy_default::<ThornthwaiteForcing>();
    }

    #[test]
    fn northern_hemisphere_floor_bin_and_50_clamp() {
        let temps = [
            5.0, 5.0, 10.0, 15.0, 20.0, 20.0, 20.0, 20.0, 15.0, 10.0, 5.0, 5.0,
        ];
        let precip = [50.0; 12];
        let forcing = ThornthwaiteForcing;

        // 40.9 deg N floors to the 40.0 deg N bin (RN[20]), not 41.0.
        let site_40_9 = sample_site(40.9, temps, precip);
        let site_40_0 = sample_site(40.0, temps, precip);
        let wb_40_9 = forcing.compute_water_balance(&site_40_9, &SiteConfig::default());
        let wb_40_0 = forcing.compute_water_balance(&site_40_0, &SiteConfig::default());
        assert_eq!(wb_40_9, wb_40_0);

        // 62.5 deg N clamps to 50.0 deg N (RN[30]).
        let site_62_5 = sample_site(62.5, temps, precip);
        let site_50_0 = sample_site(50.0, temps, precip);
        let wb_62_5 = forcing.compute_water_balance(&site_62_5, &SiteConfig::default());
        let wb_50_0 = forcing.compute_water_balance(&site_50_0, &SiteConfig::default());
        assert_eq!(wb_62_5, wb_50_0);

        // Verify analytical reference for Jan (index 0) and Jul (index 6) at 40.0 N.
        let upe = compute_unadjusted_pet(&temps);
        assert_close(wb_40_0.mpe[0], upe[0] * INZ[0][20]);
        assert_close(wb_40_0.mpe[6], upe[6] * INZ[6][20]);
        assert_close(wb_40_0.annual_rainfall, 600.0);
        let expected_jja =
            (50.0 - wb_40_0.mpe[5]) + (50.0 - wb_40_0.mpe[6]) + (50.0 - wb_40_0.mpe[7]);
        assert_close(wb_40_0.summer_water_balance, expected_jja);
    }

    #[test]
    fn southern_hemisphere_0_to_5_and_signed_negative_latitude() {
        let temps = [20.0; 12];
        let precip = [100.0; 12];

        // Positive 2.5 deg passed explicitly to SH branch interpolates INZ[i][0]..FS[i][0].
        let site_pos = sample_site(2.5, temps, precip);
        let wb_pos = compute_with_hemisphere(&site_pos, Hemisphere::South);
        let upe = compute_unadjusted_pet(&temps);
        for i in 0..12 {
            let cf = (FS[i][0] - INZ[i][0]) * 150.0 / 300.0 + INZ[i][0];
            assert_close(wb_pos.mpe[i], upe[i] * cf);
        }

        // Defect #7: negative lat_dd via compute_water_balance enters SH with signed lat_dd.
        let site_neg = sample_site(-22.5, temps, precip);
        let wb_neg = ThornthwaiteForcing.compute_water_balance(&site_neg, &SiteConfig::default());
        let lat_minutes = (-22 * 60) as f64 + (-22.5 - (-22.0)) * 60.0;
        assert_close(lat_minutes, -1350.0);
        for i in 0..12 {
            let cf = (FS[i][0] - INZ[i][0]) * lat_minutes / 300.0 + INZ[i][0];
            assert_close(wb_neg.mpe[i], upe[i] * cf);
        }

        // SH summer water balance uses DJF (indices 11, 0, 1).
        let mut varied_precip = [10.0; 12];
        varied_precip[11] = 200.0;
        varied_precip[0] = 150.0;
        varied_precip[1] = 120.0;
        let site_djf = sample_site(-10.0, temps, varied_precip);
        let wb_djf = ThornthwaiteForcing.compute_water_balance(&site_djf, &SiteConfig::default());
        let expected_djf =
            (200.0 - wb_djf.mpe[11]) + (150.0 - wb_djf.mpe[0]) + (120.0 - wb_djf.mpe[1]);
        assert_close(wb_djf.summer_water_balance, expected_djf);
    }

    #[test]
    fn southern_hemisphere_5_to_50_replicates_precedence_bug_and_50_clamp() {
        let temps = [20.0; 12];
        let precip = [80.0; 12];
        let upe = compute_unadjusted_pet(&temps);

        // Off-step 12.5 deg S: hits Defect #1 (`* 60.0` after `/` without denominator parens)
        // except September (index 8) where FS[8][1] == FS[8][2] == 1.0 uses exact bin factor.
        let site_12_5 = sample_site(12.5, temps, precip);
        let wb_12_5 = compute_with_hemisphere(&site_12_5, Hemisphere::South);

        // nrow = 2 (5.0, 10.0 <= 12.5), so nrow - 1 = 1 (10.0 deg), nrow = 2 (15.0 deg).
        // For Jan (index 0): FS[0][1] = 1.08, FS[0][2] = 1.12.
        let expected_cf_jan =
            (1.12 - 1.08) * ((12.0 - 10.0) * 60.0 + (12.5 - 12.0) * 60.0) / (15.0 - 10.0) * 60.0
                + 1.08;
        assert_close(wb_12_5.mpe[0], upe[0] * expected_cf_jan);
        // For Sep (index 8): FS[8][1] == FS[8][2] == 1.0 => exact 1.0.
        assert_close(wb_12_5.mpe[8], upe[8] * 1.0);

        // Exact RS step (e.g. 25.0 deg) has zero interpolation delta => exact FS[i][4].
        let site_25 = sample_site(25.0, temps, precip);
        let wb_25 = compute_with_hemisphere(&site_25, Hemisphere::South);
        for i in 0..12 {
            assert_close(wb_25.mpe[i], upe[i] * FS[i][4]);
        }

        // >= 50.0 deg clamps to RS[12] / FS[i][12].
        let site_60 = sample_site(60.0, temps, precip);
        let wb_60 = compute_with_hemisphere(&site_60, Hemisphere::South);
        for i in 0..12 {
            assert_close(wb_60.mpe[i], upe[i] * FS[i][12]);
        }
    }

    #[test]
    fn high_temp_step_table_and_38c_cap() {
        // Test 26.5 <= T < 38.0 (step lookup without interpolation) and T >= 38.0 (185.0).
        let temps = [
            26.49, 26.5, 26.99, 27.0, 30.25, 37.49, 37.5, 38.0, 42.0, 20.0, 20.0, 20.0,
        ];
        let upe = compute_unadjusted_pet(&temps);

        // < 26.5 uses power law, distinct from ZPE[0] = 135.0.
        assert!(upe[0] > 0.0 && upe[0] != 135.0);
        // [26.5, 27.0) -> ZPE[0] = 135.0 (no interpolation at 26.99).
        assert_eq!(upe[1], 135.0);
        assert_eq!(upe[2], 135.0);
        // [27.0, 27.5) -> ZPE[1] = 139.5.
        assert_eq!(upe[3], 139.5);
        // [30.0, 30.5) -> ZPE[7] = 162.1.
        assert_eq!(upe[4], 162.1);
        // [37.0, 37.5) -> ZPE[21] = 184.9.
        assert_eq!(upe[5], 184.9);
        // [37.5, 38.0) -> ZPE[22] = 185.0.
        assert_eq!(upe[6], 185.0);
        // >= 38.0 -> 185.0.
        assert_eq!(upe[7], 185.0);
        assert_eq!(upe[8], 185.0);
    }

    #[test]
    fn freezing_temperatures_and_is_metric_noop() {
        let cold_temps = [
            -10.0, -5.0, 0.0, -0.1, -2.0, 0.0, 0.0, -1.0, -3.0, -5.0, -8.0, -12.0,
        ];
        let precip = [25.0; 12];
        let site = sample_site(45.0, cold_temps, precip);

        let wb = ThornthwaiteForcing.compute_water_balance(&site, &SiteConfig::default());
        assert_eq!(wb.mpe, [0.0; 12]);
        assert_close(wb.annual_pet, 0.0);
        assert_close(wb.annual_rainfall, 300.0);
        assert_close(wb.annual_water_balance, 300.0);
        assert_close(wb.summer_water_balance, 75.0);

        // Defect #2: is_metric = false must produce identical outputs.
        let mixed_temps = [
            -5.0, 0.0, 5.0, 10.0, 15.0, 20.0, 22.0, 21.0, 16.0, 10.0, 3.0, -2.0,
        ];
        let mixed_site = sample_site(40.0, mixed_temps, precip);
        let wb_metric = ThornthwaiteForcing.compute_water_balance(
            &mixed_site,
            &SiteConfig {
                is_metric: true,
                ..SiteConfig::default()
            },
        );
        let wb_english = ThornthwaiteForcing.compute_water_balance(
            &mixed_site,
            &SiteConfig {
                is_metric: false,
                ..SiteConfig::default()
            },
        );
        assert_eq!(wb_metric, wb_english);
        assert_eq!(wb_metric.mpe[0], 0.0);
        assert_eq!(wb_metric.mpe[1], 0.0);
        assert_eq!(wb_metric.mpe[11], 0.0);
        assert!(wb_metric.mpe[6] > 0.0);
    }
}
