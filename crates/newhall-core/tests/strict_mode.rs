//! Integration regression tests for strict-mode remediation of audited `jNSMR` Defects #1–#10.
//!
//! Compares [`ClassicNewhall`], [`StrictNewhall`], and hydrothermal
//! `SimEngine<Thermal1DModel, StrictThornthwaiteForcing, WaterTableHydrology>`.

use newhall_core::{
    ClassicNewhall, ForcingModel, MoistureRegime, RegimeSubdivision1, RegimeSubdivision2,
    SimEngine, SiteConfig, SiteInput, StrictNewhall, StrictThornthwaiteForcing, TemperatureRegime,
    Thermal1DConfig, Thermal1DModel, ThornthwaiteForcing, WaterTableConfig, WaterTableHydrology,
    WaterTableMode,
};

fn make_site(lat_dd: f64, temp_c: [f64; 12], precip_mm: [f64; 12], awc_mm: f64) -> SiteInput {
    SiteInput {
        lat_dd,
        lon_dd: 0.0,
        temp_c,
        precip_mm,
        awc_mm,
        elev_m: 200.0,
    }
}

fn hydrothermal_strict_engine(
) -> SimEngine<Thermal1DModel, StrictThornthwaiteForcing, WaterTableHydrology> {
    SimEngine::new(
        Thermal1DModel::new(Thermal1DConfig {
            max_spinup_years: 2,
            ..Thermal1DConfig::default()
        }),
        StrictThornthwaiteForcing,
        WaterTableHydrology::new(WaterTableConfig {
            mode: WaterTableMode::PrescribedMonthly([2.0; 12]),
            ..WaterTableConfig::default()
        }),
    )
}

fn hydrothermal_classic_forcing_engine(
) -> SimEngine<Thermal1DModel, ThornthwaiteForcing, WaterTableHydrology> {
    SimEngine::new(
        Thermal1DModel::new(Thermal1DConfig {
            max_spinup_years: 2,
            ..Thermal1DConfig::default()
        }),
        ThornthwaiteForcing,
        WaterTableHydrology::new(WaterTableConfig {
            mode: WaterTableMode::PrescribedMonthly([2.0; 12]),
            ..WaterTableConfig::default()
        }),
    )
}

fn assert_close(actual: f64, expected: f64, tol: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "expected {expected:.12}, got {actual:.12} (diff {diff:.3e} > {tol:.1e})"
    );
}

#[test]
fn defect_1_and_7_southern_hemisphere_pet_and_negative_lat() {
    let temps = [20.0; 12];
    let precip = [80.0; 12];
    let cfg = SiteConfig::default();

    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let p4_classic = hydrothermal_classic_forcing_engine();
    let p4_strict = hydrothermal_strict_engine();

    // Defect #1 + #7: At -12.5 deg S, ClassicNewhall hits the 0..5 SH branch with signed
    // negative lat_dd (-750 minutes), whereas StrictNewhall uses |lat_dd| = 12.5 and
    // properly parenthesized `(RS[nrow] - RS[nrow - 1]) * 60.0`.
    let site_12_5 = make_site(-12.5, temps, precip, 200.0);
    let out_c = classic.run(&site_12_5, &cfg);
    let out_s = strict.run(&site_12_5, &cfg);
    let p4_out_c = p4_classic.run(&site_12_5, &cfg);
    let p4_out_s = p4_strict.run(&site_12_5, &cfg);

    assert!(out_s.annual_potential_evapotranspiration > 0.0);
    assert_ne!(
        out_c.annual_potential_evapotranspiration.to_bits(),
        out_s.annual_potential_evapotranspiration.to_bits()
    );
    assert_close(
        p4_out_s.annual_potential_evapotranspiration,
        out_s.annual_potential_evapotranspiration,
        1e-9,
    );
    assert_close(
        p4_out_s.summer_water_balance,
        out_s.summer_water_balance,
        1e-9,
    );
    assert_ne!(
        p4_out_c.annual_potential_evapotranspiration.to_bits(),
        p4_out_s.annual_potential_evapotranspiration.to_bits()
    );

    // At -22.5 deg S, ClassicNewhall produces distorted negative-latitude extrapolation
    // whereas StrictThornthwaiteForcing produces positive, monotonic January PET across SH.
    let lats = [-2.5, -5.0, -7.5, -12.5, -22.5, -35.0, -50.0];
    let mut prev_jan_mpe = 0.0_f64;
    for &lat in &lats {
        let site = make_site(lat, temps, precip, 200.0);
        let wb = StrictThornthwaiteForcing.compute_water_balance(&site, &cfg);
        assert!(
            wb.mpe.iter().all(|&m| m.is_finite() && m > 0.0),
            "expected all positive mpe at {lat}, got {:?}",
            wb.mpe
        );
        assert!(
            wb.mpe[0] > prev_jan_mpe,
            "expected Jan mpe at {lat} ({}) > {prev_jan_mpe}",
            wb.mpe[0]
        );
        prev_jan_mpe = wb.mpe[0];

        let s_out = strict.run(&site, &cfg);
        let p4_out = p4_strict.run(&site, &cfg);
        assert_close(
            p4_out.annual_potential_evapotranspiration,
            wb.annual_pet,
            1e-9,
        );
        assert_close(
            s_out.annual_potential_evapotranspiration,
            wb.annual_pet,
            1e-9,
        );
    }

    // Extreme negative latitude (-60.0) clamps to -50.0 in StrictNewhall / hydrothermal strict forcing,
    // whereas ClassicNewhall keeps extrapolating negatively.
    let site_50 = make_site(-50.0, temps, precip, 200.0);
    let site_60 = make_site(-60.0, temps, precip, 200.0);
    assert_close(
        strict
            .run(&site_60, &cfg)
            .annual_potential_evapotranspiration,
        strict
            .run(&site_50, &cfg)
            .annual_potential_evapotranspiration,
        1e-12,
    );
    assert_close(
        p4_strict
            .run(&site_60, &cfg)
            .annual_potential_evapotranspiration,
        p4_strict
            .run(&site_50, &cfg)
            .annual_potential_evapotranspiration,
        1e-12,
    );
    assert_ne!(
        classic
            .run(&site_60, &cfg)
            .annual_potential_evapotranspiration
            .to_bits(),
        classic
            .run(&site_50, &cfg)
            .annual_potential_evapotranspiration
            .to_bits()
    );
}

