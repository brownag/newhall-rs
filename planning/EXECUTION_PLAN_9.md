# Execution Plan 9 (`planning/EXECUTION_PLAN_9.md`)

Orchestration 9. Source: `planning/GOALS.md` (magic-wand selection, 2026-11-26). Companion artifacts: `PROMPT_TEMPLATE_9.md`, `STATE_9.md`, `tasks/9.W.id.todo.md`.

## Goal & Scope

**Goal.** Complete three goals that are unblocked now because their dependencies are done:

- **G24** `newhall_batch_phase4.character` (CSV path method) reaches full S3 parity with `newhall_batch` (`default`, `data.frame`, `character`, `SpatRaster`), with testthat coverage.
- **G28** Automated release workflow (`.github/workflows/release.yml`) builds and attaches `newhall-cli` binaries and the `newhallrs` R source tarball on `v*` tags with synchronized versions.
- **G22** User-facing R vignette(s) in `r-pkg/vignettes/` demonstrate tabular, `terra::SpatRaster`, strict-mode, and Phase 4 workflows and build cleanly under `R CMD build`.

**In scope**

- `r-pkg/R/newhall.R`, `r-pkg/NAMESPACE`, `r-pkg/man/`, `r-pkg/tests/testthat/` (G24).
- `.github/workflows/release.yml` (G28).
- `r-pkg/vignettes/`, `r-pkg/DESCRIPTION` (VignetteBuilder, Suggests) (G22).
- Corrected dependency fields in `planning/GOALS.md` (planning-only commit, done before wave 1).

**Out of scope**

- Publishing `newhall-core` or `newhall-cli` to crates.io (decided in Q2; manual step).
- Changes to Rust physics or the extendr bridge (`r-pkg/src/rust/src/lib.rs`).
- G30 (CRAN compiled-code WARNING). Still an open goal, not part of this plan.
- G23, G26 (backlog), G27 (deferred).
- Vignette content that needs `terra` to run unconditionally (see decision D3).

**Verbatim constraints**

- From GOALS G24: "completes full S3 parity with `newhall_batch` (`default`, `data.frame`, `character`, `SpatRaster`), with testthat coverage."
- From GOALS G28: "builds and attaches `newhall-cli` binaries and the `newhallrs` R source tarball on `v*` tags with synchronized versions."
- From GOALS G22: "build cleanly under `R CMD build`."
- From AGENTS.md: "Keep `crates/newhall-core` independent of R and extendr." "Use `cargo test --workspace` for Rust changes and the testthat suite for R-facing changes."

## Recon log (facts)

- `r-pkg/R/newhall.R` is 369 lines. `newhall_batch.character` (line ~127) is the pattern to port. `newhall_batch_phase4` generic at line ~231. `newhall_batch_phase4.default` line ~300, `.data.frame` line ~311. Roxygen `@method` tags precede each method.
- `r-pkg/NAMESPACE` is roxygen-generated. It currently registers `S3method(newhall_batch_phase4, default | data.frame | SpatRaster)` but not `character`. Regenerate with `roxygen2` (installed); never hand-edit.
- `r-pkg/tests/testthat/test-s3-api.R` (14 KB). Existing pattern: `newhall_batch.character reads single and multiple CSV file paths` (line ~234). Phase 4 tests: `test-phase4.R`, and `newhall_batch_phase4 works on data.frame and SpatRaster...` (line ~357 of `test-s3-api.R`).
- `r-pkg/src/rust/src/lib.rs` (~327 `newhall_batch_phase4_rust`). Not edited by this plan.
- No `r-pkg/vignettes/` directory. `DESCRIPTION` has no `VignetteBuilder`. `knitr`, `rmarkdown`, `terra`, `testthat`, `roxygen2`, `devtools` are installed locally.
- `.github/workflows/ci.yml` exists: `rust-check`, `rust-cross` (macOS, Windows), `r-check` (Ubuntu, R CMD build/check, `devtools::test`). `r-check` installs terra system libs (libgdal etc.), so vignettes with terra chunks will run in CI.
- `Cargo.toml` versions for `newhall-core`, `newhall-cli`, and `r-pkg/src/rust` are all `0.1.0`. `DESCRIPTION` Version is `0.1.0`. `CHANGELOG.md` top entry is `[0.1.0]`.
- `r-pkg/.Rbuildignore` ignores `^src/rust/target$`, `^src/.cargo$`, `^src/.*\.o$`, `^src/.*\.so$`, `^src/.*\.dll$`, `^tools$`. A standalone `r-pkg/src/rust` Cargo crate exists; `r-pkg/tools/sync-core.sh --check` is in CI.
- `actionlint` is not installed locally. Python 3 with PyYAML is available, so workflow YAML can be syntax-checked.
- `git status`: branch `main`, clean, up to date with `origin/main` at `fa67719`.
- ADRs: `docs/decisions/` does not exist. No active ADRs to honor or supersede.
- Hazards: `r-pkg/R/newhall.R` and `raster.R` are shared by G24 and G22 docs; keep G22 out of `R/` and `man/`. `NAMESPACE` and `man/*.Rd` are roxygen outputs, so regenerate them in the task that changes roxygen, not by hand. `DESCRIPTION` changes in G22 must not touch the `Version` field (G28 owns version sync).

