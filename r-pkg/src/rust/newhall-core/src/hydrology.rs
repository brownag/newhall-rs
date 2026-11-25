//! 8×8 soil moisture profile equilibration loop and 360-day calendar allocation
//! matching `BASICSimulationModel.java` (`runSimulation(double[], ...)` / `v1.6.5`).

use crate::traits::HydrologyStep;
use crate::types::{SiteConfig, SiteInput, SoilState, WaterBalance};

/// 0-indexed depletion order across the 8×8 soil moisture matrix
/// (transcribed from `BASICSimulationModelConstants.dp[0..64]`, converting `1..=64` to `0..64`
/// and omitting the unused sentinel `dp[64] = 160`).
const DP: [usize; 64] = [
    7, 6, 15, 5, 14, 23, 4, 13, 22, 31, 3, 12, 21, 30, 39, 2, 11, 20, 29, 38, 47, 1, 10, 19, 28,
    37, 46, 55, 0, 9, 18, 27, 36, 45, 54, 63, 8, 17, 26, 35, 44, 53, 62, 16, 25, 34, 43, 52, 61,
    24, 33, 42, 51, 60, 32, 41, 50, 59, 40, 49, 58, 48, 57, 56,
];

/// Depletion resistance weights for each step in [`DP`]
/// (transcribed verbatim from `BASICSimulationModelConstants.dr`).
const DR: [f64; 64] = [
    1.0, 1.0, 1.0, 1.0, 1.02, 1.03, 1.05, 1.07, 1.09, 1.11, 1.13, 1.15, 1.17, 1.19, 1.21, 1.23,
    1.26, 1.28, 1.31, 1.34, 1.37, 1.40, 1.43, 1.46, 1.49, 1.53, 1.57, 1.61, 1.65, 1.69, 1.74, 1.78,
    1.84, 1.89, 1.95, 2.01, 2.07, 2.14, 2.22, 2.30, 2.38, 2.47, 2.57, 2.68, 2.80, 2.93, 3.07, 3.22,
    3.39, 3.58, 3.80, 4.03, 4.31, 4.62, 4.98, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0,
];

/// Classic Newhall 8×8 soil moisture hydrology model (`jNSMR` `v1.6.5` parity).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClassicHydrology;

impl HydrologyStep for ClassicHydrology {
    fn simulate_hydrology(
        &self,
        input: &SiteInput,
        _config: &SiteConfig,
        water_balance: &WaterBalance,
    ) -> SoilState {
        // Defect #2: `config.is_metric` is ignored in `newhall-core` for Java parity.
        let fsl = input.awc_mm / 64.0;
        let mut sl = [0.0_f64; 64];
        let mut prmo = 0.0_f64;

        // Equilibration loop: up to 10 years, stopping early once annual profile change < 1%.
        for _year in 0..10 {
            for m in 0..12 {
                let precip_m = input.precip_mm[m];
                let mpe_m = water_balance.mpe[m];
                equilibrate_half_month(&mut sl, fsl, precip_m, mpe_m);
                storm_recharge(&mut sl, fsl, precip_m / 2.0);
                equilibrate_half_month(&mut sl, fsl, precip_m, mpe_m);
            }

            let mut tmoi = 0.0_f64;
            for &val in &sl {
                tmoi += val;
            }
            if (tmoi - prmo).abs() < prmo / 100.0 {
                break;
            }
            prmo = tmoi;
        }

        // Initial MCS inspection after equilibration (Java starts with `k = 1`).
        let mut k = inspect_mcs(&sl, 1);
        let mut iday = [0_u8; 360];
        let mut nd = [0.0_f64; 3];
        let mut day_cursor = 0_usize;

        // Final-year simulation producing the 360-day calendar and cumulative day counts.
        for m in 0..12 {
            let precip_m = input.precip_mm[m];
            let mpe_m = water_balance.mpe[m];

            simulate_half_month(
                &mut sl,
                fsl,
                precip_m,
                mpe_m,
                &mut k,
                &mut iday,
                &mut nd,
                &mut day_cursor,
            );

            storm_recharge(&mut sl, fsl, precip_m / 2.0);
            k = inspect_mcs(&sl, k);

            simulate_half_month(
                &mut sl,
                fsl,
                precip_m,
                mpe_m,
                &mut k,
                &mut iday,
                &mut nd,
                &mut day_cursor,
            );
        }

        SoilState {
            profile: sl,
            iday,
            nd,
            sat_days_mcs: 0,
            z_wt_daily: [f64::INFINITY; 360],
        }
    }
}

