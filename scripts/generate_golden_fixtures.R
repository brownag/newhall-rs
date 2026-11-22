#!/usr/bin/env Rscript
# Generate full-precision (%.17g) golden-master CSV fixtures and manifest.json
# from jNSMR for crates/newhall-core integration tests.

suppressPackageStartupMessages({
  library(jNSMR)
  library(terra)
  library(jsonlite)
})

INPUT_COLS <- c(
  "latDD", "lonDD",
  "tJan", "tFeb", "tMar", "tApr", "tMay", "tJun",
  "tJul", "tAug", "tSep", "tOct", "tNov", "tDec",
  "pJan", "pFeb", "pMar", "pApr", "pMay", "pJun",
  "pJul", "pAug", "pSep", "pOct", "pNov", "pDec",
  "awc", "elev"
)

DOUBLE_OUT_COLS <- c(
  "annualRainfall",
  "waterHoldingCapacity",
  "annualWaterBalance",
  "annualPotentialEvapotranspiration",
  "summerWaterBalance"
)

INT_OUT_COLS <- c(
  "dryDaysAfterSummerSolstice",
  "moistDaysAfterWinterSolstice",
  "numCumulativeDaysDry",
  "numCumulativeDaysMoistDry",
  "numCumulativeDaysMoist",
  "numCumulativeDaysDryOver5C",
  "numCumulativeDaysMoistDryOver5C",
  "numCumulativeDaysMoistOver5C",
  "numConsecutiveDaysMoistInSomeParts",
  "numConsecutiveDaysMoistInSomePartsOver8C"
)

CAT_OUT_COLS <- c(
  "temperatureRegime",
  "moistureRegime",
  "regimeSubdivision1",
  "regimeSubdivision2"
)

OUTPUT_COLS <- c(DOUBLE_OUT_COLS, INT_OUT_COLS, CAT_OUT_COLS)

DEFAULT_PARAMS <- list(
  unitSystem = "metric",
  soilAirOffset = 2.5,
  amplitude = 0.66,
  hasOHorizon = FALSE,
  isSaturated = FALSE
)

format_float_col <- function(x) {
  vals <- as.double(x)
  out <- rep(NA_character_, length(vals))
  ok <- !is.na(vals)
  out[ok] <- sprintf("%.17g", vals[ok])
  out
}

empty_output_template <- function(n) {
  df <- vector("list", length(OUTPUT_COLS))
  names(df) <- OUTPUT_COLS
  for (col in DOUBLE_OUT_COLS) {
    df[[col]] <- rep(NA_real_, n)
  }
  for (col in INT_OUT_COLS) {
    df[[col]] <- rep(NA_integer_, n)
  }
  for (col in CAT_OUT_COLS) {
    df[[col]] <- rep(NA_character_, n)
  }
  as.data.frame(df, stringsAsFactors = FALSE)
}

run_oracle_batch <- function(inp_df) {
  inp_df <- inp_df[, INPUT_COLS, drop = FALSE]
  n_total <- nrow(inp_df)
  valid_idx <- which(!is.na(inp_df$awc))
  n_valid <- length(valid_idx)
  n_na <- n_total - n_valid

  t0 <- proc.time()[["elapsed"]]
  res_valid <- jNSMR::newhall_batch(
    .data = inp_df[valid_idx, , drop = FALSE],
    unitSystem = DEFAULT_PARAMS$unitSystem,
    soilAirOffset = DEFAULT_PARAMS$soilAirOffset,
    amplitude = DEFAULT_PARAMS$amplitude,
    hasOHorizon = DEFAULT_PARAMS$hasOHorizon,
    isSaturated = DEFAULT_PARAMS$isSaturated,
    toString = FALSE,
    verbose = FALSE
  )
  elapsed <- proc.time()[["elapsed"]] - t0

  stopifnot(identical(colnames(res_valid), OUTPUT_COLS))
  stopifnot(nrow(res_valid) == n_valid)

  out_df <- empty_output_template(n_total)
  for (col in DOUBLE_OUT_COLS) {
    out_df[valid_idx, col] <- as.double(res_valid[[col]])
  }
  for (col in INT_OUT_COLS) {
    out_df[valid_idx, col] <- as.integer(res_valid[[col]])
  }
  for (col in CAT_OUT_COLS) {
    out_df[valid_idx, col] <- as.character(res_valid[[col]])
  }

  combined <- cbind(inp_df, out_df)
  list(
    data = combined,
    rows_total = n_total,
    rows_valid = n_valid,
    rows_na = n_na,
    runtime_secs = unname(elapsed)
  )
}

write_golden_csv <- function(df, path) {
  formatted <- df
  for (col in c(INPUT_COLS, DOUBLE_OUT_COLS)) {
    formatted[[col]] <- format_float_col(formatted[[col]])
  }
  for (col in INT_OUT_COLS) {
    formatted[[col]] <- as.integer(formatted[[col]])
  }
  for (col in CAT_OUT_COLS) {
    formatted[[col]] <- as.character(formatted[[col]])
  }

  cat_col_indices <- which(colnames(formatted) %in% CAT_OUT_COLS)
  write.table(
    formatted,
    file = path,
    sep = ",",
    dec = ".",
    qmethod = "double",
    row.names = FALSE,
    col.names = TRUE,
    na = "NA",
    quote = cat_col_indices
  )
}

