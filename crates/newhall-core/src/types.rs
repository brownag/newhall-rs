//! Core `Copy` data types, `#[repr(u8)]` regime discriminants, and input/output schemas
//! for the Newhall Simulation Model (`jNSMR` / `newhall-1.6.5.jar` parity contract).

/// Geographic hemisphere derived from latitude (`lat_dd >= 0.0` => North).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hemisphere {
    /// Northern hemisphere (`lat_dd >= 0.0`).
    North,
    /// Southern hemisphere (`lat_dd < 0.0`).
    South,
}

/// Soil Temperature Regime (STR), `0..=9` matching `jNSMR` `terra::set.cats()` codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemperatureRegime {
    /// Mean annual soil temperature (MAST) < 0 °C.
    Pergelic = 0,
    /// MAST in `[0, 8)` °C with cool summer temperatures (non-iso).
    Cryic = 1,
    /// MAST < 8 °C and warmer in summer than Cryic (non-iso).
    Frigid = 2,
    /// MAST in `[8, 15)` °C (non-iso).
    Mesic = 3,
    /// MAST in `[15, 22)` °C (non-iso).
    Thermic = 4,
    /// MAST >= 22 °C (non-iso).
    Hyperthermic = 5,
    /// MAST < 8 °C with < 6 °C difference between mean summer and winter soil temperatures.
    Isofrigid = 6,
    /// MAST in `[8, 15)` °C with < 6 °C seasonal amplitude.
    Isomesic = 7,
    /// MAST in `[15, 22)` °C with < 6 °C seasonal amplitude.
    Isothermic = 8,
    /// MAST >= 22 °C with < 6 °C seasonal amplitude.
    Isohyperthermic = 9,
}

impl TemperatureRegime {
    /// Exact Java `BASICSimulationModel` string representation.
    pub const fn as_java_str(self) -> &'static str {
        match self {
            Self::Pergelic => "Pergelic",
            Self::Cryic => "Cryic",
            Self::Frigid => "Frigid",
            Self::Mesic => "Mesic",
            Self::Thermic => "Thermic",
            Self::Hyperthermic => "Hyperthermic",
            Self::Isofrigid => "Isofrigid",
            Self::Isomesic => "Isomesic",
            Self::Isothermic => "Isothermic",
            Self::Isohyperthermic => "Isohyperthermic",
        }
    }

    /// Exact R `jNSMR` factor label (`.str()`).
    pub const fn as_r_label(self) -> &'static str {
        self.as_java_str()
    }

    /// Converts a 0-indexed `u8` discriminant back into `TemperatureRegime`.
    pub const fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Pergelic),
            1 => Some(Self::Cryic),
            2 => Some(Self::Frigid),
            3 => Some(Self::Mesic),
            4 => Some(Self::Thermic),
            5 => Some(Self::Hyperthermic),
            6 => Some(Self::Isofrigid),
            7 => Some(Self::Isomesic),
            8 => Some(Self::Isothermic),
            9 => Some(Self::Isohyperthermic),
            _ => None,
        }
    }
}

/// Soil Moisture Regime (SMR), `0..=5` matching `jNSMR` `terra::set.cats()` codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoistureRegime {
    /// Aridic / Torric moisture regime.
    Aridic = 0,
    /// Ustic moisture regime.
    Ustic = 1,
    /// Xeric moisture regime.
    Xeric = 2,
    /// Udic moisture regime.
    Udic = 3,
    /// Perudic moisture regime.
    Perudic = 4,
    /// Undefined moisture regime (e.g., Pergelic/Cryic thermal bounds or invalid input).
    Undefined = 5,
}

