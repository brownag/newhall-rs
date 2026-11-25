#!/usr/bin/env Rscript
# Reproducible SpatRaster benchmark comparing newhallrs::newhall_batch()
# against jNSMR::newhall_batch() on the sample rasters bundled with jNSMR.

load_newhallrs <- function() {
  if (file.exists("r-pkg/DESCRIPTION")) {
    if (requireNamespace("devtools", quietly = TRUE)) {
      suppressPackageStartupMessages(devtools::load_all("r-pkg", quiet = TRUE))
      return("devtools::load_all('r-pkg')")
    }
    if (requireNamespace("pkgload", quietly = TRUE)) {
      suppressPackageStartupMessages(pkgload::load_all("r-pkg", quiet = TRUE))
      return("pkgload::load_all('r-pkg')")
    }
  }
  suppressPackageStartupMessages(library(newhallrs))
  "library(newhallrs)"
}

if (!requireNamespace("jNSMR", quietly = TRUE)) {
  stop("Package 'jNSMR' must be installed to run this benchmark.", call. = FALSE)
}
if (!requireNamespace("terra", quietly = TRUE)) {
  stop("Package 'terra' must be installed to run this benchmark.", call. = FALSE)
}

newhallrs_load_mode <- load_newhallrs()
# library(jNSMR) is required so jNSMR:::.onAttach() initializes options(jNSMR.NSM_VERSION).
suppressPackageStartupMessages(library(jNSMR))
suppressPackageStartupMessages(library(terra))

n_warmup <- 1L
n_reps <- 3L
rasters <- c("prism_issr800_sample.tif", "daymet_issr800_sample.tif")

time_calls <- function(fn, n) {
  elapsed <- numeric(n)
  last_res <- NULL
  for (i in seq_len(n)) {
    gc(FALSE)
    t0 <- proc.time()[["elapsed"]]
    last_res <- fn()
    elapsed[i] <- proc.time()[["elapsed"]] - t0
  }
  list(elapsed = elapsed, result = last_res)
}

benchmark_raster <- function(raster_name, n_warmup, n_reps) {
  path <- system.file("extdata", raster_name, package = "jNSMR")
  if (!nzchar(path) || !file.exists(path)) {
    stop(sprintf("Sample raster not found in jNSMR extdata: %s", raster_name), call. = FALSE)
  }

  r <- terra::rast(path)
  total_cells <- as.integer(terra::ncell(r))
  awc_vals <- terra::values(r[["awc"]], mat = FALSE)
  valid_cells <- as.integer(sum(!is.na(awc_vals)))

  run_newhallrs <- function() {
    newhallrs::newhall_batch(r, file = "")
  }
  run_jnsmr <- function() {
    jNSMR::newhall_batch(r, verbose = FALSE, file = "")
  }

  # Warm-up (excluded from timing)
  warm_rs <- time_calls(run_newhallrs, n_warmup)$result
  warm_jn <- time_calls(run_jnsmr, n_warmup)$result
  stopifnot(
    identical(dim(warm_rs), dim(warm_jn)),
    identical(names(warm_rs), names(warm_jn))
  )

  # Timed replicates
  rs_times <- time_calls(run_newhallrs, n_reps)$elapsed
  jn_times <- time_calls(run_jnsmr, n_reps)$elapsed

  rs_med <- stats::median(rs_times)
  jn_med <- stats::median(jn_times)

  list(
    summary = data.frame(
      raster = raster_name,
      total_cells = total_cells,
      valid_cells = valid_cells,
      newhallrs_med_s = rs_med,
      jnsmr_med_s = jn_med,
      newhallrs_valid_cells_s = valid_cells / rs_med,
      jnsmr_valid_cells_s = valid_cells / jn_med,
      newhallrs_total_cells_s = total_cells / rs_med,
      jnsmr_total_cells_s = total_cells / jn_med,
      speedup = jn_med / rs_med,
      stringsAsFactors = FALSE
    ),
    replicates = list(newhallrs = rs_times, jnsmr = jn_times)
  )
}

cat("=== newhallrs vs jNSMR SpatRaster Benchmark ===\n")
cat(sprintf("Timestamp (UTC): %s\n", format(Sys.time(), "%Y-%m-%dT%H:%M:%SZ", tz = "UTC")))
cat(sprintf("R version:       %s\n", R.version.string))
cat(sprintf("Platform:        %s\n", R.version$platform))
cat(sprintf("newhallrs:       %s (%s)\n", as.character(utils::packageVersion("newhallrs")), newhallrs_load_mode))
cat(sprintf("jNSMR:           %s (Java Newhall %s, cores = 1)\n",
            as.character(utils::packageVersion("jNSMR")),
            as.character(jNSMR::newhall_version())))
cat(sprintf("terra:           %s\n", as.character(utils::packageVersion("terra"))))
cat(sprintf("Protocol:        %d warm-up + %d timed replicates (file = \"\", median wall-clock)\n\n",
            n_warmup, n_reps))

results <- lapply(rasters, benchmark_raster, n_warmup = n_warmup, n_reps = n_reps)

cat("Per-replicate elapsed times (seconds):\n")
for (item in results) {
  cat(sprintf(
    "  %s | newhallrs: [%s] | jNSMR: [%s]\n",
    item$summary$raster,
    paste(sprintf("%.4f", item$replicates$newhallrs), collapse = ", "),
    paste(sprintf("%.4f", item$replicates$jnsmr), collapse = ", ")
  ))
}
cat("\n")

summary_df <- do.call(rbind, lapply(results, `[[`, "summary"))
display_df <- data.frame(
  Raster = summary_df$raster,
  TotalCells = summary_df$total_cells,
  ValidCells = summary_df$valid_cells,
  newhallrs_s = sprintf("%.4f", summary_df$newhallrs_med_s),
  jNSMR_s = sprintf("%.4f", summary_df$jnsmr_med_s),
  newhallrs_cells_s = sprintf("%.0f", summary_df$newhallrs_valid_cells_s),
  jNSMR_cells_s = sprintf("%.0f", summary_df$jnsmr_valid_cells_s),
  Speedup = sprintf("%.1fx", summary_df$speedup),
  stringsAsFactors = FALSE
)
old_width <- getOption("width")
on.exit(options(width = old_width), add = TRUE)
options(width = 160L)
print(display_df, row.names = FALSE, right = FALSE)
