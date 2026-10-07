# Live terminal controls — independent verification

**Verdict: PASS for the sampled live terminal controls.** No product defect found in this scope.

**Profile:** tlc-implement light; no fault injection. **Range:** `8a8d6f0..a4b31891fcb15421ec57fc929e18a0f6d24877b2`. **Round:** 1, scoped additional verification. **Verifier:** independent sub-agent, author != verifier. This report does not conclude C9, C1–C19, or the full goal.

Opened `.checks/multifunction.md` C9 (including the documented output-token observation adaptation), `.design/multifunction.md` controls requirement, the requested diff, `scripts/verify-live-task-controls.mjs`, and relevant terminal/exit implementation. The source requires official available controls, exact session identity and confirmation; the sampled UI offers Esc interruption rather than Windows process suspension.

## Independent commands

- `node scripts/verify-pet-native.mjs --live-task-controls`: exit 0, all 42 individually recorded checks passed. Own artifacts: `scratch/capy-visual-8144af89-73df-4ae6-9ee0-9177598ff915/report.json`.
- `rtk npm test`: exit 0, 31 named tests passed, zero skipped/failed. Relevant named tests include `exit confirmation names exact resources and distinguishes uncertain consumption`, `Escape stays with the terminal, edited form or dialog, and only closes an unhandled panel`, and `completion requires explicit real event and never infers from stop or disappearance`.

## Located proof and observed values

| Claim sampled | Located assertion / condition | Independent evidence |
|---|---|---|
| Real output generation in the own exact turn | `scripts/verify-live-task-controls.mjs:26–34`: positive count; `tokens<=first.tokens` rejects non-increase; `user&&await sameProcess()` gates receipt | Own UUID `f10b41f3-9d81-4b06-879a-5857ad7762c0`, exact scratch project and prompt in user JSONL. Interrupt turn increased 317→376 output tokens over 1010 ms; exit turn 283→338 over 1008 ms. Receipt files retain both screen samples. Official Claude PID 27856, creation `2026-10-07T12:19:31.4090176Z`, `claude.exe` path. |
| Missing confirmation rejected; actual WebView confirm names UUID and folder | `scripts/verify-live-task-controls.mjs:40,43`: rejected `confirmed:false`; `cancel.type==='confirm'&&cancel.message.includes(task.id)&&cancel.message.includes(task.cwd)` | Both runtime checks passed. CDP captures actual `Page.javascriptDialogOpening`, then declines it; these are not mocked confirm return values. |
| Cancelling interruption preserves process and ongoing generation | `scripts/verify-live-task-controls.mjs:45,49`: `await sameProcess()` and `outputTokens(afterCancel)>interruptionReceipt.last.tokens` | Both passed during the first enumerated own response. |
| Approved fresh interruption uses official UI and preserves partial response | `scripts/verify-live-task-controls.mjs:52–59`: fresh confirm names same ID; wait requires `/Interrupted|interrompid/i.test(text)&&!active(text)`; same process survives; partial marker numbers increase | `interrupt-result.txt` contains own marker lines 47–61 followed by `Interrupted · What should Claude do instead?`. Screenshot opened and visually inspected; UI states interruption requested by official Esc. |
| Cancelling exit preserves exact process and ongoing generation | `scripts/verify-live-task-controls.mjs:64,66,70`: real confirm same UUID; `await sameProcess()`; `outputTokens(beforeApproval)>exitReceipt.last.tokens` | All passed; `exit-before-approval.txt` has second own prompt and 350 output tokens, exceeding receipt's 338. |
| Fresh approved exit identifies only the own terminal and closes its process/application | `scripts/verify-pet-native.mjs:379,382,384,388`: one exact resource/folder/account; confirm names UUID; `childExited`; `ownClosed` | `task-exit.json` records same original PID/creation/path, remaining process null, `ownClosed:true`, `appClosed:true`. |
| No tools in sampled turns | `scripts/verify-live-task-controls.mjs:72`: `!history.rows.some(...part.type==='tool_use')` | Passed on own session history. Short initial response additionally pins UUID/prompt/folder and Haiku at `scripts/verify-pet-native.mjs:175`. |

Opened `interrupt-active.png`, `interrupt-result.png` and `exit-active.png`: terminal identity visibly names the same project/account/subscription/UUID; official positive output meters and interruption text agree with receipts. The initial terminal geometry check passed without manual scrolling, covering the moved `scrollIntoView` and CSS margin in the diff.

## Sampling and level limitations

- Three own turns in one Claude Haiku subscription session on this Windows setup: initial short task, interrupted enumerated turn, and enumerated exit turn. This is integration evidence across actual Tauri commands, WebView dialogs, PTY and provider, not proof for every provider/model/account or external CLI.
- Output activity is sampled with the documented positive official `↓` meter, not inferred from PID survival or silence. The terminal suppresses streamed text until interruption; own partial marker text was verified after interruption.
- Generation is sampled after cancelled exit and immediately before preparing/approving the next exit; the runner does not persist a second meter observation at the exact instant of final dialog acceptance. Its observed 350-token turn remains far from the requested 1000-line completion, and no intervening send is issued. This is a timing sampling limit, not an atomic generation assertion.
- Actual dialog values are runtime assertions in this run; dialog payloads are not separately persisted. Screenshots cover generation/interruption, not the confirm overlays.
- Multi-terminal switching/concurrency, stale IDs, replay/expiry branches, pause where supported, external launch, effective model change, quotas, API and full UI arrangement are outside this scoped report. Existing Swept authorization/state requirements were reread in `src/tasks-ui.ts`, `src-tauri/src/main.rs`, `terminal.rs` and `exit_review.rs`; this run does not exhaust those constraints.
- Light profile: Coverage/Test policy joins and mutation faults were not performed. Build/Rust/full smoke results reported by the author are not represented here as independently rerun proofs.

Native test session closed and native surface released. Product code unchanged by verifier.