impl MoistureRegime {
    /// Exact Java `BASICSimulationModel` string representation.
    pub const fn as_java_str(self) -> &'static str {
        match self {
            Self::Aridic => "Aridic",
            Self::Ustic => "Ustic",
            Self::Xeric => "Xeric",
            Self::Udic => "Udic",
            Self::Perudic => "Perudic",
            Self::Undefined => "Undefined",
        }
    }

    /// Exact R `jNSMR` factor label (`.smr()`).
    pub const fn as_r_label(self) -> &'static str {
        self.as_java_str()
    }

    /// Converts a 0-indexed `u8` discriminant back into `MoistureRegime`.
    pub const fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Aridic),
            1 => Some(Self::Ustic),
            2 => Some(Self::Xeric),
            3 => Some(Self::Udic),
            4 => Some(Self::Perudic),
            5 => Some(Self::Undefined),
            _ => None,
        }
    }
}

/// Soil Moisture Regime Subdivision 1 (`0..=8`, holds Java modifier `q` per Defect #8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegimeSubdivision1 {
    /// Typic subdivision.
    Typic = 0,
    /// Weak subdivision (e.g., Weak Aridic).
    Weak = 1,
    /// Wet subdivision (e.g., Wet Tempustic / Tropustic).
    Wet = 2,
    /// Dry subdivision (e.g., Dry Xeric / Udic / Tempustic).
    Dry = 3,
    /// Extreme subdivision (e.g., Extreme Aridic).
    Extreme = 4,
    /// Xeric intergrade subdivision.
    Xeric = 5,
    /// Udic intergrade subdivision.
    Udic = 6,
    /// Aridic intergrade subdivision.
    Aridic = 7,
    /// Undefined subdivision (`" "` in classic Java output).
    Undefined = 8,
}

impl RegimeSubdivision1 {
    /// Exact Java `BASICSimulationModel` string representation (`" "` when undefined).
    pub const fn as_java_str(self) -> &'static str {
        match self {
            Self::Typic => "Typic",
            Self::Weak => "Weak",
            Self::Wet => "Wet",
            Self::Dry => "Dry",
            Self::Extreme => "Extreme",
            Self::Xeric => "Xeric",
            Self::Udic => "Udic",
            Self::Aridic => "Aridic",
            Self::Undefined => " ",
        }
    }

    /// Exact R `jNSMR` factor label (`.smrsub1()`), mapping `" "` to `"Undefined"`.
    pub const fn as_r_label(self) -> &'static str {
        match self {
            Self::Typic => "Typic",
            Self::Weak => "Weak",
            Self::Wet => "Wet",
            Self::Dry => "Dry",
            Self::Extreme => "Extreme",
            Self::Xeric => "Xeric",
            Self::Udic => "Udic",
            Self::Aridic => "Aridic",
            Self::Undefined => "Undefined",
        }
    }

    /// Converts a 0-indexed `u8` discriminant back into `RegimeSubdivision1`.
    pub const fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Typic),
            1 => Some(Self::Weak),
            2 => Some(Self::Wet),
            3 => Some(Self::Dry),
            4 => Some(Self::Extreme),
            5 => Some(Self::Xeric),
            6 => Some(Self::Udic),
            7 => Some(Self::Aridic),
            8 => Some(Self::Undefined),
            _ => None,
        }
    }
}

/// Soil Moisture Regime Subdivision 2 (`0..=7`, holds Java group `div` per Defect #8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegimeSubdivision2 {
    /// Aridic subdivision group.
    Aridic = 0,
    /// Tempustic subdivision group.
    Tempustic = 1,
    /// Tropustic subdivision group.
    Tropustic = 2,
    /// Tempudic subdivision group.
    Tempudic = 3,
    /// Xeric subdivision group.
    Xeric = 4,
    /// Udic subdivision group.
    Udic = 5,
    /// Tropudic subdivision group.
    Tropudic = 6,
    /// Undefined subdivision group.
    Undefined = 7,
}

