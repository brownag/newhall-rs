make_s3_test_df <- function() {
  # 4 distinct sites:
  # 1: PA-like Mesic Udic
  # 2: Southern Hemisphere Sydney-like Thermic Udic
  # 3: Cool Cryic/Frigid boundary (tmean = 4.0 C, JJA mean = 7.5 C)
  # 4: Arid desert (tmean = 18.0 C, low precip)
  temps <- rbind(
    c(-2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3),
    c(22.1, 22.0, 20.9, 18.2, 15.2, 12.8, 12.0, 13.1, 15.3, 17.6, 19.4, 21.1),
    c(-3.5, -3.5, 2.0, 2.0, 2.0, 7.5, 7.5, 7.5, 2.0, 2.0, 2.0, -3.5),
    c(9.0, 11.0, 15.0, 19.0, 24.0, 29.0, 31.0, 30.0, 26.0, 20.0, 14.0, 10.0)
  )
  precips <- rbind(
    rep(85.0, 12),
    c(102, 118, 130, 127, 120, 132, 97, 81, 69, 77, 84, 78),
    rep(75.0, 12),
    rep(8.0, 12)
  )
  colnames(temps) <- paste0("t", month.abb)
  colnames(precips) <- paste0("p", month.abb)

  df <- data.frame(
    latDD = c(40.8, -33.8688, 48.0, 33.4),
    lonDD = c(-77.8, 151.2093, -110.0, -112.0),
    awc = c(200.0, 200.0, 200.0, 150.0),
    elev = c(350.0, 19.0, 1200.0, 330.0),
    stringsAsFactors = FALSE
  )
  cbind(df, as.data.frame(temps), as.data.frame(precips))
}

make_s3_test_raster <- function(df, nrow = 5L, ncol = 2L) {
  n_cells <- nrow * ncol
  idx <- rep(seq_len(nrow(df)), length.out = n_cells)
  df_grid <- df[idx, , drop = FALSE]
  layers <- lapply(names(df_grid), function(nm) {
    terra::rast(
      nrows = nrow,
      ncols = ncol,
      xmin = -100,
      xmax = -98,
      ymin = 38,
      ymax = 40,
      crs = "EPSG:4326",
      vals = df_grid[[nm]]
    )
  })
  r <- terra::rast(layers)
  names(r) <- names(df_grid)
  r
}