/// Inspects Moisture Control Section slots (`sl[9]`, `sl[17]`, `sl[25]` -> `[8]`, `[16]`, `[24]`):
/// `1` = Dry, `2` = Moist/Dry, `3` = Moist; preserves `prev_k` if all conditions are false (`NaN`).
#[inline]
pub(crate) fn inspect_mcs(sl: &[f64; 64], prev_k: u8) -> u8 {
    let d0 = sl[8] <= 0.0;
    let d1 = sl[16] <= 0.0;
    let d2 = sl[24] <= 0.0;
    let cc1 = d0 && d1 && d2;
    let cc2 = !cc1 && (d0 || d1 || d2);
    let cc3 = sl[8] > 0.0 && sl[16] > 0.0 && sl[24] > 0.0;

    if cc3 {
        3
    } else if cc2 {
        2
    } else if cc1 {
        1
    } else {
        prev_k
    }
}

/// Instantaneous mid-month storm recharge of slots `0..64` up to `fsl` (excess discarded).
#[inline]
pub(crate) fn storm_recharge(sl: &mut [f64; 64], fsl: f64, mut hp: f64) {
    for slot in sl.iter_mut() {
        if *slot < fsl {
            let esl = fsl - *slot;
            if esl >= hp {
                *slot += hp;
                break;
            }
            *slot = fsl;
            hp -= esl;
        }
    }
}

/// One 15-day equilibration pass (discharge via `DP`/`DR` if `npe <= 0`, else sequential recharge).
#[inline]
pub(crate) fn equilibrate_half_month(sl: &mut [f64; 64], fsl: f64, precip_m: f64, mpe_m: f64) {
    let lp = precip_m / 2.0;
    let mut npe = (lp - mpe_m) / 2.0;
    if npe <= 0.0 {
        npe = 0.0 - npe;
        for idx in 0..64 {
            let nr = DP[idx];
            if sl[nr] > 0.0 {
                let rpe = sl[nr] * DR[idx];
                if npe <= rpe {
                    sl[nr] -= npe / DR[idx];
                    break;
                }
                sl[nr] = 0.0;
                npe -= rpe;
            }
        }
    } else {
        for slot in sl.iter_mut() {
            if *slot < fsl {
                let esl = fsl - *slot;
                if esl >= npe {
                    *slot += npe;
                    break;
                }
                *slot = fsl;
                npe -= esl;
            }
        }
    }
}

/// Appends `count` days of MCS state `k` to `iday` and accumulates into `nd`.
#[inline]
fn assign_days(iday: &mut [u8; 360], nd: &mut [f64; 3], day_cursor: &mut usize, k: u8, count: i32) {
    if count > 0 {
        let end = *day_cursor + count as usize;
        for slot in &mut iday[*day_cursor..end] {
            *slot = k;
        }
        *day_cursor = end;
    }
    nd[(k - 1) as usize] += count as f64;
}

/// Checks for an MCS transition during a final-year slot update and allocates elapsed days.
/// Returns `true` if `npe <= 0.0` triggered Java's early `break` (leaving `k == pmc`).
#[inline]
#[allow(clippy::too_many_arguments)]
fn maybe_record_transition(
    sl: &[f64; 64],
    k: &mut u8,
    pmc: &mut u8,
    npe: f64,
    cnpe: f64,
    dpmc: &mut i32,
    iday: &mut [u8; 360],
    nd: &mut [f64; 3],
    day_cursor: &mut usize,
) -> bool {
    let kk = inspect_mcs(sl, *k);
    // Preserve Java's exact ordering: revert `k = pmc` before testing `npe <= 0.0`.
    *k = *pmc;
    if kk != *pmc {
        if npe <= 0.0 {
            return true;
        }
        let rpe = cnpe - npe;
        let igmc = (15.0 * rpe / cnpe) as i32 - *dpmc;
        *dpmc += igmc;
        assign_days(iday, nd, day_cursor, *k, igmc);
        *pmc = kk;
        *k = kk;
    }
    false
}

