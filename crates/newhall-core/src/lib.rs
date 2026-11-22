//! Pure-Rust Newhall Simulation Model (`newhall-core`).

pub mod engine;
pub mod forcing;
pub mod hydrology;
pub mod thermal;
pub mod traits;
pub mod types;

pub use traits::{ForcingModel, HydrologyStep, SimEngine, ThermalModel};
pub use types::{
    DayCounters, Hemisphere, MoistureRegime, RegimeSubdivision1, RegimeSubdivision2, SiteConfig,
    SiteInput, SiteOutput, SoilState, TemperatureRegime, ThermalClassification, WaterBalance,
};
