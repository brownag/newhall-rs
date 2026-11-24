test_that("classic newhall_batch runs and matches expected columns", {
  n <- 4
  temp <- matrix(
    c(-2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3),
    nrow = n, ncol = 12, byrow = TRUE
  )
  precip <- matrix(85.0, nrow = n, ncol = 12)
  out <- newhall_batch(
    lat = rep(40.8, n),
    lon = rep(-77.8, n),
    temp = temp,
    precip = precip,
    awc = rep(200.0, n),
    elev = rep(350.0, n)
  )
  expect_s3_class(out, "data.frame")
  expect_equal(nrow(out), n)
  expect_true("annualRainfall" %in% names(out))
  expect_true("temperatureRegime" %in% names(out))
  expect_equal(out$temperatureRegime[1], 3L) # Mesic
})

test_that("phase4 1D thermal snow insulation warms cold winter soil", {
  temp <- matrix(
    c(-15, -12, -4, 4, 11, 16, 19, 18, 12, 5, -5, -13),
    nrow = 2, ncol = 12, byrow = TRUE
  )
  precip <- matrix(40.0, nrow = 2, ncol = 12)
  snow <- rbind(
    rep(0.0, 12),
    c(0.8, 0.8, 0.5, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 0.4, 0.7)
  )
  out <- newhall_batch_phase4(
    lat = c(48.0, 48.0),
    lon = c(-110.0, -110.0),
    temp = temp,
    precip = precip,
    awc = c(200.0, 200.0),
    snow_depth = snow,
    use_thermal_1d = TRUE,
    use_water_table = FALSE
  )
  expect_equal(nrow(out), 2)
  expect_true("satDaysMcs" %in% names(out))
  expect_true("aquicBioDays" %in% names(out))
  expect_false(any(is.na(out$temperatureRegime)))
})

test_that("phase4 shallow water table reduces dry days and records satDaysMcs", {
  temp <- matrix(
    c(2, 5, 9, 14, 19, 25, 28, 27, 22, 15, 8, 3),
    nrow = 2, ncol = 12, byrow = TRUE
  )
  precip <- matrix(15.0, nrow = 2, ncol = 12)
  out <- newhall_batch_phase4(
    lat = c(35.0, 35.0),
    lon = c(-110.0, -110.0),
    temp = temp,
    precip = precip,
    awc = c(200.0, 200.0),
    z_wt = c(100.0, 0.3),
    use_thermal_1d = TRUE,
    use_water_table = TRUE
  )
  expect_true(out$numCumulativeDaysDry[2] <= out$numCumulativeDaysDry[1])
  expect_equal(out$satDaysMcs[1], 0L)
  expect_true(out$satDaysMcs[2] > 0L)
  expect_true(out$aquicBioDays[2] > 0L)
})

test_that("phase4 prognostic water table mode executes cleanly", {
  temp <- matrix(12.0, nrow = 3, ncol = 12)
  precip <- matrix(100.0, nrow = 3, ncol = 12)
  out <- newhall_batch_phase4(
    lat = rep(42.0, 3),
    lon = rep(-85.0, 3),
    temp = temp,
    precip = precip,
    awc = rep(200.0, 3),
    z_wt = rep(1.2, 3),
    use_thermal_1d = TRUE,
    use_water_table = TRUE,
    use_prognostic_wt = TRUE
  )
  expect_equal(nrow(out), 3)
  expect_true(all(out$numCumulativeDaysMoist == 360L))
})

test_that("newhall_batch strict = TRUE fixes Southern Hemisphere PET and imperial units", {
  temp_c <- matrix(
    c(22.1, 22.0, 20.9, 18.2, 15.2, 12.8, 12.0, 13.1, 15.3, 17.6, 19.4, 21.1),
    nrow = 1, ncol = 12
  )
  precip_mm <- matrix(
    c(102, 118, 130, 127, 120, 132, 97, 81, 69, 77, 84, 78),
    nrow = 1, ncol = 12
  )

  classic_sh <- newhall_batch(
    lat = -33.8688,
    lon = 151.2093,
    temp = temp_c,
    precip = precip_mm,
    awc = 200.0,
    elev = 19.0,
    strict = FALSE
  )
  strict_sh <- newhall_batch(
    lat = -33.8688,
    lon = 151.2093,
    temp = temp_c,
    precip = precip_mm,
    awc = 200.0,
    elev = 19.0,
    strict = TRUE
  )

  expect_true(strict_sh$annualPotentialEvapotranspiration > 500.0)
  expect_true(strict_sh$annualPotentialEvapotranspiration < 1500.0)
  expect_false(isTRUE(all.equal(
    classic_sh$annualPotentialEvapotranspiration,
    strict_sh$annualPotentialEvapotranspiration
  )))

  strict_imp <- newhall_batch(
    lat = -33.8688,
    lon = 151.2093,
    temp = temp_c * 9.0 / 5.0 + 32.0,
    precip = precip_mm / 25.4,
    awc = 200.0 / 25.4,
    elev = 19.0,
    soil_air_offset = 2.5 * 9.0 / 5.0,
    is_metric = FALSE,
    strict = TRUE
  )

  expect_equal(strict_imp$annualRainfall, strict_sh$annualRainfall, tolerance = 1e-9)
  expect_equal(
    strict_imp$annualPotentialEvapotranspiration,
    strict_sh$annualPotentialEvapotranspiration,
    tolerance = 1e-9
  )
  expect_equal(strict_imp$annualWaterBalance, strict_sh$annualWaterBalance, tolerance = 1e-9)
  expect_equal(strict_imp$summerWaterBalance, strict_sh$summerWaterBalance, tolerance = 1e-9)
  expect_equal(strict_imp$temperatureRegime, strict_sh$temperatureRegime)
  expect_equal(strict_imp$moistureRegime, strict_sh$moistureRegime)
})

test_that("newhall_batch_phase4 strict_forcing = TRUE remediates SH forcing", {
  temp_c <- matrix(
    c(22.1, 22.0, 20.9, 18.2, 15.2, 12.8, 12.0, 13.1, 15.3, 17.6, 19.4, 21.1),
    nrow = 1, ncol = 12
  )
  precip_mm <- matrix(
    c(102, 118, 130, 127, 120, 132, 97, 81, 69, 77, 84, 78),
    nrow = 1, ncol = 12
  )

  p4_strict <- newhall_batch_phase4(
    lat = -33.8688,
    lon = 151.2093,
    temp = temp_c,
    precip = precip_mm,
    awc = 200.0,
    strict_forcing = TRUE
  )
  p4_classic_forcing <- newhall_batch_phase4(
    lat = -33.8688,
    lon = 151.2093,
    temp = temp_c,
    precip = precip_mm,
    awc = 200.0,
    strict_forcing = FALSE
  )

  expect_true(p4_strict$annualPotentialEvapotranspiration > 500.0)
  expect_true(p4_strict$annualPotentialEvapotranspiration < 1500.0)
  expect_false(isTRUE(all.equal(
    p4_strict$annualPotentialEvapotranspiration,
    p4_classic_forcing$annualPotentialEvapotranspiration
  )))
})