#[test]
fn defect_2_is_metric_false_equivalence() {
    let metric_site = make_site(
        -33.5,
        [
            21.0, 20.5, 18.5, 15.0, 12.0, 9.5, 9.0, 10.5, 13.0, 15.5, 18.0, 20.0,
        ],
        [
            35.0, 30.0, 40.0, 55.0, 75.0, 90.0, 85.0, 70.0, 55.0, 45.0, 40.0, 35.0,
        ],
        200.0,
    );
    let metric_cfg = SiteConfig::default();

    let mut imperial_site = metric_site;
    for m in 0..12 {
        imperial_site.temp_c[m] = metric_site.temp_c[m] * 9.0 / 5.0 + 32.0;
        imperial_site.precip_mm[m] = metric_site.precip_mm[m] / 25.4;
    }
    imperial_site.awc_mm = metric_site.awc_mm / 25.4;
    let imperial_cfg = SiteConfig {
        soil_air_offset: metric_cfg.soil_air_offset * 9.0 / 5.0,
        is_metric: false,
        ..metric_cfg
    };

    // ClassicNewhall ignores `is_metric = false` completely.
    let classic = ClassicNewhall::default();
    let classic_raw_metric_flag = classic.run(&imperial_site, &metric_cfg);
    let classic_raw_imperial_flag = classic.run(&imperial_site, &imperial_cfg);
    assert_eq!(classic_raw_metric_flag, classic_raw_imperial_flag);

    // StrictNewhall converts °F -> °C, in -> mm, and soil_air_offset * 5/9 to match metric.
    let strict = StrictNewhall::default();
    let out_metric = strict.run(&metric_site, &metric_cfg);
    let out_imperial = strict.run(&imperial_site, &imperial_cfg);

    assert_close(
        out_imperial.annual_rainfall,
        out_metric.annual_rainfall,
        1e-9,
    );
    assert_close(
        out_imperial.annual_water_balance,
        out_metric.annual_water_balance,
        1e-9,
    );
    assert_close(
        out_imperial.annual_potential_evapotranspiration,
        out_metric.annual_potential_evapotranspiration,
        1e-9,
    );
    assert_close(
        out_imperial.summer_water_balance,
        out_metric.summer_water_balance,
        1e-9,
    );
    // SimEngine::run stores raw `input.awc_mm`, while StrictHydrology scales it internally.
    assert_eq!(
        out_imperial.water_holding_capacity.to_bits(),
        imperial_site.awc_mm.to_bits()
    );
    assert_eq!(
        out_imperial.dry_days_after_summer_solstice,
        out_metric.dry_days_after_summer_solstice
    );
    assert_eq!(
        out_imperial.moist_days_after_winter_solstice,
        out_metric.moist_days_after_winter_solstice
    );
    assert_eq!(
        out_imperial.num_cumulative_days_dry,
        out_metric.num_cumulative_days_dry
    );
    assert_eq!(
        out_imperial.num_cumulative_days_moist_dry,
        out_metric.num_cumulative_days_moist_dry
    );
    assert_eq!(
        out_imperial.num_cumulative_days_moist,
        out_metric.num_cumulative_days_moist
    );
    assert_eq!(
        out_imperial.num_cumulative_days_dry_over_5c,
        out_metric.num_cumulative_days_dry_over_5c
    );
    assert_eq!(
        out_imperial.num_cumulative_days_moist_dry_over_5c,
        out_metric.num_cumulative_days_moist_dry_over_5c
    );
    assert_eq!(
        out_imperial.num_cumulative_days_moist_over_5c,
        out_metric.num_cumulative_days_moist_over_5c
    );
    assert_eq!(
        out_imperial.num_consecutive_days_moist_in_some_parts,
        out_metric.num_consecutive_days_moist_in_some_parts
    );
    assert_eq!(
        out_imperial.num_consecutive_days_moist_in_some_parts_over_8c,
        out_metric.num_consecutive_days_moist_in_some_parts_over_8c
    );
    assert_eq!(
        out_imperial.temperature_regime,
        out_metric.temperature_regime
    );
    assert_eq!(out_imperial.moisture_regime, out_metric.moisture_regime);
    assert_eq!(
        out_imperial.regime_subdivision1,
        out_metric.regime_subdivision1
    );
    assert_eq!(
        out_imperial.regime_subdivision2,
        out_metric.regime_subdivision2
    );

    // Hydrothermal with StrictThornthwaiteForcing also converts forcing inputs when `is_metric = false`.
    let p4_strict = hydrothermal_strict_engine();
    let p4_metric = p4_strict.run(&metric_site, &metric_cfg);
    let p4_imperial = p4_strict.run(&imperial_site, &imperial_cfg);
    assert_close(p4_imperial.annual_rainfall, p4_metric.annual_rainfall, 1e-9);
    assert_close(
        p4_imperial.annual_potential_evapotranspiration,
        p4_metric.annual_potential_evapotranspiration,
        1e-9,
    );
    assert_close(
        p4_imperial.summer_water_balance,
        p4_metric.summer_water_balance,
        1e-9,
    );
}

