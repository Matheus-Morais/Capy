# Capy desktop shell

Profile: light. One build batch; independent verifier required.

Sources: conversation approving Rust + Tauri and the first desktop slice; PRODUCT.md; prototypes/front-pet.svg and compact summary structure in prototypes/preview.html (binding visual references).

## Scope

A local Windows executable with one frontal capybara, transparent frameless companion, native dragging, click-to-open compact summary, separate full panel, tray controls. Sessions and quotas remain clearly simulated. Real agents, terminal launch, installers, autostart and updater are outside this slice.

## Plan

Create a Tauri 2 Rust host and Vite/TypeScript interface. Keep the original prototypes intact. Use three labelled windows: pet (200×180 logical px), summary (380×620), panel (760×680). Only pet is shown on startup. Each closes to the tray; explicit Sair exits. Keep demo state in Rust so summary and panel agree. Native positioning uses monitor work areas; normal shutdown saves pet position.

## Landing

| Decision | Literal shape | Alternative rejected |
| --- | --- | --- |
| Approved desktop stack | Rust + Tauri 2, TS + HTML/CSS/SVG | C#/WPF; user chose Rust/Tauri |
| Dependency baseline | Tauri API/CLI 2, Vite, TypeScript, serde; single-instance plugin | UI framework and database; no current need |
| Window layout | pet/summary/panel separate native windows | One desktop-sized transparent window; unnecessarily covers working area |
| Local position | app config `position.json`, physical integer x/y, clamp on restore, save on normal exit | Writing each movement event; unnecessary I/O |

## Checks

### S1 — Build and lifecycle · new shell ~12 files, ~45 KB, one batch

**C1** — Frontend type checking and production build succeed; approved frontal SVG is used.
Proof: `npm run build`; browser rendering with frontal mascot accessible name.

**C2** — Pet is the only visible startup window; configured transparent, undecorated, topmost, 200×180, skip taskbar.
Proof: `node --test tests/config.test.mjs`; `npm run verify:native` (startup assertion).

**C3** — Native toggle opens summary and second toggle hides it; full panel is independently accessible.
Proof: `npm run verify:native` (toggle and panel assertions); browser click inspection for frontend binding.

**C4** — Pointer movement above 5px invokes native dragging once; a simple click invokes toggle_summary. Arrow keys invoke a 16px movement.
Proof: TypeScript build plus independent located binding inspection in src/pet.ts. Physical pointer gesture on native WebView is a manual verification limitation if native UI automation is unavailable.

**C5** — Summary/window placement stays within available monitor work area, including negative monitor coordinates; stale position falls back within primary work area.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml geometry::tests`; `npm run verify:native` (position bounds).

**C6** — Tray `capy` exists; its commands can show/hide pet, open summary/panel and quit. Window close handlers hide rather than destroy.
Proof: `npm run verify:native` (tray and hide/show assertions); independent menu/close handler inspection.

**C7** — Normal shutdown writes position; malformed saved JSON falls back without preventing startup.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml position::tests`; `npm run verify:native` (save/restore using isolated test path).

**C8** — Demo permission/question responses change the selected session to working; hiding/restoring updates tracked count. Demo indication remains visible in summary and panel.
Proof: `cargo test --manifest-path src-tauri/Cargo.toml demo::tests`; browser summary/panel inspection.

## Swept

- Validation: C5/C7; IPC actions accept only supported window/action names.
- Failure modes: invalid position defaults; frontend displays command failures with retry through controls.
- Idempotency: hide/show set desired visibility; single-instance prevents duplicate mascot.
- Authorization: no shell/file-system frontend permissions; local bundled UI invokes narrow Rust commands only.
- Concurrency: demo state serialized behind Mutex; render from authoritative snapshots.
- Data lifecycle: only pet position retained; sessions synthetic and reset on restart.
- Dependency failure: build errors surfaced; no external service runtime dependency besides WebView2.
- State transitions: C3/C6/C8.
- Observability: native self-test writes named PASS/FAIL assertions; normal command errors surface in interface.

## Handoff

One batch (~45 KB/4 = ~12k source tokens); no build handoff necessary. Reviewer gets this complete checklist plus all changed source. No git baseline existed before this slice.
