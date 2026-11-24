use extendr_api::prelude::*;
use newhall_core::{
    ClassicHydrology, ClassicNewhall, ClassicThermal, ForcingModel, HydrologyStep, SimEngine,
    SiteConfig, SiteInput, SiteOutput, SoaBatchInput, SoilTextureClass, StrictNewhall,
    StrictThornthwaiteForcing, Thermal1DConfig, Thermal1DModel, ThermalModel, ThornthwaiteForcing,
    WaterTableConfig, WaterTableHydrology, WaterTableMode,
};
use rayon::prelude::*;

fn outputs_to_r_list(outputs: &[SiteOutput], include_phase4: bool) -> List {
    let n = outputs.len();
    let mut annual_rainfall = Vec::with_capacity(n);
    let mut water_holding_capacity = Vec::with_capacity(n);
    let mut annual_water_balance = Vec::with_capacity(n);
    let mut annual_pet = Vec::with_capacity(n);
    let mut summer_water_balance = Vec::with_capacity(n);
    let mut dry_after_ss = Vec::with_capacity(n);
    let mut moist_after_ws = Vec::with_capacity(n);
    let mut cum_dry = Vec::with_capacity(n);
    let mut cum_moist_dry = Vec::with_capacity(n);
    let mut cum_moist = Vec::with_capacity(n);
    let mut cum_dry_5c = Vec::with_capacity(n);
    let mut cum_moist_dry_5c = Vec::with_capacity(n);
    let mut cum_moist_5c = Vec::with_capacity(n);
    let mut cons_moist = Vec::with_capacity(n);
    let mut cons_moist_8c = Vec::with_capacity(n);
    let mut str_code = Vec::with_capacity(n);
    let mut smr_code = Vec::with_capacity(n);
    let mut sub1_code = Vec::with_capacity(n);
    let mut sub2_code = Vec::with_capacity(n);
    let mut sat_days_mcs = Vec::with_capacity(if include_phase4 { n } else { 0 });
    let mut aquic_bio_days = Vec::with_capacity(if include_phase4 { n } else { 0 });

    for o in outputs {
        annual_rainfall.push(o.annual_rainfall);
        water_holding_capacity.push(o.water_holding_capacity);
        annual_water_balance.push(o.annual_water_balance);
        annual_pet.push(o.annual_potential_evapotranspiration);
        summer_water_balance.push(o.summer_water_balance);
        dry_after_ss.push(o.dry_days_after_summer_solstice);
        moist_after_ws.push(o.moist_days_after_winter_solstice);
        cum_dry.push(o.num_cumulative_days_dry);
        cum_moist_dry.push(o.num_cumulative_days_moist_dry);
        cum_moist.push(o.num_cumulative_days_moist);
        cum_dry_5c.push(o.num_cumulative_days_dry_over_5c);
        cum_moist_dry_5c.push(o.num_cumulative_days_moist_dry_over_5c);
        cum_moist_5c.push(o.num_cumulative_days_moist_over_5c);
        cons_moist.push(o.num_consecutive_days_moist_in_some_parts);
        cons_moist_8c.push(o.num_consecutive_days_moist_in_some_parts_over_8c);
        str_code.push(o.temperature_regime.map(|r| r as i32));
        smr_code.push(o.moisture_regime as i32);
        sub1_code.push(o.regime_subdivision1 as i32);
        sub2_code.push(o.regime_subdivision2 as i32);
        if include_phase4 {
            sat_days_mcs.push(o.sat_days_mcs);
            aquic_bio_days.push(o.aquic_bio_days);
        }
    }

    if include_phase4 {
        list!(
            annualRainfall = annual_rainfall,
            waterHoldingCapacity = water_holding_capacity,
            annualWaterBalance = annual_water_balance,
            annualPotentialEvapotranspiration = annual_pet,
            summerWaterBalance = summer_water_balance,
            dryDaysAfterSummerSolstice = dry_after_ss,
            moistDaysAfterWinterSolstice = moist_after_ws,
            numCumulativeDaysDry = cum_dry,
            numCumulativeDaysMoistDry = cum_moist_dry,
            numCumulativeDaysMoist = cum_moist,
            numCumulativeDaysDryOver5C = cum_dry_5c,
            numCumulativeDaysMoistDryOver5C = cum_moist_dry_5c,
            numCumulativeDaysMoistOver5C = cum_moist_5c,
            numConsecutiveDaysMoistInSomeParts = cons_moist,
            numConsecutiveDaysMoistInSomePartsOver8C = cons_moist_8c,
            temperatureRegime = str_code,
            moistureRegime = smr_code,
            regimeSubdivision1 = sub1_code,
            regimeSubdivision2 = sub2_code,
            satDaysMcs = sat_days_mcs,
            aquicBioDays = aquic_bio_days
        )
    } else {
        list!(
            annualRainfall = annual_rainfall,
            waterHoldingCapacity = water_holding_capacity,
            annualWaterBalance = annual_water_balance,
            annualPotentialEvapotranspiration = annual_pet,
            summerWaterBalance = summer_water_balance,
            dryDaysAfterSummerSolstice = dry_after_ss,
            moistDaysAfterWinterSolstice = moist_after_ws,
            numCumulativeDaysDry = cum_dry,
            numCumulativeDaysMoistDry = cum_moist_dry,
            numCumulativeDaysMoist = cum_moist,
            numCumulativeDaysDryOver5C = cum_dry_5c,
            numCumulativeDaysMoistDryOver5C = cum_moist_dry_5c,
            numCumulativeDaysMoistOver5C = cum_moist_5c,
            numConsecutiveDaysMoistInSomeParts = cons_moist,
            numConsecutiveDaysMoistInSomePartsOver8C = cons_moist_8c,
            temperatureRegime = str_code,
            moistureRegime = smr_code,
            regimeSubdivision1 = sub1_code,
            regimeSubdivision2 = sub2_code
        )
    }
}

