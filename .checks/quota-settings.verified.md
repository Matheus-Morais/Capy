# Quota settings preservation verification

**Verdict: PASS — scoped preservation slice only.**

**Profile:** light. **Diff:** `fb3b727..765501c` (HEAD `765501c57799d74a9437b203217543cf45e3f1ec`). **Round:** 1 — scoped. **Verifier:** independent sub-agent; author != verifier.

This verifies the 2026-10-07 statusline-preservation decision under C11 in `.checks/multifunction.md`, not C11 real quotas or completion of C1–C19. The decision and verification instructions were read. Diff and assertions were inspected at HEAD. No product fixes were made.

| Scoped claim | Proof and settling evidence | Result |
|---|---|---|
| Unreadable, invalid and oversized settings block connection before quota artifacts | `claude_quota_configuration_rejects_unreadable_invalid_and_oversized_settings` ran. `src-tauri/src/claude_quotas.rs:147`: `assert!(configure(&profile,true).is_err()); assert!(!root(&profile).exists());`; lines 149–152 exercise `not json` and 1,048,577 bytes, assert error, exact bytes retained, and absent artifacts. | PASS |
| Missing/corrupt/incomplete/wrong-folder/incompatible-original bridge does not remove active wrapper | `claude_quota_disconnect_preserves_active_wrapper_without_valid_bridge` ran. Lines 164–167 assert rejection and `assert_eq!(std::fs::read(&path).unwrap(),active)` for missing bridge and all four listed variants. Line 165 explicitly includes `configDir:"other-folder"` and `original:{type:"other"}`. Native lines 438–445 additionally assert rejection and byte preservation across Tauri for corrupt, incomplete and missing bridge. | PASS |
| Removed or externally replaced statusline is preserved instead of restoring old origin | `claude_quota_disconnect_does_not_restore_over_origin_change` ran. Lines 178–180 cover absent statusLine and `command:"echo changed"`, asserting error and exact unchanged bytes. Native lines 447–449 independently assert changed-origin rejection and exact bytes. | PASS |
| Normal connection retains unrelated fields and intact disconnection restores original | `claude_quota_configuration_preserves_and_restores_custom_statusline` ran. Lines 215–217 assert model `sonnet`, preserved hooks, disabled state, and full original JSON equality after repeated connection then disconnection. Native line 435 asserts model/hooks/Unicode future metadata retention; line 451 uses `isDeepStrictEqual` for full settings and backup original statusline. | PASS |

Code inspection: `configuration_bytes` at lines 29–38 treats only `NotFound` as absence, limits reading to 1 MiB plus one detection byte, and propagates other errors. `read_bridge` at lines 42–48 requires an explicit original field, valid shape and the same profile folder. `configure` validates settings and original statusline before writing artifacts (lines 53–64), requires the active wrapper before restoring (lines 71–74), and compares a fresh settings-byte read before writing (line 76).

Proofs rerun by this verifier:

- `rtk proxy cargo test --manifest-path src-tauri/Cargo.toml`: exit 0, **108 passed, 0 failed, 1 ignored**; all six named `claude_quota_` tests appeared as `ok`. The ignored test sends live Claude turns and was not enabled.
- `rtk proxy npm test`: exit 0, **32 passed, 0 failed**.
- `node scripts/verify-pet-native.mjs --quota-settings`: exit 0, **26/26 passed**, including 10 quota settings assertions. Receipt: `scratch/capy-visual-1423c08a-263f-4b68-8af9-3b3cca146a3e/report.json`.

`scratch/capy-visual-1423c08a-263f-4b68-8af9-3b3cca146a3e/quota-settings.png` was visually inspected. The native panel renders the own quota fixture account and chat controls without horizontal clipping. The screenshot captures chat state after command-level settings checks; it does not visually prove a quota toggle/error flow. The settings claims are settled by Tauri invocation and filesystem assertions above.

