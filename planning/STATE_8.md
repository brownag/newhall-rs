# Orchestration 8 State (`planning/STATE_8.md`)

Plan: `planning/EXECUTION_PLAN_8.md`
Last Updated: 2026-11-24

## Baseline Checklist (Pre-Wave 1)

- [x] Working directory confirmed: `/home/andrew/workspace/newhall-rs`
- [x] Rust baseline green from `STATE_7.md` (`76` workspace PASS, `70` `--no-default-features` PASS, clippy/fmt/rustdoc clean)
- [x] R baseline green from `STATE_7.md` (`217` testthat PASS; `R CMD check --as-cran` had 0 ERROR, 1 WARNING on compiled code, 1 NOTE on `jNSMR` Suggests + 404 URL)

## Wave Board

| Wave | Task ID | Title | Status | Worker Model | Commit |
| --- | --- | --- | --- | --- | --- |
| 1 | `8.1.1` | Remove `jNSMR` from `DESCRIPTION`, gate `test-jnsmr-parity.R`, clean `ci.yml` (`G29`) | done | flash | `e5d2342` |
| 1 | `8.1.2` | Eliminate `libnewhallrs.a` compiled-code WARNING symbols via LTO/DCE (`G30`) | done (partial) | flash | `691205e` |
| 2 | `8.2.1` | Full `R CMD check --as-cran`, `testthat`, `sync-core.sh --check`, and Rust gate | done | orchestrator | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->

### 8.1.1
- **status:** done | **commit:** `e5d2342`
- **files:** `r-pkg/DESCRIPTION`, `r-pkg/tests/testthat/test-jnsmr-parity.R`, `.github/workflows/ci.yml`
- **verification:** `devtools::test('r-pkg')` PASS (`217` PASS, `0` FAIL, `0` WARN, `0` SKIP); `yaml.safe_load` PASS.

### 8.1.2
- **status:** done (partial DCE) | **commit:** `691205e`
- **files:** `r-pkg/src/rust/Cargo.toml` (`[profile.release]` `lto = true`, `codegen-units = 1`)
- **verification:** `R_NamespaceRegistry`, `_exit`, and `exit` eliminated from `libnewhallrs.a`; only `U abort` survives via Rust `std::sys::pal::unix::abort_internal`.

### 8.2.1
- **status:** done | **commit:** none (verification only)
- **verification:**
  - `sync-core.sh --check`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace --all-targets` (`76` PASS), `cargo test -p newhall-core --no-default-features` (`70` PASS), `devtools::test('r-pkg')` (`217` PASS, `0` FAIL, `0` WARN, `0` SKIP): all PASS.
  - `R CMD check --as-cran --no-manual`: exit 0 (`0` ERROR, `1` WARNING, `1` NOTE).
    - `Suggests` `jNSMR` NOTE: **eliminated**; `unstated dependencies in 'tests'`: **OK**; `installed package size`: **OK** (down from 6.8 MB).
    - `R_NamespaceRegistry`, `_exit`, `exit`: **eliminated**; only `Found 'abort'` from Rust `std::sys::pal::unix::abort_internal` remains.
    - Remaining NOTE: `New submission` + 404 on GitHub URL (pre-push).
