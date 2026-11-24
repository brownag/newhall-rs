#' Run Classic Newhall Simulation in Batch
#'
#' Generic batch entry point supporting vector/matrix inputs (`default`),
#' `data.frame` inputs with `jNSMR`-compatible column schemas, and CSV file
#' path(s) (`character`).
#'
#' @param .data Input object (`data.frame`, `character` path(s) to CSV files,
#'   or omitted/`NULL`/latitude vector for the default vector/matrix interface).
#' @param ... Arguments passed to S3 methods.
#' @param lat Numeric vector of latitudes (decimal degrees).
#' @param lon Numeric vector of longitudes (decimal degrees).
#' @param temp Numeric matrix (`n x 12`) of monthly mean air temperatures (C).
#' @param precip Numeric matrix (`n x 12`) of monthly precipitation (mm).
#' @param awc Numeric vector of available water capacity (mm).
#' @param elev Numeric vector of elevations (m).
#' @param soil_air_offset Numeric scalar or length-`n` vector of soil-air
#'   temperature offsets (C; default `2.5`).
#' @param amplitude Numeric scalar or length-`n` vector of soil-to-air
#'   amplitude ratios (default `0.66`).
#' @param has_o_horizon Logical scalar or length-`n` vector; presence of O-horizon.
#' @param is_saturated Logical scalar or length-`n` vector; saturation at 50 cm.
#' @param is_metric Logical; stored for `jNSMR` parity (converted when `strict = TRUE`).
#' @param strict Logical; use defect-remediated `StrictNewhall` physics instead of
#'   `ClassicNewhall` (default `FALSE`).
#' @param unitSystem Character unit system (`"metric"`, `"mm"`, `"cm"`, `"in"`,
#'   or `"english"`).
#' @param soilAirOffset Default soil-air temperature offset used when `.data`
#'   lacks a `soilAirOffset` column (`4.5` F for English units, `2.5` C for metric).
#' @param hasOHorizon Default logical O-horizon flag used when `.data` lacks a
#'   `hasOHorizon` column.
#' @param isSaturated Default logical saturation flag used when `.data` lacks an
#'   `isSaturated` column.
#' @useDynLib newhallrs, .registration = TRUE
#' @return A `data.frame` of Newhall simulation outputs (`newhall_batch_raw()` and
#'   `newhall_batch.default()` return 0-indexed integer regime codes; `data.frame`
#'   and `character` methods return decoded character regime names).
#' @export
newhall_batch <- function(.data, ...) {
  UseMethod("newhall_batch")
}

#' @rdname newhall_batch
#' @export
newhall_batch_raw <- function(
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
    as.integer(as.logical(has_o_horizon)),
    as.integer(as.logical(is_saturated)),
    isTRUE(is_metric),
    isTRUE(strict)
  )
  as.data.frame(res, stringsAsFactors = FALSE)
}

#' @rdname newhall_batch
#' @method newhall_batch default
#' @export
newhall_batch.default <- function(.data = NULL, ...) {
  if (is.null(.data)) {
    newhall_batch_raw(...)
  } else {
    newhall_batch_raw(.data, ...)
  }
}

#' @rdname newhall_batch
#' @method newhall_batch data.frame
#' @export
newhall_batch.data.frame <- function(
    .data,
    unitSystem = "metric",
    soilAirOffset = ifelse(tolower(unitSystem) %in% c("in", "english"), 4.5, 2.5),
    amplitude = 0.66,
    hasOHorizon = FALSE,
    isSaturated = FALSE,
    strict = FALSE,
    ...
) {
  is_metric <- .normalize_unit_system(unitSystem)
  inputs <- .extract_df_inputs(
    .data,
    soilAirOffset = soilAirOffset,
    amplitude = amplitude,
    hasOHorizon = hasOHorizon,
    isSaturated = isSaturated
  )
  res <- newhall_batch_raw(
    lat = inputs$lat,
    lon = inputs$lon,
    temp = inputs$temp,
    precip = inputs$precip,
    awc = inputs$awc,
    elev = inputs$elev,
    soil_air_offset = inputs$soilAirOffset,
    amplitude = inputs$amplitude,
    has_o_horizon = inputs$hasOHorizon,
    is_saturated = inputs$isSaturated,
    is_metric = is_metric,
    strict = strict
  )
  .decode_regimes(res)
}

#' @rdname newhall_batch
#' @method newhall_batch character
#' @export
newhall_batch.character <- function(.data, ...) {
  if (length(.data) == 0L) {
    stop("'.data' must contain at least one CSV file path.", call. = FALSE)
  }
  if (anyNA(.data) || any(!nzchar(.data))) {
    stop("File paths in '.data' cannot be NA or empty.", call. = FALSE)
  }
  missing_files <- .data[!file.exists(.data)]
  if (length(missing_files) > 0L) {
    stop(
      sprintf("CSV file(s) not found: %s", paste(missing_files, collapse = ", ")),
      call. = FALSE
    )
  }
  dfs <- lapply(.data, utils::read.csv, stringsAsFactors = FALSE)
  df <- if (length(dfs) == 1L) dfs[[1L]] else do.call(rbind, dfs)
  newhall_batch.data.frame(df, ...)
}

.extract_phase4_vec <- function(df, col_name, default_val, n) {
  val <- .extract_optional_col(df, col_name, default_val, as.double)
  if (length(val) == 1L && n != 1L) {
    rep(val, n)
  } else {
    val
  }
}

