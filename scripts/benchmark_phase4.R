#!/usr/bin/env Rscript
suppressPackageStartupMessages({
  if (requireNamespace("pkgload", quietly = TRUE)) {
    pkgload::load_all("r-pkg", quiet = TRUE)
  } else {
    library(newhallrs)
  }
})

n <- 10000L
temp <- matrix(
  rep(c(-2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3), each = n),
  nrow = n, ncol = 12
)
precip <- matrix(85.0, nrow = n, ncol = 12)
lat <- rep(40.8, n)
lon <- rep(-77.8, n)
awc <- rep(200.0, n)
elev <- rep(350.0, n)

t0 <- proc.time()[["elapsed"]]
out_classic <- newhall_batch(lat, lon, temp, precip, awc, elev)
dt_classic <- proc.time()[["elapsed"]] - t0

t1 <- proc.time()[["elapsed"]]
out_p4 <- newhall_batch_phase4(lat, lon, temp, precip, awc, elev)
dt_p4 <- proc.time()[["elapsed"]] - t1

cat(sprintf("Classic mode : %d sims in %.3f s (%.0f sims/sec)\n", n, dt_classic, n / dt_classic))
cat(sprintf("Phase 4 mode : %d sims in %.3f s (%.0f sims/sec)\n", n, dt_p4, n / dt_p4))
