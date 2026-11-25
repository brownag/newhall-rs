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
| 1 | `8.1.1` | Remove `jNSMR` from `DESCRIPTION`, gate `test-jnsmr-parity.R`, clean `ci.yml` (`G29`) | pending | flash | — |
| 1 | `8.1.2` | Remediate/document `libnewhallrs.a` compiled-code symbols (`G30`) | pending | flash | — |
| 2 | `8.2.1` | Full `R CMD check --as-cran`, `testthat`, `sync-core.sh --check`, and Rust gate | pending | flash | — |

## Task Reports

<!-- Workers append <=15-line reports below under ## <Task ID> -->
