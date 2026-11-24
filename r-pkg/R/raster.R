# Chunked terra::SpatRaster S3 methods for newhall_batch and newhall_batch_phase4.

.require_terra <- function() {
  if (!requireNamespace("terra", quietly = TRUE)) {
    stop("Package 'terra' is required for SpatRaster inputs.", call. = FALSE)
  }
}

.newhall_output_names <- function(include_phase4 = FALSE) {
  base_names <- c(
    "annualRainfall",
    "waterHoldingCapacity",
    "annualWaterBalance",
    "annualPotentialEvapotranspiration",
    "summerWaterBalance",
    "dryDaysAfterSummerSolstice",
    "moistDaysAfterWinterSolstice",
    "numCumulativeDaysDry",
    "numCumulativeDaysMoistDry",
    "numCumulativeDaysMoist",
    "numCumulativeDaysDryOver5C",
    "numCumulativeDaysMoistDryOver5C",
    "numCumulativeDaysMoistOver5C",
    "numConsecutiveDaysMoistInSomeParts",
    "numConsecutiveDaysMoistInSomePartsOver8C",
    "temperatureRegime",
    "moistureRegime",
    "regimeSubdivision1",
    "regimeSubdivision2"
  )
  if (isTRUE(include_phase4)) {
    c(base_names, "satDaysMcs", "aquicBioDays")
  } else {
    base_names
  }
}

.validate_raster_layers <- function(.data) {
  missing_layers <- setdiff(.newhall_required_cols(), names(.data))
  if (length(missing_layers) > 0L) {
    stop(
      sprintf(
        "Missing required layers in '.data': %s",
        paste(missing_layers, collapse = ", ")
      ),
      call. = FALSE
    )
  }
  invisible(TRUE)
}

.validate_phase4_snow_layers <- function(layer_names) {
  snow_cols <- paste0("snow", month.abb)
  present <- snow_cols %in% layer_names
  if (any(present) && !all(present)) {
    stop(
      sprintf(
        "Partial Phase 4 snow layers found; missing: %s",
        paste(snow_cols[!present], collapse = ", ")
      ),
      call. = FALSE
    )
  }
  all(present)
}

.compute_chunk_plan <- function(out_info, nrows = NULL) {
  chunk_nrows <- NULL
  if (!is.null(nrows)) {
    if (!is.numeric(nrows) || length(nrows) != 1L || is.na(nrows) ||
        !is.finite(nrows) || nrows < 1) {
      stop("'nrows' must be NULL or a finite numeric scalar >= 1.", call. = FALSE)
    }
    chunk_nrows <- as.integer(floor(nrows))
  }

  plan <- list()
  for (k in seq_len(out_info$n)) {
    blk_start <- as.integer(out_info$row[k])
    blk_nrows <- as.integer(out_info$nrows[k])
    if (blk_nrows <= 0L) {
      next
    }
    if (is.null(chunk_nrows) || chunk_nrows >= blk_nrows) {
      plan[[length(plan) + 1L]] <- list(row = blk_start, nrows = blk_nrows)
    } else {
      blk_end <- blk_start + blk_nrows - 1L
      starts <- seq.int(blk_start, blk_end, by = chunk_nrows)
      for (r_start in starts) {
        r_n <- min(chunk_nrows, blk_end - r_start + 1L)
        plan[[length(plan) + 1L]] <- list(row = r_start, nrows = r_n)
      }
    }
  }
  plan
}

.attach_regime_cats <- function(out) {
  cat_specs <- list(
    list(layer = 16L, name = "temperatureRegime", levels = .str_levels()),
    list(layer = 17L, name = "moistureRegime", levels = .smr_levels()),
    list(layer = 18L, name = "regimeSubdivision1", levels = .smrsub1_levels()),
    list(layer = 19L, name = "regimeSubdivision2", levels = .smrsub2_levels())
  )
  for (spec in cat_specs) {
    cat_df <- .regime_cat_df(spec$levels)
    # terra::set.cats() renames the target raster layer to colnames(value)[2].
    names(cat_df)[2L] <- spec$name
    terra::set.cats(out, layer = spec$layer, value = cat_df)
  }
  out
}

