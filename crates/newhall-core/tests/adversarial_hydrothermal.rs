//! Adversarial validation, analytical benchmarks, stability torture tests,
//! and 10,000+ case `proptest` fuzzing for hydrothermal physics extensions.

use std::f64::consts::TAU;

use newhall_core::thermal_1d::{
    interp_at_depth, step_backward_euler, stretched_grid, surface_resistance, total_enthalpy,
    NodeMaterial, SoilThermalState, THERMAL_NODES,
};
use newhall_core::{
    ClassicHydrology, ForcingModel, HydrologyStep, SimEngine, SiteConfig, SiteInput,
    SoilTextureClass, Thermal1DConfig, Thermal1DModel, ThornthwaiteForcing, WaterTableConfig,
    WaterTableHydrology, WaterTableMode, WaterTableState,
};
use proptest::prelude::*;

const DT_DAY: f64 = 86_400.0;

// ============================================================================
// 1. Conservation & Thermodynamic Invariants
// ============================================================================

#[test]
fn thermal_energy_conservation_insulated_step_profile() {
    let (z, dz) = stretched_grid();
    let mut state = SoilThermalState {
        z,
        dz,
        ..SoilThermalState::default()
    };
    for (i, t) in state.temp_c.iter_mut().enumerate() {
        *t = if i < 8 { 20.0 } else { 0.0 };
    }
    let mats = [NodeMaterial {
        theta_tot: 0.30,
        ..NodeMaterial::default()
    }; THERMAL_NODES];

    let h_init = total_enthalpy(&state, &mats);
    for _ in 0..1000 {
        step_backward_euler(&mut state, &mats, 50.0, f64::INFINITY, DT_DAY);
    }
    let h_final = total_enthalpy(&state, &mats);
    let rel_err = (h_final - h_init).abs() / h_init.abs().max(1.0);
    assert!(
        rel_err < 1e-6,
        "Enthalpy not conserved: init={h_init}, final={h_final}, rel_err={rel_err}"
    );
}

#[test]
fn second_law_monotonicity_under_constant_t_air() {
    let (z, dz) = stretched_grid();
    let mut state = SoilThermalState {
        z,
        dz,
        ..SoilThermalState::default()
    };
    for (i, t) in state.temp_c.iter_mut().enumerate() {
        *t = (i as f64) * 1.5 - 5.0; // -5.0 to +17.5 C
    }
    let t_air = 3.0;
    let t_min_bound = state.temp_c.iter().copied().fold(t_air, f64::min);
    let t_max_bound = state.temp_c.iter().copied().fold(t_air, f64::max);

    let mats = [NodeMaterial::default(); THERMAL_NODES];
    let r_surf = surface_resistance(0.2, 0.15, 0.05, 0.05, 15.0);

    for step in 0..500 {
        step_backward_euler(&mut state, &mats, t_air, r_surf, DT_DAY);
        for (i, &t) in state.temp_c.iter().enumerate() {
            assert!(
                t >= t_min_bound - 1e-10 && t <= t_max_bound + 1e-10,
                "Second Law violation at step {step}, node {i}: T={t} outside [{t_min_bound}, {t_max_bound}]"
            );
        }
    }
}

#[test]
fn prognostic_water_mass_balance_exact_per_step() {
    let wt_cfg = WaterTableConfig {
        mode: WaterTableMode::Prognostic {
            z_imp_m: 1.5,
            sy: 0.25,
            k_lat_per_day: 0.02,
        },
        z_prof_m: 1.0,
        k_sat_mm_per_day: 80.0,
        texture: SoilTextureClass::Medium,
        enable_capillary: true,
    };
    let model = WaterTableHydrology::new(wt_cfg);
    let fsl = 200.0 / 64.0;
    let mut sl = [fsl * 0.5; 64];
    let mut wt = WaterTableState {
        h_wt_m: 0.3,
        z_wt_m: 1.2,
    };

    for step in 0..240 {
        let precip = 30.0 + 40.0 * ((step as f64) * 0.2).sin().max(0.0);
        let pet = 20.0 + 15.0 * ((step as f64) * 0.2).cos().max(0.0);
        let mb = model.step_prognostic(&mut sl, &mut wt, fsl, precip, pet, 15.0);
        let lhs = mb.delta_s_matrix_mm + mb.sy_delta_h_mm;
        let rhs = mb.precip_mm - mb.aet_mm - mb.q_lat_mm - mb.r_surf_mm;
        let err = (lhs - rhs).abs();
        assert!(
            err < 1e-9,
            "Mass balance error at step {step}: lhs={lhs}, rhs={rhs}, err={err}"
        );
    }
}