fn parse_texture(s: &str) -> SoilTextureClass {
    match s.to_ascii_lowercase().as_str() {
        "coarse" => SoilTextureClass::Coarse,
        "fine" => SoilTextureClass::Fine,
        "organic" => SoilTextureClass::Organic,
        _ => SoilTextureClass::Medium,
    }
}

fn validate_soil_config_lengths(
    n: usize,
    soil_air_offset: &[f64],
    amplitude: &[f64],
    has_o_horizon: &[i32],
    is_saturated: &[i32],
) -> Result<()> {
    if n == 0 {
        return Ok(());
    }
    for (name, len) in [
        ("soil_air_offset", soil_air_offset.len()),
        ("amplitude", amplitude.len()),
        ("has_o_horizon", has_o_horizon.len()),
        ("is_saturated", is_saturated.len()),
    ] {
        if len != 1 && len != n {
            return Err(Error::Other(format!(
                "{name} length ({len}) must be 1 or n ({n})"
            )));
        }
    }
    Ok(())
}

#[inline]
fn site_config_at(
    soil_air_offset: &[f64],
    amplitude: &[f64],
    has_o_horizon: &[i32],
    is_saturated: &[i32],
    is_metric: bool,
    i: usize,
) -> SiteConfig {
    SiteConfig {
        soil_air_offset: soil_air_offset[if soil_air_offset.len() == 1 { 0 } else { i }],
        amplitude: amplitude[if amplitude.len() == 1 { 0 } else { i }],
        has_o_horizon: has_o_horizon[if has_o_horizon.len() == 1 { 0 } else { i }] != 0,
        is_saturated: is_saturated[if is_saturated.len() == 1 { 0 } else { i }] != 0,
        is_metric,
    }
}

