//! 1D vertical soil thermal model with phase change, snowpack/O-horizon series resistance,
//! and zero-allocation Backward Euler + Thomas algorithm solver (`Thermal1DModel`).

use crate::thermal::classify_smr;
use crate::traits::ThermalModel;
use crate::types::{
    DayCounters, Hemisphere, MoistureRegime, RegimeSubdivision1, RegimeSubdivision2, SiteConfig,
    SiteInput, SoilState, TemperatureRegime, ThermalClassification, WaterBalance,
};

/// Number of vertical nodes in the stretched 1D soil thermal grid.
pub const THERMAL_NODES: usize = 16;

/// Maximum depth of the 1D soil thermal domain (m).
pub const Z_MAX_M: f64 = 5.0;

/// Geometric stretching ratio for the 16-node grid (finest resolution in top 1.0 m).
const GRID_STRETCH: f64 = 1.25;

// Physical constants (PHASE4_PHYSICS_SPEC.md §1.2-1.3)
const C_MINERAL: f64 = 1.94e6; // J m^-3 K^-1
const C_OM: f64 = 2.50e6; // J m^-3 K^-1
const C_WATER: f64 = 4.18e6; // J m^-3 K^-1
const C_ICE: f64 = 1.93e6; // J m^-3 K^-1
const RHO_WATER: f64 = 1000.0; // kg m^-3
const LATENT_FUSION: f64 = 3.34e5; // J kg^-1
const RHO_L_F: f64 = RHO_WATER * LATENT_FUSION; // 3.34e8 J m^-3
const T_FREEZE_C: f64 = 0.0;
const DELTA_T_FREEZE: f64 = 0.5; // K

const LAMBDA_WATER: f64 = 0.57; // W m^-1 K^-1
const LAMBDA_ICE: f64 = 2.2; // W m^-1 K^-1
const LAMBDA_QUARTZ: f64 = 7.7; // W m^-1 K^-1
const LAMBDA_OTHER_MIN: f64 = 2.0; // W m^-1 K^-1
const LAMBDA_DRY_O_HORIZON: f64 = 0.05; // W m^-1 K^-1

const SECONDS_PER_DAY: f64 = 86_400.0;

/// Soil texture class governing Kersten number $\kappa$ and Gardner (1958) capillary parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SoilTextureClass {
    Coarse,
    #[default]
    Medium,
    Fine,
    Organic,
}

impl SoilTextureClass {
    /// Côté & Konrad (2005) Kersten parameter $\kappa$.
    #[inline]
    pub const fn kersten_kappa(self) -> f64 {
        match self {
            Self::Coarse => 4.6,
            Self::Medium => 3.55,
            Self::Fine => 1.9,
            Self::Organic => 0.6,
        }
    }

    /// Gardner (1958) upward capillary flux parameters `(a, b, n)` for $\Delta z$ in meters.
    #[inline]
    pub const fn gardner_params(self) -> (f64, f64, f64) {
        match self {
            Self::Coarse => (0.08, 0.05, 3.0),
            Self::Medium => (0.15, 0.10, 2.5),
            Self::Fine => (0.25, 0.20, 2.0),
            Self::Organic => (0.12, 0.10, 2.5),
        }
    }
}

/// Configuration for the 1D soil thermal model (`Copy`, stack-allocated).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thermal1DConfig {
    /// Soil porosity $\phi \in [0, 1]$ (default `0.45`).
    pub porosity: f64,
    /// Dry bulk density $\rho_b$ in $\text{kg m}^{-3}$ (default `1400.0`).
    pub bulk_density_kg_m3: f64,
    /// Quartz volume fraction of mineral solids $q \in [0, 1]$ (default `0.40`).
    pub quartz_fraction: f64,
    /// Organic matter volume fraction $f_{\text{om}} \in [0, 1]$ (default `0.02`).
    pub organic_fraction: f64,
    /// Surface O-horizon thickness $d_O$ in meters (default `0.0`).
    pub o_horizon_thickness_m: f64,
    /// Surface O-horizon thermal conductivity $\lambda_O$ in $\text{W m}^{-1}\text{ K}^{-1}$ (default `0.05`).
    pub o_horizon_conductivity: f64,
    /// Monthly snowpack depth $d_{\text{snow}}$ in meters, Jan..Dec (default `[0.0; 12]`).
    pub snow_depth_m: [f64; 12],
    /// Snowpack thermal conductivity $\lambda_{\text{snow}}$ in $\text{W m}^{-1}\text{ K}^{-1}$ (default `0.15`).
    pub snow_conductivity: f64,
    /// Surface convective heat transfer coefficient $h_c$ in $\text{W m}^{-2}\text{ K}^{-1}$ (default `15.0`).
    pub convective_hc: f64,
    /// Soil texture class (default `SoilTextureClass::Medium`).
    pub texture: SoilTextureClass,
    /// Optional fixed total volumetric moisture $\theta_{\text{tot}}$; if `None`, coupled to `SoilState`.
    pub fixed_theta_tot: Option<f64>,
    /// Maximum annual spin-up cycles (default `5`).
    pub max_spinup_years: usize,
}

