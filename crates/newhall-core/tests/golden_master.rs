use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use newhall_core::{
    run_batch_soa, simulate_site, SiteConfig, SiteInput, SiteOutput, SoaBatchInput,
};

const FLOAT_TOL: f64 = 1e-9;

const EXPECTED_HEADER: [&str; 47] = [
    "latDD",
    "lonDD",
    "tJan",
    "tFeb",
    "tMar",
    "tApr",
    "tMay",
    "tJun",
    "tJul",
    "tAug",
    "tSep",
    "tOct",
    "tNov",
    "tDec",
    "pJan",
    "pFeb",
    "pMar",
    "pApr",
    "pMay",
    "pJun",
    "pJul",
    "pAug",
    "pSep",
    "pOct",
    "pNov",
    "pDec",
    "awc",
    "elev",
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
    "regimeSubdivision2",
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Parses a single CSV line handling double-quoted fields (including `" "` and `"NA"`)
/// and escaped `""` quotes without external dependencies.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::with_capacity(47);
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    assert!(!in_quotes, "unterminated quoted CSV field in line: {line}");
    fields.push(current);
    fields
}

#[derive(Debug)]
struct FixtureRow {
    row_num: usize,
    input: SiteInput,
    is_na_awc: bool,
    raw_outputs: [String; 19],
}

fn parse_f64_or_nan(s: &str, row_num: usize, col: &str) -> f64 {
    if s == "NA" {
        f64::NAN
    } else {
        s.parse::<f64>()
            .unwrap_or_else(|e| panic!("row {row_num} col {col} invalid f64 '{s}': {e}"))
    }
}

fn load_fixture(filename: &str) -> Vec<FixtureRow> {
    let path = fixture_path(filename);
    let file =
        File::open(&path).unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()));
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header_line = lines
        .next()
        .unwrap_or_else(|| panic!("{filename}: missing header"))
        .unwrap_or_else(|e| panic!("{filename}: failed reading header: {e}"));
    let header = parse_csv_line(&header_line);
    assert_eq!(
        header.len(),
        EXPECTED_HEADER.len(),
        "{filename}: unexpected header column count"
    );
    for (idx, (actual, expected)) in header.iter().zip(EXPECTED_HEADER.iter()).enumerate() {
        assert_eq!(
            actual, expected,
            "{filename}: header mismatch at column {idx}"
        );
    }

    let mut rows = Vec::new();
    for (idx, line_res) in lines.enumerate() {
        let row_num = idx + 1;
        let line =
            line_res.unwrap_or_else(|e| panic!("{filename}: failed reading row {row_num}: {e}"));
        if line.is_empty() {
            continue;
        }
        let cols = parse_csv_line(&line);
        assert_eq!(
            cols.len(),
            47,
            "{filename} row {row_num}: expected 47 columns, got {}",
            cols.len()
        );

        let is_na_awc = cols[26] == "NA";
        let temp_c = std::array::from_fn(|m| {
            parse_f64_or_nan(&cols[2 + m], row_num, EXPECTED_HEADER[2 + m])
        });
        let precip_mm = std::array::from_fn(|m| {
            parse_f64_or_nan(&cols[14 + m], row_num, EXPECTED_HEADER[14 + m])
        });
        let input = SiteInput {
            lat_dd: parse_f64_or_nan(&cols[0], row_num, "latDD"),
            lon_dd: parse_f64_or_nan(&cols[1], row_num, "lonDD"),
            temp_c,
            precip_mm,
            awc_mm: parse_f64_or_nan(&cols[26], row_num, "awc"),
            elev_m: parse_f64_or_nan(&cols[27], row_num, "elev"),
        };
        let raw_outputs: [String; 19] = std::array::from_fn(|i| cols[28 + i].clone());

        rows.push(FixtureRow {
            row_num,
            input,
            is_na_awc,
            raw_outputs,
        });
    }
    rows
}

