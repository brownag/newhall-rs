# Internal categorical level tables, regime decoding, unit normalization,
# and input schema validation/extraction helpers matching jNSMR v0.3.1 / newhall-1.6.5.

.str_levels <- function() {
  c(
    "Pergelic",
    "Cryic",
    "Frigid",
    "Mesic",
    "Thermic",
    "Hyperthermic",
    "Isofrigid",
    "Isomesic",
    "Isothermic",
    "Isohyperthermic"
  )
}

.smr_levels <- function() {
  c("Aridic", "Ustic", "Xeric", "Udic", "Perudic", "Undefined")
}

.smrsub1_levels <- function() {
  c(
    "Typic",
    "Weak",
    "Wet",
    "Dry",
    "Extreme",
    "Xeric",
    "Udic",
    "Aridic",
    "Undefined"
  )
}

.smrsub2_levels <- function() {
  c(
    "Aridic",
    "Tempustic",
    "Tropustic",
    "Tempudic",
    "Xeric",
    "Udic",
    "Tropudic",
    "Undefined"
  )
}

.regime_cat_df <- function(levels) {
  data.frame(
    value = seq_along(levels) - 1L,
    category = as.character(levels),
    stringsAsFactors = FALSE
  )
}

.decode_regimes <- function(df) {
  regime_map <- list(
    temperatureRegime = .str_levels(),
    moistureRegime = .smr_levels(),
    regimeSubdivision1 = .smrsub1_levels(),
    regimeSubdivision2 = .smrsub2_levels()
  )
  for (col_name in names(regime_map)) {
    if (!col_name %in% names(df)) {
      next
    }
    raw_col <- df[[col_name]]
    if (is.character(raw_col)) {
      next
    }
    levels <- regime_map[[col_name]]
    codes <- as.integer(raw_col)
    decoded <- rep(NA_character_, length(codes))
    valid <- !is.na(codes) & codes >= 0L & codes < length(levels)
    decoded[valid] <- levels[codes[valid] + 1L]
    df[[col_name]] <- decoded
  }
  df
}

.normalize_unit_system <- function(unitSystem) {
  if (!is.character(unitSystem) || length(unitSystem) != 1L || is.na(unitSystem)) {
    stop("'unitSystem' must be a single non-NA character string.", call. = FALSE)
  }
  unit <- tolower(trimws(unitSystem))
  if (unit %in% c("metric", "mm", "cm")) {
    return(TRUE)
  }
  if (unit %in% c("in", "english")) {
    return(FALSE)
  }
  stop(
    sprintf(
      "Unknown unitSystem '%s'; expected 'metric', 'mm', 'cm', 'in', or 'english'.",
      unitSystem
    ),
    call. = FALSE
  )
}

.newhall_required_cols <- function() {
  c(
    "latDD",
    "lonDD",
    paste0("t", month.abb),
    paste0("p", month.abb),
    "awc",
    "elev"
  )
}

.extract_monthly_matrix <- function(df, cols) {
  vals <- unlist(lapply(cols, function(col) as.double(df[[col]])), use.names = FALSE)
  matrix(vals, nrow = nrow(df), ncol = length(cols))
}

.extract_optional_col <- function(df, col_name, default_val, coerce_fn) {
  if (col_name %in% names(df)) {
    coerce_fn(df[[col_name]])
  } else {
    coerce_fn(default_val)
  }
}

.extract_df_inputs <- function(
    .data,
    soilAirOffset = 2.5,
    amplitude = 0.66,
    hasOHorizon = FALSE,
    isSaturated = FALSE
) {
  if (!is.data.frame(.data)) {
    stop("'.data' must be a data.frame.", call. = FALSE)
  }
  required <- .newhall_required_cols()
  missing_cols <- setdiff(required, names(.data))
  if (length(missing_cols) > 0L) {
    stop(
      sprintf(
        "Missing required columns in '.data': %s",
        paste(missing_cols, collapse = ", ")
      ),
      call. = FALSE
    )
  }

  t_cols <- paste0("t", month.abb)
  p_cols <- paste0("p", month.abb)

  list(
    lat = as.double(.data[["latDD"]]),
    lon = as.double(.data[["lonDD"]]),
    temp = .extract_monthly_matrix(.data, t_cols),
    precip = .extract_monthly_matrix(.data, p_cols),
    awc = as.double(.data[["awc"]]),
    elev = as.double(.data[["elev"]]),
    soilAirOffset = .extract_optional_col(.data, "soilAirOffset", soilAirOffset, as.double),
    amplitude = .extract_optional_col(.data, "amplitude", amplitude, as.double),
    hasOHorizon = .extract_optional_col(.data, "hasOHorizon", hasOHorizon, as.logical),
    isSaturated = .extract_optional_col(.data, "isSaturated", isSaturated, as.logical)
  )
}