impl Default for Thermal1DConfig {
    fn default() -> Self {
        Self {
            porosity: 0.45,
            bulk_density_kg_m3: 1400.0,
            quartz_fraction: 0.40,
            organic_fraction: 0.02,
            o_horizon_thickness_m: 0.0,
            o_horizon_conductivity: 0.05,
            snow_depth_m: [0.0; 12],
            snow_conductivity: 0.15,
            convective_hc: 15.0,
            texture: SoilTextureClass::Medium,
            fixed_theta_tot: None,
            max_spinup_years: 5,
        }
    }
}

/// Stack-allocated 16-node 1D soil thermal state (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilThermalState {
    /// Node depths $z_i \in [0.0, 5.0]\text{ m}$.
    pub z: [f64; THERMAL_NODES],
    /// Control-volume thicknesses $\Delta z_i$ ($\sum \Delta z_i = 5.0\text{ m}$).
    pub dz: [f64; THERMAL_NODES],
    /// Node temperatures $T_i$ (°C).
    pub temp_c: [f64; THERMAL_NODES],
    /// Apparent volumetric heat capacity $C_{\text{app}, i}$ ($\text{J m}^{-3}\text{ K}^{-1}$).
    pub c_app: [f64; THERMAL_NODES],
    /// Thermal conductivity $\lambda_i$ ($\text{W m}^{-1}\text{ K}^{-1}$).
    pub lambda: [f64; THERMAL_NODES],
}

impl Default for SoilThermalState {
    fn default() -> Self {
        let (z, dz) = stretched_grid();
        Self {
            z,
            dz,
            temp_c: [10.0; THERMAL_NODES],
            c_app: [2.0e6; THERMAL_NODES],
            lambda: [1.5; THERMAL_NODES],
        }
    }
}

/// Material parameters for a single 1D thermal node (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeMaterial {
    pub porosity: f64,
    pub bulk_density_kg_m3: f64,
    pub quartz_fraction: f64,
    pub organic_fraction: f64,
    pub theta_tot: f64,
    pub kappa: f64,
    pub is_o_horizon: bool,
    /// Optional direct override `(c_v, lambda)` for analytical benchmarks (no phase change).
    pub uniform_override: Option<(f64, f64)>,
}

impl Default for NodeMaterial {
    fn default() -> Self {
        Self {
            porosity: 0.45,
            bulk_density_kg_m3: 1400.0,
            quartz_fraction: 0.40,
            organic_fraction: 0.02,
            theta_tot: 0.25,
            kappa: SoilTextureClass::Medium.kersten_kappa(),
            is_o_horizon: false,
            uniform_override: None,
        }
    }
}

/// Precomputed site-constant thermal parameters for fast zero-`powf` evaluation (`Copy`).
#[derive(Debug, Clone, Copy)]
struct PrecomputedMaterial {
    theta_tot: f64,
    cv_solid: f64,
    cv_unfrozen: f64,
    cv_frozen: f64,
    lambda_unfrozen: f64,
    lambda_frozen: f64,
    lambda_dry: f64,
    ke: f64,
    lambda_sat_frozen: f64,
    ln_lw_over_li: f64,
    is_linear: bool,
}