fn run_per_site_soa<T, F, H>(
    engine: &SimEngine<T, F, H>,
    batch: &SoaBatchInput<'_>,
    soil_air_offset: &[f64],
    amplitude: &[f64],
    has_o_horizon: &[i32],
    is_saturated: &[i32],
    is_metric: bool,
) -> Vec<SiteOutput>
where
    T: ThermalModel + Sync,
    F: ForcingModel + Sync,
    H: HydrologyStep + Sync,
{
    (0..batch.len())
        .into_par_iter()
        .map(|i| {
            let input = batch.site_input(i);
            let cfg = site_config_at(
                soil_air_offset,
                amplitude,
                has_o_horizon,
                is_saturated,
                is_metric,
                i,
            );
            engine.run(&input, &cfg)
        })
        .collect()
}

/// Classic or strict Newhall vectorized batch runner using SoA `&[f64]` slices and `rayon`.
#[extendr]
#[allow(clippy::too_many_arguments)]
fn newhall_batch_rust(
    lat: &[f64],
    lon: &[f64],
    temp_flat: &[f64],
    precip_flat: &[f64],
    awc: &[f64],
    elev: &[f64],
    soil_air_offset: &[f64],
    amplitude: &[f64],
    has_o_horizon: &[i32],
    is_saturated: &[i32],
    is_metric: bool,
    strict: bool,
) -> Result<List> {
    let n = lat.len();
    if lon.len() != n
        || awc.len() != n
        || elev.len() != n
        || temp_flat.len() != n * 12
        || precip_flat.len() != n * 12
    {
        return Err(Error::Other(
            "Input slice lengths do not match n (or 12*n for temp/precip)".into(),
        ));
    }
    if n == 0 {
        return Ok(outputs_to_r_list(&[], false));
    }
    validate_soil_config_lengths(n, soil_air_offset, amplitude, has_o_horizon, is_saturated)?;

    let batch = SoaBatchInput {
        lat_dd: lat,
        lon_dd: lon,
        temp_c: std::array::from_fn(|m| &temp_flat[m * n..(m + 1) * n]),
        precip_mm: std::array::from_fn(|m| &precip_flat[m * n..(m + 1) * n]),
        awc_mm: awc,
        elev_m: elev,
    };
    let all_scalar = soil_air_offset.len() == 1
        && amplitude.len() == 1
        && has_o_horizon.len() == 1
        && is_saturated.len() == 1;

    let outputs = if all_scalar {
        let cfg = site_config_at(
            soil_air_offset,
            amplitude,
            has_o_horizon,
            is_saturated,
            is_metric,
            0,
        );
        if strict {
            StrictNewhall::default().run_batch_soa(&batch, &cfg)
        } else {
            ClassicNewhall::default().run_batch_soa(&batch, &cfg)
        }
    } else if strict {
        run_per_site_soa(
            &StrictNewhall::default(),
            &batch,
            soil_air_offset,
            amplitude,
            has_o_horizon,
            is_saturated,
            is_metric,
        )
    } else {
        run_per_site_soa(
            &ClassicNewhall::default(),
            &batch,
            soil_air_offset,
            amplitude,
            has_o_horizon,
            is_saturated,
            is_metric,
        )
    };
    Ok(outputs_to_r_list(&outputs, false))
}

fn run_phase4_with_forcing<F: ForcingModel>(
    forcing: F,
    use_thermal_1d: bool,
    use_water_table: bool,
    t_cfg: Thermal1DConfig,
    wt_cfg: WaterTableConfig,
    input: &SiteInput,
    cfg: &SiteConfig,
) -> SiteOutput {
    match (use_thermal_1d, use_water_table) {
        (true, true) => SimEngine::new(
            Thermal1DModel::new(t_cfg),
            forcing,
            WaterTableHydrology::new(wt_cfg),
        )
        .run(input, cfg),
        (true, false) => {
            SimEngine::new(Thermal1DModel::new(t_cfg), forcing, ClassicHydrology).run(input, cfg)
        }
        (false, true) => SimEngine::new(ClassicThermal, forcing, WaterTableHydrology::new(wt_cfg))
            .run(input, cfg),
        (false, false) => SimEngine::new(ClassicThermal, forcing, ClassicHydrology).run(input, cfg),
    }
}