main <- function() {
  out_dir <- file.path("crates", "newhall-core", "tests", "fixtures")
  dir.create(out_dir, recursive = TRUE, showWarnings = FALSE)

  jar_ver <- jNSMR::newhall_version()
  if (!nzchar(jar_ver)) {
    stop("jNSMR::newhall_version() returned empty string; Java JAR not initialized.")
  }

  # 1. Pennsylvania metric CSV batch
  pa_path <- system.file("extdata", "All_PA_jNSM_Example_Batch_Metric.csv",
                         package = "jNSMR", mustWork = TRUE)
  pa_raw <- utils::read.csv(pa_path, stringsAsFactors = FALSE)
  pa_res <- run_oracle_batch(pa_raw)
  pa_out <- file.path(out_dir, "pa_batch_metric.csv")
  write_golden_csv(pa_res$data, pa_out)

  # 2. PRISM ISSR-800 sample raster
  # Extracted to data.frame and run via batch3() (filtering is.na(awc) identically
  # to newhall_batch.SpatRaster) so Java double[] outputs retain full 64-bit
  # precision without GeoTIFF FLT4S truncation.
  prism_path <- system.file("extdata", "prism_issr800_sample.tif",
                            package = "jNSMR", mustWork = TRUE)
  prism_rast <- terra::rast(prism_path)
  prism_raw <- terra::values(prism_rast, dataframe = TRUE)
  prism_res <- run_oracle_batch(prism_raw)
  prism_out <- file.path(out_dir, "prism_sample.csv")
  write_golden_csv(prism_res$data, prism_out)

  # 3. DAYMET ISSR-800 sample raster
  daymet_path <- system.file("extdata", "daymet_issr800_sample.tif",
                             package = "jNSMR", mustWork = TRUE)
  daymet_rast <- terra::rast(daymet_path)
  daymet_raw <- terra::values(daymet_rast, dataframe = TRUE)
  daymet_res <- run_oracle_batch(daymet_raw)
  daymet_out <- file.path(out_dir, "daymet_sample.csv")
  write_golden_csv(daymet_res$data, daymet_out)

  manifest <- list(
    jnsmr_version = as.character(utils::packageVersion("jNSMR")),
    jar_version = jar_ver,
    jar_filename = sprintf("newhall-%s.jar", jar_ver),
    generated_at = format(Sys.time(), "%Y-%m-%dT%H:%M:%SZ", tz = "UTC"),
    r_version = R.version.string,
    parameters = DEFAULT_PARAMS,
    formatting = list(
      float_format = "%.17g",
      na_string = "NA",
      na_policy = paste(
        "All rows/cells preserved in original order;",
        "rows/cells with is.na(awc) receive NA outputs,",
        "matching newhall_batch.SpatRaster() skip logic."
      ),
      raster_oracle_note = paste(
        "Raster cells extracted via terra::values(..., dataframe = TRUE)",
        "and evaluated via newhall_batch() data.frame/batch3 path to preserve",
        "exact 64-bit Java double[] and String[] outputs without GeoTIFF FLT4S quantization."
      )
    ),
    input_columns = INPUT_COLS,
    output_columns = OUTPUT_COLS,
    fixtures = list(
      pa_batch_metric = list(
        file = "pa_batch_metric.csv",
        source_file = basename(pa_path),
        source_type = "csv",
        rows_total = pa_res$rows_total,
        rows_valid = pa_res$rows_valid,
        rows_na = pa_res$rows_na,
        runtime_secs = pa_res$runtime_secs
      ),
      prism_sample = list(
        file = "prism_sample.csv",
        source_file = basename(prism_path),
        source_type = "raster",
        raster_dims = list(
          nrow = terra::nrow(prism_rast),
          ncol = terra::ncol(prism_rast),
          ncell = terra::ncell(prism_rast)
        ),
        rows_total = prism_res$rows_total,
        rows_valid = prism_res$rows_valid,
        rows_na = prism_res$rows_na,
        runtime_secs = prism_res$runtime_secs
      ),
      daymet_sample = list(
        file = "daymet_sample.csv",
        source_file = basename(daymet_path),
        source_type = "raster",
        raster_dims = list(
          nrow = terra::nrow(daymet_rast),
          ncol = terra::ncol(daymet_rast),
          ncell = terra::ncell(daymet_rast)
        ),
        rows_total = daymet_res$rows_total,
        rows_valid = daymet_res$rows_valid,
        rows_na = daymet_res$rows_na,
        runtime_secs = daymet_res$runtime_secs
      )
    )
  )

  manifest_path <- file.path(out_dir, "manifest.json")
  jsonlite::write_json(manifest, manifest_path, auto_unbox = TRUE, pretty = TRUE)

  message(sprintf(
    "Generated golden fixtures in %s (jNSMR %s, %s): PA=%d, PRISM=%d (%d valid), DAYMET=%d (%d valid)",
    out_dir,
    manifest$jnsmr_version,
    manifest$jar_filename,
    pa_res$rows_total,
    prism_res$rows_total, prism_res$rows_valid,
    daymet_res$rows_total, daymet_res$rows_valid
  ))
}

if (sys.nframe() == 0L) {
  main()
}
