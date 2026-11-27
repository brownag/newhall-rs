use std::env;
use std::fs::File;
use std::hint::black_box;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use newhall_core::{
    ClassicHydrology, ClassicNewhall, ClassicThermal, ForcingModel, HydrologyStep, SimEngine,
    SiteConfig, SiteInput, SiteOutput, SoaBatchInput, StrictNewhall, Thermal1DConfig,
    Thermal1DModel, ThermalModel, ThornthwaiteForcing, WaterTableConfig, WaterTableHydrology,
};

const FLOAT_TOL: f64 = 1e-9;
const DEFAULT_BENCH_SITES: usize = 100_000;

const INPUT_HEADER: [&str; 28] = [
    "latDD", "lonDD", "tJan", "tFeb", "tMar", "tApr", "tMay", "tJun", "tJul", "tAug", "tSep",
    "tOct", "tNov", "tDec", "pJan", "pFeb", "pMar", "pApr", "pMay", "pJun", "pJul", "pAug", "pSep",
    "pOct", "pNov", "pDec", "awc", "elev",
];

const OUTPUT_HEADER: [&str; 19] = [
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

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run {
        input: PathBuf,
        output: Option<PathBuf>,
        strict: bool,
    },
    Verify {
        input: PathBuf,
        expected: PathBuf,
    },
    Bench {
        n: usize,
    },
    Help,
}

fn usage() -> &'static str {
    "Usage:
  newhall-cli run <input.csv> [--output <out.csv>] [--strict]
  newhall-cli verify <input.csv> <expected.csv>
  newhall-cli bench [N]"
}

fn parse_args<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iter = args.into_iter();
    let Some(subcmd) = iter.next() else {
        return Err(usage().to_string());
    };

    match subcmd.as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "run" => {
            let mut input: Option<PathBuf> = None;
            let mut output: Option<PathBuf> = None;
            let mut strict = false;

            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--strict" => strict = true,
                    "--output" | "-o" => {
                        let val = iter
                            .next()
                            .ok_or_else(|| format!("missing path after {arg}\n\n{}", usage()))?;
                        output = Some(PathBuf::from(val));
                    }
                    _ if arg.starts_with('-') => {
                        return Err(format!("unknown flag '{arg}'\n\n{}", usage()));
                    }
                    _ => {
                        if input.is_some() {
                            return Err(format!(
                                "unexpected extra argument '{arg}'\n\n{}",
                                usage()
                            ));
                        }
                        input = Some(PathBuf::from(arg));
                    }
                }
            }

            let input = input.ok_or_else(|| format!("missing <input.csv>\n\n{}", usage()))?;
            Ok(Command::Run {
                input,
                output,
                strict,
            })
        }
        "verify" => {
            let input = iter
                .next()
                .ok_or_else(|| format!("missing <input.csv>\n\n{}", usage()))?;
            let expected = iter
                .next()
                .ok_or_else(|| format!("missing <expected.csv>\n\n{}", usage()))?;
            if let Some(extra) = iter.next() {
                return Err(format!(
                    "unexpected extra argument '{extra}'\n\n{}",
                    usage()
                ));
            }
            Ok(Command::Verify {
                input: PathBuf::from(input),
                expected: PathBuf::from(expected),
            })
        }
        "bench" => {
            let n = match iter.next() {
                Some(raw) => {
                    let parsed = raw
                        .parse::<usize>()
                        .map_err(|e| format!("invalid site count '{raw}': {e}\n\n{}", usage()))?;
                    if parsed == 0 {
                        return Err(format!("bench site count must be > 0\n\n{}", usage()));
                    }
                    parsed
                }
                None => DEFAULT_BENCH_SITES,
            };
            if let Some(extra) = iter.next() {
                return Err(format!(
                    "unexpected extra argument '{extra}'\n\n{}",
                    usage()
                ));
            }
            Ok(Command::Bench { n })
        }
        other => Err(format!("unknown subcommand '{other}'\n\n{}", usage())),
    }
}