/// Phase 4 vectorized batch runner supporting `Thermal1DModel` and `WaterTableHydrology`.
#[extendr]
#[allow(clippy::too_many_arguments)]
fn newhall_batch_phase4_rust(
    lat: &[f64],
    lon: &[f64],
    temp_flat: &[f64],
    precip_flat: &[f64],
    awc: &[f64],
    elev: &[f64],
    snow_flat: &[f64],
    o_thickness: &[f64],
    bulk_density: &[f64],
    quartz_frac: &[f64],
    porosity: &[f64],
    z_wt: &[f64],
    use_thermal_1d: bool,
    use_water_table: bool,
    use_prognostic_wt: bool,
    k_sat: f64,
    z_prof: f64,
    sy: f64,
    k_lat: f64,
    texture: &str,
    soil_air_offset: &[f64],
    amplitude: &[f64],
    has_o_horizon: &[i32],
    is_saturated: &[i32],
    strict_forcing: bool,
) -> Result<List> {
    let n = lat.len();
    if lon.len() != n
        || awc.len() != n
        || elev.len() != n
        || temp_flat.len() != n * 12
        || precip_flat.len() != n * 12
        || snow_flat.len() != n * 12
        || o_thickness.len() != n
        || bulk_density.len() != n
        || quartz_frac.len() != n
        || porosity.len() != n
        || z_wt.len() != n
    {
        return Err(Error::Other(
            "Phase 4 input slice lengths do not match n (or 12*n for monthly slices)".into(),
        ));
    }
    if n == 0 {
        return Ok(outputs_to_r_list(&[], true));
    }
    validate_soil_config_lengths(n, soil_air_offset, amplitude, has_o_horizon, is_saturated)?;

    let tex = parse_texture(texture);

    let outputs: Vec<SiteOutput> = (0..n)
        .into_par_iter()
        .map(|i| {
            let cfg = site_config_at(
                soil_air_offset,
                amplitude,
                has_o_horizon,
                is_saturated,
                true,
                i,
            );
            let input = SiteInput {
                lat_dd: lat[i],
                lon_dd: lon[i],
                temp_c: std::array::from_fn(|m| temp_flat[m * n + i]),
                precip_mm: std::array::from_fn(|m| precip_flat[m * n + i]),
                awc_mm: awc[i],
                elev_m: elev[i],
            };
            let t_cfg = Thermal1DConfig {
                porosity: porosity[i],
                bulk_density_kg_m3: bulk_density[i],
                quartz_fraction: quartz_frac[i],
                o_horizon_thickness_m: o_thickness[i],
                snow_depth_m: std::array::from_fn(|m| snow_flat[m * n + i]),
                texture: tex,
                ..Thermal1DConfig::default()
            };
            let wt_mode = if use_prognostic_wt {
                WaterTableMode::Prognostic {
                    z_imp_m: z_wt[i],
                    sy,
                    k_lat_per_day: k_lat,
                }
            } else {
                WaterTableMode::PrescribedMonthly([z_wt[i]; 12])
            };
            let wt_cfg = WaterTableConfig {
                mode: wt_mode,
                z_prof_m: z_prof,
                k_sat_mm_per_day: k_sat,
                texture: tex,
                enable_capillary: true,
            };

            if strict_forcing {
                run_phase4_with_forcing(
                    StrictThornthwaiteForcing,
                    use_thermal_1d,
                    use_water_table,
                    t_cfg,
                    wt_cfg,
                    &input,
                    &cfg,
                )
            } else {
                run_phase4_with_forcing(
                    ThornthwaiteForcing,
                    use_thermal_1d,
                    use_water_table,
                    t_cfg,
                    wt_cfg,
                    &input,
                    &cfg,
                )
            }
        })
        .collect();

    Ok(outputs_to_r_list(&outputs, true))
}

extendr_module! {
    mod newhallrs;
    fn newhall_batch_rust;
    fn newhall_batch_phase4_rust;
}