#[test]
fn defect_3_pergelic_not_overwritten_by_frigid_or_isofrigid() {
    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let cfg = SiteConfig::default();

    // Case A: tma < 0 and st < 0 with dif*fcd >= 6 -> Classic overwrites Pergelic with Frigid.
    let site_frigid = make_site(
        65.0,
        [
            -20.0, -20.0, -15.0, -10.0, -8.0, -5.0, -5.0, -5.0, -10.0, -12.0, -15.0, -20.0,
        ],
        [40.0; 12],
        200.0,
    );
    assert_eq!(
        classic.run(&site_frigid, &cfg).temperature_regime,
        Some(TemperatureRegime::Frigid)
    );
    assert_eq!(
        strict.run(&site_frigid, &cfg).temperature_regime,
        Some(TemperatureRegime::Pergelic)
    );

    // Case B: tma < 0 and st < 0 with dif*fcd < 6 -> Classic overwrites Pergelic with Isofrigid.
    let site_isofrigid = make_site(65.0, [-5.0; 12], [40.0; 12], 200.0);
    assert_eq!(
        classic.run(&site_isofrigid, &cfg).temperature_regime,
        Some(TemperatureRegime::Isofrigid)
    );
    assert_eq!(
        strict.run(&site_isofrigid, &cfg).temperature_regime,
        Some(TemperatureRegime::Pergelic)
    );

    // Case C: tma < 0 and st >= cryic_ht (O-horizon + saturated => cryic_ht = 6.0, st = 6.5).
    let cfg_o_sat = SiteConfig {
        has_o_horizon: true,
        is_saturated: true,
        ..cfg
    };
    let site_high_summer = make_site(
        65.0,
        [
            -10.0, -10.0, -6.0, -4.0, -2.0, 4.0, 4.0, 4.0, -2.0, -4.0, -6.0, -10.0,
        ],
        [40.0; 12],
        200.0,
    );
    assert_eq!(
        classic
            .run(&site_high_summer, &cfg_o_sat)
            .temperature_regime,
        Some(TemperatureRegime::Frigid)
    );
    assert_eq!(
        strict.run(&site_high_summer, &cfg_o_sat).temperature_regime,
        Some(TemperatureRegime::Pergelic)
    );
}