/// Parses a single CSV line handling double-quoted fields (including `" "` and `"NA"`)
/// and escaped `""` quotes without external dependencies.
fn parse_csv_line(line: &str) -> Result<Vec<String>, String> {
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
    if in_quotes {
        return Err(format!("unterminated quoted CSV field in line: {line}"));
    }
    fields.push(current);
    Ok(fields)
}

fn validate_header_slice(
    actual: &[String],
    expected: &[&str],
    context: &str,
) -> Result<(), String> {
    for (idx, (got, exp)) in actual.iter().zip(expected.iter()).enumerate() {
        if got != exp {
            return Err(format!(
                "{context}: header mismatch at column {idx}: expected '{exp}', got '{got}'"
            ));
        }
    }
    Ok(())
}

fn parse_f64_or_nan(s: &str, row_num: usize, col: &str) -> Result<f64, String> {
    if s == "NA" {
        Ok(f64::NAN)
    } else {
        s.parse::<f64>()
            .map_err(|e| format!("row {row_num} col {col}: invalid f64 '{s}': {e}"))
    }
}

fn read_input_csv(path: &Path) -> Result<Vec<SiteInput>, String> {
    let file = File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header_line = lines
        .next()
        .ok_or_else(|| format!("{}: missing CSV header", path.display()))?
        .map_err(|e| format!("{}: failed reading header: {e}", path.display()))?;
    let header = parse_csv_line(&header_line)?;
    match header.len() {
        28 => validate_header_slice(&header, &INPUT_HEADER, &path.display().to_string())?,
        47 => {
            validate_header_slice(&header[..28], &INPUT_HEADER, &path.display().to_string())?;
            validate_header_slice(&header[28..], &OUTPUT_HEADER, &path.display().to_string())?;
        }
        n => {
            return Err(format!(
                "{}: expected 28 input or 47 combined columns, got {n}",
                path.display()
            ));
        }
    }
    let expected_cols = header.len();

    let mut inputs = Vec::new();
    for (idx, line_res) in lines.enumerate() {
        let row_num = idx + 1;
        let line = line_res
            .map_err(|e| format!("{}: failed reading row {row_num}: {e}", path.display()))?;
        if line.is_empty() {
            continue;
        }
        let cols = parse_csv_line(&line)?;
        if cols.len() != expected_cols {
            return Err(format!(
                "{} row {row_num}: expected {expected_cols} columns, got {}",
                path.display(),
                cols.len()
            ));
        }

        let mut temp_c = [0.0_f64; 12];
        for m in 0..12 {
            temp_c[m] = parse_f64_or_nan(&cols[2 + m], row_num, INPUT_HEADER[2 + m])?;
        }
        let mut precip_mm = [0.0_f64; 12];
        for m in 0..12 {
            precip_mm[m] = parse_f64_or_nan(&cols[14 + m], row_num, INPUT_HEADER[14 + m])?;
        }

        inputs.push(SiteInput {
            lat_dd: parse_f64_or_nan(&cols[0], row_num, "latDD")?,
            lon_dd: parse_f64_or_nan(&cols[1], row_num, "lonDD")?,
            temp_c,
            precip_mm,
            awc_mm: parse_f64_or_nan(&cols[26], row_num, "awc")?,
            elev_m: parse_f64_or_nan(&cols[27], row_num, "elev")?,
        });
    }

    Ok(inputs)
}

fn read_expected_csv(path: &Path) -> Result<Vec<[String; 19]>, String> {
    let file = File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header_line = lines
        .next()
        .ok_or_else(|| format!("{}: missing CSV header", path.display()))?
        .map_err(|e| format!("{}: failed reading header: {e}", path.display()))?;
    let header = parse_csv_line(&header_line)?;
    let offset = match header.len() {
        19 => {
            validate_header_slice(&header, &OUTPUT_HEADER, &path.display().to_string())?;
            0
        }
        47 => {
            validate_header_slice(&header[..28], &INPUT_HEADER, &path.display().to_string())?;
            validate_header_slice(&header[28..], &OUTPUT_HEADER, &path.display().to_string())?;
            28
        }
        n => {
            return Err(format!(
                "{}: expected 19 output or 47 combined columns, got {n}",
                path.display()
            ));
        }
    };
    let expected_cols = header.len();

    let mut rows = Vec::new();
    for (idx, line_res) in lines.enumerate() {
        let row_num = idx + 1;
        let line = line_res
            .map_err(|e| format!("{}: failed reading row {row_num}: {e}", path.display()))?;
        if line.is_empty() {
            continue;
        }
        let cols = parse_csv_line(&line)?;
        if cols.len() != expected_cols {
            return Err(format!(
                "{} row {row_num}: expected {expected_cols} columns, got {}",
                path.display(),
                cols.len()
            ));
        }
        let row: [String; 19] = std::array::from_fn(|i| cols[offset + i].clone());
        rows.push(row);
    }

    Ok(rows)
}