fn assert_float_close(fixture: &str, row_num: usize, field: &str, actual: f64, expected_str: &str) {
    let expected: f64 = expected_str.parse().unwrap_or_else(|e| {
        panic!("{fixture} row {row_num} {field}: invalid f64 '{expected_str}': {e}")
    });
    let diff = (actual - expected).abs();
    assert!(
        actual.is_finite() && diff <= FLOAT_TOL,
        "{fixture} row {row_num} {field}: actual={actual} expected={expected} diff={diff} > {FLOAT_TOL}"
    );
}

fn assert_i32_exact(fixture: &str, row_num: usize, field: &str, actual: i32, expected_str: &str) {
    let expected: i32 = expected_str.parse().unwrap_or_else(|e| {
        panic!("{fixture} row {row_num} {field}: invalid i32 '{expected_str}': {e}")
    });
    assert_eq!(
        actual, expected,
        "{fixture} row {row_num} {field}: actual={actual} expected={expected}"
    );
}

fn assert_valid_row_matches(fixture: &str, row: &FixtureRow, actual: &SiteOutput) {
    let r = row.row_num;
    let exp = &row.raw_outputs;

    assert_float_close(
        fixture,
        r,
        "annualRainfall",
        actual.annual_rainfall,
        &exp[0],
    );
    assert_float_close(
        fixture,
        r,
        "waterHoldingCapacity",
        actual.water_holding_capacity,
        &exp[1],
    );
    assert_float_close(
        fixture,
        r,
        "annualWaterBalance",
        actual.annual_water_balance,
        &exp[2],
    );
    assert_float_close(
        fixture,
        r,
        "annualPotentialEvapotranspiration",
        actual.annual_potential_evapotranspiration,
        &exp[3],
    );
    assert_float_close(
        fixture,
        r,
        "summerWaterBalance",
        actual.summer_water_balance,
        &exp[4],
    );

    assert_i32_exact(
        fixture,
        r,
        "dryDaysAfterSummerSolstice",
        actual.dry_days_after_summer_solstice,
        &exp[5],
    );
    assert_i32_exact(
        fixture,
        r,
        "moistDaysAfterWinterSolstice",
        actual.moist_days_after_winter_solstice,
        &exp[6],
    );
    assert_i32_exact(
        fixture,
        r,
        "numCumulativeDaysDry",
        actual.num_cumulative_days_dry,
        &exp[7],
    );
    assert_i32_exact(
        fixture,
        r,
        "numCumulativeDaysMoistDry",
        actual.num_cumulative_days_moist_dry,
        &exp[8],
    );
    assert_i32_exact(
        fixture,
        r,
        "numCumulativeDaysMoist",
        actual.num_cumulative_days_moist,
        &exp[9],
    );
    assert_i32_exact(
        fixture,
        r,
        "numCumulativeDaysDryOver5C",
        actual.num_cumulative_days_dry_over_5c,
        &exp[10],
    );
    assert_i32_exact(
        fixture,
        r,
        "numCumulativeDaysMoistDryOver5C",
        actual.num_cumulative_days_moist_dry_over_5c,
        &exp[11],
    );
    assert_i32_exact(
        fixture,
        r,
        "numCumulativeDaysMoistOver5C",
        actual.num_cumulative_days_moist_over_5c,
        &exp[12],
    );
    assert_i32_exact(
        fixture,
        r,
        "numConsecutiveDaysMoistInSomeParts",
        actual.num_consecutive_days_moist_in_some_parts,
        &exp[13],
    );
    assert_i32_exact(
        fixture,
        r,
        "numConsecutiveDaysMoistInSomePartsOver8C",
        actual.num_consecutive_days_moist_in_some_parts_over_8c,
        &exp[14],
    );

    let temp_str = actual.temperature_regime.map_or("", |t| t.as_java_str());
    let moist_str = actual.moisture_regime.as_java_str();
    let sub1_str = actual.regime_subdivision1.as_java_str();
    let sub2_str = actual.regime_subdivision2_java_str();

    assert_eq!(
        temp_str, exp[15],
        "{fixture} row {r} temperatureRegime mismatch"
    );
    assert_eq!(
        moist_str, exp[16],
        "{fixture} row {r} moistureRegime mismatch"
    );
    assert_eq!(
        sub1_str, exp[17],
        "{fixture} row {r} regimeSubdivision1 mismatch"
    );
    assert_eq!(
        sub2_str, exp[18],
        "{fixture} row {r} regimeSubdivision2 mismatch"
    );
}