#[test]
fn defect_4_attenuated_abs_dif_in_smr() {
    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let cfg = SiteConfig::default(); // amplitude = 0.66

    // Site with 30 <= nd[0] + nd[1] < 90 (Udic Dry, nccd < 45) and air (st - wt) = 6.0 °C:
    // Unattenuated dif = 6.0 >= 5.0 (Classic -> Tempudic),
    // Attenuated dif = 6.0 * 0.66 = 3.96 < 5.0 (Strict -> Tropudic).
    let site_attenuated = make_site(
        15.0,
        [
            10.0, 10.0, 13.0, 13.0, 13.0, 16.0, 16.0, 16.0, 13.0, 13.0, 13.0, 10.0,
        ],
        [
            80.0, 0.0, 0.0, 0.0, 80.0, 80.0, 80.0, 80.0, 80.0, 80.0, 80.0, 80.0,
        ],
        40.0,
    );
    let out_c = classic.run(&site_attenuated, &cfg);
    let out_s = strict.run(&site_attenuated, &cfg);
    assert_eq!(out_c.moisture_regime, MoistureRegime::Udic);
    assert_eq!(out_s.moisture_regime, MoistureRegime::Udic);
    assert_eq!(out_c.regime_subdivision2, RegimeSubdivision2::Tempudic);
    assert_eq!(out_s.regime_subdivision2, RegimeSubdivision2::Tropudic);

    // Sign-inverted seasonal temperatures (wt > st, so st - wt = -10.0, |st - wt| * 0.66 = 6.6 >= 5.0):
    // Classic lacks `.abs()` (dif = -10.0 < 5.0 -> Tropustic), Strict uses `.abs()` (-> Tempustic).
    let site_inverted = make_site(
        15.0,
        [
            22.0, 22.0, 17.0, 17.0, 17.0, 12.0, 12.0, 12.0, 17.0, 17.0, 17.0, 22.0,
        ],
        [
            5.0, 5.0, 5.0, 90.0, 90.0, 90.0, 90.0, 90.0, 90.0, 90.0, 5.0, 5.0,
        ],
        80.0,
    );
    let inv_c = classic.run(&site_inverted, &cfg);
    let inv_s = strict.run(&site_inverted, &cfg);
    assert_eq!(inv_c.moisture_regime, MoistureRegime::Ustic);
    assert_eq!(inv_s.moisture_regime, MoistureRegime::Ustic);
    assert_eq!(inv_c.regime_subdivision2, RegimeSubdivision2::Tropustic);
    assert_eq!(inv_s.regime_subdivision2, RegimeSubdivision2::Tempustic);
}

#[test]
fn defect_5_cold_site_ncpm2_is_zero() {
    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let cfg = SiteConfig::default();

    // All 12 months < 8.0°C (np8 == 0), with dry summer/autumn so ncpm1 is strictly in (0, 360).
    let cold_site = make_site(
        55.0,
        [
            -5.0, -3.0, 0.0, 2.0, 4.0, 6.0, 7.0, 6.0, 4.0, 2.0, 0.0, -3.0,
        ],
        [
            80.0, 80.0, 80.0, 80.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 80.0, 80.0,
        ],
        50.0,
    );

    let out_c = classic.run(&cold_site, &cfg);
    let out_s = strict.run(&cold_site, &cfg);

    assert!(out_c.num_consecutive_days_moist_in_some_parts > 0);
    assert!(out_c.num_consecutive_days_moist_in_some_parts < 360);
    assert_eq!(
        out_c.num_consecutive_days_moist_in_some_parts_over_8c,
        out_c.num_consecutive_days_moist_in_some_parts
    );
    assert_eq!(
        out_s.num_consecutive_days_moist_in_some_parts,
        out_c.num_consecutive_days_moist_in_some_parts
    );
    assert_eq!(out_s.num_consecutive_days_moist_in_some_parts_over_8c, 0);
}