Sampling limits: unreadability is represented by a directory at `settings.json`, not an ACL denial. Wrong-folder bridge and removed origin are covered at Rust level; the native fixture samples changed origin and corrupt/incomplete/missing bridge. No injected concurrent edit or exact 1 MiB valid-JSON boundary test was run; the reread and size bound were inspected. Only fixture settings in scratch/temp directories were written; no login, provider request or user `.claude` configuration modification was performed. Real quotas, account identity/billing and the full feature remain unverified here.

Light profile: binding-source composition enumeration, Coverage/Test policy joins and fault injection are not required and were not performed. No product-preservation failure was found in this slice. Existing compiler warnings were emitted; the suite passed.

## Supplemental source-only Unicode wrapper verification

**Verdict: PASS — additional working-tree slice after `765501c`, not part of the native binary evidence above.** The additional Landing decision in `.checks/multifunction.md`, `src-tauri/scripts/claude-statusline.ps1` diff and `tests/statusline-wrapper.test.mjs` were inspected. The only wrapper change explicitly reads bridge JSON with `Get-Content -Encoding UTF8` (line 4).

`rtk proxy npm test` was rerun: exit 0, **33 passed, 0 failed, 0 skipped**. The new named test `statusline wrapper preserves Unicode in the existing command` ran and passed. Its line 19 assertion is `assert.equal(stdout,expected)` with line 14 literal `expected='ação_日本語_🦫'`. It copies the actual wrapper (line 13), supplies a UTF-8 JSON fixture containing the original command (line 15), executes Windows PowerShell, closes stdin with fixture JSON (line 17), and observes real subprocess output. This is an observable behavior requirement test, not an assertion mirroring the encoding implementation: deleting or changing the encoding is not what it asserts; corruption of the existing Unicode command changes the measured stdout.

The fixture points the collector executable at Node with unsupported collector flags and supplies `bash:null`, exercising the PowerShell original-command path while collector success is unavailable. The proof asserts preserved original stdout, not the collector exit status or successful quota collection. It uses an isolated temp directory and a literal own command, with no login/provider call or user configuration write. It does not sample the bash path, real collector output, Unicode input payload, or deployment of the updated embedded script. No native rerun or rebuild was performed for this supplement; the prior native receipt remains verified only at `765501c`. No fault injection was run under the light profile. Full feature scope remains open.

## Supplemental rebuilt native wrapper verification

**Verdict: PASS — rebuilt native settings preservation and embedded Unicode wrapper.** After the source-only supplement, the author rebuilt the binary and added a native assertion. This verifier inspected that runner diff and reran `node scripts/verify-pet-native.mjs --quota-settings`: exit 0, **27/27 passed**, including the prior preservation assertions and the added embedded-wrapper proof. Receipt: `scratch/capy-visual-63135e98-4506-47f8-84f8-23a5d49edd25/report.json`. Binary SHA-256 inspected after the run: `4E38B68CA2E1202C790FBB0ECEDA0573E5A028C2ED8FDAD548C68B3778993309`.

`scripts/verify-pet-native.mjs:436` executes the `statusline.ps1` emitted by the binary into its own profile; line 437 supplies `{}` and closes stdin. Line 438 settles observable behavior with `(await wrapper).stdout.trim()==='capy_ação_日本語_🦫'`. The emitted bridge was inspected and names the real own binary collector and `C:\Program Files\Git\bin/bash.exe`; its original command is the literal own fixture `echo capy_ação_日本語_🦫`. The emitted script was inspected and includes the explicit UTF-8 bridge read. Thus this additional receipt proves the rebuilt embedded script and current Git bash original-command path, supplementing the separate PowerShell fallback proof.

The new screenshot `scratch/capy-visual-63135e98-4506-47f8-84f8-23a5d49edd25/quota-settings.png` was inspected and renders the own fixture account with the same limits as the earlier screenshot. The runner completed before notifying the orchestrator that the main application could reopen. Rust and JS suites were not redundantly rerun: their prior 108/1 ignored and 33-pass receipts carry forward, while the changed native proof was rerun. The input deliberately lacks collector session metadata; this proves continuation of the original command while collector success is unavailable, not real quota gathering or an asserted collector exit status. Unicode payload forwarding, other bash distributions, real provider quotas and full feature completion remain outside this proof. No provider/login call or user settings modification was performed.