fn run_engine_on_inputs(inputs: &[SiteInput], strict: bool) -> Vec<SiteOutput> {
    let n = inputs.len();
    let mut lat = Vec::with_capacity(n);
    let mut lon = Vec::with_capacity(n);
    let mut awc = Vec::with_capacity(n);
    let mut elev = Vec::with_capacity(n);
    let mut temp: [Vec<f64>; 12] = std::array::from_fn(|_| Vec::with_capacity(n));
    let mut precip: [Vec<f64>; 12] = std::array::from_fn(|_| Vec::with_capacity(n));

    for site in inputs {
        lat.push(site.lat_dd);
        lon.push(site.lon_dd);
        awc.push(site.awc_mm);
        elev.push(site.elev_m);
        for m in 0..12 {
            temp[m].push(site.temp_c[m]);
            precip[m].push(site.precip_mm[m]);
        }
    }

    let batch = SoaBatchInput {
        lat_dd: &lat,
        lon_dd: &lon,
        temp_c: std::array::from_fn(|m| temp[m].as_slice()),
        precip_mm: std::array::from_fn(|m| precip[m].as_slice()),
        awc_mm: &awc,
        elev_m: &elev,
    };
    let cfg = SiteConfig::default();

    if strict {
        StrictNewhall::default().run_batch_soa(&batch, &cfg)
    } else {
        ClassicNewhall::default().run_batch_soa(&batch, &cfg)
    }
}

fn write_csv_quoted<W: Write>(w: &mut W, s: &str) -> io::Result<()> {
    w.write_all(b"\"")?;
    for byte in s.as_bytes() {
        if *byte == b'"' {
            w.write_all(b"\"\"")?;
        } else {
            w.write_all(&[*byte])?;
        }
    }
    w.write_all(b"\"")
}

fn format_f64_csv(val: f64) -> String {
    if val.is_nan() {
        "NA".to_string()
    } else {
        format!("{val}")
    }
}

fn write_output_csv<W: Write>(
    inputs: &[SiteInput],
    outputs: &[SiteOutput],
    w: &mut W,
) -> io::Result<()> {
    for (idx, col) in OUTPUT_HEADER.iter().enumerate() {
        if idx > 0 {
            w.write_all(b",")?;
        }
        write_csv_quoted(w, col)?;
    }
    w.write_all(b"\n")?;

    for (input, out) in inputs.iter().zip(outputs.iter()) {
        if input.awc_mm.is_nan() {
            w.write_all(b"NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA\n")?;
            continue;
        }
        write!(
            w,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},",
            format_f64_csv(out.annual_rainfall),
            format_f64_csv(out.water_holding_capacity),
            format_f64_csv(out.annual_water_balance),
            format_f64_csv(out.annual_potential_evapotranspiration),
            format_f64_csv(out.summer_water_balance),
            out.dry_days_after_summer_solstice,
            out.moist_days_after_winter_solstice,
            out.num_cumulative_days_dry,
            out.num_cumulative_days_moist_dry,
            out.num_cumulative_days_moist,
            out.num_cumulative_days_dry_over_5c,
            out.num_cumulative_days_moist_dry_over_5c,
            out.num_cumulative_days_moist_over_5c,
            out.num_consecutive_days_moist_in_some_parts,
            out.num_consecutive_days_moist_in_some_parts_over_8c,
        )?;
        write_csv_quoted(w, out.temperature_regime_java_str())?;
        w.write_all(b",")?;
        write_csv_quoted(w, out.moisture_regime_java_str())?;
        w.write_all(b",")?;
        write_csv_quoted(w, out.regime_subdivision1_java_str())?;
        w.write_all(b",")?;
        write_csv_quoted(w, out.regime_subdivision2_java_str())?;
        w.write_all(b"\n")?;
    }
    w.flush()
}

