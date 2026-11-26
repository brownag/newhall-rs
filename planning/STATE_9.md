# Orchestration 9 State (`planning/STATE_9.md`)

Plan: `planning/EXECUTION_PLAN_9.md`. Prompt: `planning/PROMPT_TEMPLATE_9.md`.
Goals: G24, G28, G22 (from `planning/GOALS.md`).

## Baseline checklist (complete before wave 1)

- [x] `git status` clean on the working branch; record branch name: `feat/phase4-csv-release-vignettes` (created from `main` at `392a715`)
- [x] `cargo test --workspace` green (record pass count): 76 passed, 0 failed
- [x] `cargo test -p newhall-core --no-default-features` green: 70 passed, 0 failed
- [x] `bash r-pkg/tools/sync-core.sh --check` green
- [x] `R -q -e 'testthat::test_dir("r-pkg/tests/testthat")'` green (record PASS/FAIL/SKIP counts): FAIL 0 / WARN 0 / SKIP 3 / PASS 132. Note: needs `R CMD INSTALL r-pkg` first and `library(newhallrs);` before `test_dir`/`test_file`, or tests fail with "could not find function".
- [x] Known pre-existing: `R CMD check --as-cran` WARNING (G30, out of scope). Record as the baseline to keep unchanged. Baseline `R CMD check --no-manual` on `3bd23bd`: Status 1 WARNING (`checking compiled code`), no NOTEs or ERRORs.

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 9.1.1 G24 character method | done | `2373dfa` |
| 1 | 9.1.2 G28 release workflow | done | `12b583c` |
| 2 | 9.2.1 G22 vignettes | pending (depends on 9.1.1, 9.1.2; scope widened by D6) | — |

Gate after wave 1: serial R test run. Gate after wave 2: `R CMD build` + `R CMD check` serial run, then the final verification list in EXECUTION_PLAN_9.md.

## Task reports

(Workers append `## {{id}}` sections here.)

## 9.1.2

- Status: done (haiku worker, approved plan, no revisions).
- Files: `.github/workflows/release.yml` (new, 1-138). Jobs `version-check`, `cli-binaries` (ubuntu/macos/windows matrix), `r-source-tarball`, `release` (`softprops/action-gh-release@v2`, `contents: write`).
- Commit: `12b583c` (cherry-picked from worktree `c371ac3`).
- Tests: spec YAML check `ok`. Version-check script extracted from the YAML: `v0.1.0` exit 0, `v0.2.0` exit 1 (reproduced by the orchestrator).
- Deviations: none. The `<target>` labels are `linux-x86_64`, `macos-arm64`, `windows-x86_64` (worker's choice; macos-latest runners are arm64).
- Gotchas: runtime is unverified until the first real tag push (no actionlint). `r-source-tarball` lacks pandoc/knitr/rmarkdown and will fail once vignettes land. Fixed by D6 in 9.2.1.

## 9.1.1

- Status: done (sonnet worker). One FAILED VERIFICATION retry: the first commit left SpatRaster out of the roxygen description and had a line over 80 chars. The amend fixed both.
- Files: `r-pkg/R/newhall.R` (description 177-181; new `newhall_batch_phase4.character` at end of file), `r-pkg/NAMESPACE` (+`S3method(newhall_batch_phase4,character)`), `r-pkg/man/newhall_batch_phase4.Rd` (regen), `r-pkg/tests/testthat/test-s3-api.R` (new test after the `newhall_batch.character` test, ~253-281).
- Commit: `2373dfa` (cherry-picked from worktree `a139255`).
- Tests: `R CMD INSTALL r-pkg && R -q -e 'library(newhallrs); testthat::test_file("r-pkg/tests/testthat/test-s3-api.R")'` FAIL 0 / PASS 114.
- Deviations: none after the retry.
- Gotchas for 9.2.1: CSVs must carry the Phase 4 columns (`zWt`, `oHorizonThickness`, `snowJan`..`snowDec`). Optional columns default as in the data.frame method. Error messages match `newhall_batch.character`.

## Wave 1 gate

- `R CMD INSTALL r-pkg && R -q -e 'library(newhallrs); testthat::test_dir("r-pkg/tests/testthat")'`: FAIL 0 / WARN 0 / SKIP 3 / PASS 141 (baseline 132, +9 new expectations). Wave 1 done.
