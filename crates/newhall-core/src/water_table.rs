//! Fluctuating water-table boundary condition and Gardner (1958) capillary rise
//! for the Newhall 8×8 soil moisture matrix (`WaterTableHydrology`).

use std::f64::consts::TAU;

use crate::hydrology::{equilibrate_half_month, inspect_mcs, simulate_half_month, storm_recharge};
use crate::thermal_1d::SoilTextureClass;
use crate::traits::HydrologyStep;
use crate::types::{SiteConfig, SiteInput, SoilState, WaterBalance};

/// Water-table forcing mode (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaterTableMode {
    /// Prescribed monthly water-table depths (m, positive downward), Jan..Dec.
    PrescribedMonthly([f64; 12]),
    /// Prescribed seasonal harmonic $z_{\text{wt}}(t) = \bar{z} - A \cos(2\pi (t - t_{\text{peak}})/365)$.
    PrescribedHarmonic {
        mean_depth_m: f64,
        amplitude_m: f64,
        peak_day: f64,
    },
    /// Prognostic perched water table above an impermeable layer $z_{\text{imp}}$.
    Prognostic {
        z_imp_m: f64,
        sy: f64,
        k_lat_per_day: f64,
    },
}

impl Default for WaterTableMode {
    fn default() -> Self {
        Self::PrescribedHarmonic {
            mean_depth_m: 1.5,
            amplitude_m: 0.5,
            peak_day: 90.0,
        }
    }
}

/// Configuration for [`WaterTableHydrology`] (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterTableConfig {
    pub mode: WaterTableMode,
    /// Total rooting / profile depth $Z_{\text{prof}}$ (m; default `1.0`).
    pub z_prof_m: f64,
    /// Saturated hydraulic conductivity $K_{\text{sat}}$ ($\text{mm day}^{-1}$; default `100.0`).
    pub k_sat_mm_per_day: f64,
    /// Soil texture class for Gardner (1958) parameters (default `Medium`).
    pub texture: SoilTextureClass,
    /// Enables upward capillary flux above $z_{\text{wt}}$ (default `true`).
    pub enable_capillary: bool,
}

impl Default for WaterTableConfig {
    fn default() -> Self {
        Self {
            mode: WaterTableMode::default(),
            z_prof_m: 1.0,
            k_sat_mm_per_day: 100.0,
            texture: SoilTextureClass::Medium,
            enable_capillary: true,
        }
    }
}

/// Stack-allocated water-table state (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WaterTableState {
    /// Perched water-table height above $z_{\text{imp}}$ (m).
    pub h_wt_m: f64,
    /// Current water-table depth below surface (m, positive downward).
    pub z_wt_m: f64,
}

/// Mass-balance diagnostic for a single prognostic water-table step (mm).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StepMassBalance {
    pub delta_s_matrix_mm: f64,
    pub sy_delta_h_mm: f64,
    pub precip_mm: f64,
    pub aet_mm: f64,
    pub q_lat_mm: f64,
    pub r_surf_mm: f64,
}

/// Fluctuating water-table hydrology model implementing [`HydrologyStep`].
#[derive(Debug, Clone, Copy, Default)]
pub struct WaterTableHydrology {
    pub config: WaterTableConfig,
}

impl WaterTableHydrology {
    /// Constructs a `WaterTableHydrology` with the given configuration.
    pub const fn new(config: WaterTableConfig) -> Self {
        Self { config }
    }

    /// Evaluates prescribed water-table depth $z_{\text{wt}}$ (m) at day `day` (`0..360`).
    #[inline]
    pub fn prescribed_depth(&self, day: usize) -> f64 {
        match self.config.mode {
            WaterTableMode::PrescribedMonthly(depths) => {
                let d = day % 360;
                let shifted = (d + 345) % 360;
                let seg = shifted / 30;
                let rem = (shifted % 30) as f64 / 30.0;
                let m0 = (seg + 11) % 12;
                let m1 = seg % 12;
                depths[m0] + rem * (depths[m1] - depths[m0])
            }
            WaterTableMode::PrescribedHarmonic {
                mean_depth_m,
                amplitude_m,
                peak_day,
            } => {
                let phase = TAU * (day as f64 - peak_day) / 365.0;
                mean_depth_m - amplitude_m * phase.cos()
            }
            WaterTableMode::Prognostic { z_imp_m, .. } => z_imp_m,
        }
    }

