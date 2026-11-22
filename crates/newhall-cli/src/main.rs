use std::hint::black_box;
use std::time::Instant;

use newhall_core::{
    ClassicHydrology, ClassicNewhall, ClassicThermal, ForcingModel, HydrologyStep, SimEngine,
    SiteConfig, SiteInput, SoaBatchInput, Thermal1DConfig, Thermal1DModel, ThermalModel,
    ThornthwaiteForcing, WaterTableConfig, WaterTableHydrology,
};

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

    // 4. Full Phase 4 (rayon SoA)
    let p4_engine = SimEngine::new(
        Thermal1DModel::new(Thermal1DConfig::default()),
        ThornthwaiteForcing,
        WaterTableHydrology::new(WaterTableConfig::default()),
    );
    let t3 = Instant::now();
    let out_p4 = p4_engine.run_batch_soa(&batch, &cfg);
    black_box(&out_p4);
    report("Full Phase 4 (rayon)", t3.elapsed().as_secs_f64());

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

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("bench") => {
            let n = args
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10_000);
            run_bench(n);
        }
        _ => {
            let _default_cfg = SiteConfig::default();
            println!("newhall-cli (use `newhall-cli bench [n_sites]` to run benchmarks)");
        }
    }
}
