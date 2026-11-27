# newhall-rs Agent Guide

## Quick Start

- `cargo build --workspace` — build the Rust workspace.
- `cargo test --workspace` — run Rust unit and integration tests.
- `cargo test -p newhall-core` — test the simulation core.
- `cargo run -p newhall-cli` — run the CLI (see `crates/newhall-cli/src/main.rs` for arguments).
- `cargo fmt --all -- --check` — check Rust formatting.
- `R CMD INSTALL r-pkg` — build and install the R package; requires R and Cargo.
- `R -q -e 'testthat::test_dir("r-pkg/tests/testthat")'` — run R tests; requires testthat and relevant optional packages.

## Find Things

- `crates/newhall-core/src/` — pure-Rust physics, simulation engine, and public types.
- `crates/newhall-core/tests/` — parity, strict-mode, and hydrothermal validation tests; `fixtures/` contains reference data.
- `crates/newhall-cli/src/` — native CLI and benchmark harness.
- `r-pkg/R/` — R API, raster methods, categories, and generated extendr wrappers.
- `r-pkg/src/rust/src/` — Rust-to-R bridge; `r-pkg/src/rust/newhall-core/` is the vendored core copy.
- `r-pkg/tests/testthat/` — R package tests.
- `r-pkg/tools/sync-core.sh` — synchronize the core source into the standalone R crate.
- `scripts/` — R benchmark and fixture-generation helpers.
- `planning/` — specifications, audits, plans, and execution records.
- `ARCHITECTURE.md` — detailed architecture and extension guide.

## Workflows

- Keep `crates/newhall-core` independent of R and extendr; it is the reusable physics library.
- After changing the core, run `r-pkg/tools/sync-core.sh` to update the vendored copy before validating the R package.
- Use `cargo test --workspace` for Rust changes and the testthat suite for R-facing changes.
- Golden parity and hydrothermal invariants are specified in `ARCHITECTURE.md` and the relevant files under `planning/`.

## Infrastructure

- Root `Cargo.toml` defines the Rust workspace (`newhall-core`, `newhall-cli`) and excludes `r-pkg/src/rust`, which is a standalone Cargo crate.
- `newhall-core` enables its optional Rayon `parallel` feature by default; `cargo test -p newhall-core --no-default-features` checks the sequential configuration.
- R package metadata and build configuration live in `r-pkg/DESCRIPTION`, `r-pkg/src/Makevars`, and `r-pkg/src/rust/Cargo.toml`.

## References

- See `ARCHITECTURE.md` for module responsibilities, physics pipeline, and extension points.
- See `planning/PHASE4_PHYSICS_SPEC.md` and `planning/ADVERSARIAL_VALIDATION.md` for hydrothermal behavior and validation requirements.
- See `planning/AUDIT_JNSM.md` for classic jNSMR parity context.