    /// Applies saturation override and Gardner (1958) capillary rise for a given `z_wt` (m) and `dt_days`.
    /// Returns total capillary water added to the 8×8 matrix (mm).
    pub fn apply_water_table(&self, sl: &mut [f64; 64], fsl: f64, z_wt: f64, dt_days: f64) -> f64 {
        if fsl <= 0.0 || !fsl.is_finite() || !z_wt.is_finite() {
            return 0.0;
        }
        let z_prof = self.config.z_prof_m.max(1e-6);
        let row_dz = z_prof / 8.0;
        let row_awc_max = 8.0 * fsl;
        let mut total_cap_mm = 0.0_f64;

        for r in 0..8 {
            let z_r = r as f64 * row_dz;
            let z_next = (r + 1) as f64 * row_dz;
            let base = r * 8;

            if z_r >= z_wt {
                // Row fully below water table: set all 8 columns to full AWC
                for c in 0..8 {
                    sl[base + c] = fsl;
                }
            } else if z_next > z_wt {
                // Row straddles water table: scale minimum moisture floor by submerged fraction
                let f_sub = ((z_next - z_wt) / row_dz).clamp(0.0, 1.0);
                let floor_slot = f_sub * fsl;
                for c in 0..8 {
                    if sl[base + c] < floor_slot {
                        sl[base + c] = floor_slot;
                    }
                }
            } else if self.config.enable_capillary && self.config.k_sat_mm_per_day > 0.0 {
                // Unsaturated row above z_wt: Gardner (1958) upward capillary flux
                let z_bar = 0.5 * (z_r + z_next);
                let dz_wt = (z_wt - z_bar).max(0.0);
                let (a, b, n) = self.config.texture.gardner_params();
                let denom = (dz_wt.powf(n) + b).max(1e-12);
                let q_cap_max = (self.config.k_sat_mm_per_day.max(0.0) * a) / denom;
                let w_row: f64 = sl[base..base + 8].iter().sum();
                let deficit = (row_awc_max - w_row).max(0.0);
                let delta_w = (q_cap_max * dt_days.max(0.0)).min(deficit);
                if delta_w > 0.0 {
                    distribute_row_moisture(&mut sl[base..base + 8], fsl, delta_w);
                    total_cap_mm += delta_w;
                }
            }
        }

        total_cap_mm
    }

