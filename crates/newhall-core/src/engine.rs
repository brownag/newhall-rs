//! Static-dispatch simulation runner (`SimEngine::run`), classic defaults,
//! and Struct-of-Arrays (SoA) batch execution.

use crate::forcing::ThornthwaiteForcing;
use crate::hydrology::ClassicHydrology;
use crate::thermal::ClassicThermal;
use crate::traits::{ForcingModel, HydrologyStep, SimEngine, ThermalModel};
use crate::types::{SiteConfig, SiteInput, SiteOutput};

/// Default classic Newhall engine matching `jNSMR` / `newhall-1.6.5.jar` (`runBatch2`).
pub type ClassicNewhall = SimEngine<ClassicThermal, ThornthwaiteForcing, ClassicHydrology>;

/// Struct-of-Arrays (SoA) flat slice view over `n` simulation sites.
///
/// Every slice (`lat_dd`, `lon_dd`, all 12 slices of `temp_c` and `precip_mm`,
/// `awc_mm`, and `elev_m`) must have identical length `n`.
#[derive(Debug, Clone, Copy)]
pub struct SoaBatchInput<'a> {
    pub lat_dd: &'a [f64],
    pub lon_dd: &'a [f64],
    /// Monthly mean air temperature slices (°C), Jan..Dec (`[0..12]`).
    pub temp_c: [&'a [f64]; 12],
    /// Monthly precipitation slices (mm), Jan..Dec (`[0..12]`).
    pub precip_mm: [&'a [f64]; 12],
    pub awc_mm: &'a [f64],
    pub elev_m: &'a [f64],
}

impl<'a> SoaBatchInput<'a> {
    /// Number of sites in the batch.
    #[inline]
    pub fn len(&self) -> usize {
        self.lat_dd.len()
    }

    /// Returns `true` if the batch contains zero sites.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.lat_dd.is_empty()
    }

    /// Verifies that all SoA input slices have matching length `self.len()`.
    pub fn validate(&self) {
        let n = self.len();
        assert_eq!(
            self.lon_dd.len(),
            n,
            "lon_dd slice length ({}) does not match lat_dd ({n})",
            self.lon_dd.len()
        );
        assert_eq!(
            self.awc_mm.len(),
            n,
            "awc_mm slice length ({}) does not match lat_dd ({n})",
            self.awc_mm.len()
        );
        assert_eq!(
            self.elev_m.len(),
            n,
            "elev_m slice length ({}) does not match lat_dd ({n})",
            self.elev_m.len()
        );
        for (m, slice) in self.temp_c.iter().enumerate() {
            assert_eq!(
                slice.len(),
                n,
                "temp_c[{m}] slice length ({}) does not match lat_dd ({n})",
                slice.len()
            );
        }
        for (m, slice) in self.precip_mm.iter().enumerate() {
            assert_eq!(
                slice.len(),
                n,
                "precip_mm[{m}] slice length ({}) does not match lat_dd ({n})",
                slice.len()
            );
        }
    }

    /// Materializes a stack-allocated `Copy` [`SiteInput`] for site `idx`.
    #[inline]
    pub fn site_input(&self, idx: usize) -> SiteInput {
        SiteInput {
            lat_dd: self.lat_dd[idx],
            lon_dd: self.lon_dd[idx],
            temp_c: std::array::from_fn(|m| self.temp_c[m][idx]),
            precip_mm: std::array::from_fn(|m| self.precip_mm[m][idx]),
            awc_mm: self.awc_mm[idx],
            elev_m: self.elev_m[idx],
        }
    }
}