.extract_phase4_vector <- function(df, layer_name, default_val, n) {
  if (layer_name %in% names(df)) {
    as.double(df[[layer_name]])
  } else {
    rep(as.double(default_val)[1L], n)
  }
}

.run_spatraster_chunks <- function(
    .data,
    out_names,
    file = "",
    nrows = NULL,
    overwrite = TRUE,
    skip_all_required = FALSE,
    runner_fn
) {
  .require_terra()
  .validate_raster_layers(.data)

  nlyrs <- length(out_names)
  out <- terra::rast(.data, nlyrs = nlyrs)
  names(out) <- out_names

  read_open <- FALSE
  write_open <- FALSE
  on.exit(
    {
      if (write_open) {
        try(terra::writeStop(out), silent = TRUE)
      }
      if (read_open) {
        try(terra::readStop(.data), silent = TRUE)
      }
    },
    add = TRUE
  )

  read_open <- isTRUE(suppressWarnings(terra::readStart(.data)))
  out_info <- terra::writeStart(out, filename = file, overwrite = overwrite)
  write_open <- TRUE

  plan <- .compute_chunk_plan(out_info, nrows)
  req_cols <- .newhall_required_cols()

  for (blk in plan) {
    chunk <- terra::readValues(
      .data,
      row = blk$row,
      nrows = blk$nrows,
      dataframe = TRUE
    )
    n_cells <- nrow(chunk)
    mat <- matrix(NA_real_, nrow = n_cells, ncol = nlyrs)

    skip <- if (isTRUE(skip_all_required)) {
      Reduce(`|`, lapply(req_cols, function(col) is.na(chunk[[col]])))
    } else {
      is.na(chunk[["awc"]])
    }
    valid_idx <- which(!skip)

    if (length(valid_idx) > 0L) {
      valid_chunk <- if (length(valid_idx) == n_cells) {
        chunk
      } else {
        chunk[valid_idx, , drop = FALSE]
      }
      res <- runner_fn(valid_chunk)
      for (j in seq_len(nlyrs)) {
        mat[valid_idx, j] <- as.double(res[[out_names[j]]])
      }
    }

    terra::writeValues(out, mat, blk$row, nrows = blk$nrows)
  }

  out <- terra::writeStop(out)
  write_open <- FALSE
  terra::readStop(.data)
  read_open <- FALSE

  .attach_regime_cats(out)
}