impl RegimeSubdivision2 {
    /// Exact Java `BASICSimulationModel` string representation.
    ///
    /// Note: For `MoistureRegime::Perudic`, Java emits `" "` rather than `"Undefined"`;
    /// use [`SiteOutput::regime_subdivision2_java_str`] when formatting full outputs.
    pub const fn as_java_str(self) -> &'static str {
        match self {
            Self::Aridic => "Aridic",
            Self::Tempustic => "Tempustic",
            Self::Tropustic => "Tropustic",
            Self::Tempudic => "Tempudic",
            Self::Xeric => "Xeric",
            Self::Udic => "Udic",
            Self::Tropudic => "Tropudic",
            Self::Undefined => "Undefined",
        }
    }

    /// Exact R `jNSMR` factor label (`.smrsub2()`).
    pub const fn as_r_label(self) -> &'static str {
        self.as_java_str()
    }

    /// Converts a 0-indexed `u8` discriminant back into `RegimeSubdivision2`.
    pub const fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Aridic),
            1 => Some(Self::Tempustic),
            2 => Some(Self::Tropustic),
            3 => Some(Self::Tempudic),
            4 => Some(Self::Xeric),
            5 => Some(Self::Udic),
            6 => Some(Self::Tropudic),
            7 => Some(Self::Undefined),
            _ => None,
        }
    }
}

/// Per-site climate and soil input parameters (stack-allocated, `Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiteInput {
    /// Site latitude in decimal degrees (`>= 0.0` is Northern hemisphere).
    pub lat_dd: f64,
    /// Site longitude in decimal degrees.
    pub lon_dd: f64,
    /// Monthly mean air temperature (°C), Jan..Dec (`[0..12]`).
    pub temp_c: [f64; 12],
    /// Monthly precipitation (mm), Jan..Dec (`[0..12]`).
    pub precip_mm: [f64; 12],
    /// Available water capacity (mm; standard default `200.0`).
    pub awc_mm: f64,
    /// Elevation (m; unused in `runSimulation(double[], ...)`).
    pub elev_m: f64,
}

impl SiteInput {
    /// Derives hemisphere from `lat_dd` matching `jNSMR::batch3()` (`lat_dd >= 0.0 => North`).
    #[inline]
    pub fn hemisphere(&self) -> Hemisphere {
        if self.lat_dd >= 0.0 {
            Hemisphere::North
        } else {
            Hemisphere::South
        }
    }
}

/// Per-site soil thermal and configuration flags (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiteConfig {
    /// Soil-air temperature offset (°C; default `2.5`).
    pub soil_air_offset: f64,
    /// Soil-to-air temperature amplitude ratio (`fcd`; default `0.66`).
    pub amplitude: f64,
    /// Presence of an organic O-horizon (`false` by default).
    pub has_o_horizon: bool,
    /// Saturation at 50 cm depth (`false` by default).
    pub is_saturated: bool,
    /// Stored for `jNSMR` parity (Defect #2: must NOT trigger unit conversions in `newhall-core`).
    pub is_metric: bool,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            soil_air_offset: 2.5,
            amplitude: 0.66,
            has_o_horizon: false,
            is_saturated: false,
            is_metric: true,
        }
    }
}

/// Compact half-monthly water-table depth profile (24 half-months × 15 days = 360 days, `Copy`).
///
/// Indexed by simulation day `0..360` (`day / 15`) to eliminate unconditional `[f64; 360]`
/// stack copies while preserving exact meter depths per half-month.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfMonthlyWaterTable(pub [f64; 24]);

impl HalfMonthlyWaterTable {
    /// Returns the 24 half-monthly water-table depths (m, positive downward).
    #[inline]
    pub const fn as_half_months(&self) -> &[f64; 24] {
        &self.0
    }

    /// Sets the water-table depth (m) for half-month `hm_idx` (`0..24`).
    #[inline]
    pub fn set_half_month(&mut self, hm_idx: usize, z_wt_m: f64) {
        self.0[hm_idx] = z_wt_m;
    }
}

impl Default for HalfMonthlyWaterTable {
    fn default() -> Self {
        Self([f64::INFINITY; 24])
    }
}

