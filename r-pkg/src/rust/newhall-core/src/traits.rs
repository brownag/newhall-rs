//! Static-dispatch physics traits and generic simulation engine definition.

use crate::types::{SiteConfig, SiteInput, SoilState, ThermalClassification, WaterBalance};

/// Atmospheric forcing / potential evapotranspiration (PET) model.
pub trait ForcingModel {
    /// Computes monthly PET and seasonal/annual water balances for a single site.
    fn compute_water_balance(&self, input: &SiteInput, config: &SiteConfig) -> WaterBalance;
}

/// Soil water balance and moisture control section (MCS) hydrology model.
pub trait HydrologyStep {
    /// Simulates soil moisture equilibration and produces the final 360-day calendar.
    fn simulate_hydrology(
        &self,
        input: &SiteInput,
        config: &SiteConfig,
        water_balance: &WaterBalance,
    ) -> SoilState;
}

/// Soil thermal regime, growing-season day counter, and SMR/STR classifier.
pub trait ThermalModel {
    /// Evaluates thermal regimes, day counters, and moisture regime classifications.
    fn evaluate(
        &self,
        input: &SiteInput,
        config: &SiteConfig,
        water_balance: &WaterBalance,
        soil_state: &SoilState,
    ) -> ThermalClassification;
}

/// Static-dispatch Newhall simulation engine composing thermal, forcing, and hydrology models.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimEngine<T, F, H> {
    pub thermal: T,
    pub forcing: F,
    pub hydrology: H,
}

impl<T, F, H> SimEngine<T, F, H> {
    /// Constructs a new static-dispatch simulation engine from component models.
    pub const fn new(thermal: T, forcing: F, hydrology: H) -> Self {
        Self {
            thermal,
            forcing,
            hydrology,
        }
    }
}