impl<T, F, H> SimEngine<T, F, H>
where
    T: ThermalModel,
    F: ForcingModel,
    H: HydrologyStep,
{
    /// Runs the full 3-stage Newhall simulation for a single site with zero heap allocations.
    ///
    /// Leaves `config.is_metric` unconverted for `jNSMR` / `newhall-1.6.5.jar` parity (Defect #2).
    pub fn run(&self, input: &SiteInput, config: &SiteConfig) -> SiteOutput {
        let water_balance = self.forcing.compute_water_balance(input, config);
        let soil_state = self
            .hydrology
            .simulate_hydrology(input, config, &water_balance);
        let thermal = self
            .thermal
            .evaluate(input, config, &water_balance, &soil_state);

        let dc = thermal.day_counters;
        SiteOutput {
            annual_rainfall: water_balance.annual_rainfall,
            water_holding_capacity: input.awc_mm,
            annual_water_balance: water_balance.annual_water_balance,
            annual_potential_evapotranspiration: water_balance.annual_pet,
            summer_water_balance: water_balance.summer_water_balance,
            dry_days_after_summer_solstice: dc.dry_days_after_summer_solstice,
            moist_days_after_winter_solstice: dc.moist_days_after_winter_solstice,
            num_cumulative_days_dry: dc.num_cumulative_days_dry,
            num_cumulative_days_moist_dry: dc.num_cumulative_days_moist_dry,
            num_cumulative_days_moist: dc.num_cumulative_days_moist,
            num_cumulative_days_dry_over_5c: dc.num_cumulative_days_dry_over_5c,
            num_cumulative_days_moist_dry_over_5c: dc.num_cumulative_days_moist_dry_over_5c,
            num_cumulative_days_moist_over_5c: dc.num_cumulative_days_moist_over_5c,
            num_consecutive_days_moist_in_some_parts: dc.num_consecutive_days_moist_in_some_parts,
            num_consecutive_days_moist_in_some_parts_over_8c: dc
                .num_consecutive_days_moist_in_some_parts_over_8c,
            temperature_regime: thermal.temperature_regime,
            moisture_regime: thermal.moisture_regime,
            regime_subdivision1: thermal.regime_subdivision1,
            regime_subdivision2: thermal.regime_subdivision2,
        }
    }
}

impl<T, F, H> SimEngine<T, F, H>
where
    T: ThermalModel + Sync,
    F: ForcingModel + Sync,
    H: HydrologyStep + Sync,
{
    /// Executes an SoA batch into a caller-provided `out` slice (zero per-site heap allocations).
    pub fn run_batch_soa_into(
        &self,
        batch: &SoaBatchInput<'_>,
        config: &SiteConfig,
        out: &mut [SiteOutput],
    ) {
        batch.validate();
        assert_eq!(
            out.len(),
            batch.len(),
            "output slice length ({}) does not match batch length ({})",
            out.len(),
            batch.len()
        );

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            out.par_iter_mut().enumerate().for_each(|(idx, slot)| {
                let site = batch.site_input(idx);
                *slot = self.run(&site, config);
            });
        }

        #[cfg(not(feature = "parallel"))]
        {
            for (idx, slot) in out.iter_mut().enumerate() {
                let site = batch.site_input(idx);
                *slot = self.run(&site, config);
            }
        }
    }

    /// Executes an SoA batch and collects the results into a `Vec<SiteOutput>`.
    pub fn run_batch_soa(
        &self,
        batch: &SoaBatchInput<'_>,
        config: &SiteConfig,
    ) -> Vec<SiteOutput> {
        batch.validate();
        let n = batch.len();

        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            (0..n)
                .into_par_iter()
                .map(|idx| {
                    let site = batch.site_input(idx);
                    self.run(&site, config)
                })
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut out = Vec::with_capacity(n);
            for idx in 0..n {
                let site = batch.site_input(idx);
                out.push(self.run(&site, config));
            }
            out
        }
    }
}

/// Runs the classic Newhall simulation (`ClassicNewhall::default()`) for a single site.
#[inline]
pub fn simulate_site(input: &SiteInput, config: &SiteConfig) -> SiteOutput {
    ClassicNewhall::default().run(input, config)
}