// ============================================================================
// 2. Analytical & Limiting-Case Degeneracy Tests
// ============================================================================

#[test]
fn carslaw_jaeger_harmonic_damping_benchmark() {
    let cv = 2.2e6_f64;
    let lambda = 1.5_f64;
    let mats = [NodeMaterial {
        uniform_override: Some((cv, lambda)),
        ..NodeMaterial::default()
    }; THERMAL_NODES];

    let (z, dz) = stretched_grid();
    let t_mean = 12.0_f64;
    let amp = 15.0_f64;
    // 30-day period => damping depth d ~ 0.75 m << 5.0 m domain, avoiding bottom reflection
    let period_days = 30_usize;
    let period_sec = (period_days as f64) * DT_DAY;
    let omega = TAU / period_sec;
    let d_damp = (2.0 * lambda / (omega * cv)).sqrt();

    let mut state = SoilThermalState {
        z,
        dz,
        temp_c: [t_mean; THERMAL_NODES],
        c_app: [cv; THERMAL_NODES],
        lambda: [lambda; THERMAL_NODES],
    };

    // Substep 2 hours for high time accuracy against exact analytical PDE solution
    let substeps = 12;
    let dt = DT_DAY / substeps as f64;
    let r_surf = surface_resistance(0.0, 0.15, 0.0, 0.05, f64::INFINITY);
    assert_eq!(r_surf, 0.0);

    // Run 10 cycles (300 days) to reach periodic steady state
    for day in 0..(10 * period_days) {
        for s in 0..substeps {
            let t_sec = (day as f64 + (s as f64 + 1.0) / substeps as f64) * DT_DAY;
            let t_air = t_mean + amp * (omega * t_sec).sin();
            step_backward_euler(&mut state, &mats, t_air, r_surf, dt);
        }
    }

    // Measure amplitude and phase over the 11th cycle at z = 0.50 m
    let z_target = 0.50_f64;
    let expected_amp = amp * (-z_target / d_damp).exp();
    let expected_phase_lag = z_target / d_damp;

    let mut max_num = f64::NEG_INFINITY;
    let mut min_num = f64::INFINITY;
    let mut sum_sin = 0.0_f64;
    let mut sum_cos = 0.0_f64;
    let mut sum_exact_sin = 0.0_f64;
    let mut sum_exact_cos = 0.0_f64;

    for day in 0..period_days {
        for s in 0..substeps {
            let t_sec =
                ((10 * period_days + day) as f64 + (s as f64 + 1.0) / substeps as f64) * DT_DAY;
            let t_air = t_mean + amp * (omega * t_sec).sin();
            step_backward_euler(&mut state, &mats, t_air, r_surf, dt);
        }
        let t_sec_end = ((10 * period_days + day + 1) as f64) * DT_DAY;
        let t_num = interp_at_depth(&state.z, &state.temp_c, z_target);
        let t_exact = t_mean + expected_amp * (omega * t_sec_end - expected_phase_lag).sin();
        max_num = max_num.max(t_num);
        min_num = min_num.min(t_num);

        let theta = omega * t_sec_end;
        sum_sin += (t_num - t_mean) * theta.sin();
        sum_cos += (t_num - t_mean) * theta.cos();
        sum_exact_sin += (t_exact - t_mean) * theta.sin();
        sum_exact_cos += (t_exact - t_mean) * theta.cos();
    }

    let num_amp = 0.5 * (max_num - min_num);
    let amp_rel_err = (num_amp - expected_amp).abs() / expected_amp;
    let phase_num = sum_cos.atan2(sum_sin);
    let phase_exact = sum_exact_cos.atan2(sum_exact_sin);
    let phase_rel_err = (phase_num - phase_exact).abs() / expected_phase_lag;

    assert!(
        amp_rel_err < 0.01,
        "Carslaw & Jaeger amplitude error {amp_rel_err:.4} >= 1% (num={num_amp}, exact={expected_amp})"
    );
    assert!(
        phase_rel_err < 0.01,
        "Carslaw & Jaeger phase error {phase_rel_err:.4} >= 1% (num={phase_num}, exact={phase_exact})"
    );
}

