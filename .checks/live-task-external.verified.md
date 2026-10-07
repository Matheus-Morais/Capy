# Live external task verification

**Verdict: PASS for the scoped Claude external launcher proof.**
**Profile:** light; no faults injected. No Coverage/Test policy sections declared.
**Range:** `d401de9` plus reviewed working-tree external launcher and proof changes.
**Round:** 1, additional scoped verification; independent sub-agent, author != verifier.

Opened `.checks/multifunction.md` C17 and external launcher Landing paragraphs, `.design/multifunction.md` approved external-default behavior, `src-tauri/scripts/claude-terminal.ps1`, `src-tauri/src/profiles.rs` external launch path, `scripts/verify-live-task-external.mjs`, shared runner changes and `tests/external-launcher.test.mjs`. Scope is one Claude subscription profile in an own scratch project, not the whole goal.

## Independent proof executions

- `node scripts/verify-pet-native.mjs --live-task-external`: exit 0; 25 named checks passed. Artifacts: `scratch/capy-visual-152d63a8-be2c-48e4-bea7-72492a1cdbbd/report.json`, `external-task-receipt.json`, `external-processes.json`, `external-task-panel.png`.
- `rtk npm test`: exit 0; all 32 named tests passed, none skipped. `external launcher preserves UTF-8 literal arguments folder and isolated authentication environment` ran on Windows and passed.
- Separate preceding integrated model runner completed before external runner started; no simultaneous native instances.

| Claim | Located assertion/evidence | Result |
| --- | --- | --- |
| External is default form mode | `scripts/verify-live-task-external.mjs:51`: assertion `mode==='external'` before changing mode; native check passed | PASS |
| Pin existing subscription and exact folder/account/model/instruction | `scripts/verify-live-task-external.mjs:50,56`: identity subscription and complete task field equality; task UUID `921aa63e-6c3a-4d7d-82ac-faacac22775d`, Haiku, own-external-project | PASS |
| Actual provider receipt, no echo inference | `scripts/verify-live-task-external.mjs:60-65`: selects assistant row with exact sessionId, Haiku model and random prompt marker; user content/cwd asserted. Receipt is assistant `claude-haiku-4-5-20251001`, literal `PROBE_ed0ee397f48f48c7b327e62d10e852c8`, exact own UUID/cwd | PASS |
| No tools observed | `scripts/verify-live-task-external.mjs:66`: rejects any assistant tool_use in exact history | PASS |
| External process belongs to own launcher | `scripts/verify-live-task-external.mjs:14-23,57`: CLI command line contains exact UUID; parent is powershell.exe with this proof-root name and claude-terminal.ps1; one CLI plus launcher observed | PASS |
| No integrated terminal replay | `scripts/verify-live-task-external.mjs:67`: `terminal_replay` rejects for exact external task | PASS |
| Capy exit excludes and preserves external origin | `scripts/verify-live-task-external.mjs:70,73,75`: zero exit resources, app exits, then CLI/launcher match prior PID+started+path | PASS |
| UTF-8, exact literal arguments, cwd/profile and removed auth overrides | `tests/external-launcher.test.mjs:21`: `assert.deepEqual(result.args,args); assert.equal(result.cwd,cwd); assert.equal(result.config,config); assert.equal(result.removed,true)`; expected args include accents/Japanese/emoji, quotes, backslashes, shell metacharacters, empty string and trailing slash | PASS |

The launcher explicitly reads its JSON as UTF-8 and passes an escaped Windows argument string directly to ProcessStartInfo with UseShellExecute false and explicit working directory. The fixture checks actual child argv/environment, so shell interpretation or dropped quotes/empty arguments would fail the deepEqual assertion. The native proof uses the same packaged launcher through the form/backend boundary.

Cleanup revalidates PID/path/exact UTC creation ticks before stopping only the recorded CLI and launcher (`scripts/verify-live-task-external.mjs:33-40`). Runner exit was successful; a supplemental process query found no command line containing this own proof root after cleanup. No external terminal keystrokes, trust approvals, or permissions were automated. No product/proof edits by the verifier.

Independently inspected task panel screenshot: own-external-project card labels `modelo inicial: haiku`, account/subscription, CLI externo and exact UUID; no integrated-terminal control is offered on that card. Screenshot is evidence of the Capy card, not an assertion about native external terminal visual layout.

Limits: no foreign-process fixture was spawned for this run; preservation is supported by scoped ownership/revalidation logic and the positive exact-origin survival checks, not a broad process-system audit. Resuming external conversations, other accounts/providers, unavailable CLI, quota transitions, API and the complete C17/multifunction goal are not proven here. Native app and own external processes are closed; ready for the next proof.