impl std::ops::Index<usize> for HalfMonthlyWaterTable {
    type Output = f64;

    #[inline]
    fn index(&self, day: usize) -> &Self::Output {
        &self.0[day / 15]
    }
}

/// Stack-allocated 8×8 soil moisture state and 360-day calendar (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilState {
    /// 64-slot soil moisture profile (`sl[1..=64]` in Java mapped to `0..64`).
    pub profile: [f64; 64],
    /// 360-day moisture condition calendar (`1` = Dry, `2` = Moist/Dry, `3` = Moist).
    pub iday: [u8; 360],
    /// Cumulative days Dry (`nd[0]`), Moist/Dry (`nd[1]`), and Moist (`nd[2]`).
    pub nd: [f64; 3],
    /// Cumulative days where water table is within the Moisture Control Section (`0` in classic mode).
    pub sat_days_mcs: i32,
    /// Half-monthly water-table depth profile (m, positive downward; `f64::INFINITY` when absent).
    pub z_wt_daily: HalfMonthlyWaterTable,
}

impl Default for SoilState {
    fn default() -> Self {
        Self {
            profile: [0.0; 64],
            iday: [0; 360],
            nd: [0.0; 3],
            sat_days_mcs: 0,
            z_wt_daily: HalfMonthlyWaterTable::default(),
        }
    }
}

/// Thornthwaite monthly PET and seasonal water balance summary (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterBalance {
    /// Adjusted monthly potential evapotranspiration (`mpe`, mm), Jan..Dec (`[0..12]`).
    pub mpe: [f64; 12],
    /// Total annual precipitation (mm).
    pub annual_rainfall: f64,
    /// Total annual potential evapotranspiration (mm).
    pub annual_pet: f64,
    /// Annual precipitation minus annual PET (mm).
    pub annual_water_balance: f64,
    /// Summer-quarter precipitation minus summer-quarter PET (mm).
    pub summer_water_balance: f64,
}

impl Default for WaterBalance {
    fn default() -> Self {
        Self {
            mpe: [0.0; 12],
            annual_rainfall: 0.0,
            annual_pet: 0.0,
            annual_water_balance: 0.0,
            summer_water_balance: 0.0,
        }
    }
}

/// All 10 integer day-counter outputs produced by the simulation (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DayCounters {
    /// Consecutive days dry in the moisture control section (MCS) following the summer solstice.
    pub dry_days_after_summer_solstice: i32,
    /// Consecutive days moist in the MCS following the winter solstice.
    pub moist_days_after_winter_solstice: i32,
    /// Cumulative days the MCS is completely dry.
    pub num_cumulative_days_dry: i32,
    /// Cumulative days the MCS is partly moist and partly dry.
    pub num_cumulative_days_moist_dry: i32,
    /// Cumulative days the MCS is completely moist.
    pub num_cumulative_days_moist: i32,
    /// Cumulative days the MCS is dry while soil temperature at 50 cm exceeds 5 °C.
    pub num_cumulative_days_dry_over_5c: i32,
    /// Cumulative days the MCS is moist/dry while soil temperature at 50 cm exceeds 5 °C.
    pub num_cumulative_days_moist_dry_over_5c: i32,
    /// Cumulative days the MCS is moist while soil temperature at 50 cm exceeds 5 °C.
    pub num_cumulative_days_moist_over_5c: i32,
    /// Maximum consecutive days the MCS is moist in some part.
    pub num_consecutive_days_moist_in_some_parts: i32,
    /// Maximum consecutive days the MCS is moist in some part while soil temperature at 50 cm exceeds 8 °C.
    pub num_consecutive_days_moist_in_some_parts_over_8c: i32,
}

