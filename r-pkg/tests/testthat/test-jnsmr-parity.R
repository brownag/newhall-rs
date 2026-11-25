expect_spatraster_jnsmr_parity <- function(extdata_rel_path) {
  skip_on_cran()
  skip_if_not(requireNamespace("jNSMR", quietly = TRUE), "jNSMR not installed")
  skip_if_not_installed("terra")

  tif_path <- system.file(extdata_rel_path, package = "jNSMR")
  expect_true(nzchar(tif_path) && file.exists(tif_path))

  r <- terra::rast(tif_path)
  actual <- newhallrs::newhall_batch(r)
  expected <- getExportedValue("jNSMR", "newhall_batch")(r)

  expect_s4_class(actual, "SpatRaster")
  expect_equal(terra::nlyr(actual), 19L)
  expect_identical(names(actual), names(expected))
  expect_identical(terra::is.factor(actual), terra::is.factor(expected))
  expect_equal(terra::cats(actual)[16:19], terra::cats(expected)[16:19])

  actual_mat <- terra::values(actual, mat = TRUE)
  expected_mat <- terra::values(expected, mat = TRUE)
  expect_identical(dim(actual_mat), dim(expected_mat))
  expect_identical(is.na(actual_mat), is.na(expected_mat))

  for (j in 1:15) {
    valid <- !is.na(expected_mat[, j])
    expect_equal(
      unname(actual_mat[valid, j]),
      unname(expected_mat[valid, j]),
      tolerance = 1e-9
    )
  }

  actual_df <- terra::values(actual, dataframe = TRUE)
  expected_df <- terra::values(expected, dataframe = TRUE)
  for (j in 16:19) {
    valid <- !is.na(expected_mat[, j])
    expect_identical(
      as.integer(actual_mat[valid, j]),
      as.integer(expected_mat[valid, j])
    )
    expect_identical(
      as.character(actual_df[[j]]),
      as.character(expected_df[[j]])
    )
  }
}

test_that("newhall_batch CSV output matches jNSMR across all 19 columns", {
  skip_on_cran()
  skip_if_not(requireNamespace("jNSMR", quietly = TRUE), "jNSMR not installed")
  skip_if_not_installed("terra")

  csv_path <- system.file(
    "extdata/All_PA_jNSM_Example_Batch_Metric.csv",
    package = "jNSMR"
  )
  expect_true(nzchar(csv_path) && file.exists(csv_path))

  actual <- newhallrs::newhall_batch(csv_path)
  expected <- getExportedValue("jNSMR", "newhall_batch")(csv_path)

  expect_s3_class(actual, "data.frame")
  expect_identical(dim(actual), dim(expected))
  expect_identical(names(actual), names(expected))

  for (j in 1:15) {
    expect_equal(
      as.numeric(actual[[j]]),
      as.numeric(expected[[j]]),
      tolerance = 1e-9
    )
  }

  for (j in 16:19) {
    expect_identical(actual[[j]], expected[[j]])
  }
})

test_that("newhall_batch SpatRaster matches jNSMR on prism_issr800_sample.tif", {
  expect_spatraster_jnsmr_parity("extdata/prism_issr800_sample.tif")
})

test_that("newhall_batch SpatRaster matches jNSMR on daymet_issr800_sample.tif", {
  expect_spatraster_jnsmr_parity("extdata/daymet_issr800_sample.tif")
})