expected_output_names <- function(include_hydrothermal = FALSE) {
  base <- c(
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
  if (include_hydrothermal) c(base, "satDaysMcs", "aquicBioDays") else base
}

test_that("newhall_batch.data.frame returns 19 columns with decoded regimes matching raw", {
  df <- make_s3_test_df()
  res_df <- newhall_batch(df)

  t_cols <- paste0("t", month.abb)
  p_cols <- paste0("p", month.abb)
  res_raw <- newhall_batch_raw(
    lat = df$latDD,
    lon = df$lonDD,
    temp = as.matrix(df[, t_cols]),
    precip = as.matrix(df[, p_cols]),
    awc = df$awc,
    elev = df$elev
  )

  expect_s3_class(res_df, "data.frame")
  expect_equal(nrow(res_df), nrow(df))
  expect_equal(ncol(res_df), 19L)
  expect_equal(names(res_df), expected_output_names())

  regime_cols <- c(
    "temperatureRegime",
    "moistureRegime",
    "regimeSubdivision1",
    "regimeSubdivision2"
  )
  for (col in regime_cols) {
    expect_type(res_df[[col]], "character")
    expect_false(anyNA(res_df[[col]]))
  }

  numeric_cols <- setdiff(names(res_df), regime_cols)
  for (col in numeric_cols) {
    expect_equal(res_df[[col]], res_raw[[col]])
  }

  expect_equal(
    res_df$temperatureRegime,
    newhallrs:::.str_levels()[res_raw$temperatureRegime + 1L]
  )
  expect_equal(
    res_df$moistureRegime,
    newhallrs:::.smr_levels()[res_raw$moistureRegime + 1L]
  )
  expect_equal(
    res_df$regimeSubdivision1,
    newhallrs:::.smrsub1_levels()[res_raw$regimeSubdivision1 + 1L]
  )
  expect_equal(
    res_df$regimeSubdivision2,
    newhallrs:::.smrsub2_levels()[res_raw$regimeSubdivision2 + 1L]
  )
})

test_that("per-row soil config columns in data.frame override scalar defaults", {
  # Row 1: baseline Mesic (unchanged)
  # Row 2: soilAirOffset 2.5 -> 8.5 flips Mesic -> Thermic (tmean = 7.0 C)
  # Row 3: amplitude 0.66 -> 0.40 flips Mesic -> Isomesic (dif_abs = 10.0 C)
  # Row 4: hasOHorizon FALSE -> TRUE flips Cryic -> Frigid (tmean = 4.0 C, JJA = 7.5 C)
  # Row 5: isSaturated FALSE -> TRUE flips Cryic -> Frigid (tmean = 5.0 C, JJA = 11.5 C)
  cos_profile <- function(tmean, amp_half) {
    tmean - amp_half * cos(2 * pi * (0:11) / 12)
  }
  temps <- rbind(
    cos_profile(9.5, 12.0),
    cos_profile(7.0, 10.0),
    cos_profile(9.5, 5.0),
    c(-3.5, -3.5, 2.0, 2.0, 2.0, 7.5, 7.5, 7.5, 2.0, 2.0, 2.0, -3.5),
    c(-1.5, -1.5, 2.0, 5.0, 8.0, 11.5, 11.5, 11.5, 8.0, 5.0, 2.0, -1.5)
  )
  precips <- matrix(80.0, nrow = 5, ncol = 12)
  colnames(temps) <- paste0("t", month.abb)
  colnames(precips) <- paste0("p", month.abb)

  df_base <- cbind(
    data.frame(
      latDD = rep(45.0, 5),
      lonDD = rep(-93.0, 5),
      awc = rep(200.0, 5),
      elev = rep(300.0, 5),
      stringsAsFactors = FALSE
    ),
    as.data.frame(temps),
    as.data.frame(precips)
  )

  df_per_row <- df_base
  df_per_row$soilAirOffset <- c(2.5, 8.5, 2.5, 2.5, 2.5)
  df_per_row$amplitude <- c(0.66, 0.66, 0.40, 0.66, 0.66)
  df_per_row$hasOHorizon <- c(FALSE, FALSE, FALSE, TRUE, FALSE)
  df_per_row$isSaturated <- c(FALSE, FALSE, FALSE, FALSE, TRUE)

  res_default <- newhall_batch(df_base)
  res_per_row <- newhall_batch(df_per_row)

  expect_equal(res_per_row[1, ], res_default[1, ])
  expect_false(res_per_row$temperatureRegime[2] == res_default$temperatureRegime[2])
  expect_false(res_per_row$temperatureRegime[3] == res_default$temperatureRegime[3])
  expect_false(res_per_row$temperatureRegime[4] == res_default$temperatureRegime[4])
  expect_false(res_per_row$temperatureRegime[5] == res_default$temperatureRegime[5])

  expect_equal(res_per_row$temperatureRegime[2], "Thermic")
  expect_equal(res_per_row$temperatureRegime[3], "Isomesic")
  expect_equal(res_per_row$temperatureRegime[4], "Frigid")
  expect_equal(res_per_row$temperatureRegime[5], "Frigid")

  res_raw_vec <- newhall_batch_raw(
    lat = df_per_row$latDD,
    lon = df_per_row$lonDD,
    temp = temps,
    precip = precips,
    awc = df_per_row$awc,
    elev = df_per_row$elev,
    soil_air_offset = df_per_row$soilAirOffset,
    amplitude = df_per_row$amplitude,
    has_o_horizon = df_per_row$hasOHorizon,
    is_saturated = df_per_row$isSaturated
  )
  expect_equal(res_per_row$annualPotentialEvapotranspiration, res_raw_vec$annualPotentialEvapotranspiration)
  expect_equal(
    res_per_row$temperatureRegime,
    newhallrs:::.str_levels()[res_raw_vec$temperatureRegime + 1L]
  )
})

test_that("unitSystem = 'english' with strict = TRUE matches metric strict = TRUE", {
  df_metric <- make_s3_test_df()
  t_cols <- paste0("t", month.abb)
  p_cols <- paste0("p", month.abb)

  df_english <- df_metric
  df_english[, t_cols] <- df_metric[, t_cols] * 9.0 / 5.0 + 32.0
  df_english[, p_cols] <- df_metric[, p_cols] / 25.4
  df_english$awc <- df_metric$awc / 25.4

  res_metric <- newhall_batch(df_metric, unitSystem = "metric", strict = TRUE)
  res_english <- newhall_batch(df_english, unitSystem = "english", strict = TRUE)

  expect_equal(res_english$annualRainfall, res_metric$annualRainfall, tolerance = 1e-9)
  expect_equal(
    res_english$annualPotentialEvapotranspiration,
    res_metric$annualPotentialEvapotranspiration,
    tolerance = 1e-9
  )
  expect_equal(res_english$annualWaterBalance, res_metric$annualWaterBalance, tolerance = 1e-9)
  expect_equal(res_english$summerWaterBalance, res_metric$summerWaterBalance, tolerance = 1e-9)
  expect_equal(res_english$numCumulativeDaysDry, res_metric$numCumulativeDaysDry)
  expect_equal(res_english$numCumulativeDaysMoist, res_metric$numCumulativeDaysMoist)
  expect_equal(res_english$temperatureRegime, res_metric$temperatureRegime)
  expect_equal(res_english$moistureRegime, res_metric$moistureRegime)
  expect_equal(res_english$regimeSubdivision1, res_metric$regimeSubdivision1)
  expect_equal(res_english$regimeSubdivision2, res_metric$regimeSubdivision2)

  expect_error(newhall_batch(df_metric, unitSystem = "kelvin"), "Unknown unitSystem")
})

test_that("newhall_batch.character reads single and multiple CSV file paths", {
  df <- make_s3_test_df()
  tmp1 <- tempfile(fileext = ".csv")
  tmp2 <- tempfile(fileext = ".csv")
  on.exit(unlink(c(tmp1, tmp2)), add = TRUE)

  utils::write.csv(df[1:2, , drop = FALSE], tmp1, row.names = FALSE)
  utils::write.csv(df[3:4, , drop = FALSE], tmp2, row.names = FALSE)

  res_single <- newhall_batch(tmp1)
  expect_equal(res_single, newhall_batch(df[1:2, , drop = FALSE]))

  res_multi <- newhall_batch(c(tmp1, tmp2))
  expect_equal(res_multi, newhall_batch(df))

  expect_error(newhall_batch(character(0)), "at least one CSV")
  expect_error(newhall_batch(tempfile(fileext = ".csv")), "not found")
})

test_that("newhall_batch_hydrothermal.character reads single and multiple CSV file paths", {
  df <- make_s3_test_df()
  df$zWt <- c(0.3, 100.0, 100.0, 100.0)
  df$oHorizonThickness <- c(0.0, 0.0, 0.12, 0.0)
  for (m in month.abb) {
    df[[paste0("snow", m)]] <- 0.0
  }
  df$snowJan[3] <- 0.8

  tmp1 <- tempfile(fileext = ".csv")
  tmp2 <- tempfile(fileext = ".csv")
  on.exit(unlink(c(tmp1, tmp2)), add = TRUE)
  utils::write.csv(df[1:2, , drop = FALSE], tmp1, row.names = FALSE)
  utils::write.csv(df[3:4, , drop = FALSE], tmp2, row.names = FALSE)

  res_single <- newhall_batch_hydrothermal(tmp1)
  expect_equal(res_single, newhall_batch_hydrothermal(df[1:2, , drop = FALSE]))

  res_multi <- newhall_batch_hydrothermal(c(tmp1, tmp2))
  expect_equal(res_multi, newhall_batch_hydrothermal(df))
  expect_equal(names(res_multi), expected_output_names(include_hydrothermal = TRUE))
  expect_type(res_multi$temperatureRegime, "character")
  expect_type(res_multi$moistureRegime, "character")

  expect_error(newhall_batch_hydrothermal(character(0)), "at least one CSV")
  expect_error(newhall_batch_hydrothermal(NA_character_), "cannot be NA or empty")
  expect_error(newhall_batch_hydrothermal(""), "cannot be NA or empty")
  expect_error(newhall_batch_hydrothermal(tempfile(fileext = ".csv")), "not found")
})

test_that("newhall_batch.SpatRaster handles NA cells, multi-block chunking, cats, and strict mode", {
  skip_if_not_installed("terra")

  df <- make_s3_test_df()
  r <- make_s3_test_raster(df, nrow = 5L, ncol = 2L)

  # Force an all-NA chunk in rows 1-2 (cells 1..4) and partial NAs in rows 3-5
  r[["awc"]][c(1:4, 6L)] <- NA_real_
  r[["tJan"]][8L] <- NA_real_

  out_classic <- newhall_batch(r, nrows = 2L, strict = FALSE)
  expect_s4_class(out_classic, "SpatRaster")
  expect_equal(terra::nlyr(out_classic), 19L)
  expect_equal(names(out_classic), expected_output_names())

  expect_equal(
    unname(terra::is.factor(out_classic)),
    c(rep(FALSE, 15L), rep(TRUE, 4L))
  )

  cats_list <- terra::cats(out_classic)
  expect_equal(
    cats_list[[16]],
    data.frame(
      value = 0:9,
      temperatureRegime = newhallrs:::.str_levels(),
      stringsAsFactors = FALSE
    )
  )
  expect_equal(
    cats_list[[17]],
    data.frame(
      value = 0:5,
      moistureRegime = newhallrs:::.smr_levels(),
      stringsAsFactors = FALSE
    )
  )
  expect_equal(
    cats_list[[18]],
    data.frame(
      value = 0:8,
      regimeSubdivision1 = newhallrs:::.smrsub1_levels(),
      stringsAsFactors = FALSE
    )
  )
  expect_equal(
    cats_list[[19]],
    data.frame(
      value = 0:7,
      regimeSubdivision2 = newhallrs:::.smrsub2_levels(),
      stringsAsFactors = FALSE
    )
  )

  vals_classic <- terra::values(out_classic, dataframe = TRUE)
  expect_equal(nrow(vals_classic), 10L)
  for (na_cell in c(1L, 2L, 3L, 4L, 6L)) {
    expect_true(all(is.na(vals_classic[na_cell, ])))
  }
  # Classic mode only skips NA in awc; cell 8 (tJan = NA, awc valid) is passed to Rust
  expect_false(is.na(vals_classic$annualRainfall[8L]))

  valid_cells <- c(5L, 7L, 9L, 10L)
  r_df_all <- terra::values(r, dataframe = TRUE)
  expected_classic <- newhall_batch(r_df_all[valid_cells, , drop = FALSE], strict = FALSE)
  expect_equal(
    vals_classic[valid_cells, "annualRainfall"],
    expected_classic$annualRainfall,
    tolerance = 1e-9
  )
  expect_equal(
    as.character(vals_classic[valid_cells, "temperatureRegime"]),
    expected_classic$temperatureRegime
  )
  expect_equal(
    as.character(vals_classic[valid_cells, "moistureRegime"]),
    expected_classic$moistureRegime
  )

  # Strict mode skips NA in any required layer (so cell 8 is also skipped)
  out_strict <- newhall_batch(r, nrows = 2L, strict = TRUE)
  vals_strict <- terra::values(out_strict, dataframe = TRUE)
  for (na_cell in c(1L, 2L, 3L, 4L, 6L, 8L)) {
    expect_true(all(is.na(vals_strict[na_cell, ])))
  }
  expected_strict <- newhall_batch(r_df_all[valid_cells, , drop = FALSE], strict = TRUE)
  expect_equal(
    vals_strict[valid_cells, "annualPotentialEvapotranspiration"],
    expected_strict$annualPotentialEvapotranspiration,
    tolerance = 1e-9
  )
  expect_equal(
    as.character(vals_strict[valid_cells, "temperatureRegime"]),
    expected_strict$temperatureRegime
  )

  # File-backed GeoTIFF output round-trip
  tif_path <- tempfile(fileext = ".tif")
  on.exit(unlink(tif_path), add = TRUE)
  out_tif <- newhall_batch(r, file = tif_path, nrows = 2L, overwrite = TRUE)
  expect_true(file.exists(tif_path))
  expect_equal(terra::nlyr(out_tif), 19L)
})

test_that("newhall_batch_hydrothermal works on data.frame and SpatRaster with 21 layers and cats", {
  df <- make_s3_test_df()
  df_p4 <- df
  df_p4$zWt <- c(0.3, 100.0, 100.0, 100.0)
  df_p4$oHorizonThickness <- c(0.0, 0.0, 0.12, 0.0)
  for (m in month.abb) {
    df_p4[[paste0("snow", m)]] <- 0.0
  }
  df_p4$snowJan[3] <- 0.8
  df_p4$snowFeb[3] <- 0.8
  df_p4$snowDec[3] <- 0.7

  res_p4_df <- newhall_batch_hydrothermal(df_p4)
  expect_s3_class(res_p4_df, "data.frame")
  expect_equal(nrow(res_p4_df), nrow(df_p4))
  expect_equal(ncol(res_p4_df), 21L)
  expect_equal(names(res_p4_df), expected_output_names(include_hydrothermal = TRUE))
  expect_type(res_p4_df$temperatureRegime, "character")
  expect_type(res_p4_df$moistureRegime, "character")
  expect_equal(res_p4_df$satDaysMcs[4], 0L)
  expect_true(res_p4_df$satDaysMcs[1] > 0L)
  expect_true(res_p4_df$aquicBioDays[1] > 0L)

  skip_if_not_installed("terra")
  r_p4_in <- make_s3_test_raster(df_p4, nrow = 4L, ncol = 2L)
  r_p4_in[["awc"]][2L] <- NA_real_

  out_p4_rast <- newhall_batch_hydrothermal(r_p4_in, nrows = 2L)
  expect_s4_class(out_p4_rast, "SpatRaster")
  expect_equal(terra::nlyr(out_p4_rast), 21L)
  expect_equal(names(out_p4_rast), expected_output_names(include_hydrothermal = TRUE))
  expect_equal(
    unname(terra::is.factor(out_p4_rast)),
    c(rep(FALSE, 15L), rep(TRUE, 4L), FALSE, FALSE)
  )
  expect_length(terra::cats(out_p4_rast)[[16]]$temperatureRegime, 10L)
  expect_length(terra::cats(out_p4_rast)[[17]]$moistureRegime, 6L)

  vals_p4 <- terra::values(out_p4_rast, dataframe = TRUE)
  expect_true(all(is.na(vals_p4[2L, ])))
  # Cell 1 corresponds to row 1 of df_p4 (shallow water table)
  expect_equal(vals_p4$satDaysMcs[1L], res_p4_df$satDaysMcs[1L])
  expect_equal(vals_p4$aquicBioDays[1L], res_p4_df$aquicBioDays[1L])
  expect_equal(
    as.character(vals_p4$temperatureRegime[1L]),
    res_p4_df$temperatureRegime[1L]
  )
  # Cell 3 corresponds to row 3 of df_p4 (O-horizon + snow)
  expect_equal(
    as.character(vals_p4$temperatureRegime[3L]),
    res_p4_df$temperatureRegime[3L]
  )
})
