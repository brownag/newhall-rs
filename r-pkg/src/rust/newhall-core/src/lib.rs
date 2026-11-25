//! Pure-Rust Newhall Simulation Model (`newhall-core`).

pub mod engine;
pub mod forcing;
pub mod hydrology;
pub mod thermal;
pub mod thermal_1d;
pub mod traits;
pub mod types;
pub mod water_table;

pub use engine::{
    run_batch_soa, simulate_site, ClassicNewhall, SoaBatchInput, StrictHydrology, StrictNewhall,
};
pub use forcing::{StrictThornthwaiteForcing, ThornthwaiteForcing};
pub use hydrology::ClassicHydrology;
pub use thermal::{ClassicThermal, StrictThermal};
pub use thermal_1d::{SoilTextureClass, SoilThermalState, Thermal1DConfig, Thermal1DModel};
pub use traits::{ForcingModel, HydrologyStep, SimEngine, ThermalModel};
pub use types::{
    DayCounters, Hemisphere, MoistureRegime, RegimeSubdivision1, RegimeSubdivision2, SiteConfig,
    SiteInput, SiteOutput, SoilState, TemperatureRegime, ThermalClassification, WaterBalance,
};
pub use water_table::{
    StepMassBalance, WaterTableConfig, WaterTableHydrology, WaterTableMode, WaterTableState,
};