fn check_float_close(
    row_num: usize,
    field: &str,
    actual: f64,
    expected_str: &str,
) -> Result<(), String> {
    let expected: f64 = expected_str
        .parse()
        .map_err(|e| format!("row {row_num} {field}: invalid f64 '{expected_str}': {e}"))?;
    let diff = (actual - expected).abs();
    if !actual.is_finite() || diff > FLOAT_TOL {
        return Err(format!(
            "row {row_num} {field}: actual={actual} expected={expected} diff={diff} > {FLOAT_TOL}"
        ));
    }
    Ok(())
}

fn check_i32_exact(
    row_num: usize,
    field: &str,
    actual: i32,
    expected_str: &str,
) -> Result<(), String> {
    let expected: i32 = expected_str
        .parse()
        .map_err(|e| format!("row {row_num} {field}: invalid i32 '{expected_str}': {e}"))?;
    if actual != expected {
        return Err(format!(
            "row {row_num} {field}: actual={actual} expected={expected}"
        ));
    }
    Ok(())
}

fn check_str_exact(
    row_num: usize,
    field: &str,
    actual: &str,
    expected: &str,
) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "row {row_num} {field}: actual='{actual}' expected='{expected}'"
        ));
    }
    Ok(())
}

fn verify_outputs(
    inputs: &[SiteInput],
    outputs: &[SiteOutput],
    expected_rows: &[[String; 19]],
) -> Result<(), String> {
    if inputs.len() != expected_rows.len() {
        return Err(format!(
            "row count mismatch: input has {} rows, expected has {} rows",
            inputs.len(),
            expected_rows.len()
        ));
    }

    for (idx, ((input, actual), exp)) in inputs
        .iter()
        .zip(outputs.iter())
        .zip(expected_rows.iter())
        .enumerate()
    {
        let r = idx + 1;
        if input.awc_mm.is_nan() {
            for (col_idx, val) in exp.iter().enumerate() {
                if val != "NA" {
                    return Err(format!(
                        "row {r} {}: expected 'NA' when awc == NA, got '{val}'",
                        OUTPUT_HEADER[col_idx]
                    ));
                }
            }
            if !actual.water_holding_capacity.is_nan() {
                return Err(format!(
                    "row {r} waterHoldingCapacity: expected NaN when awc is NaN, got {}",
                    actual.water_holding_capacity
                ));
            }
            continue;
        }

        check_float_close(r, OUTPUT_HEADER[0], actual.annual_rainfall, &exp[0])?;
        check_float_close(r, OUTPUT_HEADER[1], actual.water_holding_capacity, &exp[1])?;
        check_float_close(r, OUTPUT_HEADER[2], actual.annual_water_balance, &exp[2])?;
        check_float_close(
            r,
            OUTPUT_HEADER[3],
            actual.annual_potential_evapotranspiration,
            &exp[3],
        )?;
        check_float_close(r, OUTPUT_HEADER[4], actual.summer_water_balance, &exp[4])?;

        check_i32_exact(
            r,
            OUTPUT_HEADER[5],
            actual.dry_days_after_summer_solstice,
            &exp[5],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[6],
            actual.moist_days_after_winter_solstice,
            &exp[6],
        )?;
        check_i32_exact(r, OUTPUT_HEADER[7], actual.num_cumulative_days_dry, &exp[7])?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[8],
            actual.num_cumulative_days_moist_dry,
            &exp[8],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[9],
            actual.num_cumulative_days_moist,
            &exp[9],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[10],
            actual.num_cumulative_days_dry_over_5c,
            &exp[10],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[11],
            actual.num_cumulative_days_moist_dry_over_5c,
            &exp[11],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[12],
            actual.num_cumulative_days_moist_over_5c,
            &exp[12],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[13],
            actual.num_consecutive_days_moist_in_some_parts,
            &exp[13],
        )?;
        check_i32_exact(
            r,
            OUTPUT_HEADER[14],
            actual.num_consecutive_days_moist_in_some_parts_over_8c,
            &exp[14],
        )?;

        check_str_exact(
            r,
            OUTPUT_HEADER[15],
            actual.temperature_regime_java_str(),
            &exp[15],
        )?;
        check_str_exact(
            r,
            OUTPUT_HEADER[16],
            actual.moisture_regime_java_str(),
            &exp[16],
        )?;
        check_str_exact(
            r,
            OUTPUT_HEADER[17],
            actual.regime_subdivision1_java_str(),
            &exp[17],
        )?;
        check_str_exact(
            r,
            OUTPUT_HEADER[18],
            actual.regime_subdivision2_java_str(),
            &exp[18],
        )?;
    }

    Ok(())
}

