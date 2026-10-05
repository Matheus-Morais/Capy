---
name: Capy
description: A calm, compact capybara companion for agent sessions on a synthetic Windows desktop.
colors:
  paper: "#faf7ed"
  paper-soft: "#f0ecdf"
  text: "#302b21"
  muted: "#716957"
  rule: "#dfd8c8"
  accent: "#435b35"
  alert: "#8a5b18"
  button-border: "#b9b09b"
  button-hover: "#eae2d0"
  waiting-row: "#f7f0df"
  command: "#ede7d8"
  quota-track: "#e3ddce"
  quota-fill: "#73845b"
  selected-scenario: "#d9e1cc"
  selected-border: "#8fa37b"
  focus: "#8a5b18"
  error: "#7d352a"
  pet-outline: "#5d3a22"
  pet-fur: "#cf9565"
  pet-shade: "#b87d52"
  pet-muzzle: "#a8693f"
  pet-belly: "#e2b98f"
typography:
  title:
    fontFamily: "'Segoe UI', sans-serif"
    fontSize: "22px"
    fontWeight: 650
    letterSpacing: "-0.025em"
  panel-title:
    fontFamily: "'Segoe UI', sans-serif"
    fontSize: "26px"
    fontWeight: 650
    letterSpacing: "-0.025em"
  body:
    fontFamily: "'Segoe UI', sans-serif"
    fontSize: "14px"
    fontWeight: 400
  compact:
    fontFamily: "'Segoe UI', sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "'Segoe UI', sans-serif"
    fontSize: "12px"
    fontWeight: 600
rounded:
  track: "3px"
  command: "5px"
  agent: "6px"
  control: "7px"
  label: "8px"
  keyboard: "12px"
spacing:
  icon: "5px"
  action-gap: "7px"
  inline: "8px"
  control: "11px"
  section: "12px"
  footer: "13px"
  row: "15px"
  block: "16px"
  header: "18px"
  inset: "20px"
  panel-inset: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "#ffffff"
    rounded: "{rounded.control}"
    padding: "8px 11px"
    typography: "{typography.label}"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "8px 11px"
    typography: "{typography.label}"
  waiting-session:
    backgroundColor: "{colors.waiting-row}"
    textColor: "{colors.text}"
    rounded: "0px"
    padding: "15px 20px"
  summary-sheet:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.text}"
    rounded: "0px"
    padding: "0px"
---

# Design System: Capy

## Overview

**Creative North Star: "The Oat-Sheet Companion"**

Capy places a warm, front-facing animated capivara over a quiet synthetic Windows desktop. The approved front-facing SVG in `prototypes/front-pet.svg` is the standard artwork and is loaded unchanged by the TypeScript interface. A compact oat-colored summary carries the useful session detail; the wider panel gives the demonstration controls room without changing the product's calm, companion-like world.

The Rust and Tauri shell provides three native surfaces: a transparent 200×180 pet window, a 380×620 summary, and a 760×680 panel. The summary favors compact, readable rows, soft dividers, and clear labels. All displayed sessions, replies, scenarios, and quota values are explicitly synthetic demo data; the interface does not imply live agent integrations.

**Key Characteristics:**
- The approved front-facing capivara is the persistent companion and remains movable.
- The native summary and full panel share warm oat surfaces, dark brown text, olive actions, and amber attention states.
- Session rows stay flat and use soft dividers; demo controls live in the full panel.
- Segoe UI and compact spacing keep session details legible.

## Colors

Oat paper and dark brown ink form the reading surface, with olive actions and warm amber attention states; the companion keeps its established caramel artwork.

### Primary
- **Quiet Olive** (`{colors.accent}`): Primary actions and links.

### Tertiary
- **Warm Alert Amber** (`{colors.alert}`): Waiting status, pet count badge, and visible keyboard focus.

### Secondary
- **Caramel Fur** (`{colors.pet-fur}`), **Deep Caramel** (`{colors.pet-shade}`), **Muzzle Brown** (`{colors.pet-muzzle}`), and **Belly Cream** (`{colors.pet-belly}`): Existing colors in the approved companion SVG.