impl PrecomputedMaterial {
    #[inline]
    fn from_node(mat: &NodeMaterial) -> Self {
        if let Some((cv_ovr, lam_ovr)) = mat.uniform_override {
            let cv = cv_ovr.max(1.0);
            let lam = lam_ovr.max(1e-6);
            return Self {
                theta_tot: 0.0,
                cv_solid: cv,
                cv_unfrozen: cv,
                cv_frozen: cv,
                lambda_unfrozen: lam,
                lambda_frozen: lam,
                lambda_dry: lam,
                ke: 0.0,
                lambda_sat_frozen: lam,
                ln_lw_over_li: 0.0,
                is_linear: true,
            };
        }

        let phi = mat.porosity.clamp(0.0, 1.0);
        let theta_tot = mat.theta_tot.clamp(0.0, phi);
        let om = mat.organic_fraction.clamp(0.0, 1.0 - phi);
        let mineral = (1.0 - phi - om).max(0.0);
        let cv_solid = C_MINERAL * mineral + C_OM * om;
        let cv_unfrozen = (cv_solid + C_WATER * theta_tot).max(1.0);
        let cv_frozen = (cv_solid + C_ICE * theta_tot).max(1.0);

        let rho_b_clamped = mat.bulk_density_kg_m3.clamp(0.0, 2650.0);
        let lambda_dry = if mat.is_o_horizon {
            LAMBDA_DRY_O_HORIZON
        } else {
            let denom = (2700.0 - 0.947 * rho_b_clamped).max(1.0);
            (0.135 * rho_b_clamped + 64.7) / denom
        };

        let q = mat.quartz_fraction.clamp(0.0, 1.0);
        let lambda_s = LAMBDA_QUARTZ.powf(q) * LAMBDA_OTHER_MIN.powf(1.0 - q);
        let lambda_s_pow = lambda_s.powf(1.0 - phi);

        let sr = if phi > 1e-12 {
            (theta_tot / phi).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let k = mat.kappa.max(0.01);
        let ke = (k * sr) / (1.0 + (k - 1.0) * sr).max(1e-12);

        let lambda_sat_unfrozen = lambda_s_pow * LAMBDA_WATER.powf(theta_tot);
        let lambda_sat_frozen = lambda_s_pow * LAMBDA_ICE.powf(theta_tot);
        let lambda_unfrozen = ((lambda_sat_unfrozen - lambda_dry) * ke + lambda_dry).max(1e-4);
        let lambda_frozen = ((lambda_sat_frozen - lambda_dry) * ke + lambda_dry).max(1e-4);

        Self {
            theta_tot,
            cv_solid,
            cv_unfrozen,
            cv_frozen,
            lambda_unfrozen,
            lambda_frozen,
            lambda_dry,
            ke,
            lambda_sat_frozen,
            ln_lw_over_li: (LAMBDA_WATER / LAMBDA_ICE).ln(),
            is_linear: theta_tot == 0.0,
        }
    }

    #[inline]
    fn thermo(&self, temp_c: f64) -> (f64, f64, f64, f64) {
        if self.is_linear {
            return (
                self.cv_solid,
                self.cv_solid,
                self.lambda_unfrozen,
                self.cv_solid * temp_c,
            );
        }
        let dt = temp_c - T_FREEZE_C;
        if dt > 6.0 {
            return (
                self.cv_unfrozen,
                self.cv_unfrozen,
                self.lambda_unfrozen,
                self.cv_unfrozen * temp_c + RHO_L_F * self.theta_tot,
            );
        }
        if dt < -6.0 {
            return (
                self.cv_frozen,
                self.cv_frozen,
                self.lambda_frozen,
                self.cv_frozen * temp_c,
            );
        }
        let th = (dt / DELTA_T_FREEZE).tanh();
        let theta_l = 0.5 * self.theta_tot * (1.0 + th);
        let theta_i = self.theta_tot - theta_l;
        let dtheta_dt = (0.5 * self.theta_tot / DELTA_T_FREEZE) * (1.0 - th * th);
        let cv = (self.cv_solid + C_WATER * theta_l + C_ICE * theta_i).max(1.0);
        let c_app = (cv + RHO_L_F * dtheta_dt).max(1.0);
        let lambda_sat = self.lambda_sat_frozen * (theta_l * self.ln_lw_over_li).exp();
        let lam = ((lambda_sat - self.lambda_dry) * self.ke + self.lambda_dry).max(1e-4);
        let enthalpy = cv * temp_c + RHO_L_F * theta_l;
        (cv, c_app, lam, enthalpy)
    }
}

/// Computes the 16-node geometrically stretched grid over $[0.0, 5.0]\text{ m}$ and control-volume widths.
pub fn stretched_grid() -> ([f64; THERMAL_NODES], [f64; THERMAL_NODES]) {
    let mut z = [0.0_f64; THERMAL_NODES];
    let denom = GRID_STRETCH.powi((THERMAL_NODES - 1) as i32) - 1.0;
    for (i, zi) in z.iter_mut().enumerate() {
        *zi = Z_MAX_M * (GRID_STRETCH.powi(i as i32) - 1.0) / denom;
    }
    z[0] = 0.0;
    z[THERMAL_NODES - 1] = Z_MAX_M;

    let mut dz = [0.0_f64; THERMAL_NODES];
    dz[0] = 0.5 * (z[1] - z[0]);
    for i in 1..(THERMAL_NODES - 1) {
        dz[i] = 0.5 * (z[i + 1] - z[i - 1]);
    }
    dz[THERMAL_NODES - 1] = 0.5 * (z[THERMAL_NODES - 1] - z[THERMAL_NODES - 2]);
    (z, dz)
}

/// Smooth unfrozen water content $\theta_l(T)$ and derivative $\partial \theta_l / \partial T$.
#[inline]
pub fn unfrozen_water_and_deriv(temp_c: f64, theta_tot: f64) -> (f64, f64) {
    let theta_tot = theta_tot.max(0.0);
    if theta_tot == 0.0 {
        return (0.0, 0.0);
    }
    let arg = (temp_c - T_FREEZE_C) / DELTA_T_FREEZE;
    let th = arg.tanh();
    let theta_l = 0.5 * theta_tot * (1.0 + th);
    let dtheta_dt = (0.5 * theta_tot / DELTA_T_FREEZE) * (1.0 - th * th);
    (theta_l, dtheta_dt)
}

/// de Vries (1963) volumetric heat capacity $C_v$ ($\text{J m}^{-3}\text{ K}^{-1}$).
#[inline]
pub fn volumetric_heat_capacity(porosity: f64, f_om: f64, theta_l: f64, theta_i: f64) -> f64 {
    let phi = porosity.clamp(0.0, 1.0);
    let om = f_om.clamp(0.0, 1.0 - phi);
    let mineral = (1.0 - phi - om).max(0.0);
    C_MINERAL * mineral + C_OM * om + C_WATER * theta_l.max(0.0) + C_ICE * theta_i.max(0.0)
}

/// Johansen (1975) / Côté & Konrad (2005) thermal conductivity $\lambda$ ($\text{W m}^{-1}\text{ K}^{-1}$).
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn thermal_conductivity(
    porosity: f64,
    rho_b: f64,
    quartz_frac: f64,
    theta_l: f64,
    theta_i: f64,
    kappa: f64,
    is_o_horizon: bool,
) -> f64 {
    let phi = porosity.clamp(0.0, 1.0);
    let rho_b_clamped = rho_b.clamp(0.0, 2650.0);

    let lambda_dry = if is_o_horizon {
        LAMBDA_DRY_O_HORIZON
    } else {
        let denom = (2700.0 - 0.947 * rho_b_clamped).max(1.0);
        (0.135 * rho_b_clamped + 64.7) / denom
    };

    let q = quartz_frac.clamp(0.0, 1.0);
    let lambda_s = LAMBDA_QUARTZ.powf(q) * LAMBDA_OTHER_MIN.powf(1.0 - q);
    let tl = theta_l.max(0.0);
    let ti = theta_i.max(0.0);
    let lambda_sat = lambda_s.powf(1.0 - phi) * LAMBDA_WATER.powf(tl) * LAMBDA_ICE.powf(ti);

    let sr = if phi > 1e-12 {
        ((tl + ti) / phi).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let k = kappa.max(0.01);
    let ke_denom = (1.0 + (k - 1.0) * sr).max(1e-12);
    let ke = (k * sr) / ke_denom;

    ((lambda_sat - lambda_dry) * ke + lambda_dry).max(1e-4)
}

/// Series surface thermal resistance $R_{\text{surf}}$ ($\text{m}^2\text{ K W}^{-1}$).
///
/// Returns `f64::INFINITY` if `h_c <= 0.0` (insulated top) and `0.0` if `d_snow == 0, d_O == 0, h_c == INFINITY`.
#[inline]
pub fn surface_resistance(d_snow: f64, lambda_snow: f64, d_o: f64, lambda_o: f64, h_c: f64) -> f64 {
    if h_c <= 0.0 {
        return f64::INFINITY;
    }
    let r_snow = if d_snow > 0.0 {
        d_snow / lambda_snow.max(1e-4)
    } else {
        0.0
    };
    let r_o = if d_o > 0.0 {
        d_o / lambda_o.max(1e-4)
    } else {
        0.0
    };
    let r_conv = if h_c.is_finite() { 1.0 / h_c } else { 0.0 };
    r_snow + r_o + r_conv
}

/// Evaluates `(c_v, c_app, lambda, enthalpy)` for a single node at temperature `temp_c`.
#[inline]
fn node_thermo(temp_c: f64, mat: &NodeMaterial) -> (f64, f64, f64, f64) {
    if let Some((cv_ovr, lam_ovr)) = mat.uniform_override {
        let cv = cv_ovr.max(1.0);
        let lam = lam_ovr.max(1e-6);
        return (cv, cv, lam, cv * temp_c);
    }

    let phi = mat.porosity.clamp(0.0, 1.0);
    let theta_tot = mat.theta_tot.clamp(0.0, phi);
    let (theta_l, dtheta_dt) = unfrozen_water_and_deriv(temp_c, theta_tot);
    let theta_i = (theta_tot - theta_l).max(0.0);

    let cv = volumetric_heat_capacity(phi, mat.organic_fraction, theta_l, theta_i).max(1.0);
    let c_app = (cv + RHO_L_F * dtheta_dt).max(1.0);
    let lam = thermal_conductivity(
        phi,
        mat.bulk_density_kg_m3,
        mat.quartz_fraction,
        theta_l,
        theta_i,
        mat.kappa,
        mat.is_o_horizon,
    );
    let enthalpy = cv * temp_c + RHO_L_F * theta_l;
    (cv, c_app, lam, enthalpy)
}

/// Computes total column enthalpy $H = \sum_i (C_{v,i} T_i + \rho_w L_f \theta_{l,i}) \Delta z_i$ ($\text{J m}^{-2}$).
pub fn total_enthalpy(state: &SoilThermalState, mats: &[NodeMaterial; THERMAL_NODES]) -> f64 {
    let mut sum = 0.0_f64;
    for (i, mat) in mats.iter().enumerate() {
        let (_, _, _, h_i) = node_thermo(state.temp_c[i], mat);
        sum += h_i * state.dz[i];
    }
    sum
}

/// Solves a tridiagonal system $a_i x_{i-1} + b_i x_i + c_i x_{i+1} = d_i$ in-place on stack `[f64; 16]`.
#[inline]
fn thomas_solve(
    a: &[f64; THERMAL_NODES],
    b: &mut [f64; THERMAL_NODES],
    c: &[f64; THERMAL_NODES],
    d: &mut [f64; THERMAL_NODES],
    x: &mut [f64; THERMAL_NODES],
) {
    for i in 1..THERMAL_NODES {
        let denom = b[i - 1];
        if denom.abs() < 1e-30 {
            continue;
        }
        let w = a[i] / denom;
        b[i] -= w * c[i - 1];
        d[i] -= w * d[i - 1];
    }
    let last = THERMAL_NODES - 1;
    x[last] = if b[last].abs() >= 1e-30 {
        d[last] / b[last]
    } else {
        0.0
    };
    for i in (0..last).rev() {
        let denom = b[i];
        x[i] = if denom.abs() >= 1e-30 {
            (d[i] - c[i] * x[i + 1]) / denom
        } else {
            0.0
        };
    }
}

/// Advances `state` by one implicit Backward Euler step of duration `dt_sec` (s) using stack arrays.
///
/// Uses a single-pass linear solve when all nodes are outside the freeze-thaw band (`|T - T_f| > 6 K`),
/// and Newton-Picard enthalpy iteration inside the phase-change zone.
pub fn step_backward_euler(
    state: &mut SoilThermalState,
    mats: &[NodeMaterial; THERMAL_NODES],
    t_air_c: f64,
    r_surf: f64,
    dt_sec: f64,
) {
    let pre = PrecomputedMaterial::from_node(&mats[0]);
    if mats.iter().all(|m| m == &mats[0]) {
        let pre_arr = [pre; THERMAL_NODES];
        step_backward_euler_precomputed(state, &pre_arr, t_air_c, r_surf, dt_sec);
    } else {
        let pre_arr = std::array::from_fn(|i| PrecomputedMaterial::from_node(&mats[i]));
        step_backward_euler_precomputed(state, &pre_arr, t_air_c, r_surf, dt_sec);
    }
}

#[inline]
fn step_backward_euler_precomputed(
    state: &mut SoilThermalState,
    mats: &[PrecomputedMaterial; THERMAL_NODES],
    t_air_c: f64,
    r_surf: f64,
    dt_sec: f64,
) {
    if !t_air_c.is_finite() || dt_sec <= 0.0 {
        return;
    }

    let is_insulated = !r_surf.is_finite();
    let is_dirichlet = r_surf <= 1e-12;
    let g_top = if is_insulated || is_dirichlet {
        0.0
    } else {
        1.0 / r_surf
    };

    // Fast-path: check if all nodes and air temperature are on the same side outside [-6, +6] C
    let mut all_linear = true;
    let mut all_above = t_air_c > 6.0 || is_insulated;
    let mut all_below = t_air_c < -6.0 || is_insulated;
    for (i, mat) in mats.iter().enumerate() {
        if !mat.is_linear {
            let t = state.temp_c[i];
            if (-6.0..=6.0).contains(&t) {
                all_linear = false;
                break;
            }
            all_above &= t > 6.0;
            all_below &= t < -6.0;
        }
    }
    if all_linear && (mats[0].is_linear || all_above || all_below) {
        let mut a = [0.0_f64; THERMAL_NODES];
        let mut b = [0.0_f64; THERMAL_NODES];
        let mut c = [0.0_f64; THERMAL_NODES];
        let mut d = [0.0_f64; THERMAL_NODES];
        for (i, mat) in mats.iter().enumerate() {
            let (_, c_app_i, lam_i, _) = mat.thermo(state.temp_c[i]);
            state.c_app[i] = c_app_i;
            state.lambda[i] = lam_i;
        }
        let mut g_int = [0.0_f64; THERMAL_NODES - 1];
        for (i, g_slot) in g_int.iter_mut().enumerate() {
            let dz_node = (state.z[i + 1] - state.z[i]).max(1e-9);
            let l0 = state.lambda[i];
            let l1 = state.lambda[i + 1];
            let l_harm = if l0 + l1 > 1e-12 {
                2.0 * l0 * l1 / (l0 + l1)
            } else {
                0.0
            };
            *g_slot = l_harm / dz_node;
        }

        if is_dirichlet {
            b[0] = 1.0;
            d[0] = t_air_c;
        } else {
            let cap_dt = state.c_app[0] * state.dz[0] / dt_sec;
            b[0] = cap_dt + g_top + g_int[0];
            c[0] = -g_int[0];
            d[0] = cap_dt * state.temp_c[0] + g_top * t_air_c;
        }
        for i in 1..(THERMAL_NODES - 1) {
            let cap_dt = state.c_app[i] * state.dz[i] / dt_sec;
            a[i] = -g_int[i - 1];
            b[i] = cap_dt + g_int[i - 1] + g_int[i];
            c[i] = -g_int[i];
            d[i] = cap_dt * state.temp_c[i];
        }
        let last = THERMAL_NODES - 1;
        let cap_dt_last = state.c_app[last] * state.dz[last] / dt_sec;
        a[last] = -g_int[last - 1];
        b[last] = cap_dt_last + g_int[last - 1];
        d[last] = cap_dt_last * state.temp_c[last];

        let mut t_next = [0.0_f64; THERMAL_NODES];
        thomas_solve(&a, &mut b, &c, &mut d, &mut t_next);
        state.temp_c = t_next;
        return;
    }

    let mut h_old = [0.0_f64; THERMAL_NODES];
    for i in 0..THERMAL_NODES {
        let (_, _, _, h_i) = mats[i].thermo(state.temp_c[i]);
        h_old[i] = h_i;
    }

    let mut t_cur = state.temp_c;
    let mut a = [0.0_f64; THERMAL_NODES];
    let mut b = [0.0_f64; THERMAL_NODES];
    let mut c = [0.0_f64; THERMAL_NODES];
    let mut d = [0.0_f64; THERMAL_NODES];
    let mut delta_t = [0.0_f64; THERMAL_NODES];

    for _iter in 0..16 {
        for i in 0..THERMAL_NODES {
            let (_, c_app_i, lam_i, _) = mats[i].thermo(t_cur[i]);
            state.c_app[i] = c_app_i;
            state.lambda[i] = lam_i;
        }

        // Interface conductances g_{i+1/2} = lambda_{i+1/2} / (z_{i+1} - z_i)
        let mut g_int = [0.0_f64; THERMAL_NODES - 1];
        for (i, g_slot) in g_int.iter_mut().enumerate() {
            let dz_node = (state.z[i + 1] - state.z[i]).max(1e-9);
            let l0 = state.lambda[i];
            let l1 = state.lambda[i + 1];
            let l_harm = if l0 + l1 > 1e-12 {
                2.0 * l0 * l1 / (l0 + l1)
            } else {
                0.0
            };
            *g_slot = l_harm / dz_node;
        }

        // Top node (i = 0)
        if is_dirichlet {
            a[0] = 0.0;
            b[0] = 1.0;
            c[0] = 0.0;
            d[0] = t_air_c - t_cur[0];
        } else {
            let (_, _, _, h_cur0) = mats[0].thermo(t_cur[0]);
            let cap_dt = state.c_app[0] * state.dz[0] / dt_sec;
            let flux_top = if is_insulated {
                0.0
            } else {
                g_top * (t_air_c - t_cur[0])
            };
            let flux_down = g_int[0] * (t_cur[0] - t_cur[1]);
            let res0 = (h_cur0 - h_old[0]) * state.dz[0] / dt_sec - flux_top + flux_down;
            a[0] = 0.0;
            b[0] = cap_dt + g_top + g_int[0];
            c[0] = -g_int[0];
            d[0] = -res0;
        }

        // Interior nodes (1..15)
        for i in 1..(THERMAL_NODES - 1) {
            let (_, _, _, h_cur) = mats[i].thermo(t_cur[i]);
            let cap_dt = state.c_app[i] * state.dz[i] / dt_sec;
            let flux_up = g_int[i - 1] * (t_cur[i - 1] - t_cur[i]);
            let flux_down = g_int[i] * (t_cur[i] - t_cur[i + 1]);
            let res = (h_cur - h_old[i]) * state.dz[i] / dt_sec - flux_up + flux_down;
            a[i] = -g_int[i - 1];
            b[i] = cap_dt + g_int[i - 1] + g_int[i];
            c[i] = -g_int[i];
            d[i] = -res;
        }

        // Bottom node (i = 15, Neumann zero-flux)
        let last = THERMAL_NODES - 1;
        let (_, _, _, h_last) = mats[last].thermo(t_cur[last]);
        let cap_dt_last = state.c_app[last] * state.dz[last] / dt_sec;
        let flux_up_last = g_int[last - 1] * (t_cur[last - 1] - t_cur[last]);
        let res_last = (h_last - h_old[last]) * state.dz[last] / dt_sec - flux_up_last;
        a[last] = -g_int[last - 1];
        b[last] = cap_dt_last + g_int[last - 1];
        c[last] = 0.0;
        d[last] = -res_last;

        thomas_solve(&a, &mut b, &c, &mut d, &mut delta_t);

        let mut max_dt = 0.0_f64;
        for i in 0..THERMAL_NODES {
            t_cur[i] += delta_t[i];
            let abs_d = delta_t[i].abs();
            if abs_d > max_dt {
                max_dt = abs_d;
            }
        }
        if max_dt < 1e-11 {
            break;
        }
    }

    state.temp_c = t_cur;
    for (i, mat) in mats.iter().enumerate() {
        let (_, c_app_i, lam_i, _) = mat.thermo(state.temp_c[i]);
        state.c_app[i] = c_app_i;
        state.lambda[i] = lam_i;
    }
}

/// Linearly interpolates $T(z = 0.50\text{ m})$ from the 16-node profile.
#[inline]
pub fn interp_at_depth(
    z: &[f64; THERMAL_NODES],
    temp_c: &[f64; THERMAL_NODES],
    target_z: f64,
) -> f64 {
    if target_z <= z[0] {
        return temp_c[0];
    }
    for i in 0..(THERMAL_NODES - 1) {
        if target_z <= z[i + 1] {
            let span = (z[i + 1] - z[i]).max(1e-12);
            let frac = (target_z - z[i]) / span;
            return temp_c[i] + frac * (temp_c[i + 1] - temp_c[i]);
        }
    }
    temp_c[THERMAL_NODES - 1]
}

/// Linearly interpolates a 12-month array to day `d` (`0..360`) with periodic wrapping.
#[inline]
pub fn interp_monthly_to_day(monthly: &[f64; 12], day: usize) -> f64 {
    // Month m midpoint is at day 15 + 30*m (15, 45, ..., 345)
    let d = day % 360;
    let shifted = (d + 345) % 360; // distance past Dec midpoint (day 345)
    let seg = shifted / 30; // 0..12
    let rem = (shifted % 30) as f64 / 30.0;
    let m0 = (seg + 11) % 12;
    let m1 = seg % 12;
    monthly[m0] + rem * (monthly[m1] - monthly[m0])
}

/// 1D coupled soil thermal model implementing [`ThermalModel`] with zero heap allocations.
#[derive(Debug, Clone, Copy, Default)]
pub struct Thermal1DModel {
    pub config: Thermal1DConfig,
}

impl Thermal1DModel {
    /// Constructs a `Thermal1DModel` with the given configuration.
    pub const fn new(config: Thermal1DConfig) -> Self {
        Self { config }
    }

    /// Simulates spin-up + final 360-day year and returns `(daily_t50_c, final_state)`.
    pub fn simulate_year(
        &self,
        input: &SiteInput,
        soil_state: &SoilState,
    ) -> ([f64; 360], SoilThermalState) {
        let mut state = SoilThermalState::default();

        if input.temp_c.iter().any(|t| !t.is_finite()) {
            state.temp_c = [f64::NAN; THERMAL_NODES];
            return ([f64::NAN; 360], state);
        }

        let mean_t = input.temp_c.iter().sum::<f64>() / 12.0;
        state.temp_c = [mean_t; THERMAL_NODES];

        let theta_tot = match self.config.fixed_theta_tot {
            Some(v) => v.clamp(0.0, self.config.porosity.clamp(0.0, 1.0)),
            None => {
                let awc = if input.awc_mm.is_finite() && input.awc_mm > 0.0 {
                    input.awc_mm
                } else {
                    0.0
                };
                if awc > 0.0 {
                    let total_m: f64 = soil_state.profile.iter().sum();
                    let rel_sat = (total_m / awc).clamp(0.0, 1.0);
                    rel_sat * self.config.porosity.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
        };

        let mat = NodeMaterial {
            porosity: self.config.porosity,
            bulk_density_kg_m3: self.config.bulk_density_kg_m3,
            quartz_fraction: self.config.quartz_fraction,
            organic_fraction: self.config.organic_fraction,
            theta_tot,
            kappa: self.config.texture.kersten_kappa(),
            is_o_horizon: self.config.texture == SoilTextureClass::Organic,
            uniform_override: None,
        };
        let pre_mats = [PrecomputedMaterial::from_node(&mat); THERMAL_NODES];

        // Coarse-to-fine spin-up: 3 fast 12-step monthly cycles equilibrate deep nodes,
        // followed by up to `max_spinup_years` daily refinement cycles (early-exit < 1e-3 K).
        for _coarse_yr in 0..3 {
            for m in 0..12 {
                let r_surf = surface_resistance(
                    self.config.snow_depth_m[m].max(0.0),
                    self.config.snow_conductivity,
                    self.config.o_horizon_thickness_m.max(0.0),
                    self.config.o_horizon_conductivity,
                    self.config.convective_hc,
                );
                step_backward_euler_precomputed(
                    &mut state,
                    &pre_mats,
                    input.temp_c[m],
                    r_surf,
                    30.0 * SECONDS_PER_DAY,
                );
            }
        }

        let spinup_years = self.config.max_spinup_years.clamp(1, 2);
        let mut prev_year_end = state.temp_c;
        for year in 0..spinup_years {
            for day in 0..360 {
                let t_air = interp_monthly_to_day(&input.temp_c, day);
                let d_snow = interp_monthly_to_day(&self.config.snow_depth_m, day).max(0.0);
                let r_surf = surface_resistance(
                    d_snow,
                    self.config.snow_conductivity,
                    self.config.o_horizon_thickness_m.max(0.0),
                    self.config.o_horizon_conductivity,
                    self.config.convective_hc,
                );
                step_backward_euler_precomputed(
                    &mut state,
                    &pre_mats,
                    t_air,
                    r_surf,
                    SECONDS_PER_DAY,
                );
            }
            if year >= 1 {
                let mut max_diff = 0.0_f64;
                for (i, &prev_t) in prev_year_end.iter().enumerate() {
                    let diff = (state.temp_c[i] - prev_t).abs();
                    if diff > max_diff {
                        max_diff = diff;
                    }
                }
                if max_diff < 1e-3 {
                    break;
                }
            }
            prev_year_end = state.temp_c;
        }

        // Final recording year (360 days)
        let mut t50_daily = [0.0_f64; 360];
        for (day, t50_slot) in t50_daily.iter_mut().enumerate() {
            let t_air = interp_monthly_to_day(&input.temp_c, day);
            let d_snow = interp_monthly_to_day(&self.config.snow_depth_m, day).max(0.0);
            let r_surf = surface_resistance(
                d_snow,
                self.config.snow_conductivity,
                self.config.o_horizon_thickness_m.max(0.0),
                self.config.o_horizon_conductivity,
                self.config.convective_hc,
            );
            step_backward_euler_precomputed(&mut state, &pre_mats, t_air, r_surf, SECONDS_PER_DAY);
            *t50_slot = interp_at_depth(&state.z, &state.temp_c, 0.50);
        }

        (t50_daily, state)
    }
}

impl ThermalModel for Thermal1DModel {
    fn evaluate(
        &self,
        input: &SiteInput,
        config: &SiteConfig,
        water_balance: &WaterBalance,
        soil_state: &SoilState,
    ) -> ThermalClassification {
        let hemisphere = input.hemisphere();
        let (t50_daily, _) = self.simulate_year(input, soil_state);

        if t50_daily.iter().any(|t| !t.is_finite()) {
            return ThermalClassification {
                day_counters: DayCounters {
                    num_cumulative_days_dry: soil_state.nd[0] as i32,
                    num_cumulative_days_moist_dry: soil_state.nd[1] as i32,
                    num_cumulative_days_moist: soil_state.nd[2] as i32,
                    ..DayCounters::default()
                },
                temperature_regime: None,
                moisture_regime: MoistureRegime::Undefined,
                regime_subdivision1: RegimeSubdivision1::Undefined,
                regime_subdivision2: RegimeSubdivision2::Undefined,
                aquic_bio_days: 0,
            };
        }

        let mast = t50_daily.iter().sum::<f64>() / 360.0;
        let jja_mean = t50_daily[150..240].iter().sum::<f64>() / 90.0;
        let djf_mean =
            (t50_daily[330..360].iter().sum::<f64>() + t50_daily[0..60].iter().sum::<f64>()) / 90.0;

        let (msst, mwst) = match hemisphere {
            Hemisphere::North => (jja_mean, djf_mean),
            Hemisphere::South => (djf_mean, jja_mean),
        };
        let dif = msst - mwst;
        let dif_abs = dif.abs();

        let has_o = self.config.o_horizon_thickness_m > 0.0 || config.has_o_horizon;
        let cryic_ht = match (has_o, config.is_saturated) {
            (true, true) => 6.0,
            (true, false) => 8.0,
            (false, true) => 13.0,
            (false, false) => 15.0,
        };

        let is_iso = dif_abs < 6.0;
        let temperature_regime = if mast < 0.0 {
            Some(TemperatureRegime::Pergelic)
        } else if mast < 8.0 {
            if msst >= 0.0 && msst < cryic_ht {
                Some(TemperatureRegime::Cryic)
            } else if is_iso {
                Some(TemperatureRegime::Isofrigid)
            } else {
                Some(TemperatureRegime::Frigid)
            }
        } else if mast < 15.0 {
            if is_iso {
                Some(TemperatureRegime::Isomesic)
            } else {
                Some(TemperatureRegime::Mesic)
            }
        } else if mast < 22.0 {
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

        // >5°C cumulative day counts and >8°C consecutive moist-in-some-parts
        let mut nsd = [0_i32; 3];
        let mut lt5c = 0_i32;
        for (d, &t50) in t50_daily.iter().enumerate() {
            if t50 > 5.0 {
                lt5c += 1;
                let ik = soil_state.iday[d];
                if (1..=3).contains(&ik) {
                    nsd[(ik - 1) as usize] += 1;
                }
            }
        }

        let ncpm1 = max_consecutive_360(|d| soil_state.iday[d] != 1);
        let ncpm2 = max_consecutive_360(|d| soil_state.iday[d] != 1 && t50_daily[d] > 8.0);
        let aquic_bio_days =
            max_consecutive_360(|d| soil_state.z_wt_daily[d] <= 0.50 && t50_daily[d] > 5.0);

        let (summer_start, winter_start) = match hemisphere {
            Hemisphere::North => (180_usize, 0_usize),
            Hemisphere::South => (0_usize, 180_usize),
        };
        let nccd = max_consecutive_window(&soil_state.iday, summer_start, 120, 1);
        let nccm = max_consecutive_window(&soil_state.iday, winter_start, 120, 3);

        let swt_nonzero = !(0..12).any(|i| water_balance.mpe[i] > input.precip_mm[i]);
        let (moisture_regime, regime_subdivision1, regime_subdivision2) = classify_smr(
            swt_nonzero,
            nsd[0] as f64,
            lt5c as f64,
            ncpm2,
            soil_state.nd,
            mast,
            dif,
            nccd,
            nccm,
            temperature_regime,
        );

        ThermalClassification {
            day_counters: DayCounters {
                dry_days_after_summer_solstice: nccd,
                moist_days_after_winter_solstice: nccm,
                num_cumulative_days_dry: soil_state.nd[0] as i32,
                num_cumulative_days_moist_dry: soil_state.nd[1] as i32,
                num_cumulative_days_moist: soil_state.nd[2] as i32,
                num_cumulative_days_dry_over_5c: nsd[0],
                num_cumulative_days_moist_dry_over_5c: nsd[1],
                num_cumulative_days_moist_over_5c: nsd[2],
                num_consecutive_days_moist_in_some_parts: ncpm1,
                num_consecutive_days_moist_in_some_parts_over_8c: ncpm2,
            },
            temperature_regime,
            moisture_regime,
            regime_subdivision1,
            regime_subdivision2,
            aquic_bio_days,
        }
    }
}

/// Longest consecutive run in a 360-day periodic year where `pred(day)` holds.
#[inline]
fn max_consecutive_360<F: Fn(usize) -> bool>(pred: F) -> i32 {
    let mut all_true = true;
    for d in 0..360 {
        if !pred(d) {
            all_true = false;
            break;
        }
    }
    if all_true {
        return 360;
    }
    let mut best = 0_i32;
    let mut cur = 0_i32;
    for step in 0..720 {
        if pred(step % 360) {
            cur += 1;
            if cur > best {
                best = cur;
            }
        } else {
            cur = 0;
        }
    }
    best.min(360)
}

/// Longest consecutive run of `target` in a 120-day window starting at `start` (0-indexed).
#[inline]
fn max_consecutive_window(iday: &[u8; 360], start: usize, len: usize, target: u8) -> i32 {
    let mut best = 0_i32;
    let mut cur = 0_i32;
    for offset in 0..len {
        if iday[(start + offset) % 360] == target {
            cur += 1;
            if cur > best {
                best = cur;
            }
        } else {
            cur = 0;
        }
    }
    best
}
