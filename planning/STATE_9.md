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
| 2 | 9.2.1 G22 vignettes | done | `e3b499d`, `dc6d122`, `0d5e588` |

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

## 9.2.1

- Status: done (sonnet worker). The orchestrator saved the micro-TODO from the plan report, because the worker didn't write the file. One FAILED VERIFICATION retry: the vignette gave `zWt` in cm (it is m) and described `strict = TRUE` backwards. Fixed in a follow-up commit, which also corrected snow columns from "need" to "can also carry" (they are all-or-none, default 0).
- Files: `r-pkg/DESCRIPTION` (+knitr, rmarkdown in Suggests; `VignetteBuilder: knitr`; Version unchanged), `r-pkg/vignettes/newhall-workflows.Rmd` (new, 5 sections, terra chunks guarded per D3), `.github/workflows/ci.yml` (`r-check`: setup-pandoc, knitr/rmarkdown), `.github/workflows/release.yml` (`r-source-tarball`: setup-pandoc, knitr/rmarkdown install).
- Commits: `e3b499d` (vignette + DESCRIPTION), `dc6d122` (CI deps, D6), `0d5e588` (vignette text corrections).
- Tests: `R CMD build` output includes `vignettes/newhall-workflows.Rmd` and `inst/doc/newhall-workflows.html`. Rendered with terra. The no-terra render was simulated on a scratch copy with the terra chunks set to `eval = FALSE` (terra shares the user library). YAML parse ok.
- Deviations: none beyond the retry.

## Final verification (orchestrator, serial, on `0d5e588`)

1. `cargo test --workspace`: 76 passed, 0 failed (= baseline).
2. `bash r-pkg/tools/sync-core.sh --check`: OK.
3. R suite: FAIL 0 / WARN 0 / SKIP 3 / PASS 141.
4. `R CMD build r-pkg && R CMD check --no-manual`: Status 1 WARNING (`checking compiled code`, G30), identical to the baseline. No NOTEs, no ERRORs. The vignette builds and is in the tarball.
5. YAML check on `release.yml` (and `ci.yml` parse): ok.

Orchestration 9 complete. Not verified locally: `release.yml` and the CI changes run only on GitHub (first `v*` tag push / next CI run).