fn assert_na_row(fixture: &str, row: &FixtureRow, actual: &SiteOutput) {
    for (idx, val) in row.raw_outputs.iter().enumerate() {
        assert_eq!(
            val,
            "NA",
            "{fixture} row {} col {}: expected NA when awc == NA, got '{val}'",
            row.row_num,
            EXPECTED_HEADER[28 + idx]
        );
    }
    assert!(
        actual.water_holding_capacity.is_nan(),
        "{fixture} row {}: expected NaN water_holding_capacity when awc is NaN",
        row.row_num
    );
}

fn run_soa_for_rows(rows: &[FixtureRow], config: &SiteConfig) -> Vec<SiteOutput> {
    let lat_dd: Vec<f64> = rows.iter().map(|r| r.input.lat_dd).collect();
    let lon_dd: Vec<f64> = rows.iter().map(|r| r.input.lon_dd).collect();
    let awc_mm: Vec<f64> = rows.iter().map(|r| r.input.awc_mm).collect();
    let elev_m: Vec<f64> = rows.iter().map(|r| r.input.elev_m).collect();
    let temp_cols: [Vec<f64>; 12] =
        std::array::from_fn(|m| rows.iter().map(|r| r.input.temp_c[m]).collect());
    let precip_cols: [Vec<f64>; 12] =
        std::array::from_fn(|m| rows.iter().map(|r| r.input.precip_mm[m]).collect());

    let batch = SoaBatchInput {
        lat_dd: &lat_dd,
        lon_dd: &lon_dd,
        temp_c: std::array::from_fn(|m| temp_cols[m].as_slice()),
        precip_mm: std::array::from_fn(|m| precip_cols[m].as_slice()),
        awc_mm: &awc_mm,
        elev_m: &elev_m,
    };
    run_batch_soa(&batch, config)
}

fn verify_fixture(
    filename: &str,
    expected_total: usize,
    expected_valid: usize,
    expected_na: usize,
) {
    let rows = load_fixture(filename);
    assert_eq!(rows.len(), expected_total, "{filename}: row count mismatch");

    let config = SiteConfig {
        soil_air_offset: 2.5,
        amplitude: 0.66,
        has_o_horizon: false,
        is_saturated: false,
        is_metric: true,
    };
    assert_eq!(config, SiteConfig::default());

    let mut valid_count = 0usize;
    let mut na_count = 0usize;

    for row in &rows {
        let out = simulate_site(&row.input, &config);
        if row.is_na_awc {
            na_count += 1;
            assert_na_row(filename, row, &out);
        } else {
            valid_count += 1;
            assert_valid_row_matches(filename, row, &out);
        }
    }

    assert_eq!(valid_count, expected_valid, "{filename}: valid row count");
    assert_eq!(na_count, expected_na, "{filename}: NA row count");

    let soa_outputs = run_soa_for_rows(&rows, &config);
    assert_eq!(soa_outputs.len(), rows.len());
    for (row, soa_out) in rows.iter().zip(soa_outputs.iter()) {
        if row.is_na_awc {
            assert_na_row(filename, row, soa_out);
        } else {
            assert_valid_row_matches(filename, row, soa_out);
        }
    }
}

#[test]
fn csv_parser_handles_quotes_spaces_and_na() {
    let cols = parse_csv_line(r#"40.65,-75.45,"Mesic","Perudic"," ","NA""#);
    assert_eq!(cols, vec!["40.65", "-75.45", "Mesic", "Perudic", " ", "NA"]);
}

#[test]
fn golden_master_pa_batch_metric() {
    verify_fixture("pa_batch_metric.csv", 108, 108, 0);
}

#[test]
fn golden_master_prism_sample() {
    verify_fixture("prism_sample.csv", 18_984, 18_790, 194);
}

#[test]
fn golden_master_daymet_sample() {
    verify_fixture("daymet_sample.csv", 18_984, 18_790, 194);
}