#' Run Newhall Batch Simulation on a `terra::SpatRaster`
#'
#' @param .data A `terra::SpatRaster` containing the 28 required layers
#'   (`latDD`, `lonDD`, `tJan`..`tDec`, `pJan`..`pDec`, `awc`, `elev`) and
#'   optional per-cell soil parameter layers (`soilAirOffset`, `amplitude`,
#'   `hasOHorizon`, `isSaturated`).
#' @param unitSystem Character unit system (`"metric"`, `"mm"`, `"cm"`, `"in"`,
#'   or `"english"`).
#' @param soilAirOffset Scalar soil-air temperature offset used when the
#'   `soilAirOffset` layer is absent.
#' @param amplitude Scalar soil-to-air amplitude ratio used when the
#'   `amplitude` layer is absent.
#' @param hasOHorizon Logical scalar used when the `hasOHorizon` layer is
#'   absent.
#' @param isSaturated Logical scalar used when the `isSaturated` layer is
#'   absent.
#' @param strict Logical; if `FALSE` (default, classic `jNSMR` parity), only
#'   cells with `NA` in `awc` are skipped. If `TRUE`, cells with `NA` in any
#'   required layer are skipped and `StrictNewhall` physics are used.
#' @param file Output raster filename passed to `terra::writeStart()` (default
#'   `""` for an in-memory `SpatRaster`).
#' @param nrows Optional positive integer maximum number of rows per chunk;
#'   subdivides `terra` write blocks when smaller than the block height.
#' @param overwrite Logical; passed to `terra::writeStart()`.
#' @param ... Additional arguments (currently unused).
#' @return A 19-layer `terra::SpatRaster` with 0-indexed categorical tables
#'   attached to layers 16--19 via `terra::set.cats()`.
#' @method newhall_batch SpatRaster
#' @export
newhall_batch.SpatRaster <- function(
    .data,
    unitSystem = "metric",
    soilAirOffset = ifelse(tolower(unitSystem) %in% c("in", "english"), 4.5, 2.5),
    amplitude = 0.66,
    hasOHorizon = FALSE,
    isSaturated = FALSE,
    strict = FALSE,
    file = "",
    nrows = NULL,
    overwrite = TRUE,
    ...
) {
  is_metric <- .normalize_unit_system(unitSystem)
  force(soilAirOffset)

  out_names <- .newhall_output_names(include_phase4 = FALSE)
  .run_spatraster_chunks(
    .data = .data,
    out_names = out_names,
    file = file,
    nrows = nrows,
    overwrite = overwrite,
    skip_all_required = isTRUE(strict),
    runner_fn = function(valid_chunk) {
      inp <- .extract_df_inputs(
        valid_chunk,
        soilAirOffset = soilAirOffset,
        amplitude = amplitude,
        hasOHorizon = hasOHorizon,
        isSaturated = isSaturated
      )
      newhall_batch_raw(
        lat = inp$lat,
        lon = inp$lon,
        temp = inp$temp,
        precip = inp$precip,
        awc = inp$awc,
        elev = inp$elev,
        soil_air_offset = inp$soilAirOffset,
        amplitude = inp$amplitude,
        has_o_horizon = inp$hasOHorizon,
        is_saturated = inp$isSaturated,
        is_metric = is_metric,
        strict = isTRUE(strict)
      )
    }
  )
}

