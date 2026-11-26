# Worker Prompt Template 9 (`planning/PROMPT_TEMPLATE_9.md`)

Orchestration 9. Fill the slots from the task spec in `EXECUTION_PLAN_9.md`. Skeleton follows wave-orchestration.

```markdown
You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim from planning/EXECUTION_PLAN_9.md}}

## Repository conventions
- Repo: /home/andrew/workspace/newhall-rs
- Read first: AGENTS.md (quick start, workflows, infrastructure).
- Rust: `cargo test --workspace`; R-facing: testthat suite. Do not touch `crates/` or `r-pkg/src/`.
- R roxygen outputs (`NAMESPACE`, `man/*.Rd`) are regenerated with `R -q -e 'roxygen2::roxygenise("r-pkg")'`. Never hand-edit them.
- Scratch and temp files go in `.development/` (gitignored).
- Commit style: follow git-workflow-standards at ~/.claude/skills/git-workflow-standards/SKILL.md (Conventional Commits, one sentence, no phase/wave numbers). Planning-only changes use `docs(planning):`. Do not add attribution trailers.

## Ground rules (non-negotiable)
- NEVER make implementation changes or run state-changing commands before your plan is approved. Creating `planning/tasks/{{id}}.todo.md` is allowed before approval.
- After approval, a focused local commit is part of executing the approved task. This does not authorize pushes, tags, pull requests, comments, or any remote action.
- NEVER touch files outside "Files in scope"; if required, STOP and report.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still ambiguous becomes numbered questions (max 5) in your plan report.
- If a test fails twice for the same reason, stop and report. Do not thrash.
- Do not create or push git tags.

## [Worker] Analysis
Read: AGENTS.md; the named plan section(s) in planning/EXECUTION_PLAN_9.md; planning/STATE_9.md reports of your dependency tasks; the grep anchors; the named reference implementation.

## [Worker] Micro-planning
Write planning/tasks/{{id}}.todo.md:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps incl. explicit test steps
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match surrounding code style. Run only the targeted tests named in the spec. Commit each verified logical unit locally. Keep planning artifact updates out of code commits. Report each code commit ID.

## [Worker] Reporting
Append to planning/STATE_9.md under `## {{id}}` (<=15 lines): status; files changed with line ranges; code commit IDs; exact test commands with pass/fail counts; deviations from the approved plan (or "none"); gotchas for dependent tasks.

## Slot values (per task)

| id | title | acceptance command |
| --- | --- | --- |
| 9.1.1 | G24: newhall_batch_phase4.character S3 parity | `R -q -e 'testthat::test_file("r-pkg/tests/testthat/test-s3-api.R")'` |
| 9.1.2 | G28: release workflow | python3 YAML check (see EXECUTION_PLAN_9.md 9.1.2) |
| 9.2.1 | G22: R vignettes | `R CMD build r-pkg` and tarball grep (see EXECUTION_PLAN_9.md 9.2.1) |
```