#[test]
fn deep_water_table_recovers_classic_newhall_parity() {
    let wt_model = WaterTableHydrology::new(WaterTableConfig {
        mode: WaterTableMode::PrescribedHarmonic {
            mean_depth_m: 100.0,
            amplitude_m: 0.0,
            peak_day: 0.0,
        },
        z_prof_m: 1.0,
        k_sat_mm_per_day: 0.0,
        texture: SoilTextureClass::Medium,
        enable_capillary: false,
    });

    let input = SiteInput {
        lat_dd: 40.79,
        lon_dd: -77.86,
        temp_c: [
            -2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3,
        ],
        precip_mm: [
            71.0, 65.0, 84.0, 86.0, 101.0, 105.0, 96.0, 93.0, 91.0, 81.0, 84.0, 77.0,
        ],
        awc_mm: 200.0,
        elev_m: 350.0,
    };
    let config = SiteConfig::default();
    let wb = ThornthwaiteForcing.compute_water_balance(&input, &config);

    let classic = ClassicHydrology.simulate_hydrology(&input, &config, &wb);
    let deep_wt = wt_model.simulate_hydrology(&input, &config, &wb);

    assert_eq!(deep_wt.iday, classic.iday);
    assert_eq!(deep_wt.nd, classic.nd);
    for (a, b) in deep_wt.profile.iter().zip(classic.profile.iter()) {
        assert!((a - b).abs() < 1e-9, "Profile mismatch: {a} vs {b}");
    }
    assert_eq!(deep_wt.sat_days_mcs, 0);
}

#[test]
fn zero_thickness_insulation_matches_bare_soil() {
    let r_zero = surface_resistance(0.0, 0.15, 0.0, 0.05, 15.0);
    let r_bare = 1.0 / 15.0;
    assert!((r_zero - r_bare).abs() < 1e-15);

    let (z, dz) = stretched_grid();
    let mut s1 = SoilThermalState {
        z,
        dz,
        ..SoilThermalState::default()
    };
    let mut s2 = s1;
    let mats = [NodeMaterial::default(); THERMAL_NODES];

    for day in 0..30 {
        let t_air = 5.0 + 10.0 * (day as f64 * 0.2).sin();
        step_backward_euler(&mut s1, &mats, t_air, r_zero, DT_DAY);
        step_backward_euler(&mut s2, &mats, t_air, r_bare, DT_DAY);
    }
    assert_eq!(s1.temp_c, s2.temp_c);
}

// ============================================================================
// 3. Numerical Stability & Stiffness Torture Tests
// ============================================================================

#[test]
fn extreme_conductivity_contrast_50x_with_minus_30c_shock() {
    let (z, dz) = stretched_grid();
    let mut state = SoilThermalState {
        z,
        dz,
        temp_c: [2.0; THERMAL_NODES],
        ..SoilThermalState::default()
    };
    let mats = [NodeMaterial {
        porosity: 0.45,
        theta_tot: 0.45,
        quartz_fraction: 0.8,
        ..NodeMaterial::default()
    }; THERMAL_NODES];

    // 1.5 m snow (lambda=0.05) => R_snow = 30.0 over saturated mineral soil (lambda ~ 2.5)
    let r_surf = surface_resistance(1.5, 0.05, 0.0, 0.05, 15.0);
    for _ in 0..120 {
        step_backward_euler(&mut state, &mats, -30.0, r_surf, DT_DAY);
        for (i, &t) in state.temp_c.iter().enumerate() {
            assert!(t.is_finite(), "NaN/Inf at node {i}");
            assert!(
                (-30.0..=2.0).contains(&t),
                "Out-of-bounds oscillation at node {i}: T={t}"
            );
            assert!(t > -273.15, "Unphysical negative absolute temp: {t}");
        }
    }
}