#' Run Phase 4 Newhall Batch Simulation on a `terra::SpatRaster`
#'
#' @inheritParams newhall_batch.SpatRaster
#' @param o_horizon_thickness Scalar O-horizon thickness (m) used when the
#'   `oHorizonThickness` layer is absent.
#' @param bulk_density Scalar dry bulk density (kg/m^3) used when the
#'   `bulkDensity` layer is absent.
#' @param quartz_fraction Scalar quartz volume fraction `[0, 1]` used when the
#'   `quartzFraction` layer is absent.
#' @param porosity Scalar soil porosity `[0, 1]` used when the `porosity` layer
#'   is absent.
#' @param z_wt Scalar water-table depth (m) or `z_imp` used when the `zWt`
#'   layer is absent.
#' @param snow_depth Scalar snowpack depth (m) used when `snowJan`..`snowDec`
#'   layers are absent.
#' @param use_thermal_1d Logical; enable 1D soil thermal model.
#' @param use_water_table Logical; enable fluctuating water-table hydrology.
#' @param use_prognostic_wt Logical; use prognostic perched water table instead
#'   of prescribed.
#' @param k_sat Saturated hydraulic conductivity (mm/day).
#' @param z_prof Profile depth (m).
#' @param sy Drainable porosity for prognostic mode.
#' @param k_lat Lateral recession constant (1/day) for prognostic mode.
#' @param texture Soil texture class (`"coarse"`, `"medium"`, `"fine"`,
#'   `"organic"`).
#' @param strict_forcing Logical; use defect-remediated
#'   `StrictThornthwaiteForcing` instead of `ThornthwaiteForcing` (default
#'   `TRUE`).
#' @return A 21-layer `terra::SpatRaster` (`annualRainfall`..`regimeSubdivision2`,
#'   `satDaysMcs`, `aquicBioDays`) with 0-indexed categorical tables attached to
#'   layers 16--19 via `terra::set.cats()`.
#' @method newhall_batch_phase4 SpatRaster
#' @export
newhall_batch_phase4.SpatRaster <- function(
    .data,
    unitSystem = "metric",
    soilAirOffset = 2.5,
    amplitude = 0.66,
    hasOHorizon = FALSE,
    isSaturated = FALSE,
    o_horizon_thickness = 0.0,
    bulk_density = 1400.0,
    quartz_fraction = 0.4,
    porosity = 0.45,
    z_wt = 100.0,
    snow_depth = 0.0,
    use_thermal_1d = TRUE,
    use_water_table = TRUE,
    use_prognostic_wt = FALSE,
    k_sat = 100.0,
    z_prof = 1.0,
    sy = 0.2,
    k_lat = 0.02,
    texture = "medium",
    strict_forcing = TRUE,
    file = "",
    nrows = NULL,
    overwrite = TRUE,
    ...
) {
  .require_terra()
  is_metric <- .normalize_unit_system(unitSystem)
  if (!isTRUE(is_metric)) {
    stop(
      "newhall_batch_phase4() only supports metric 'unitSystem' ('metric', 'mm', 'cm').",
      call. = FALSE
    )
  }
  has_snow_layers <- .validate_phase4_snow_layers(names(.data))
  snow_cols <- paste0("snow", month.abb)
  snow_scalar <- as.double(snow_depth)[1L]

  out_names <- .newhall_output_names(include_phase4 = TRUE)
  .run_spatraster_chunks(
    .data = .data,
    out_names = out_names,
    file = file,
    nrows = nrows,
    overwrite = overwrite,
    skip_all_required = TRUE,
    runner_fn = function(valid_chunk) {
      n_valid <- nrow(valid_chunk)
      inp <- .extract_df_inputs(
        valid_chunk,
        soilAirOffset = soilAirOffset,
        amplitude = amplitude,
        hasOHorizon = hasOHorizon,
        isSaturated = isSaturated
      )
      snow_mat <- if (has_snow_layers) {
        .extract_monthly_matrix(valid_chunk, snow_cols)
      } else {
        matrix(snow_scalar, nrow = n_valid, ncol = 12L)
      }
      newhall_batch_phase4_raw(
        lat = inp$lat,
        lon = inp$lon,
        temp = inp$temp,
        precip = inp$precip,
        awc = inp$awc,
        elev = inp$elev,
        snow_depth = snow_mat,
        o_horizon_thickness = .extract_phase4_vector(
          valid_chunk, "oHorizonThickness", o_horizon_thickness, n_valid
        ),
        bulk_density = .extract_phase4_vector(
          valid_chunk, "bulkDensity", bulk_density, n_valid
        ),
        quartz_fraction = .extract_phase4_vector(
          valid_chunk, "quartzFraction", quartz_fraction, n_valid
        ),
        porosity = .extract_phase4_vector(
          valid_chunk, "porosity", porosity, n_valid
        ),
        z_wt = .extract_phase4_vector(
          valid_chunk, "zWt", z_wt, n_valid
        ),
        use_thermal_1d = isTRUE(use_thermal_1d),
        use_water_table = isTRUE(use_water_table),
        use_prognostic_wt = isTRUE(use_prognostic_wt),
        k_sat = as.double(k_sat)[1L],
        z_prof = as.double(z_prof)[1L],
        sy = as.double(sy)[1L],
        k_lat = as.double(k_lat)[1L],
        texture = as.character(texture)[1L],
        soil_air_offset = inp$soilAirOffset,
        amplitude = inp$amplitude,
        has_o_horizon = inp$hasOHorizon,
        is_saturated = inp$isSaturated,
        strict_forcing = isTRUE(strict_forcing)
      )
    }
  )
}