fn cmd_run(input_path: &Path, output_path: Option<&Path>, strict: bool) -> Result<(), String> {
    let inputs = read_input_csv(input_path)?;
    let outputs = run_engine_on_inputs(&inputs, strict);

    match output_path {
        Some(path) => {
            let file = File::create(path)
                .map_err(|e| format!("failed to create {}: {e}", path.display()))?;
            let mut writer = BufWriter::new(file);
            write_output_csv(&inputs, &outputs, &mut writer)
                .map_err(|e| format!("failed writing {}: {e}", path.display()))
        }
        None => {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            write_output_csv(&inputs, &outputs, &mut writer)
                .map_err(|e| format!("failed writing stdout: {e}"))
        }
    }
}

fn cmd_verify(input_path: &Path, expected_path: &Path) -> Result<(), String> {
    let inputs = read_input_csv(input_path)?;
    let expected = read_expected_csv(expected_path)?;
    let outputs = run_engine_on_inputs(&inputs, false);
    verify_outputs(&inputs, &outputs, &expected)?;
    println!(
        "verify: PASS ({} rows matched within <= {FLOAT_TOL:.0e}, 100% categorical parity)",
        inputs.len()
    );
    Ok(())
}

struct BatchBuffers {
    lat: Vec<f64>,
    lon: Vec<f64>,
    temp: [Vec<f64>; 12],
    precip: [Vec<f64>; 12],
    awc: Vec<f64>,
    elev: Vec<f64>,
}

fn build_synthetic_batch(n: usize) -> BatchBuffers {
    let base_temp = [
        -2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3,
    ];
    let mut lat = Vec::with_capacity(n);
    let mut lon = Vec::with_capacity(n);
    let mut awc = Vec::with_capacity(n);
    let mut elev = Vec::with_capacity(n);
    let mut temp: [Vec<f64>; 12] = std::array::from_fn(|_| Vec::with_capacity(n));
    let mut precip: [Vec<f64>; 12] = std::array::from_fn(|_| Vec::with_capacity(n));

    for i in 0..n {
        let shift = ((i % 20) as f64 - 10.0) * 0.5;
        lat.push(40.8);
        lon.push(-77.8);
        awc.push(200.0);
        elev.push(350.0);
        for m in 0..12 {
            temp[m].push(base_temp[m] + shift);
            precip[m].push(85.0);
        }
    }
    BatchBuffers {
        lat,
        lon,
        temp,
        precip,
        awc,
        elev,
    }
}