/// Output from [`crate::traits::ThermalModel::evaluate`] (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThermalClassification {
    /// Integer day counters evaluated over the 360-day year.
    pub day_counters: DayCounters,
    /// Classified soil temperature regime (`None` when input temperatures contain `NaN`).
    pub temperature_regime: Option<TemperatureRegime>,
    /// Classified soil moisture regime.
    pub moisture_regime: MoistureRegime,
    /// First-level moisture regime subdivision.
    pub regime_subdivision1: RegimeSubdivision1,
    /// Second-level moisture regime subdivision.
    pub regime_subdivision2: RegimeSubdivision2,
    /// Consecutive days with `z_wt <= 0.50 m` and `T(0.50 m) > 5.0 °C` (`0` in classic mode).
    pub aquic_bio_days: i32,
}

/// Complete simulation output matching `jNSMR::batch3()` plus Phase 4 diagnostics (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiteOutput {
    /// Total annual precipitation (mm).
    pub annual_rainfall: f64,
    /// Soil available water capacity (mm).
    pub water_holding_capacity: f64,
    /// Annual precipitation minus annual PET (mm).
    pub annual_water_balance: f64,
    /// Total annual potential evapotranspiration (mm).
    pub annual_potential_evapotranspiration: f64,
    /// Summer-quarter precipitation minus summer-quarter PET (mm).
    pub summer_water_balance: f64,
    /// Consecutive days dry in the MCS following the summer solstice.
    pub dry_days_after_summer_solstice: i32,
    /// Consecutive days moist in the MCS following the winter solstice.
    pub moist_days_after_winter_solstice: i32,
    /// Cumulative days the MCS is completely dry.
    pub num_cumulative_days_dry: i32,
    /// Cumulative days the MCS is partly moist and partly dry.
    pub num_cumulative_days_moist_dry: i32,
    /// Cumulative days the MCS is completely moist.
    pub num_cumulative_days_moist: i32,
    /// Cumulative days the MCS is dry while soil temperature at 50 cm exceeds 5 °C.
    pub num_cumulative_days_dry_over_5c: i32,
    /// Cumulative days the MCS is moist/dry while soil temperature at 50 cm exceeds 5 °C.
    pub num_cumulative_days_moist_dry_over_5c: i32,
    /// Cumulative days the MCS is moist while soil temperature at 50 cm exceeds 5 °C.
    pub num_cumulative_days_moist_over_5c: i32,
    /// Maximum consecutive days the MCS is moist in some part.
    pub num_consecutive_days_moist_in_some_parts: i32,
    /// Maximum consecutive days the MCS is moist in some part while soil temperature at 50 cm exceeds 8 °C.
    pub num_consecutive_days_moist_in_some_parts_over_8c: i32,
    /// `None` represents Java `""` / R `NA` when temperature contains `NaN`.
    pub temperature_regime: Option<TemperatureRegime>,
    /// Classified soil moisture regime.
    pub moisture_regime: MoistureRegime,
    /// First-level moisture regime subdivision.
    pub regime_subdivision1: RegimeSubdivision1,
    /// Second-level moisture regime subdivision.
    pub regime_subdivision2: RegimeSubdivision2,
    /// Cumulative days where water table is within the MCS (`0` in classic mode).
    pub sat_days_mcs: i32,
    /// Consecutive days where `z_wt <= 0.50 m` and `T(0.50 m) > 5.0 °C` (`0` in classic mode).
    pub aquic_bio_days: i32,
}