#[test]
fn freeze_thaw_chatter_matches_10x_finer_dt_reference() {
    let (z, dz) = stretched_grid();
    let mut coarse = SoilThermalState {
        z,
        dz,
        temp_c: [0.0; THERMAL_NODES],
        ..SoilThermalState::default()
    };
    let mut fine = coarse;
    let mats = [NodeMaterial {
        porosity: 0.45,
        theta_tot: 0.35,
        ..NodeMaterial::default()
    }; THERMAL_NODES];
    let r_surf = surface_resistance(0.0, 0.15, 0.0, 0.05, 15.0);

    for day in 0..365 {
        let t_air = if day % 2 == 0 { -2.0 } else { 2.0 };
        step_backward_euler(&mut coarse, &mats, t_air, r_surf, DT_DAY);
        for _ in 0..10 {
            step_backward_euler(&mut fine, &mats, t_air, r_surf, DT_DAY / 10.0);
        }
        let t50_c = interp_at_depth(&coarse.z, &coarse.temp_c, 0.50);
        let t50_f = interp_at_depth(&fine.z, &fine.temp_c, 0.50);
        assert!(t50_c.is_finite() && t50_f.is_finite());
        assert!(
            (t50_c - t50_f).abs() < 0.1,
            "Chatter divergence at day {day}: coarse={t50_c}, fine={t50_f}"
        );
    }
}

#[test]
fn water_table_step_shock_0m_to_3m_and_back() {
    let wt_model = WaterTableHydrology::new(WaterTableConfig {
        mode: WaterTableMode::PrescribedHarmonic {
            mean_depth_m: 1.5,
            amplitude_m: 0.0,
            peak_day: 0.0,
        },
        z_prof_m: 1.0,
        k_sat_mm_per_day: 500.0,
        texture: SoilTextureClass::Fine,
        enable_capillary: true,
    });
    let fsl = 200.0 / 64.0;
    let mut sl = [0.5 * fsl; 64];

    for step in 0..40 {
        let z_wt = if step % 2 == 0 { 0.0 } else { 3.0 };
        let cap = wt_model.apply_water_table(&mut sl, fsl, z_wt, 15.0);
        assert!(cap.is_finite() && cap >= 0.0);
        for (idx, &v) in sl.iter().enumerate() {
            assert!(
                v.is_finite() && v >= 0.0 && v <= fsl + 1e-12,
                "Slot {idx} out of bounds [0, {fsl}]: {v}"
            );
        }
    }
}

#[test]
fn degenerate_inputs_no_panic_or_nan_propagation() {
    let mut input = SiteInput {
        lat_dd: 45.0,
        lon_dd: -90.0,
        temp_c: [5.0; 12],
        precip_mm: [50.0; 12],
        awc_mm: 0.0,
        elev_m: 100.0,
    };
    let config = SiteConfig::default();

    // phi = 0, rho_b = 0, awc = 0, K_sat = 0, z_wt = -0.5 (ponded)
    let engine = SimEngine::new(
        Thermal1DModel::new(Thermal1DConfig {
            porosity: 0.0,
            bulk_density_kg_m3: 0.0,
            ..Thermal1DConfig::default()
        }),
        ThornthwaiteForcing,
        WaterTableHydrology::new(WaterTableConfig {
            mode: WaterTableMode::PrescribedMonthly([-0.5; 12]),
            z_prof_m: 1.0,
            k_sat_mm_per_day: 0.0,
            texture: SoilTextureClass::Coarse,
            enable_capillary: true,
        }),
    );

    let out = engine.run(&input, &config);
    assert!(out.temperature_regime.is_some());
    assert!(out.annual_rainfall.is_finite());

    // Single-month NaN in temperature
    input.temp_c[6] = f64::NAN;
    let out_nan = engine.run(&input, &config);
    assert_eq!(out_nan.temperature_regime, None);
}