fn run_bench(n: usize) {
    let b = build_synthetic_batch(n);
    let batch = SoaBatchInput {
        lat_dd: &b.lat,
        lon_dd: &b.lon,
        temp_c: std::array::from_fn(|m| b.temp[m].as_slice()),
        precip_mm: std::array::from_fn(|m| b.precip[m].as_slice()),
        awc_mm: &b.awc,
        elev_m: &b.elev,
    };
    let cfg = SiteConfig::default();

    let report = |label: &str, elapsed_s: f64| {
        let rate = (n as f64) / elapsed_s;
        let us_per_site = (elapsed_s * 1e6) / (n as f64);
        println!("{label:<32} : {n} sites in {elapsed_s:.4} s ({rate:>10.0} sims/sec, {us_per_site:>7.2} us/site)");
    };

    // 1. Classic Newhall (rayon SoA)
    let t0 = Instant::now();
    let out_classic = ClassicNewhall::default().run_batch_soa(&batch, &cfg);
    black_box(&out_classic);
    report("ClassicNewhall (rayon)", t0.elapsed().as_secs_f64());

    // 2. WaterTableHydrology + ClassicThermal (rayon SoA)
    let wt_engine = SimEngine::new(
        ClassicThermal,
        ThornthwaiteForcing,
        WaterTableHydrology::new(WaterTableConfig::default()),
    );
    let t1 = Instant::now();
    let out_wt = wt_engine.run_batch_soa(&batch, &cfg);
    black_box(&out_wt);
    report("WaterTable + ClassicThermal", t1.elapsed().as_secs_f64());

    // 3. Thermal1DModel + ClassicHydrology (rayon SoA)
    let t1d_engine = SimEngine::new(
        Thermal1DModel::new(Thermal1DConfig::default()),
        ThornthwaiteForcing,
        ClassicHydrology,
    );
    let t2 = Instant::now();
    let out_t1d = t1d_engine.run_batch_soa(&batch, &cfg);
    black_box(&out_t1d);
    report("Thermal1D + ClassicHydro", t2.elapsed().as_secs_f64());

    // 4. Full Hydrothermal (rayon SoA)
    let p4_engine = SimEngine::new(
        Thermal1DModel::new(Thermal1DConfig::default()),
        ThornthwaiteForcing,
        WaterTableHydrology::new(WaterTableConfig::default()),
    );
    let t3 = Instant::now();
    let out_p4 = p4_engine.run_batch_soa(&batch, &cfg);
    black_box(&out_p4);
    report("Full Hydrothermal (rayon)", t3.elapsed().as_secs_f64());

    // 5. Single-thread per-stage breakdown on 1,000 sites
    let k = n.min(1000);
    let site0 = SiteInput {
        lat_dd: 40.8,
        lon_dd: -77.8,
        temp_c: [
            -2.9, -1.7, 3.2, 9.4, 15.1, 19.9, 22.1, 21.3, 17.2, 10.9, 5.2, -0.3,
        ],
        precip_mm: [85.0; 12],
        awc_mm: 200.0,
        elev_m: 350.0,
    };
    let wb = ThornthwaiteForcing.compute_water_balance(&site0, &cfg);
    let soil = ClassicHydrology.simulate_hydrology(&site0, &cfg, &wb);
    let t1d = Thermal1DModel::new(Thermal1DConfig::default());
    let wt_h = WaterTableHydrology::new(WaterTableConfig::default());

    let tf_start = Instant::now();
    for _ in 0..k {
        black_box(ThornthwaiteForcing.compute_water_balance(&site0, &cfg));
    }
    let tf_ns = tf_start.elapsed().as_nanos() as f64 / k as f64;

    let wt_start = Instant::now();
    for _ in 0..k {
        black_box(wt_h.simulate_hydrology(&site0, &cfg, &wb));
    }
    let wt_ns = wt_start.elapsed().as_nanos() as f64 / k as f64;

    let t1d_start = Instant::now();
    for _ in 0..k {
        black_box(t1d.evaluate(&site0, &cfg, &wb, &soil));
    }
    let t1d_ns = t1d_start.elapsed().as_nanos() as f64 / k as f64;

    println!("\nSingle-thread stage breakdown ({k} sites):");
    println!("  ThornthwaiteForcing : {tf_ns:>10.1} ns/site");
    println!("  WaterTableHydrology : {wt_ns:>10.1} ns/site");
    println!("  Thermal1DModel      : {t1d_ns:>10.1} ns/site");
}