    /// Executes one prognostic water-table step with exact water mass-balance tracking.
    pub fn step_prognostic(
        &self,
        sl: &mut [f64; 64],
        wt: &mut WaterTableState,
        fsl: f64,
        precip_mm: f64,
        pet_mm: f64,
        dt_days: f64,
    ) -> StepMassBalance {
        let s_mat_before: f64 = sl.iter().sum();
        let (z_imp, sy, k_lat) = match self.config.mode {
            WaterTableMode::Prognostic {
                z_imp_m,
                sy,
                k_lat_per_day,
            } => (z_imp_m.max(0.0), sy.max(0.0), k_lat_per_day.max(0.0)),
            _ => (1.5, 0.2, 0.05),
        };
        let h_before = wt.h_wt_m;

        // 1. Infiltration & AET in the 8x8 matrix
        let aet_mm: f64;
        let mut d_excess_mm = 0.0_f64;

        let p_in = precip_mm.max(0.0);
        let pet = pet_mm.max(0.0);
        if p_in >= pet {
            aet_mm = pet;
            let mut rem = p_in - pet;
            if fsl > 0.0 {
                for slot in sl.iter_mut() {
                    if *slot < fsl {
                        let cap = fsl - *slot;
                        if cap >= rem {
                            *slot += rem;
                            rem = 0.0;
                            break;
                        }
                        *slot = fsl;
                        rem -= cap;
                    }
                }
            }
            d_excess_mm = rem;
        } else {
            let mut actual_aet = p_in;
            let mut demand = pet - p_in;
            for slot in sl.iter_mut().rev() {
                if *slot > 0.0 {
                    if *slot >= demand {
                        *slot -= demand;
                        actual_aet += demand;
                        break;
                    }
                    actual_aet += *slot;
                    demand -= *slot;
                    *slot = 0.0;
                }
            }
            aet_mm = actual_aet;
        }

        // 2. Capillary rise from perched water table into matrix
        let z_wt_cur = z_imp - wt.h_wt_m;
        let q_cap_pot = if sy > 0.0 && wt.h_wt_m > 0.0 {
            let s_wt_avail_mm = 1000.0 * sy * wt.h_wt_m;
            let cap_req = self.apply_capillary_only(sl, fsl, z_wt_cur, dt_days, s_wt_avail_mm);
            cap_req.min(s_wt_avail_mm)
        } else {
            0.0
        };

        // 3. Update perched water table storage (mm)
        let mut q_lat_mm = 0.0_f64;
        let mut r_surf_mm = 0.0_f64;
        if sy > 0.0 {
            let mut s_wt_mm = 1000.0 * sy * wt.h_wt_m + d_excess_mm - q_cap_pot;
            s_wt_mm = s_wt_mm.max(0.0);
            q_lat_mm = s_wt_mm * (k_lat * dt_days).clamp(0.0, 1.0);
            s_wt_mm -= q_lat_mm;

            let max_s_wt_mm = 1000.0 * sy * z_imp;
            if s_wt_mm > max_s_wt_mm {
                r_surf_mm = s_wt_mm - max_s_wt_mm;
                s_wt_mm = max_s_wt_mm;
            }
            wt.h_wt_m = s_wt_mm / (1000.0 * sy);
        } else {
            r_surf_mm = d_excess_mm;
            wt.h_wt_m = 0.0;
        }
        wt.z_wt_m = z_imp - wt.h_wt_m;

        let s_mat_after: f64 = sl.iter().sum();
        StepMassBalance {
            delta_s_matrix_mm: s_mat_after - s_mat_before,
            sy_delta_h_mm: 1000.0 * sy * (wt.h_wt_m - h_before),
            precip_mm: p_in,
            aet_mm,
            q_lat_mm,
            r_surf_mm,
        }
    }

    fn apply_capillary_only(
        &self,
        sl: &mut [f64; 64],
        fsl: f64,
        z_wt: f64,
        dt_days: f64,
        max_avail_mm: f64,
    ) -> f64 {
        if !self.config.enable_capillary || self.config.k_sat_mm_per_day <= 0.0 || fsl <= 0.0 {
            return 0.0;
        }
        let z_prof = self.config.z_prof_m.max(1e-6);
        let row_dz = z_prof / 8.0;
        let row_awc_max = 8.0 * fsl;
        let mut remaining = max_avail_mm.max(0.0);
        let mut added = 0.0_f64;

        for r in (0..8).rev() {
            if remaining <= 0.0 {
                break;
            }
            let z_r = r as f64 * row_dz;
            let z_next = (r + 1) as f64 * row_dz;
            if z_next <= z_wt {
                let z_bar = 0.5 * (z_r + z_next);
                let dz_wt = (z_wt - z_bar).max(0.0);
                let (a, b, n) = self.config.texture.gardner_params();
                let denom = (dz_wt.powf(n) + b).max(1e-12);
                let q_cap_max = (self.config.k_sat_mm_per_day * a) / denom;
                let base = r * 8;
                let w_row: f64 = sl[base..base + 8].iter().sum();
                let deficit = (row_awc_max - w_row).max(0.0);
                let delta_w = (q_cap_max * dt_days.max(0.0)).min(deficit).min(remaining);
                if delta_w > 0.0 {
                    distribute_row_moisture(&mut sl[base..base + 8], fsl, delta_w);
                    added += delta_w;
                    remaining -= delta_w;
                }
            }
        }
        added
    }
}

/// Distributes `delta_w` across a row's 8 columns from driest (`c = 7`) to wettest (`c = 0`),
/// capping every slot at `fsl`.
#[inline]
fn distribute_row_moisture(row: &mut [f64], fsl: f64, mut delta_w: f64) {
    for c in (0..8).rev() {
        if delta_w <= 0.0 {
            break;
        }
        if row[c] < fsl {
            let cap = fsl - row[c];
            let add = cap.min(delta_w);
            row[c] += add;
            delta_w -= add;
        }
    }
}