impl SiteOutput {
    /// Exact Java string for `temperatureRegime` (`""` when `None`).
    #[inline]
    pub const fn temperature_regime_java_str(&self) -> &'static str {
        match self.temperature_regime {
            Some(r) => r.as_java_str(),
            None => "",
        }
    }

    /// Exact Java string for `moistureRegime`.
    #[inline]
    pub const fn moisture_regime_java_str(&self) -> &'static str {
        self.moisture_regime.as_java_str()
    }

    /// Exact Java string for `regimeSubdivision1` (`" "` when undefined).
    #[inline]
    pub const fn regime_subdivision1_java_str(&self) -> &'static str {
        self.regime_subdivision1.as_java_str()
    }

    /// Exact Java string for `regimeSubdivision2` (`" "` for `Perudic`, `"Undefined"` for `Undefined`).
    #[inline]
    pub const fn regime_subdivision2_java_str(&self) -> &'static str {
        if matches!(self.moisture_regime, MoistureRegime::Perudic)
            && matches!(self.regime_subdivision2, RegimeSubdivision2::Undefined)
        {
            " "
        } else {
            self.regime_subdivision2.as_java_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_copy<T: Copy>() {}

    #[test]
    fn all_core_types_are_copy() {
        assert_copy::<Hemisphere>();
        assert_copy::<TemperatureRegime>();
        assert_copy::<MoistureRegime>();
        assert_copy::<RegimeSubdivision1>();
        assert_copy::<RegimeSubdivision2>();
        assert_copy::<SiteInput>();
        assert_copy::<SiteConfig>();
        assert_copy::<HalfMonthlyWaterTable>();
        assert_copy::<SoilState>();
        assert_copy::<WaterBalance>();
        assert_copy::<DayCounters>();
        assert_copy::<ThermalClassification>();
        assert_copy::<SiteOutput>();
    }

    #[test]
    fn soil_state_is_lean_and_indexes_by_half_month() {
        assert!(
            std::mem::size_of::<SoilState>() <= 1120,
            "SoilState unexpectedly large: {} bytes",
            std::mem::size_of::<SoilState>()
        );
        let mut wt = HalfMonthlyWaterTable::default();
        for d in 0..360 {
            assert_eq!(wt[d], f64::INFINITY);
        }
        for hm in 0..24 {
            wt.set_half_month(hm, (hm as f64) * 0.1);
        }
        for d in 0..360 {
            assert_eq!(wt[d], ((d / 15) as f64) * 0.1);
        }
    }

    #[test]
    fn temperature_regime_discriminants_and_labels() {
        let cases = [
            (0_u8, TemperatureRegime::Pergelic, "Pergelic"),
            (1, TemperatureRegime::Cryic, "Cryic"),
            (2, TemperatureRegime::Frigid, "Frigid"),
            (3, TemperatureRegime::Mesic, "Mesic"),
            (4, TemperatureRegime::Thermic, "Thermic"),
            (5, TemperatureRegime::Hyperthermic, "Hyperthermic"),
            (6, TemperatureRegime::Isofrigid, "Isofrigid"),
            (7, TemperatureRegime::Isomesic, "Isomesic"),
            (8, TemperatureRegime::Isothermic, "Isothermic"),
            (9, TemperatureRegime::Isohyperthermic, "Isohyperthermic"),
        ];
        for (code, variant, label) in cases {
            assert_eq!(variant as u8, code);
            assert_eq!(TemperatureRegime::from_u8(code), Some(variant));
            assert_eq!(variant.as_java_str(), label);
            assert_eq!(variant.as_r_label(), label);
        }
        assert_eq!(TemperatureRegime::from_u8(10), None);
    }

    #[test]
    fn moisture_regime_discriminants_and_labels() {
        let cases = [
            (0_u8, MoistureRegime::Aridic, "Aridic"),
            (1, MoistureRegime::Ustic, "Ustic"),
            (2, MoistureRegime::Xeric, "Xeric"),
            (3, MoistureRegime::Udic, "Udic"),
            (4, MoistureRegime::Perudic, "Perudic"),
            (5, MoistureRegime::Undefined, "Undefined"),
        ];
        for (code, variant, label) in cases {
            assert_eq!(variant as u8, code);
            assert_eq!(MoistureRegime::from_u8(code), Some(variant));
            assert_eq!(variant.as_java_str(), label);
            assert_eq!(variant.as_r_label(), label);
        }
        assert_eq!(MoistureRegime::from_u8(6), None);
    }

    #[test]
    fn subdivision1_discriminants_and_blank_vs_undefined() {
        let cases = [
            (0_u8, RegimeSubdivision1::Typic, "Typic", "Typic"),
            (1, RegimeSubdivision1::Weak, "Weak", "Weak"),
            (2, RegimeSubdivision1::Wet, "Wet", "Wet"),
            (3, RegimeSubdivision1::Dry, "Dry", "Dry"),
            (4, RegimeSubdivision1::Extreme, "Extreme", "Extreme"),
            (5, RegimeSubdivision1::Xeric, "Xeric", "Xeric"),
            (6, RegimeSubdivision1::Udic, "Udic", "Udic"),
            (7, RegimeSubdivision1::Aridic, "Aridic", "Aridic"),
            (8, RegimeSubdivision1::Undefined, " ", "Undefined"),
        ];
        for (code, variant, java_s, r_lbl) in cases {
            assert_eq!(variant as u8, code);
            assert_eq!(RegimeSubdivision1::from_u8(code), Some(variant));
            assert_eq!(variant.as_java_str(), java_s);
            assert_eq!(variant.as_r_label(), r_lbl);
        }
        assert_eq!(RegimeSubdivision1::from_u8(9), None);
    }

    #[test]
    fn subdivision2_discriminants_and_perudic_java_str() {
        let cases = [
            (0_u8, RegimeSubdivision2::Aridic, "Aridic"),
            (1, RegimeSubdivision2::Tempustic, "Tempustic"),
            (2, RegimeSubdivision2::Tropustic, "Tropustic"),
            (3, RegimeSubdivision2::Tempudic, "Tempudic"),
            (4, RegimeSubdivision2::Xeric, "Xeric"),
            (5, RegimeSubdivision2::Udic, "Udic"),
            (6, RegimeSubdivision2::Tropudic, "Tropudic"),
            (7, RegimeSubdivision2::Undefined, "Undefined"),
        ];
        for (code, variant, label) in cases {
            assert_eq!(variant as u8, code);
            assert_eq!(RegimeSubdivision2::from_u8(code), Some(variant));
            assert_eq!(variant.as_java_str(), label);
            assert_eq!(variant.as_r_label(), label);
        }
        assert_eq!(RegimeSubdivision2::from_u8(8), None);

        let base = SiteOutput {
            annual_rainfall: 0.0,
            water_holding_capacity: 200.0,
            annual_water_balance: 0.0,
            annual_potential_evapotranspiration: 0.0,
            summer_water_balance: 0.0,
            dry_days_after_summer_solstice: 0,
            moist_days_after_winter_solstice: 0,
            num_cumulative_days_dry: 0,
            num_cumulative_days_moist_dry: 0,
            num_cumulative_days_moist: 360,
            num_cumulative_days_dry_over_5c: 0,
            num_cumulative_days_moist_dry_over_5c: 0,
            num_cumulative_days_moist_over_5c: 0,
            num_consecutive_days_moist_in_some_parts: 360,
            num_consecutive_days_moist_in_some_parts_over_8c: 0,
            temperature_regime: None,
            moisture_regime: MoistureRegime::Perudic,
            regime_subdivision1: RegimeSubdivision1::Undefined,
            regime_subdivision2: RegimeSubdivision2::Undefined,
            sat_days_mcs: 0,
            aquic_bio_days: 0,
        };
        assert_eq!(base.temperature_regime_java_str(), "");
        assert_eq!(base.regime_subdivision1_java_str(), " ");
        assert_eq!(base.regime_subdivision2_java_str(), " ");

        let undef = SiteOutput {
            moisture_regime: MoistureRegime::Undefined,
            temperature_regime: Some(TemperatureRegime::Cryic),
            ..base
        };
        assert_eq!(undef.temperature_regime_java_str(), "Cryic");
        assert_eq!(undef.regime_subdivision1_java_str(), " ");
        assert_eq!(undef.regime_subdivision2_java_str(), "Undefined");
    }
}