## Decision log

- **2026-11-26 D1 (planning tracking):** Track orchestration artifacts. Commit plan/state/task updates separately with `docs(planning):`. Never mix with code commits.
- **2026-11-26 D2 (G28 scope):** The release workflow builds `newhall-cli` binaries for Linux, macOS, and Windows, and the `newhallrs` R source tarball, attached to `v*` tags. It does **not** publish to crates.io. Publishing remains a manual step.
- **2026-11-26 D3 (G22 terra handling):** Vignette chunks that need `terra` are guarded with `eval = requireNamespace("terra", quietly = TRUE)`. Tabular and CSV chunks always run. The vignettes must build without terra installed.
- **2026-11-26 D4 (wave ordering):** G22 depends on G24, not only G20. The vignette demonstrates the CSV path workflow, which needs the `character` method. This corrects the stale `Depends on: G20` in GOALS.md. G22 also demonstrates `newhall_batch_phase4`, which already exists.
- **2026-11-26 D5 (version):** No version bumps in this plan. G28 verifies the existing `0.1.0` sync across `Cargo.toml` files and `DESCRIPTION`. A bump is a separate release decision.

## Wave table

| Wave | Task | Goal | Repo | Depends on | Files (must edit) |
| --- | --- | --- | --- | --- | --- |
| 1 | 9.1.1 | G24 | newhall-rs | — | `r-pkg/R/newhall.R`, `r-pkg/NAMESPACE` (regen), `r-pkg/man/newhall_batch_phase4.Rd` (regen), `r-pkg/tests/testthat/test-s3-api.R` |
| 1 | 9.1.2 | G28 | newhall-rs | — | `.github/workflows/release.yml` (new) |
| 2 | 9.2.1 | G22 | newhall-rs | 9.1.1 | `r-pkg/vignettes/` (new), `r-pkg/DESCRIPTION` (VignetteBuilder, Suggests), `r-pkg/.Rbuildignore` (only if needed) |

**Disjointness.** Wave 1 pair: 9.1.1 touches only R package files; 9.1.2 touches only a new workflow file. Disjoint. Wave 2 has one task, so no intra-wave conflict.

**Gate between waves.** After wave 1, run the R suite serially: `R -q -e 'testthat::test_dir("r-pkg/tests/testthat")'`. After wave 2, run `R CMD build r-pkg` and `R CMD check --no-manual newhallrs_*.tar.gz` serially.

## Task specs

### 9.1.1 — G24: `newhall_batch_phase4.character` S3 parity

- **Repo:** newhall-rs
- **Depends on:** none
- **Files in scope**
  - Must edit: `r-pkg/R/newhall.R`, `r-pkg/NAMESPACE` (regenerate with roxygen2 only), `r-pkg/man/newhall_batch_phase4.Rd` (regenerate only), `r-pkg/tests/testthat/test-s3-api.R`.
  - Can create: none. Scratch files go in `.development/` (gitignored).
  - Do not touch: `r-pkg/src/`, `crates/`, `r-pkg/R/raster.R`, `r-pkg/R/categories.R`, any `vignettes/` path.