#[test]
fn defect_6_bimodal_over_5c_accumulates_all_intervals() {
    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let cfg = SiteConfig::default();

    // Two distinct >5°C intervals: Feb-Mar (moist) and Jul-Aug (dry).
    let bimodal_site = make_site(
        40.0,
        [2.0, 8.0, 8.0, 2.0, 2.0, 2.0, 8.0, 8.0, 2.0, 2.0, 2.0, 2.0],
        [
            100.0, 100.0, 100.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 100.0, 100.0, 100.0,
        ],
        40.0,
    );

    let out_c = classic.run(&bimodal_site, &cfg);
    let out_s = strict.run(&bimodal_site, &cfg);

    // Classic stops accumulating `nsd` after the first >5°C interval (ignoring dry Jul-Aug).
    assert_eq!(out_c.num_cumulative_days_dry_over_5c, 0);
    // Strict accumulates `nsd` across all >5°C intervals.
    assert!(out_s.num_cumulative_days_dry_over_5c > 0);
    assert_eq!(
        out_s.num_cumulative_days_moist_over_5c,
        out_c.num_cumulative_days_moist_over_5c
    );
    let total_c = out_c.num_cumulative_days_dry_over_5c
        + out_c.num_cumulative_days_moist_dry_over_5c
        + out_c.num_cumulative_days_moist_over_5c;
    let total_s = out_s.num_cumulative_days_dry_over_5c
        + out_s.num_cumulative_days_moist_dry_over_5c
        + out_s.num_cumulative_days_moist_over_5c;
    assert!(total_s > total_c);
}

#[test]
fn defect_9_nan_handling_yields_undefined_not_perudic() {
    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let p4_strict = hydrothermal_strict_engine();
    let cfg = SiteConfig::default();

    // NaN in precipitation on an otherwise very wet site:
    // Classic misclassifies as Perudic; Strict yields MoistureRegime::Undefined.
    let mut nan_precip_site = make_site(40.0, [15.0; 12], [200.0; 12], 200.0);
    nan_precip_site.precip_mm[6] = f64::NAN;

    let out_c = classic.run(&nan_precip_site, &cfg);
    let out_s = strict.run(&nan_precip_site, &cfg);
    assert_eq!(out_c.moisture_regime, MoistureRegime::Perudic);
    assert_eq!(out_s.moisture_regime, MoistureRegime::Undefined);
    assert_eq!(out_s.regime_subdivision1, RegimeSubdivision1::Undefined);
    assert_eq!(out_s.regime_subdivision2, RegimeSubdivision2::Undefined);
    assert!(out_s.annual_rainfall.is_nan());
    assert!(out_s.annual_potential_evapotranspiration.is_nan());

    // Hydrothermal with StrictThornthwaiteForcing propagates NaN forcing cleanly without panicking.
    let p4_nan_p = p4_strict.run(&nan_precip_site, &cfg);
    assert!(p4_nan_p.annual_rainfall.is_nan());
    assert!(p4_nan_p.annual_potential_evapotranspiration.is_nan());

    // NaN in temperature: StrictNewhall yields temperature_regime=None and Undefined SMR.
    let mut nan_temp_site = make_site(40.0, [15.0; 12], [80.0; 12], 200.0);
    nan_temp_site.temp_c[3] = f64::NAN;
    let out_nan_t = strict.run(&nan_temp_site, &cfg);
    assert_eq!(out_nan_t.temperature_regime, None);
    assert_eq!(out_nan_t.moisture_regime, MoistureRegime::Undefined);
    assert_eq!(out_nan_t.regime_subdivision1, RegimeSubdivision1::Undefined);
    assert_eq!(out_nan_t.regime_subdivision2, RegimeSubdivision2::Undefined);
    assert!(out_nan_t.annual_potential_evapotranspiration.is_nan());
}

#[test]
fn defect_10_exact_5_and_8_plateaus_without_perturbation() {
    let classic = ClassicNewhall::default();
    let strict = StrictNewhall::default();
    let cfg = SiteConfig {
        soil_air_offset: 0.0,
        ..SiteConfig::default()
    };

    // Plateau touching 5.0°C from below without ever exceeding 5.0°C.
    let touch_5 = make_site(
        50.0,
        [0.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [80.0; 12],
        200.0,
    );
    let out5_c = classic.run(&touch_5, &cfg);
    let out5_s = strict.run(&touch_5, &cfg);
    assert!(out5_c.num_cumulative_days_moist_over_5c > 0);
    assert_eq!(out5_s.num_cumulative_days_moist_over_5c, 0);

    // Plateau touching 8.0°C from below without ever exceeding 8.0°C.
    let touch_8 = make_site(
        50.0,
        [0.0, 8.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [80.0; 12],
        200.0,
    );
    let out8_c = classic.run(&touch_8, &cfg);
    let out8_s = strict.run(&touch_8, &cfg);
    assert!(out8_c.num_consecutive_days_moist_in_some_parts_over_8c > 0);
    assert_eq!(out8_s.num_consecutive_days_moist_in_some_parts_over_8c, 0);
}