/// Runs the classic Newhall simulation (`ClassicNewhall::default()`) across an SoA batch.
pub fn run_batch_soa(batch: &SoaBatchInput<'_>, config: &SiteConfig) -> Vec<SiteOutput> {
    ClassicNewhall::default().run_batch_soa(batch, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MoistureRegime, TemperatureRegime};

    fn sample_nh_site() -> SiteInput {
        SiteInput {
            lat_dd: 40.7934,
            lon_dd: -77.8600,
            temp_c: [
                -2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3,
            ],
            precip_mm: [
                71.0, 65.0, 84.0, 86.0, 101.0, 105.0, 96.0, 93.0, 91.0, 81.0, 84.0, 77.0,
            ],
            awc_mm: 200.0,
            elev_m: 350.0,
        }
    }

    fn sample_sh_site() -> SiteInput {
        SiteInput {
            lat_dd: -33.8688,
            lon_dd: 151.2093,
            temp_c: [
                22.1, 22.0, 20.9, 18.2, 15.2, 12.8, 12.0, 13.1, 15.3, 17.6, 19.4, 21.1,
            ],
            precip_mm: [
                102.0, 118.0, 130.0, 127.0, 120.0, 132.0, 97.0, 81.0, 69.0, 77.0, 84.0, 78.0,
            ],
            awc_mm: 200.0,
            elev_m: 19.0,
        }
    }

    fn sample_cold_site() -> SiteInput {
        SiteInput {
            lat_dd: 64.8378,
            lon_dd: -147.7164,
            temp_c: [
                -22.0, -18.0, -11.0, -1.0, 6.0, 7.5, 7.8, 6.2, 1.5, -6.0, -16.0, -21.0,
            ],
            precip_mm: [
                15.0, 11.0, 9.0, 8.0, 15.0, 35.0, 50.0, 48.0, 28.0, 20.0, 17.0, 16.0,
            ],
            awc_mm: 150.0,
            elev_m: 136.0,
        }
    }

    fn sample_nan_awc_site() -> SiteInput {
        let mut site = sample_nh_site();
        site.awc_mm = f64::NAN;
        site
    }

    fn assert_site_output_eq(actual: &SiteOutput, expected: &SiteOutput) {
        let eq_f64 = |a: f64, b: f64| (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits();
        assert!(eq_f64(actual.annual_rainfall, expected.annual_rainfall));
        assert!(eq_f64(
            actual.water_holding_capacity,
            expected.water_holding_capacity
        ));
        assert!(eq_f64(
            actual.annual_water_balance,
            expected.annual_water_balance
        ));
        assert!(eq_f64(
            actual.annual_potential_evapotranspiration,
            expected.annual_potential_evapotranspiration
        ));
        assert!(eq_f64(
            actual.summer_water_balance,
            expected.summer_water_balance
        ));
        assert_eq!(
            actual.dry_days_after_summer_solstice,
            expected.dry_days_after_summer_solstice
        );
        assert_eq!(
            actual.moist_days_after_winter_solstice,
            expected.moist_days_after_winter_solstice
        );
        assert_eq!(
            actual.num_cumulative_days_dry,
            expected.num_cumulative_days_dry
        );
        assert_eq!(
            actual.num_cumulative_days_moist_dry,
            expected.num_cumulative_days_moist_dry
        );
        assert_eq!(
            actual.num_cumulative_days_moist,
            expected.num_cumulative_days_moist
        );
        assert_eq!(
            actual.num_cumulative_days_dry_over_5c,
            expected.num_cumulative_days_dry_over_5c
        );
        assert_eq!(
            actual.num_cumulative_days_moist_dry_over_5c,
            expected.num_cumulative_days_moist_dry_over_5c
        );
        assert_eq!(
            actual.num_cumulative_days_moist_over_5c,
            expected.num_cumulative_days_moist_over_5c
        );
        assert_eq!(
            actual.num_consecutive_days_moist_in_some_parts,
            expected.num_consecutive_days_moist_in_some_parts
        );
        assert_eq!(
            actual.num_consecutive_days_moist_in_some_parts_over_8c,
            expected.num_consecutive_days_moist_in_some_parts_over_8c
        );
        assert_eq!(actual.temperature_regime, expected.temperature_regime);
        assert_eq!(actual.moisture_regime, expected.moisture_regime);
        assert_eq!(actual.regime_subdivision1, expected.regime_subdivision1);
        assert_eq!(actual.regime_subdivision2, expected.regime_subdivision2);
    }

    #[test]
    fn sim_engine_run_matches_manual_trait_pipeline() {
        let input = sample_nh_site();
        let config = SiteConfig::default();

        let engine = ClassicNewhall::default();
        let out = engine.run(&input, &config);
        let helper_out = simulate_site(&input, &config);

        let wb = ThornthwaiteForcing.compute_water_balance(&input, &config);
        let ss = ClassicHydrology.simulate_hydrology(&input, &config, &wb);
        let tc = ClassicThermal.evaluate(&input, &config, &wb, &ss);

        assert_site_output_eq(&out, &helper_out);
        assert_eq!(out.annual_rainfall.to_bits(), wb.annual_rainfall.to_bits());
        assert_eq!(out.water_holding_capacity.to_bits(), input.awc_mm.to_bits());
        assert_eq!(
            out.annual_potential_evapotranspiration.to_bits(),
            wb.annual_pet.to_bits()
        );
        assert_eq!(out.temperature_regime, Some(TemperatureRegime::Mesic));
        assert_eq!(out.moisture_regime, MoistureRegime::Udic);
        assert_eq!(
            out.num_cumulative_days_moist,
            tc.day_counters.num_cumulative_days_moist
        );
    }

    #[test]
    fn is_metric_false_does_not_convert_units() {
        let input = sample_nh_site();
        let metric = SiteConfig {
            is_metric: true,
            ..SiteConfig::default()
        };
        let imperial_flag = SiteConfig {
            is_metric: false,
            ..SiteConfig::default()
        };

        let out_metric = simulate_site(&input, &metric);
        let out_flag = simulate_site(&input, &imperial_flag);
        assert_site_output_eq(&out_flag, &out_metric);
    }

    #[test]
    fn run_batch_soa_and_into_match_simulate_site() {
        let sites = [
            sample_nh_site(),
            sample_sh_site(),
            sample_cold_site(),
            sample_nan_awc_site(),
        ];
        let config = SiteConfig::default();

        let lat_dd: Vec<f64> = sites.iter().map(|s| s.lat_dd).collect();
        let lon_dd: Vec<f64> = sites.iter().map(|s| s.lon_dd).collect();
        let awc_mm: Vec<f64> = sites.iter().map(|s| s.awc_mm).collect();
        let elev_m: Vec<f64> = sites.iter().map(|s| s.elev_m).collect();
        let temp_cols: [Vec<f64>; 12] =
            std::array::from_fn(|m| sites.iter().map(|s| s.temp_c[m]).collect());
        let precip_cols: [Vec<f64>; 12] =
            std::array::from_fn(|m| sites.iter().map(|s| s.precip_mm[m]).collect());

        let batch = SoaBatchInput {
            lat_dd: &lat_dd,
            lon_dd: &lon_dd,
            temp_c: std::array::from_fn(|m| temp_cols[m].as_slice()),
            precip_mm: std::array::from_fn(|m| precip_cols[m].as_slice()),
            awc_mm: &awc_mm,
            elev_m: &elev_m,
        };

        let batch_out = run_batch_soa(&batch, &config);
        assert_eq!(batch_out.len(), sites.len());

        let mut into_out = vec![simulate_site(&sites[0], &config); sites.len()];
        ClassicNewhall::default().run_batch_soa_into(&batch, &config, &mut into_out);

        for (i, site) in sites.iter().enumerate() {
            let expected = simulate_site(site, &config);
            assert_site_output_eq(&batch_out[i], &expected);
            assert_site_output_eq(&into_out[i], &expected);
        }
        assert!(batch_out[3].water_holding_capacity.is_nan());
    }

    #[test]
    fn run_batch_soa_handles_empty_batch() {
        let empty: [f64; 0] = [];
        let batch = SoaBatchInput {
            lat_dd: &empty,
            lon_dd: &empty,
            temp_c: [&empty; 12],
            precip_mm: [&empty; 12],
            awc_mm: &empty,
            elev_m: &empty,
        };
        assert!(batch.is_empty());

        let out = run_batch_soa(&batch, &SiteConfig::default());
        assert!(out.is_empty());

        let mut into_out: [SiteOutput; 0] = [];
        ClassicNewhall::default().run_batch_soa_into(&batch, &SiteConfig::default(), &mut into_out);
    }

    #[test]
    #[should_panic(expected = "temp_c[0] slice length")]
    fn run_batch_soa_panics_on_mismatched_slice_lengths() {
        let one = [40.0_f64];
        let two = [10.0_f64, 20.0_f64];
        let batch = SoaBatchInput {
            lat_dd: &one,
            lon_dd: &one,
            temp_c: [
                &two, &one, &one, &one, &one, &one, &one, &one, &one, &one, &one, &one,
            ],
            precip_mm: [&one; 12],
            awc_mm: &one,
            elev_m: &one,
        };
        let _ = run_batch_soa(&batch, &SiteConfig::default());
    }
}
