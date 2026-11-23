# Worker Prompt Template (Orchestration 3)

Use this template for every worker spawned in Orchestration 3 (`EXECUTION_PLAN_3.md`).
Worker model: `flash`. Escalate to `pro`/frontier after 2 failed attempts per the escalation ladder. Max 3 concurrent workers per wave.

```markdown
You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code or files.

## Task {{id}}: {{title}}
{{task spec verbatim from planning/EXECUTION_PLAN_3.md}}

## Ground rules (non-negotiable)
- NEVER write implementation files or run state-changing commands before your plan is approved. (Creating `planning/tasks/{{id}}.todo.md` during Micro-planning is allowed.)
- After approval, commit your verified, reviewable implementation locally using a focused Conventional Commit (one sentence, no phase/wave/orchestration numbers); keep `planning/` updates out of code commits. This does not authorize pushes, pull requests, comments, or any remote/outward-facing actions.
- NEVER touch files outside "Files in scope"; if required, STOP and report.
- NEVER modify `ClassicNewhall`, `ThornthwaiteForcing`, `ClassicHydrology`, or `ClassicThermal` parity behavior.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still
  ambiguous becomes numbered questions (max 5) in your plan report.
- If a command or test fails twice for the same reason, stop and report -- do not thrash.

## [Worker] Analysis
Read:
1. `planning/EXECUTION_PLAN_3.md` (Goal & Scope, Recon & Hazard Notes, and your Task Spec)
2. `planning/STATE_3.md` reports of your dependency tasks
3. `planning/GOALS.md` (`## Source Defects`)
4. The grep anchors named in your task spec

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps (<=12 steps) incl. explicit test steps
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.
Wait for `APPROVED -- execute the plan as written.` before executing.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`.
Run only the targeted verification commands named in the spec.
Commit your verified code changes locally (excluding `planning/`) and record the commit ID.

## [Worker] Reporting
Append to `planning/STATE_3.md` under `## {{id}}` (<=15 lines):
- status (`done` / `blocked`)
- files created/changed with line counts
- code commit ID
- exact verification commands + pass/fail results
- deviations from the approved plan (or "none")
- gotchas for dependent tasks
```
