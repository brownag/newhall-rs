# newhall-rs

`newhall-rs` is a Rust implementation and extension of the Newhall Simulation Model, with an R package interface. It provides the classic model for compatibility with jNSMR and additional strict-mode and Phase 4 physics options.

## Choose an interface

- **Rust library:** `newhall-core` provides the simulation engine and physics models.
- **Command line:** `newhall-cli` provides a native runner and benchmark harness.
- **R package:** `r-pkg/` contains `newhallrs`, including batch and raster-oriented APIs.

The default Rust engine, `ClassicNewhall`, targets output parity with `newhall-1.6.5.jar`. The core crate does not depend on R or extendr, so it can be used independently. The R package contains a synchronized copy of the core for standalone package builds.

## Rust example

```rust
use newhall_core::{simulate_site, SiteConfig, SiteInput};

fn main() {
    let input = SiteInput {
        lat_dd: 40.0,
        lon_dd: -75.0,
        temp_c: [0.0, 1.0, 5.0, 11.0, 17.0, 22.0, 25.0, 24.0, 20.0, 14.0, 8.0, 2.0],
        precip_mm: [80.0; 12],
        awc_mm: 200.0,
        elev_m: 100.0,
    };
    let result = simulate_site(&input, &SiteConfig::default());
    println!("{result:?}");
}
```

Add `newhall-core` to a Rust project's dependencies with `cargo add newhall-core --path crates/newhall-core` when working from this repository.

See the [`newhall-core` public API](crates/newhall-core/src/lib.rs) and [architecture guide](ARCHITECTURE.md) for input types, model choices, and batch execution.

## R interface

Install the package from the repository with:

```sh
R CMD INSTALL r-pkg
```

The main R API is implemented in [`r-pkg/R/newhall.R`](r-pkg/R/newhall.R); raster methods are in [`r-pkg/R/raster.R`](r-pkg/R/raster.R). Package dependencies and optional integrations are listed in [`r-pkg/DESCRIPTION`](r-pkg/DESCRIPTION).

## Validation and reference material

Parity fixtures and Rust integration tests live in `crates/newhall-core/tests/`. R tests are in `r-pkg/tests/testthat/`. The [architecture guide](ARCHITECTURE.md) explains the components and their relationships; model specifications and parity audit are maintained under [`planning/`](planning/).

For repository-specific agent guidance, see [`AGENTS.md`](AGENTS.md). Contribution workflow is documented in `CONTRIBUTING.md` when provided by the project.
