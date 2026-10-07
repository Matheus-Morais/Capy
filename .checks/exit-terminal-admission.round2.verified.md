# Terminal admission and exit, round 2: PASS

The input-first test now kills the lock-release mutant that survived round 1. All scoped admission criteria have exact evidence. This report verifies the terminal admission task group only; C9 and the complete multifunction feature remain open.

- Date: 2026-10-07.
- Independent verifier: same independent reviewer as round 1, not the implementation author.
- Reviewed range: `0632052..5243622`, HEAD `52436229ee326410f93802e63ff9fd0a02b8e43b`.
- Re-verification delta: `df5da08..5243622`, one additional Rust test; production guard unchanged.
- Authority: `.design/multifunction.md:20`, `.design/multifunction.md:54`, `.checks/multifunction.md:92`.
- Initial FAIL report remains preserved: `.checks/exit-terminal-admission.verified.md`.

## Spec-anchored outcomes

| Criterion | Exact evidence and assertion | Result |
| --- | --- | --- |
| Input invalidates a pending approval before writing | `src-tauri/src/exit_review.rs:45` clears pending before line 46 invokes send. Lines 136–137 assert `sent == vec!["own prompt"]` and `approve(first_nonce).is_err()`. | PASS |
| Partial-write failure still invalidates approval | `src-tauri/src/exit_review.rs:139` callback appends `"partial input"` then returns `Err("Write failed")`; line 140 asserts both sends remain; line 141 asserts approval of second nonce fails. Actual terminal `write_all`/`flush` can fail after writing, `src-tauri/src/terminal.rs:61`; invalidation precedes invocation. | PASS |
| Successful final approval prevents callback execution | `src-tauri/src/exit_review.rs:44` rejects closing state; lines 143–145 approve, reject the callback containing `"must not send"`, and assert unchanged vector `["own prompt", "partial input"]`. | PASS |
| Approval-first snapshot and input are serialized | `src-tauri/src/exit_review.rs:154` holds snapshot; line 165 asserts approval succeeds and input fails; line 166 asserts timeout during snapshot; line 167 asserts disconnected send channel without bytes. | PASS |
| Input-first write and review/approval are serialized | `src-tauri/src/exit_review.rs:176` starts input and blocks inside callback; line 180 waits for callback entry before launching review worker. Line 187 sends the acquired snapshot's observed write-finished flag. Line 194 asserts input and fresh approval both succeed; line 195 asserts snapshot timeout during writing; line 196 asserts snapshot observes finished writing and subsequent admission fails. | PASS |
| All three Tauri routes use admission gate | `src-tauri/src/main.rs:302`, `:308`, `:312` wrap terminal input, confirmed interrupt, model picker. `scripts/verify-pet-native.mjs:261` tests exactly those commands, line 267 requires nonexistent-terminal error, line 268 requires old approval rejection with `Solicite a saída novamente`, line 269 requires retained chat nonce and application. Previous supplied native report has all nine route checks true. | PASS for binding/error invalidation |

The scoped requirements define precise outcomes. No spec-precision gaps or uncovered scoped criteria remain. The new test closes the input-first coverage gap without changing production behavior or weakening previous assertions.

## Independent gates

- Real workspace: `rtk cargo test --manifest-path src-tauri/Cargo.toml exit_review` → **7 passed, 0 failed, 121 filtered out**, no ignored tests in this filter.
- Isolated unmutated worktree at `5243622`, same exit-review filter and shared debug target → **7 passed, 0 failed, 121 filtered out**.
- Exit-review test counts: four before original guard change, six in round 1, seven now. No tests removed or assertions weakened.
- Native report from original task remains reviewed evidence: `scratch/capy-visual-2388c2f5-a366-4131-aa25-0f25e637cfca/report.json`, **36 checks true**, `passed: true`, `error: null`. No UI/provider rerun in this verification.

## Discrimination sensor

Both mutations targeted the actual production source in isolated detached worktree `capy-exit-sensor-round2-126c875b-ff47-406f-a63f-f90644d8fc63`, using full-project `rtk proxy cargo test --manifest-path <scratch>/src-tauri/Cargo.toml exit_review`. The source was restored in that worktree between mutants. No stubs or copied implementation were used.

| Production mutation | Exact failure | Result |
| --- | --- | --- |
| Insert `drop(state);` before `send()`, production `src-tauri/src/exit_review.rs:46` | **6 passed, 1 failed**, nonzero tool exit 1. New `exit_review_waits_for_in_progress_terminal_write_before_new_review_and_approval` failed: `assertion failed: matches!(during_write,Err(std::sync::mpsc::RecvTimeoutError::Timeout))`, scratch line 196, corresponding to real line 195. | KILLED |
| Remove `state.pending=None` before `send()`, production `src-tauri/src/exit_review.rs:45` | **6 passed, 1 failed**, nonzero tool exit 1. `exit_review_terminal_input_invalidates_approval_even_after_partial_write_failure` failed: `service.approve(&first.nonce, true, 1, || Ok(active.clone())).is_err()`, scratch line 136, corresponding to real line 137. | KILLED |

Sensor: **2 injected, 2 killed, 0 survived**. The previously surviving mutation now fails at the spec-defined no-overlap assertion.

Isolation was confirmed after worktree cleanup and before creating this report. Real porcelain before and after matched exactly:

```text
?? .checks/interventions-codex.round2.verified.md
?? .checks/interventions-codex.round3.verified.md
?? .checks/interventions-codex.verified.md
?? releases/0.4.0-alpha.1/Capy.exe.sha256
```

Cleanup verified the resolved absolute target remained under the Windows temp directory with the sensor prefix. No real production/test edits, stash, commits, release builds, app shutdown, UI runs, or provider calls occurred. Only the shared debug build cache was changed outside the scratch source.

## Quality, remaining scope, and gaps

**Ranked scoped gaps: none.** Production changes remain surgical and reuse the existing mutex. The input-first test maps directly to the existing serialization criterion. It proves the write callback finishes before the fresh snapshot and approval, complementing the approval-first test.

Route evidence remains limited to binding and conservative nonexistent-terminal failure. This verification does not claim additional real terminal/provider/account sampling, autonomous activity observation at final approval, or whole-feature closure. It does not run whole-feature state validation or mark C9 complete.

The previous grounded signal was already recorded by the orchestrator as candidate lesson L-001. No new grounded failure or lesson was produced in this clean round.
