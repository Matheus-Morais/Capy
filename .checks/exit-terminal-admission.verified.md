# Terminal admission and exit: FAIL

The implementation holds the required lock, but the tests do not prove serialization when terminal input starts first. A production mutation releasing the guard before the callback survived the complete exit-review quick gate. This scoped verification does not mark C9 or the multifunction feature complete.

- Date: 2026-10-07.
- Independent verifier: fresh agent, author differs from verifier.
- Reviewed commit range: `0632052..df5da08`, HEAD `df5da083577a9ae4d73c7f05c8926fbfe5cbeb80`.
- Authority: `.design/multifunction.md:20` (official pause/stop controls, exact session, confirmation), `.design/multifunction.md:54` (revalidation), and `.checks/multifunction.md:92` (terminal admission outcomes).
- Changed production surface: `src-tauri/src/exit_review.rs`, `src-tauri/src/main.rs`. Test/evidence changes: `scripts/verify-pet-native.mjs`, `.checks/multifunction.md`, `.specs/STATE.md`.

## Criteria and evidence

| Criterion | Exact expected outcome and evidence | Result |
| --- | --- | --- |
| Input invalidates pending approval before writing | `src-tauri/src/exit_review.rs:45` clears `pending` before `send()` at line 46. Test line 136 asserts `sent == vec!["own prompt"]`; line 137 asserts approval of the prior nonce returns an error. | PASS |
| Partial failure also invalidates approval | `src-tauri/src/exit_review.rs:139` invokes a callback that appends `"partial input"` before returning `Err("Write failed")`. Line 140 asserts both observed sends remain; line 141 asserts the prior approval fails. The guard invalidates before the callback, independently of its return value. Actual terminal writes use `write_all` and `flush`, `src-tauri/src/terminal.rs:61`, so conservative invalidation applies to partial-write failures. | PASS |
| After final approval input callbacks never execute | `src-tauri/src/exit_review.rs:44` rejects closing state before callback execution. Lines 143–145 approve, attempt `"must not send"`, assert rejection, and assert the sent vector remains exactly `["own prompt", "partial input"]`. | PASS |
| Input and final snapshot are serialized | Approval-first coverage: `src-tauri/src/exit_review.rs:154` holds the snapshot until released; lines 165–167 assert successful approval, failed input, timeout while snapshot is held, and disconnected send channel without a sent message. Production line 43 acquires the same mutex used by prepare/approve and retains its guard through line 46. No assertion starts and holds the input callback first while a fresh review and approval attempt run. Releasing the input guard before line 46 survives all existing tests. | FAIL: discrimination gap |
| Three Tauri input routes use the guard | `src-tauri/src/main.rs:302`, `:308`, `:312` wrap input, confirmed interrupt, and model picker in `exit.send_terminal_input`. Native runner `scripts/verify-pet-native.mjs:261` loops over exactly those three routes; line 267 requires `Terminal não encontrado`, line 268 requires the old approval fail with `Solicite a saída novamente`, and line 269 requires unchanged chat nonce and a living application. Supplied native report records these nine checks as true. | PASS for route binding/error invalidation |

The outcomes are precise in the scoped authority. There is no spec-precision gap. Three criteria pass fully, one route criterion passes within its stated binding/error scope, and one serialization criterion lacks discriminating input-first coverage.

## Gates and test integrity

- Independently executed `rtk cargo test --manifest-path src-tauri/Cargo.toml exit_review`: **6 passed, 0 failed, 121 filtered out**, no ignored tests in this filter.
- Unmutated detached worktree at `df5da08`, same command with its manifest and shared debug target directory: **6 passed, 0 failed, 121 filtered out**.
- Diff adds two Rust tests and strengthens the new concurrency test. No old tests or assertions were removed or weakened. Exit-review count before this range: four; after: six.
- Reviewed the author's `scratch/capy-visual-2388c2f5-a366-4131-aa25-0f25e637cfca/report.json`: `passed: true`, all **36** recorded checks true, `error: null`. This verifier inspected the runner assertions and supplied report; it did not rerun the UI.
- Author-reported full suite **125 passed / 2 optional live proofs ignored**, strengthened concurrency rerun, and desktop build are contextual evidence, not independently rerun gates here.

## Discrimination sensor

| Mutation | Execution target | Outcome |
| --- | --- | --- |
| Insert `drop(state);` immediately before `send()` in `send_terminal_input`, releasing the approval mutex during writing | Actual production `src-tauri/src/exit_review.rs:46` in detached temp worktree `capy-exit-sensor-b654aab8-ae4e-443e-b171-4e977cfb01bd` at `df5da08`; full project cargo quick gate, no model stubs or rewritten harness | **SURVIVED: 6 passed, 0 failed, 121 filtered out** |

Sensor depth: one targeted behavior mutation. The mutant preserves the current approval-first test because input still waits to acquire the mutex. It breaks input-first serialization: after invalidation and guard release, a new prepare/approve can run while the admitted callback is still writing. The existing sequential partial-failure test also does not observe this overlap.

Isolation: real porcelain captured before sensor and checked after deleting the temp worktree, before report creation. Both outputs matched exactly:

```text
?? .checks/interventions-codex.round2.verified.md
?? .checks/interventions-codex.round3.verified.md
?? .checks/interventions-codex.verified.md
?? releases/0.4.0-alpha.1/Capy.exe.sha256
```

Only isolated source was mutated. No stash, release rebuild, app shutdown, provider calls, or UI runs occurred. Shared debug target cache was used. Cleanup checked that the resolved absolute path stayed in the Windows temp directory with the sensor prefix before removing the detached worktree.

## Ranked gap and required proof

1. **Major verification gap: input-first serialization is not discriminating**, `src-tauri/src/exit_review.rs:148`. Add a test that starts terminal input and blocks inside its callback, then starts fresh prepare/final approval. Assert neither snapshot nor approval completes until the callback finishes. Release input, require the snapshot observe completed writing, and require fresh approval succeed. The test must fail when `drop(state)` is inserted before `send()`. Re-run the quick gate and isolated sensor.

The implementation itself has no observed lock-order defect: the current guard remains in scope through the callback. The gap concerns regression detection and prevents this verifier from issuing PASS.

## Quality and scope limits

The change is surgical and reuses the existing exit mutex. The three routes preserve their existing bytes and interrupt confirmation requirement. The two new Rust tests map directly to the scoped criteria. No unrelated production behavior was added. RTK instructions and the skill's coding principles were followed.

Route evidence covers conservative rejection of nonexistent terminals and invalidation. It does not prove real terminal writes, post-approval route execution in the running application, other provider/account controls, or autonomous provider activity observed atomically with approval. Prior live proofs remain outside this commit-range review. Whole-feature closure/state validation and marking C9 complete remain outside this report.

Grounded lesson: concurrency criteria need both operation orderings; holding only the approval side can miss early release of the input guard. No lesson artifact was written because this delegated review authorizes only this report and prohibits fixes; the orchestrator must record the lesson alongside the fix cycle.
