#' Run Classic Newhall Simulation in Batch
#'
#' @param lat Numeric vector of latitudes (decimal degrees).
#' @param lon Numeric vector of longitudes (decimal degrees).
#' @param temp Numeric matrix (`n x 12`) of monthly mean air temperatures (C).
#' @param precip Numeric matrix (`n x 12`) of monthly precipitation (mm).
#' @param awc Numeric vector of available water capacity (mm).
#' @param elev Numeric vector of elevations (m).
#' @param soil_air_offset Soil-air temperature offset (C; default `2.5`).
#' @param amplitude Soil-to-air amplitude ratio (default `0.66`).
#' @param has_o_horizon Logical; presence of O-horizon.
#' @param is_saturated Logical; saturation at 50 cm.
#' @param is_metric Logical; stored for `jNSMR` parity (converted when `strict = TRUE`).
#' @param strict Logical; use defect-remediated `StrictNewhall` physics instead of `ClassicNewhall` (default `FALSE`).
#' @useDynLib newhallrs, .registration = TRUE
#' @return A `data.frame` of Newhall simulation outputs.
#' @export
newhall_batch <- function(
    lat,
    lon,
    temp,
    precip,
    awc,
    elev = rep(0, length(lat)),
    soil_air_offset = 2.5,
    amplitude = 0.66,
    has_o_horizon = FALSE,
    is_saturated = FALSE,
    is_metric = TRUE,
    strict = FALSE
) {
  temp_mat <- as.matrix(temp)
  precip_mat <- as.matrix(precip)
  res <- newhall_batch_rust(
    as.double(lat),
    as.double(lon),
    as.double(temp_mat),
    as.double(precip_mat),
    as.double(awc),
    as.double(elev),
    as.double(soil_air_offset),
    as.double(amplitude),
    isTRUE(has_o_horizon),
    isTRUE(is_saturated),
    isTRUE(is_metric),
    isTRUE(strict)
  )
  as.data.frame(res, stringsAsFactors = FALSE)
}

#' Run Phase 4 Newhall Simulation with 1D Thermal and Water-Table Physics
#'
#' @param lat Numeric vector of latitudes (decimal degrees).
#' @param lon Numeric vector of longitudes (decimal degrees).
#' @param temp Numeric matrix (`n x 12`) of monthly mean air temperatures (C).
#' @param precip Numeric matrix (`n x 12`) of monthly precipitation (mm).
#' @param awc Numeric vector of available water capacity (mm).
#' @param elev Numeric vector of elevations (m).
#' @param snow_depth Numeric matrix (`n x 12`) of monthly snowpack depth (m).
#' @param o_horizon_thickness Numeric vector of O-horizon thickness (m).
#' @param bulk_density Numeric vector of dry bulk density (kg/m^3).
#' @param quartz_fraction Numeric vector of quartz volume fraction `[0, 1]`.
#' @param porosity Numeric vector of soil porosity `[0, 1]`.
#' @param z_wt Numeric vector of water-table depth (m) or `z_imp` in prognostic mode.
#' @param use_thermal_1d Logical; enable 1D soil thermal model.
#' @param use_water_table Logical; enable fluctuating water-table hydrology.
#' @param use_prognostic_wt Logical; use prognostic perched water table instead of prescribed.
#' @param k_sat Saturated hydraulic conductivity (mm/day).
#' @param z_prof Profile depth (m).
#' @param sy Drainable porosity for prognostic mode.
#' @param k_lat Lateral recession constant (1/day) for prognostic mode.
#' @param texture Soil texture class (`"coarse"`, `"medium"`, `"fine"`, `"organic"`).
#' @param soil_air_offset Fallback classic soil-air offset (C).
#' @param amplitude Fallback classic amplitude ratio.
#' @param has_o_horizon Logical; presence of O-horizon.
#' @param is_saturated Logical; saturation at 50 cm.
#' @param strict_forcing Logical; use defect-remediated `StrictThornthwaiteForcing` instead of `ThornthwaiteForcing` (default `TRUE`).
#' @return A `data.frame` of Newhall simulation outputs including `satDaysMcs` and `aquicBioDays`.
#' @export
newhall_batch_phase4 <- function(
    lat,
    lon,
    temp,
    precip,
    awc,
    elev = rep(0, length(lat)),
    snow_depth = matrix(0, nrow = length(lat), ncol = 12),
    o_horizon_thickness = rep(0, length(lat)),
    bulk_density = rep(1400, length(lat)),
    quartz_fraction = rep(0.4, length(lat)),
    porosity = rep(0.45, length(lat)),
    z_wt = rep(100.0, length(lat)),
    use_thermal_1d = TRUE,
    use_water_table = TRUE,
    use_prognostic_wt = FALSE,
    k_sat = 100.0,
    z_prof = 1.0,
    sy = 0.2,
    k_lat = 0.02,
    texture = "medium",
    soil_air_offset = 2.5,
    amplitude = 0.66,
    has_o_horizon = FALSE,
    is_saturated = FALSE,
    strict_forcing = TRUE
) {
  temp_mat <- as.matrix(temp)
  precip_mat <- as.matrix(precip)
  snow_mat <- as.matrix(snow_depth)
  res <- newhall_batch_phase4_rust(
    as.double(lat),
    as.double(lon),
    as.double(temp_mat),
    as.double(precip_mat),
    as.double(awc),
    as.double(elev),
    as.double(snow_mat),
    as.double(o_horizon_thickness),
    as.double(bulk_density),
    as.double(quartz_fraction),
    as.double(porosity),
    as.double(z_wt),
    isTRUE(use_thermal_1d),
    isTRUE(use_water_table),
    isTRUE(use_prognostic_wt),
    as.double(k_sat),
    as.double(z_prof),
    as.double(sy),
    as.double(k_lat),
    as.character(texture)[1],
    as.double(soil_air_offset),
    as.double(amplitude),
    isTRUE(has_o_horizon),
    isTRUE(is_saturated),
    isTRUE(strict_forcing)
  )
  as.data.frame(res, stringsAsFactors = FALSE)
}