.extract_phase4_snow <- function(df, n) {
  snow_cols <- paste0("snow", month.abb)
  present <- snow_cols %in% names(df)
  if (all(present)) {
    return(.extract_monthly_matrix(df, snow_cols))
  }
  if (any(present)) {
    stop(
      sprintf(
        "Missing snow depth columns in '.data': %s",
        paste(snow_cols[!present], collapse = ", ")
      ),
      call. = FALSE
    )
  }
  matrix(0.0, nrow = n, ncol = 12L)
}

#' Run Phase 4 Newhall Simulation with 1D Thermal and Water-Table Physics
#'
#' Generic Phase 4 batch entry point supporting vector/matrix inputs (`default`)
#' and `data.frame` inputs (with optional `oHorizonThickness`, `bulkDensity`,
#' `quartzFraction`, `porosity`, `zWt`, and `snowJan`..`snowDec` columns).
#'
#' @param .data Input `data.frame`, or omitted/`NULL`/latitude vector for the
#'   default vector/matrix interface.
#' @param ... Arguments passed to S3 methods.
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
#' @param soil_air_offset Numeric scalar or length-`n` vector of fallback classic
#'   soil-air offsets (C).
#' @param amplitude Numeric scalar or length-`n` vector of fallback classic
#'   amplitude ratios.
#' @param has_o_horizon Logical scalar or length-`n` vector; presence of O-horizon.
#' @param is_saturated Logical scalar or length-`n` vector; saturation at 50 cm.
#' @param strict_forcing Logical; use defect-remediated `StrictThornthwaiteForcing`
#'   instead of `ThornthwaiteForcing` (default `TRUE`).
#' @param soilAirOffset Default soil-air temperature offset (C) used when `.data`
#'   lacks a `soilAirOffset` column.
#' @param hasOHorizon Default logical O-horizon flag used when `.data` lacks a
#'   `hasOHorizon` column.
#' @param isSaturated Default logical saturation flag used when `.data` lacks an
#'   `isSaturated` column.
#' @param oHorizonThickness Default O-horizon thickness (m) used when `.data`
#'   lacks an `oHorizonThickness` column.
#' @param bulkDensity Default dry bulk density (kg/m^3) used when `.data` lacks a
#'   `bulkDensity` column.
#' @param quartzFraction Default quartz volume fraction used when `.data` lacks a
#'   `quartzFraction` column.
#' @param porosity Default soil porosity used when `.data` lacks a `porosity` column.
#' @param zWt Default water-table depth (m) used when `.data` lacks a `zWt` column.
#' @return A `data.frame` of Newhall simulation outputs including `satDaysMcs` and
#'   `aquicBioDays` (`newhall_batch_phase4_raw()` and `newhall_batch_phase4.default()`
#'   return integer regime codes; `newhall_batch_phase4.data.frame()` returns
#'   decoded character regime names).
#' @export
newhall_batch_phase4 <- function(.data, ...) {
  UseMethod("newhall_batch_phase4")
}

#' @rdname newhall_batch_phase4
#' @export
newhall_batch_phase4_raw <- function(
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
    as.integer(as.logical(has_o_horizon)),
    as.integer(as.logical(is_saturated)),
    isTRUE(strict_forcing)
  )
  as.data.frame(res, stringsAsFactors = FALSE)
}

#' @rdname newhall_batch_phase4
#' @method newhall_batch_phase4 default
#' @export
newhall_batch_phase4.default <- function(.data = NULL, ...) {
  if (is.null(.data)) {
    newhall_batch_phase4_raw(...)
  } else {
    newhall_batch_phase4_raw(.data, ...)
  }
}

#' @rdname newhall_batch_phase4
#' @method newhall_batch_phase4 data.frame
#' @export
newhall_batch_phase4.data.frame <- function(
    .data,
    soilAirOffset = 2.5,
    amplitude = 0.66,
    hasOHorizon = FALSE,
    isSaturated = FALSE,
    oHorizonThickness = 0.0,
    bulkDensity = 1400.0,
    quartzFraction = 0.4,
    porosity = 0.45,
    zWt = 100.0,
    use_thermal_1d = TRUE,
    use_water_table = TRUE,
    use_prognostic_wt = FALSE,
    k_sat = 100.0,
    z_prof = 1.0,
    sy = 0.2,
    k_lat = 0.02,
    texture = "medium",
    strict_forcing = TRUE,
    ...
) {
  inputs <- .extract_df_inputs(
    .data,
    soilAirOffset = soilAirOffset,
    amplitude = amplitude,
    hasOHorizon = hasOHorizon,
    isSaturated = isSaturated
  )
  n <- nrow(.data)
  res <- newhall_batch_phase4_raw(
    lat = inputs$lat,
    lon = inputs$lon,
    temp = inputs$temp,
    precip = inputs$precip,
    awc = inputs$awc,
    elev = inputs$elev,
    snow_depth = .extract_phase4_snow(.data, n),
    o_horizon_thickness = .extract_phase4_vec(.data, "oHorizonThickness", oHorizonThickness, n),
    bulk_density = .extract_phase4_vec(.data, "bulkDensity", bulkDensity, n),
    quartz_fraction = .extract_phase4_vec(.data, "quartzFraction", quartzFraction, n),
    porosity = .extract_phase4_vec(.data, "porosity", porosity, n),
    z_wt = .extract_phase4_vec(.data, "zWt", zWt, n),
    use_thermal_1d = use_thermal_1d,
    use_water_table = use_water_table,
    use_prognostic_wt = use_prognostic_wt,
    k_sat = k_sat,
    z_prof = z_prof,
    sy = sy,
    k_lat = k_lat,
    texture = texture,
    soil_air_offset = inputs$soilAirOffset,
    amplitude = inputs$amplitude,
    has_o_horizon = inputs$hasOHorizon,
    is_saturated = inputs$isSaturated,
    strict_forcing = strict_forcing
  )
  .decode_regimes(res)
}