- **Spec.** Add `newhall_batch_phase4.character(.data, ...)` mirroring `newhall_batch.character` at `newhall.R` ~line 127. It validates `.data` (non-empty, no NA or empty paths, files exist), reads CSVs with `utils::read.csv(stringsAsFactors = FALSE)`, row-binds multiple files, and delegates to `newhall_batch_phase4.data.frame(df, ...)`. Keep the same error messages and wording style as the `newhall_batch.character` method. Add `#' @rdname newhall_batch_phase4` and `#' @method newhall_batch_phase4 character` with `#' @export` so roxygen writes `S3method(newhall_batch_phase4,character)`. Regenerate `NAMESPACE` and `man/newhall_batch_phase4.Rd` with `roxygen2::roxygenise("r-pkg")`. Do not hand-edit those outputs. Update the `newhall_batch_phase4` roxygen description to list the character method alongside default, data.frame, and SpatRaster.
- **Tests.** In `test-s3-api.R`, add `test_that("newhall_batch_phase4.character reads single and multiple CSV file paths", ...)` modeled on the existing `newhall_batch.character` test at ~line 234. Write temp CSVs with the Phase 4 column schema, verify the output equals `newhall_batch_phase4.data.frame` on the same data (same names, same regime categories), and cover the error branches: empty input, NA path, missing file. Match the `expected_output_names(include_phase4 = TRUE)` helper.
- **Grep anchors:** `newhall_batch.character` (newhall.R ~127), `newhall_batch_phase4.data.frame` (newhall.R ~311), `newhall_batch_phase4.default` (~300), `newhall_batch.character reads single and multiple CSV file paths` (test-s3-api.R ~234), `expected_output_names` (test-s3-api.R ~53).
- **Reference implementation:** `newhall_batch.character` in `newhall.R`.
- **Acceptance:** `R -q -e 'roxygen2::roxygenise("r-pkg")'` produces no diff in unrelated Rd files and adds `S3method(newhall_batch_phase4,character)` to `NAMESPACE`. The new test passes. Command: `R -q -e 'testthat::test_file("r-pkg/tests/testthat/test-s3-api.R")'`.
- **Targeted test command:** `R -q -e 'testthat::test_file("r-pkg/tests/testthat/test-s3-api.R")'`
- **Constraints:** Do not change the behavior of existing methods. Do not touch Rust.

### 9.1.2 — G28: Automated release workflow

- **Repo:** newhall-rs
- **Depends on:** none
- **Files in scope**
  - Must edit: none.
  - Can create: `.github/workflows/release.yml`.
  - Do not touch: `ci.yml`, `Cargo.toml` files, `DESCRIPTION`, `CHANGELOG.md`, any `r-pkg/src` path.
- **Spec.** Create `.github/workflows/release.yml`, triggered on `push: tags: ['v*']`. Job 1 (`version-check`): verify that the tag version (strip `v`) equals the version in `crates/newhall-core/Cargo.toml`, `crates/newhall-cli/Cargo.toml`, `r-pkg/src/rust/Cargo.toml`, and `r-pkg/DESCRIPTION` `Version:`. Fail with a clear message on mismatch. Job 2 (`cli-binaries`, matrix over `ubuntu-latest`, `macos-latest`, `windows-latest`, needs `version-check`): `cargo build --release -p newhall-cli`, then package the binary with a platform-specific name (`newhall-cli-vX.Y.Z-<target>` plus `.tar.gz` on Unix, `.zip` on Windows), and upload as an artifact. Job 3 (`r-source-tarball`, `ubuntu-latest`, needs `version-check`): `R CMD build r-pkg` (uses the same `setup-r` and Rust steps as `ci.yml`), upload `newhallrs_*.tar.gz`. Job 4 (`release`, needs jobs 2 and 3, `permissions: contents: write`): download all artifacts and attach them to the GitHub release for the tag with `softprops/action-gh-release@v2` or `gh release`. Use the same action versions as `ci.yml` (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `r-lib/actions/setup-r@v2`). Do not publish to crates.io. Add `# Release process: see CHANGELOG.md` comment only if useful; no other docs.
- **Grep anchors:** `.github/workflows/ci.yml` (job names `rust-cross`, `r-check`, action versions), `version = "0.1.0"` in each Cargo.toml, `Version:` in `r-pkg/DESCRIPTION`.
- **Reference implementation:** `.github/workflows/ci.yml`.
- **Acceptance:** The YAML parses, and jobs, triggers, and `needs` edges match the spec. Command: `python3 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/release.yml')); assert set(d['jobs'])=={'version-check','cli-binaries','r-source-tarball','release'}; assert d['on']['push']['tags']==['v*']; print('ok')"`. Also run `bash -n` on any shell fragment if one is added. Local `actionlint` is not available, so workflow runtime is verified only by the first real tag push, which is a manual user step.
- **Targeted test command:** the Python YAML check above.
- **Constraints:** Do not run `git tag` or push tags. Do not publish anywhere.