### Neutral
- **Oat Paper** (`{colors.paper}`) and **Soft Oat** (`{colors.paper-soft}`): Main sheet and footer/aside surfaces.
- **Dark Brown Ink** (`{colors.text}`) and **Muted Bark** (`{colors.muted}`): Primary and supporting text.
- **Soft Divider** (`{colors.rule}`): Session and section separators.
- **Warm Control Border** (`{colors.button-border}`), **Button Hover Oat** (`{colors.button-hover}`), and **Waiting Oat** (`{colors.waiting-row}`): Control outlines, hover, and waiting rows.
- **Command Oat** (`{colors.command}`), **Quota Track** (`{colors.quota-track}`), and **Quota Fill** (`{colors.quota-fill}`): Command snippet and quota meter.
- **Selected Scenario** (`{colors.selected-scenario}`) and **Selected Border** (`{colors.selected-border}`): Pressed scenario control.
- **Error Brown** (`{colors.error}`) and **Mascot Outline** (`{colors.pet-outline}`): Error copy and companion linework.

### Named Rules
**The Attention Amber Rule.** Use amber for requests and attention states; keep routine actions olive.

## Typography

**Display Font:** Segoe UI (with sans-serif fallback)
**Body Font:** Segoe UI (with sans-serif fallback)
**Label/Mono Font:** Consolas for synthetic command snippets only.

**Character:** Segoe UI keeps the compact Windows utility familiar. Hierarchy comes from modest size and weight changes, not decorative type.

### Hierarchy
- **Sheet title** (650, 22px): Summary heading.
- **Panel title** (650, 26px): Full panel heading.
- **Body** (400, 14px): Default page text.
- **Compact** (400, 12px, 1.5): Session metadata, explanatory copy, and most controls.
- **Labels** (600, 12px): Actions and session states.
- **Session name** (650, 14px) and **section headings** (650, 13–14px): Row and group hierarchy.
- **Command** (400, 12px, Consolas, 1.5): Synthetic command sample.

### Named Rules
**The One Family Rule.** Use Segoe UI throughout the interface; reserve Consolas for command-like demo content.

## Layout

The native pet window is 200×180px and transparent; its artwork stays within the compact surface. The summary opens at 380×620px (minimum 340×360px), with session rows, quotas, and a sticky footer. The 760×680px full panel (minimum 540×420px) uses a session column and a 270px side area for quota and demo controls; below 620px, those areas stack. The summary and panel use 20px content insets, with 24px panel header insets. Session rows use 15px vertical and 20px horizontal padding; sections use 16px. No browser-first responsive composition is part of the native window contract.

## Elevation & Depth

The reading surfaces use tonal shifts and fine dividers to communicate grouping. The stylesheet does not assign shadows to the summary or panel. The pet window itself is transparent and shadowless at the native-shell level.

## Shapes

Controls use a 7px radius; status labels use 8px, command snippets 5px, and agent marks 6px. The quota track has a 3px radius. The pet's keyboard-focus contour uses a 12px radius. Session rows and the native summary remain square-edged, separated by fine rules.

## Components

### Buttons
- Standard buttons have a warm border, transparent fill, 7px corners, and 8px 11px padding. Hover fills with the warm oat surface.
- Primary session actions use olive fill and white text; hover deepens to `#324b27`. Actions inside session rows use 12px semibold labels.
- Visible keyboard focus uses a 3px amber outline with a 3px offset.
- Quiet footer navigation removes the border and uses olive text.

### Session Rows
- Rows use flat 15px 20px padding and a top divider. Waiting rows use a pale oat fill and amber status text.
- Each row keeps project, agent, and origin visible with its request/status and available simulated action.
- Command samples use a 5px rounded oat inset; quota rows use a 4px meter with an olive fill.

### Native Surfaces
- The compact summary combines session rows, quotas, restore access, and a sticky soft-oat footer.
- The full panel places sessions beside quotas and scenario controls; the side area becomes a stacked section under 620px.
- The transparent pet surface shows a status label, front-facing artwork, and an amber waiting-count badge.

### Capy Companion
- `prototypes/front-pet.svg` is the approved front-facing standard and is sourced unchanged by `src/pet.ts`.
- The pet changes pose with aggregate session state, can be dragged, and opens the summary on click. Reduced-motion preference disables motion.

## Do's and Don'ts

### Do:
- **Do** preserve the approved front-facing SVG as the standard companion artwork.
- **Do** keep olive for routine actions and amber for attention states.
- **Do** keep project, agent, and origin legible together in each session row.
- **Do** label sessions, replies, and quota values as synthetic demonstration data.
- **Do** keep the summary compact and expose the full panel for scenario controls.

### Don't:
- **Don't** nest cards inside the session list; use rows and soft dividers.
- **Don't** imply live responses or quota access from the demonstration. Native real mode identifies open Claude Code/Codex sessions with unknown activity; show source diagnostics and no simulated action or quota percentage on real rows.
- **Don't** replace or modify the approved front-facing mascot artwork.