impl HydrologyStep for WaterTableHydrology {
    fn simulate_hydrology(
        &self,
        input: &SiteInput,
        _config: &SiteConfig,
        water_balance: &WaterBalance,
    ) -> SoilState {
        let fsl = input.awc_mm / 64.0;
        let mut sl = [0.0_f64; 64];
        let mut prmo = 0.0_f64;
        let mut wt_state = WaterTableState::default();

        let z_prof = self.config.z_prof_m.max(1e-6);
        let row_dz = z_prof / 8.0;
        let mcs_top = 1.0 * row_dz; // row 1 top
        let mcs_bot = 4.0 * row_dz; // row 3 bottom (rows 1, 2, 3)

        // Equilibration loop (up to 10 years)
        for _year in 0..10 {
            for m in 0..12 {
                let precip_m = input.precip_mm[m];
                let mpe_m = water_balance.mpe[m];
                let day_mid = m * 30 + 15;

                equilibrate_half_month(&mut sl, fsl, precip_m, mpe_m);
                storm_recharge(&mut sl, fsl, precip_m / 2.0);
                equilibrate_half_month(&mut sl, fsl, precip_m, mpe_m);

                let z_wt = match self.config.mode {
                    WaterTableMode::Prognostic {
                        z_imp_m,
                        sy,
                        k_lat_per_day,
                    } => {
                        let excess = (precip_m - mpe_m).max(0.0);
                        if sy > 0.0 {
                            let mut s_wt_mm = 1000.0 * sy * wt_state.h_wt_m + excess;
                            s_wt_mm *= 1.0 - (k_lat_per_day.max(0.0) * 30.0).clamp(0.0, 1.0);
                            s_wt_mm = s_wt_mm.min(1000.0 * sy * z_imp_m.max(0.0)).max(0.0);
                            wt_state.h_wt_m = s_wt_mm / (1000.0 * sy);
                        }
                        z_imp_m.max(0.0) - wt_state.h_wt_m
                    }
                    _ => self.prescribed_depth(day_mid),
                };
                self.apply_water_table(&mut sl, fsl, z_wt, 30.0);
            }

            let tmoi: f64 = sl.iter().sum();
            if (tmoi - prmo).abs() < prmo / 100.0 {
                break;
            }
            prmo = tmoi;
        }

        let mut k = inspect_mcs(&sl, 1);
        let mut iday = [0_u8; 360];
        let mut nd = [0.0_f64; 3];
        let mut day_cursor = 0_usize;
        let mut z_wt_daily = [f64::INFINITY; 360];
        let mut sat_days_mcs = 0_i32;

        for m in 0..12 {
            let precip_m = input.precip_mm[m];
            let mpe_m = water_balance.mpe[m];

            for half in 0..2 {
                let start_day = m * 30 + half * 15;
                if half == 1 {
                    storm_recharge(&mut sl, fsl, precip_m / 2.0);
                    k = inspect_mcs(&sl, k);
                }
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

                let z_wt = match self.config.mode {
                    WaterTableMode::Prognostic {
                        z_imp_m,
                        sy,
                        k_lat_per_day,
                    } => {
                        let excess = ((precip_m - mpe_m) * 0.5).max(0.0);
                        if sy > 0.0 {
                            let mut s_wt_mm = 1000.0 * sy * wt_state.h_wt_m + excess;
                            s_wt_mm *= 1.0 - (k_lat_per_day.max(0.0) * 15.0).clamp(0.0, 1.0);
                            s_wt_mm = s_wt_mm.min(1000.0 * sy * z_imp_m.max(0.0)).max(0.0);
                            wt_state.h_wt_m = s_wt_mm / (1000.0 * sy);
                        }
                        z_imp_m.max(0.0) - wt_state.h_wt_m
                    }
                    _ => self.prescribed_depth(start_day + 7),
                };

                self.apply_water_table(&mut sl, fsl, z_wt, 15.0);
                for slot in &mut z_wt_daily[start_day..(start_day + 15)] {
                    *slot = z_wt;
                    if z_wt <= mcs_bot || z_wt < mcs_top {
                        sat_days_mcs += 1;
                    }
                }
            }
        }

        SoilState {
            profile: sl,
            iday,
            nd,
            sat_days_mcs,
            z_wt_daily,
        }
    }
}