### 9.2.1 — G22: R vignettes

- **Repo:** newhall-rs
- **Depends on:** 9.1.1 (the CSV-path vignette chunk calls `newhall_batch_phase4.character`).
- **Files in scope**
  - Must edit: `r-pkg/DESCRIPTION` (add `VignetteBuilder: knitr`, add `knitr` and `rmarkdown` to `Suggests`; do **not** change `Version:`).
  - Can create: `r-pkg/vignettes/newhall-workflows.Rmd`, plus any `r-pkg/vignettes/*.Rmd` needed. Optional `r-pkg/vignettes/.gitignore` only if build artifacts appear.
  - Can edit only if needed: `r-pkg/.Rbuildignore`.
  - Do not touch: `r-pkg/R/`, `r-pkg/NAMESPACE`, `r-pkg/man/`, `r-pkg/src/`, `.github/`, `CHANGELOG.md`.
- **Spec.** Write one vignette, `newhall-workflows.Rmd`, with `output: rmarkdown::html_vignette` and `%\VignetteIndexEntry{newhall workflows}`, `%\VignetteEngine{knitr::rmarkdown}`, and `%\VignetteEncoding{UTF-8}`. Sections: (1) tabular `newhall_batch.data.frame` workflow; (2) CSV path workflow via `newhall_batch.character` and `newhall_batch_phase4.character` (from 9.1.1); (3) strict mode (`strict = TRUE`) comparison with the classic run; (4) Phase 4 workflow with `newhall_batch_phase4.data.frame`; (5) `terra::SpatRaster` workflow with `newhall_batch.SpatRaster`/`newhall_batch_phase4.SpatRaster`. Chunks for section 5 use `eval = requireNamespace("terra", quietly = TRUE)` (decision D3). Tabular, CSV, and Phase 4 data.frame chunks always run. Keep chunks small so the vignette builds fast. Use only exported functions and documented arguments; check exact argument names against the roxygen in `r-pkg/R/newhall.R` and `r-pkg/R/raster.R`. Use `temp` CSVs created with `tempfile()` in the CSV section; do not depend on files outside the package.
- **Grep anchors:** `newhall_batch.data.frame` (newhall.R ~91), `newhall_batch.character` (~127), `newhall_batch_phase4.data.frame` (~311), `newhall_batch.SpatRaster` (raster.R), `newhall_batch_phase4.SpatRaster` (raster.R ~311), `strict` argument (newhall.R ~44 `newhall_batch_raw`).
- **Reference implementation:** the test fixtures in `r-pkg/tests/testthat/test-s3-api.R` and `test-phase4.R` show valid input frames and argument names.
- **Acceptance:** `R CMD build r-pkg` succeeds and produces the vignette in the tarball (`tar -tzf newhallrs_*.tar.gz | grep vignettes/newhall-workflows.Rmd`). The vignette builds with and without terra: `R -q -e 'rmarkdown::render("r-pkg/vignettes/newhall-workflows.Rmd", output_dir=tempdir())'` on this machine (terra installed) and a run with `R_LIBS` set to exclude terra is optional but recommended.
- **Targeted test command:** `R CMD build r-pkg && tar -tzf newhallrs_*.tar.gz | grep -q 'vignettes/newhall-workflows.Rmd' && echo vignette-in-tarball`
- **Constraints:** Do not add exports, change function behavior, or bump `Version:`. Vignette must not write outside `tempdir()`.

## Final verification (orchestrator, serial, after wave 2)

1. `cargo test --workspace` (confirms Rust untouched).
2. `bash r-pkg/tools/sync-core.sh --check`.
3. `R -q -e 'testthat::test_dir("r-pkg/tests/testthat")'`.
4. `R CMD build r-pkg && R CMD check --no-manual newhallrs_*.tar.gz` (note existing G30 WARNING is expected and out of scope; record it, do not fix it).
5. `python3` YAML check on `release.yml`.