fn dispatch(cmd: Command) -> Result<(), String> {
    match cmd {
        Command::Help => {
            println!("{}", usage());
            Ok(())
        }
        Command::Run {
            input,
            output,
            strict,
        } => cmd_run(&input, output.as_deref(), strict),
        Command::Verify { input, expected } => cmd_verify(&input, &expected),
        Command::Bench { n } => {
            run_bench(n);
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match parse_args(env::args().skip(1)).and_then(dispatch) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_csv_path(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "newhall_cli_test_{tag}_{}_{nanos}.csv",
            std::process::id()
        ))
    }

    fn sample_nh_site() -> SiteInput {
        SiteInput {
            lat_dd: 40.65,
            lon_dd: -75.45,
            temp_c: [
                -2.72, -1.17, 3.78, 9.44, 15.33, 20.28, 22.94, 21.78, 17.44, 11.11, 5.56, 0.0,
            ],
            precip_mm: [
                88.90, 69.85, 90.42, 88.65, 113.54, 101.35, 108.46, 110.49, 111.0, 84.58, 93.98,
                86.11,
            ],
            awc_mm: 200.0,
            elev_m: 119.0,
        }
    }

    fn sample_sh_site() -> SiteInput {
        SiteInput {
            lat_dd: -33.8688,
            lon_dd: 151.2093,
            temp_c: [
                22.1, 22.0, 20.9, 18.2, 15.2, 12.8, 12.0, 13.1, 15.3, 17.6, 19.4, 21.1,
            ],
            precip_mm: [
                102.0, 118.0, 130.0, 127.0, 120.0, 132.0, 97.0, 81.0, 69.0, 77.0, 84.0, 78.0,
            ],
            awc_mm: 200.0,
            elev_m: 19.0,
        }
    }

    fn write_28col_input_csv(path: &Path, sites: &[SiteInput]) {
        let mut out = String::new();
        for (i, col) in INPUT_HEADER.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(col);
            out.push('"');
        }
        out.push('\n');
        for s in sites {
            out.push_str(&format!("{},{}", s.lat_dd, s.lon_dd));
            for t in s.temp_c {
                out.push(',');
                out.push_str(&format_f64_csv(t));
            }
            for p in s.precip_mm {
                out.push(',');
                out.push_str(&format_f64_csv(p));
            }
            out.push(',');
            out.push_str(&format_f64_csv(s.awc_mm));
            out.push(',');
            out.push_str(&format_f64_csv(s.elev_m));
            out.push('\n');
        }
        std::fs::write(path, out).expect("write temp input csv");
    }

    #[test]
    fn parse_args_supports_run_verify_bench_and_errors() {
        let args = |parts: &[&str]| parts.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();

        assert_eq!(
            parse_args(args(&["run", "in.csv"])).unwrap(),
            Command::Run {
                input: PathBuf::from("in.csv"),
                output: None,
                strict: false,
            }
        );
        assert_eq!(
            parse_args(args(&["run", "in.csv", "--output", "out.csv", "--strict"])).unwrap(),
            Command::Run {
                input: PathBuf::from("in.csv"),
                output: Some(PathBuf::from("out.csv")),
                strict: true,
            }
        );
        assert_eq!(
            parse_args(args(&["run", "--strict", "-o", "out.csv", "in.csv"])).unwrap(),
            Command::Run {
                input: PathBuf::from("in.csv"),
                output: Some(PathBuf::from("out.csv")),
                strict: true,
            }
        );
        assert_eq!(
            parse_args(args(&["verify", "in.csv", "exp.csv"])).unwrap(),
            Command::Verify {
                input: PathBuf::from("in.csv"),
                expected: PathBuf::from("exp.csv"),
            }
        );
        assert_eq!(
            parse_args(args(&["bench"])).unwrap(),
            Command::Bench { n: 100_000 }
        );
        assert_eq!(
            parse_args(args(&["bench", "2500"])).unwrap(),
            Command::Bench { n: 2500 }
        );
        assert_eq!(parse_args(args(&["--help"])).unwrap(), Command::Help);

        assert!(parse_args(Vec::<String>::new()).is_err());
        assert!(parse_args(args(&["unknown"])).is_err());
        assert!(parse_args(args(&["run"])).is_err());
        assert!(parse_args(args(&["run", "in.csv", "--output"])).is_err());
        assert!(parse_args(args(&["run", "in.csv", "--bogus"])).is_err());
        assert!(parse_args(args(&["verify", "in.csv"])).is_err());
        assert!(parse_args(args(&["bench", "0"])).is_err());
        assert!(parse_args(args(&["bench", "abc"])).is_err());
    }

    #[test]
    fn csv_parser_handles_quotes_escapes_spaces_and_na() {
        let cols = parse_csv_line(r#"40.65,-75.45,"Mesic","Perudic"," ","a""b",NA"#).unwrap();
        assert_eq!(
            cols,
            vec!["40.65", "-75.45", "Mesic", "Perudic", " ", r#"a"b"#, "NA"]
        );
        assert!(parse_csv_line(r#""unterminated,field"#).is_err());
    }

    #[test]
    fn run_formats_19_columns_including_perudic_and_na_and_roundtrips_verify() {
        let mut perudic = sample_nh_site();
        perudic.precip_mm = [250.0; 12];
        let mut na_awc = sample_nh_site();
        na_awc.awc_mm = f64::NAN;
        let sites = vec![sample_nh_site(), perudic, na_awc];

        let in_path = temp_csv_path("roundtrip_in");
        let out_path = temp_csv_path("roundtrip_out");
        write_28col_input_csv(&in_path, &sites);

        cmd_run(&in_path, Some(&out_path), false).expect("cmd_run classic");

        let expected = read_expected_csv(&out_path).expect("read 19-col output");
        assert_eq!(expected.len(), 3);
        assert_eq!(expected[1][16], "Perudic");
        assert_eq!(expected[1][17], " ");
        assert_eq!(expected[1][18], " ");
        assert!(expected[2].iter().all(|v| v == "NA"));

        cmd_verify(&in_path, &out_path).expect("verify should pass on run output");

        let _ = std::fs::remove_file(&in_path);
        let _ = std::fs::remove_file(&out_path);
    }

    #[test]
    fn strict_flag_uses_strict_newhall_engine() {
        let sh = sample_sh_site();
        let classic = run_engine_on_inputs(&[sh], false);
        let strict = run_engine_on_inputs(&[sh], true);

        let cfg = SiteConfig::default();
        assert_eq!(classic[0], ClassicNewhall::default().run(&sh, &cfg));
        assert_eq!(strict[0], StrictNewhall::default().run(&sh, &cfg));
        assert_ne!(
            classic[0].annual_potential_evapotranspiration,
            strict[0].annual_potential_evapotranspiration
        );
    }

    #[test]
    fn verify_detects_float_int_categorical_na_and_row_count_mismatches() {
        let sites = vec![sample_nh_site()];
        let outputs = run_engine_on_inputs(&sites, false);

        let mut buf = Vec::new();
        write_output_csv(&sites, &outputs, &mut buf).unwrap();
        let csv_str = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = csv_str.lines().collect();
        let base_row: [String; 19] = parse_csv_line(lines[1]).unwrap().try_into().unwrap();

        assert!(verify_outputs(&sites, &outputs, std::slice::from_ref(&base_row)).is_ok());

        // 1. Float tolerance breach (> 1e-9)
        let mut float_bad = base_row.clone();
        let orig_rain: f64 = float_bad[0].parse().unwrap();
        float_bad[0] = format!("{}", orig_rain + 1e-6);
        let err = verify_outputs(&sites, &outputs, &[float_bad]).unwrap_err();
        assert!(err.contains("annualRainfall"), "unexpected err: {err}");

        // 2. Integer day counter mismatch
        let mut int_bad = base_row.clone();
        int_bad[9] = format!("{}", outputs[0].num_cumulative_days_moist + 1);
        let err = verify_outputs(&sites, &outputs, &[int_bad]).unwrap_err();
        assert!(
            err.contains("numCumulativeDaysMoist"),
            "unexpected err: {err}"
        );

        // 3. Categorical mismatch
        let mut cat_bad = base_row.clone();
        cat_bad[15] = "Thermic".to_string();
        let err = verify_outputs(&sites, &outputs, &[cat_bad]).unwrap_err();
        assert!(err.contains("temperatureRegime"), "unexpected err: {err}");

        // 4. Row count mismatch
        let err = verify_outputs(&sites, &outputs, &[base_row.clone(), base_row]).unwrap_err();
        assert!(err.contains("row count mismatch"), "unexpected err: {err}");
    }

    #[test]
    fn verify_supports_47_column_pa_batch_fixture() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../newhall-core/tests/fixtures/pa_batch_metric.csv");
        cmd_verify(&fixture, &fixture).expect("pa_batch_metric.csv should verify against itself");
    }
}