// ============================================================================
// 4. Property-Based Fuzzing (`proptest`) on Soil Taxonomy Boundaries
// ============================================================================

fn taxonomy_climate_strategy() -> impl Strategy<Value = (SiteInput, f64)> {
    // Cluster mean annual temperature near Soil Taxonomy thresholds: 0, 8, 15, 22 C
    let base_t_strat = prop_oneof![
        (-3.0_f64..3.0_f64),
        (5.0_f64..11.0_f64),
        (12.0_f64..18.0_f64),
        (19.0_f64..25.0_f64),
    ];
    // Cluster seasonal amplitude near MSST - MWST ~ 6 C iso-regime boundary
    let amp_strat = prop_oneof![(0.0_f64..4.5_f64), (4.5_f64..9.0_f64), (9.0_f64..18.0_f64)];
    let lat_strat = -65.0_f64..65.0_f64;
    let precip_base_strat = 5.0_f64..180.0_f64;
    let awc_strat = 50.0_f64..300.0_f64;
    let z_wt_strat = 0.2_f64..3.0_f64;

    (
        base_t_strat,
        amp_strat,
        lat_strat,
        precip_base_strat,
        awc_strat,
        z_wt_strat,
    )
        .prop_map(|(base_t, amp, lat, p_base, awc, z_wt)| {
            let sign = if lat >= 0.0 { 1.0 } else { -1.0 };
            let mut temp_c = [0.0_f64; 12];
            let mut precip_mm = [0.0_f64; 12];
            for m in 0..12 {
                let phase = TAU * (m as f64 - 0.5) / 12.0;
                temp_c[m] = base_t - sign * amp * phase.cos();
                precip_mm[m] = (p_base + 25.0 * phase.sin()).max(0.0);
            }
            (
                SiteInput {
                    lat_dd: lat,
                    lon_dd: 0.0,
                    temp_c,
                    precip_mm,
                    awc_mm: awc,
                    elev_m: 100.0,
                },
                z_wt,
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn fuzz_soil_taxonomy_invariants((input, z_wt) in taxonomy_climate_strategy()) {
        let config = SiteConfig::default();
        let thermal_cfg = Thermal1DConfig {
            max_spinup_years: 3,
            ..Thermal1DConfig::default()
        };
        let engine = SimEngine::new(
            Thermal1DModel::new(thermal_cfg),
            ThornthwaiteForcing,
            WaterTableHydrology::new(WaterTableConfig {
                mode: WaterTableMode::PrescribedMonthly([z_wt; 12]),
                ..WaterTableConfig::default()
            }),
        );

        let out = engine.run(&input, &config);
        prop_assert!(out.temperature_regime.is_some());
        prop_assert!(out.annual_rainfall.is_finite());
        prop_assert!(out.annual_potential_evapotranspiration.is_finite());
        prop_assert_eq!(
            out.num_cumulative_days_dry + out.num_cumulative_days_moist_dry + out.num_cumulative_days_moist,
            360
        );
    }
}

#[test]
fn monotonicity_snow_warms_cold_mwst_and_shallow_wt_reduces_dry_days() {
    let cold_input = SiteInput {
        lat_dd: 48.0,
        lon_dd: -110.0,
        temp_c: [
            -15.0, -12.0, -4.0, 4.0, 11.0, 16.0, 19.0, 18.0, 12.0, 5.0, -5.0, -13.0,
        ],
        precip_mm: [20.0; 12],
        awc_mm: 200.0,
        elev_m: 800.0,
    };
    let soil = ClassicHydrology.simulate_hydrology(
        &cold_input,
        &SiteConfig::default(),
        &ThornthwaiteForcing.compute_water_balance(&cold_input, &SiteConfig::default()),
    );

    let bare = Thermal1DModel::new(Thermal1DConfig::default());
    let snowy = Thermal1DModel::new(Thermal1DConfig {
        snow_depth_m: [0.8, 0.8, 0.5, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 0.4, 0.7],
        ..Thermal1DConfig::default()
    });
    let (t50_bare, _) = bare.simulate_year(&cold_input, &soil);
    let (t50_snow, _) = snowy.simulate_year(&cold_input, &soil);
    let mwst_bare =
        (t50_bare[330..360].iter().sum::<f64>() + t50_bare[0..60].iter().sum::<f64>()) / 90.0;
    let mwst_snow =
        (t50_snow[330..360].iter().sum::<f64>() + t50_snow[0..60].iter().sum::<f64>()) / 90.0;
    assert!(
        mwst_snow >= mwst_bare,
        "Winter snowpack lowered MWST: bare={mwst_bare}, snow={mwst_snow}"
    );

    // Raising water table (smaller z_wt) never increases cumulative dry days
    let dry_input = SiteInput {
        lat_dd: 35.0,
        lon_dd: -110.0,
        temp_c: [
            2.0, 5.0, 9.0, 14.0, 19.0, 25.0, 28.0, 27.0, 22.0, 15.0, 8.0, 3.0,
        ],
        precip_mm: [15.0; 12],
        awc_mm: 200.0,
        elev_m: 1000.0,
    };
    let wb = ThornthwaiteForcing.compute_water_balance(&dry_input, &SiteConfig::default());
    let deep = WaterTableHydrology::new(WaterTableConfig {
        mode: WaterTableMode::PrescribedMonthly([3.0; 12]),
        ..WaterTableConfig::default()
    })
    .simulate_hydrology(&dry_input, &SiteConfig::default(), &wb);
    let shallow = WaterTableHydrology::new(WaterTableConfig {
        mode: WaterTableMode::PrescribedMonthly([0.3; 12]),
        ..WaterTableConfig::default()
    })
    .simulate_hydrology(&dry_input, &SiteConfig::default(), &wb);

    assert!(
        shallow.nd[0] <= deep.nd[0],
        "Raising water table increased dry days: shallow={}, deep={}",
        shallow.nd[0],
        deep.nd[0]
    );
}

#[test]
fn deterministic_across_single_vs_rayon_parallel_batch() {
    let n = 64;
    let mut lat_dd = Vec::with_capacity(n);
    let mut lon_dd = Vec::with_capacity(n);
    let mut awc_mm = Vec::with_capacity(n);
    let mut elev_m = Vec::with_capacity(n);
    let mut temp_cols: [Vec<f64>; 12] = std::array::from_fn(|_| Vec::with_capacity(n));
    let mut precip_cols: [Vec<f64>; 12] = std::array::from_fn(|_| Vec::with_capacity(n));

    for i in 0..n {
        lat_dd.push(-50.0 + (i as f64) * 1.6);
        lon_dd.push(-100.0 + (i as f64));
        awc_mm.push(100.0 + (i as f64) * 3.0);
        elev_m.push(200.0);
        for m in 0..12 {
            temp_cols[m].push(5.0 + (i as f64) * 0.2 + 10.0 * ((m as f64) * 0.5).sin());
            precip_cols[m].push(30.0 + (m as f64) * 5.0);
        }
    }

    let batch = newhall_core::SoaBatchInput {
        lat_dd: &lat_dd,
        lon_dd: &lon_dd,
        temp_c: std::array::from_fn(|m| temp_cols[m].as_slice()),
        precip_mm: std::array::from_fn(|m| precip_cols[m].as_slice()),
        awc_mm: &awc_mm,
        elev_m: &elev_m,
    };

    let engine = SimEngine::new(
        Thermal1DModel::default(),
        ThornthwaiteForcing,
        WaterTableHydrology::default(),
    );
    let config = SiteConfig::default();
    let parallel_out = engine.run_batch_soa(&batch, &config);

    for (i, par_item) in parallel_out.iter().enumerate() {
        let single = engine.run(&batch.site_input(i), &config);
        assert_eq!(*par_item, single, "Mismatch at site {i}");
    }
}