/// Simulates one 15-day final-year half-month pass and assigns exactly 15 days to `iday`/`nd`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn simulate_half_month(
    sl: &mut [f64; 64],
    fsl: f64,
    precip_m: f64,
    mpe_m: f64,
    k: &mut u8,
    iday: &mut [u8; 360],
    nd: &mut [f64; 3],
    day_cursor: &mut usize,
) {
    let mut dpmc: i32 = 0;
    let mut pmc: u8 = *k;

    let lp = precip_m / 2.0;
    let mut npe = (lp - mpe_m) / 2.0;

    if npe < 0.0 {
        npe = -npe;
        let cnpe = npe;
        for idx in 0..64 {
            if npe <= 0.0 {
                break;
            }
            let nr = DP[idx];
            if sl[nr] > 0.0 {
                let rpd = sl[nr] * DR[idx];
                if npe <= rpd {
                    sl[nr] -= npe / DR[idx];
                    npe = 0.0;
                } else {
                    sl[nr] = 0.0;
                    npe -= rpd;
                }
                if maybe_record_transition(
                    sl, k, &mut pmc, npe, cnpe, &mut dpmc, iday, nd, day_cursor,
                ) {
                    break;
                }
            }
        }
    } else if npe > 0.0 || npe.is_nan() {
        let cnpe = npe;
        for idx in 0..64 {
            if npe <= 0.0 {
                break;
            }
            if sl[idx] < fsl {
                let esl = fsl - sl[idx];
                if esl >= npe {
                    sl[idx] += npe;
                    npe = 0.0;
                } else {
                    sl[idx] = fsl;
                    npe -= esl;
                }
                if maybe_record_transition(
                    sl, k, &mut pmc, npe, cnpe, &mut dpmc, iday, nd, day_cursor,
                ) {
                    break;
                }
            }
        }
    }

    let rem = 15 - dpmc;
    assign_days(iday, nd, day_cursor, *k, rem);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input(awc_mm: f64, precip_mm: [f64; 12]) -> SiteInput {
        SiteInput {
            lat_dd: 40.0,
            lon_dd: -77.0,
            temp_c: [0.0; 12],
            precip_mm,
            awc_mm,
            elev_m: 300.0,
        }
    }

    fn sample_wb(mpe: [f64; 12]) -> WaterBalance {
        WaterBalance {
            mpe,
            ..WaterBalance::default()
        }
    }

    #[test]
    fn dp_is_valid_permutation_of_0_to_63() {
        let mut seen = [false; 64];
        for &slot in &DP {
            assert!(slot < 64, "DP slot out of range: {slot}");
            assert!(!seen[slot], "Duplicate DP slot: {slot}");
            seen[slot] = true;
        }
        assert!(seen.iter().all(|&s| s));
        // Spot-check first and last elements against Java 1-based dp[0]=8, dp[63]=57.
        assert_eq!(DP[0], 7);
        assert_eq!(DP[63], 56);
    }

    #[test]
    fn dr_invariants_and_endpoints() {
        assert_eq!(DR[0], 1.0);
        assert_eq!(DR[3], 1.0);
        assert_eq!(DR[4], 1.02);
        assert_eq!(DR[55], 5.0);
        assert_eq!(DR[63], 5.0);
        for w in DR.windows(2) {
            assert!(w[0] <= w[1], "DR not non-decreasing: {} > {}", w[0], w[1]);
        }
    }

    #[test]
    fn storm_recharge_fills_sequentially_and_conserves_up_to_capacity() {
        let fsl = 200.0 / 64.0;
        let mut sl = [0.0_f64; 64];
        storm_recharge(&mut sl, fsl, 10.0);
        let total: f64 = sl.iter().sum();
        assert!((total - 10.0).abs() < 1e-12);
        assert_eq!(sl[0], fsl);
        assert_eq!(sl[1], fsl);
        assert_eq!(sl[2], fsl);
        assert!((sl[3] - (10.0 - 3.0 * fsl)).abs() < 1e-12);
        assert_eq!(sl[4], 0.0);

        // Excess beyond total capacity (200.0) runs off cleanly.
        storm_recharge(&mut sl, fsl, 500.0);
        let full_total: f64 = sl.iter().sum();
        assert!((full_total - 200.0).abs() < 1e-12);
    }

    #[test]
    fn discharge_respects_dp_order_and_dr_resistance() {
        let fsl = 3.125; // 200 / 64
        let mut sl = [fsl; 64];
        // Deficit of 2.0 mm in first half-month: P = 0.0, MPE = 4.0 => npe = (0 - 4)/2 = -2.0.
        equilibrate_half_month(&mut sl, fsl, 0.0, 4.0);
        // First slot in DP is index 7 (DR[7-indexed 0] = 1.0), holding 3.125 > 2.0.
        assert!((sl[DP[0]] - (fsl - 2.0)).abs() < 1e-12);
        // Remaining 63 slots untouched.
        for &slot_idx in &DP[1..] {
            assert_eq!(sl[slot_idx], fsl);
        }

        // Now deplete across multiple slots including DR > 1.0 (index 4 has DR = 1.02).
        let mut sl2 = [1.0_f64; 64];
        // P = 0.0, MPE = 10.0 => npe = 5.0.
        // DP[0..4] have DR = 1.0 -> consume 4.0, remaining npe = 1.0.
        // DP[4] has DR = 1.02 -> rpe = 1.02 >= 1.0 -> sl[DP[4]] = 1.0 - 1.0 / 1.02.
        equilibrate_half_month(&mut sl2, fsl, 0.0, 10.0);
        for &slot_idx in &DP[0..4] {
            assert_eq!(sl2[slot_idx], 0.0);
        }
        assert!((sl2[DP[4]] - (1.0 - 1.0 / 1.02)).abs() < 1e-12);
        assert_eq!(sl2[DP[5]], 1.0);
    }

    #[test]
    fn perudic_all_moist_calendar_sums_to_360() {
        let model = ClassicHydrology;
        let input = sample_input(200.0, [150.0; 12]);
        let wb = sample_wb([40.0; 12]);
        let state = model.simulate_hydrology(&input, &SiteConfig::default(), &wb);

        assert_eq!(state.nd, [0.0, 0.0, 360.0]);
        assert!(state.iday.iter().all(|&d| d == 3));
        assert!((state.profile.iter().sum::<f64>() - 200.0).abs() < 1e-12);
    }

    #[test]
    fn arid_zero_precip_calendar_sums_to_360_all_dry() {
        let model = ClassicHydrology;
        let input = sample_input(200.0, [0.0; 12]);
        let wb = sample_wb([50.0; 12]);
        let state = model.simulate_hydrology(&input, &SiteConfig::default(), &wb);

        assert_eq!(state.nd, [360.0, 0.0, 0.0]);
        assert!(state.iday.iter().all(|&d| d == 1));
        assert!(state.profile.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn seasonal_transition_calendar_totals_360_and_records_all_three_states() {
        let model = ClassicHydrology;
        // Wet winter, bone-dry summer to force transitions across Dry (1), Moist/Dry (2), Moist (3).
        let precip = [
            120.0, 120.0, 80.0, 20.0, 5.0, 0.0, 0.0, 0.0, 10.0, 60.0, 100.0, 120.0,
        ];
        let mpe = [
            10.0, 15.0, 30.0, 60.0, 100.0, 140.0, 160.0, 140.0, 90.0, 40.0, 20.0, 10.0,
        ];
        let input = sample_input(200.0, precip);
        let wb = sample_wb(mpe);

        // Defect #2 check: is_metric = false must produce bit-identical state.
        let state_metric = model.simulate_hydrology(&input, &SiteConfig::default(), &wb);
        let cfg_english = SiteConfig {
            is_metric: false,
            ..SiteConfig::default()
        };
        let state_english = model.simulate_hydrology(&input, &cfg_english, &wb);
        assert_eq!(state_metric, state_english);

        let nd_sum: f64 = state_metric.nd.iter().sum();
        assert_eq!(nd_sum, 360.0);
        assert!(state_metric.nd[0] > 0.0);
        assert!(state_metric.nd[1] > 0.0);
        assert!(state_metric.nd[2] > 0.0);

        let mut counts = [0_usize; 3];
        for &d in &state_metric.iday {
            assert!((1..=3).contains(&d), "Invalid iday value: {d}");
            counts[(d - 1) as usize] += 1;
        }
        assert_eq!(counts[0] as f64, state_metric.nd[0]);
        assert_eq!(counts[1] as f64, state_metric.nd[1]);
        assert_eq!(counts[2] as f64, state_metric.nd[2]);
    }

    #[test]
    fn zero_or_negative_awc_always_dry() {
        let model = ClassicHydrology;
        let wb = sample_wb([20.0; 12]);
        for awc in [0.0, -10.0] {
            let input = sample_input(awc, [100.0; 12]);
            let state = model.simulate_hydrology(&input, &SiteConfig::default(), &wb);
            assert_eq!(state.nd, [360.0, 0.0, 0.0]);
            assert!(state.iday.iter().all(|&d| d == 1));
            assert!(state.profile.iter().all(|&v| v == 0.0));
        }
    }

    #[test]
    fn exact_zero_npe_transition_break_edge_case() {
        // Test half-month where npe reaches 0.0 on the exact slot that changes MCS (slot 8).
        let fsl = 10.0;
        let mut sl = [0.0_f64; 64];
        // Pre-fill slots 0..8 to fsl, leave slot 8 empty (so sl[8]==0, sl[16]==0, sl[24]==0 => k=1).
        for s in &mut sl[0..8] {
            *s = fsl;
        }
        let mut k = 1_u8;
        let mut iday = [0_u8; 360];
        let mut nd = [0.0_f64; 3];
        let mut cursor = 0_usize;

        // Need npe = (P/2 - MPE)/2 = 5.0 => e.g. P = 20.0, MPE = 0.0 => lp = 10, npe = 5.0.
        // Slot 8 has esl = 10.0 >= 5.0 => sl[8] = 5.0, npe = 0.0, kk becomes 2 (!= pmc=1).
        // Because npe <= 0.0, Java breaks with k reverted to pmc=1 and assigns all 15 days to 1.
        simulate_half_month(
            &mut sl,
            fsl,
            20.0,
            0.0,
            &mut k,
            &mut iday,
            &mut nd,
            &mut cursor,
        );
        assert_eq!(k, 1);
        assert_eq!(cursor, 15);
        assert_eq!(nd, [15.0, 0.0, 0.0]);
        assert!(iday[..15].iter().all(|&d| d == 1));
    }
}
