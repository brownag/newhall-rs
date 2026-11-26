# Orchestration 9 State (`planning/STATE_9.md`)

Plan: `planning/EXECUTION_PLAN_9.md`. Prompt: `planning/PROMPT_TEMPLATE_9.md`.
Goals: G24, G28, G22 (from `planning/GOALS.md`).

## Baseline checklist (complete before wave 1)

- [x] `git status` clean on the working branch; record branch name: `feat/phase4-csv-release-vignettes` (created from `main` at `392a715`)
- [x] `cargo test --workspace` green (record pass count): 76 passed, 0 failed
- [x] `cargo test -p newhall-core --no-default-features` green: 70 passed, 0 failed
- [x] `bash r-pkg/tools/sync-core.sh --check` green
- [x] `R -q -e 'testthat::test_dir("r-pkg/tests/testthat")'` green (record PASS/FAIL/SKIP counts): FAIL 0 / WARN 0 / SKIP 3 / PASS 132. Note: needs `R CMD INSTALL r-pkg` first and `library(newhallrs);` before `test_dir`/`test_file`, or tests fail with "could not find function".
- [ ] Known pre-existing: `R CMD check --as-cran` WARNING (G30, out of scope). Record as the baseline to keep unchanged.

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 9.1.1 G24 character method | pending | — |
| 1 | 9.1.2 G28 release workflow | pending | — |
| 2 | 9.2.1 G22 vignettes | pending (depends on 9.1.1) | — |

Gate after wave 1: serial R test run. Gate after wave 2: `R CMD build` + `R CMD check` serial run, then the final verification list in EXECUTION_PLAN_9.md.

## Task reports

(Workers append `## {{id}}` sections here.)
