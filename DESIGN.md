---
name: azdocs Desktop
description: A spatial Azure observatory for exploring stored estate evidence.
colors:
  azure-command: "#0078d4"
  spectral-cyan: "#4bd5ff"
  signal-coral: "#d7492f"
  live-green: "#38d58c"
  midnight-shell: "#07121f"
  graph-ink: "#071321"
  deep-navy: "#081525"
  ink: "#11213b"
  muted: "#65758b"
  paper: "#ffffff"
  pale-workspace: "#eaf1f7"
  quiet-surface: "#f3f7fb"
  hairline: "#dce5ee"
typography:
  headline:
    fontFamily: "Segoe UI Variable, Segoe UI, Plex, ui-sans-serif, system-ui, sans-serif"
    fontSize: "23px"
    fontWeight: 650
    lineHeight: 1.15
    letterSpacing: "-0.035em"
  title:
    fontFamily: "Segoe UI Variable, Segoe UI, Plex, ui-sans-serif, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.025em"
  body:
    fontFamily: "Segoe UI Variable, Segoe UI, Plex, ui-sans-serif, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Segoe UI Variable, Segoe UI, Plex, ui-sans-serif, system-ui, sans-serif"
    fontSize: "9px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "0.08em"
  evidence:
    fontFamily: "Plex Mono, ui-monospace, monospace"
    fontSize: "9px"
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: "0.04em"
rounded:
  label: "5px"
  control: "10px"
  panel: "17px"
  immersive: "20px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "20px"
  xl: "28px"
components:
  button-primary:
    backgroundColor: "{colors.azure-command}"
    textColor: "{colors.paper}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
    height: "38px"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
  panel-evidence:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.panel}"
    padding: "20px"
  panel-immersive:
    backgroundColor: "{colors.graph-ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.immersive}"
    padding: "20px"
---

# Design System: azdocs Desktop

## Overview

**Creative North Star: "The Spatial Azure Observatory"**

azdocs pairs a bright, evidence-dense Azure inventory with a nocturnal spatial explorer. The application should feel like a professional desktop instrument: calm around stored facts, vivid only where selection, direction, risk, or live state needs attention.

The topology is the authored focal surface. Native Microsoft Azure service artwork provides recognition, spectral cyan explains relationships, and restrained tonal layering separates tools from evidence. Controls remain transparent or softly tinted; they do not sit on white pill-shaped backplates.

**Key Characteristics:**

- Desktop-dense information with a clear pane hierarchy.
- Native Azure artwork on transparent icon surfaces.
- Dark spatial graph beside pale evidence workspaces.
- Fine boundary-anchored connectors with restrained directional motion.
- State communicated through color, stroke, and text—not decoration alone.

## Colors

The palette moves between midnight operational space and cool paper evidence, with Azure blue and spectral cyan reserved for command and relationship states.

### Primary

- **Azure Command:** Primary actions, active controls, and selected inventory markers.
- **Spectral Cyan:** Relationship direction, graph focus, and active spatial affordances.

### Secondary

- **Signal Coral:** Findings and risk counts that require attention.
- **Live Green:** Successful collection and stored-resource state.

### Neutral

- **Midnight Shell:** Persistent navigation and application chrome.
- **Graph Ink:** The topology canvas and immersive inspector.
- **Paper and Pale Workspace:** Evidence panes, tables, and the main working field.
- **Ink, Muted, and Hairline:** Text hierarchy and quiet structural separators.

**The Accent Rationing Rule.** Azure blue drives actions; cyan explains topology. Do not flood neutral evidence surfaces with either accent.

**The Semantic Signal Rule.** Coral denotes findings and green denotes healthy or complete state. Neither is decorative.

## Typography

**Display Font:** Segoe UI Variable, with Segoe UI, Plex, and system sans fallbacks

**Body Font:** Segoe UI Variable, with Segoe UI, Plex, and system sans fallbacks

**Label/Mono Font:** Plex Mono, with a monospace fallback

**Character:** The interface uses a compact Microsoft-native sans hierarchy, with Plex Mono reserved for ARM IDs, tags, and machine evidence. Weight and contrast establish hierarchy without oversized dashboard typography.

### Hierarchy

- **Headline:** Resource and view titles; compact, semibold, and slightly tightened.
- **Title:** Topology and panel titles; semibold with restrained negative tracking.
- **Body:** Descriptions and supporting evidence.
- **Label:** Controls, column labels, counts, and status language; uppercase only for short metadata labels.
- **Evidence:** ARM IDs, tag keys and values, and technical identifiers.

**The Evidence Voice Rule.** Use monospace only when the content is machine-shaped; resource names and actions remain sans serif.

## Layout

The shell is a fixed desktop grid with a narrow command rail, persistent top bar, flexible workspace, and compact status strip. Evidence views use three panes at wide sizes, step down to two panes below 1260px, and move the inspector into a focus-managed overlay below 1060px. The topology uses a fluid graph stage with a 372px inspector that contracts to 330px before becoming the overlay.

Spacing follows a compact 4/8/12/20/28 rhythm. Panels are separated by 12px, internal evidence padding is usually 20px, and dense rows stay comfortably targetable rather than collapsing into spreadsheet scale.

**The Focal Surface Rule.** Let the topology consume the available work area; command chrome and legends float at its edges rather than shrinking it into a card within a card.

## Elevation & Depth

The light evidence world uses low ambient shadows and hairline borders. The dark topology uses tonal gradients, inset highlights, and deeper ambient shadows. Shadows establish panel hierarchy; they are not hard decorative offsets.

### Shadow Vocabulary

- **Evidence lift** (`0 18px 44px rgb(44 72 96 / 9%)`): Pale panes over the workspace.
- **Immersive lift** (`0 24px 55px rgb(8 35 56 / 24%)`): Topology stage and inspector.
- **Focus halo** (`0 0 0 3px rgb(29 145 220 / 14%)`): Search and keyboard focus reinforcement.

**The Tonal-First Rule.** Establish hierarchy with surface tone and borders before adding shadow.

## Shapes

Controls use gently curved corners, evidence panels use broader corners, and immersive graph surfaces use the largest radius. Compact labels and connector captions stay tighter. Azure service icons retain their native silhouettes and transparent backgrounds; never wrap them in generic white rounded squares or circles.

## Components

### Buttons

- **Primary:** Azure blue command treatment with a compact rectangular silhouette and a subtle directional gradient.
- **Quiet:** Transparent at rest with a cool hairline border; hover adds a low-opacity blue tint.
- **Focus:** A visible two-pixel focus outline remains distinct from hover.

### Chips

- **Style:** Compact, semibold, and softly tinted. Status chips use their semantic signal color; tag chips use the cool neutral palette.
- **State:** Selected graph modes gain an inset cyan underline rather than a filled white capsule.

### Cards / Containers

- **Evidence:** Cool-white surface, fine blue-gray border, broad panel corner, low ambient lift.
- **Immersive:** Deep navy gradient, translucent border, broad immersive corner, and controlled inner highlight.

### Inputs / Fields

- **Style:** Dark translucent top-bar fields and pale compact evidence filters use clear borders and compact control corners.
- **Focus:** Border shifts to Azure blue with a soft halo; placeholder text remains subordinate.
- **Disabled:** Preserve layout and reduce opacity without removing labels.

### Navigation

The command rail is icon-led with a two-line-safe text label. Active items use a tinted navy field and a narrow luminous cyan edge marker. Labels must never truncate core destinations such as Findings or Relationships.

### Cytoscape Resource Graph

The graph begins with resource groups as aggregate Azure containers, then drills into a selected group's resources before moving to a one-hop resource neighbourhood. It uses native Azure SVG icons inside transparent labels, deterministic component-aware layouts, boundary-anchored round-taxi connectors, and small target arrows. Selection changes emphasis without resetting user drag, pan, or zoom. Flow is a tiny moving dash overlay; it pauses when disabled, hidden, or reduced motion is requested.

## Do's and Don'ts

### Do:

- **Do** use the supplied Microsoft Azure icon for every known service type.
- **Do** keep graph ordering and layout deterministic for the same stored snapshot.
- **Do** anchor connectors to node boundaries and make arrow direction legible.
- **Do** preserve user graph state while selection and inspection change.
- **Do** keep compact overlays keyboard-operable and their long evidence lists scrollable.

### Don't:

- **Don't** put icons or ordinary controls on opaque white rounded backplates.
- **Don't** use oversized pulsing halos or large animated flow marks.
- **Don't** draw straight center-to-center relationship lines or containment edges.
- **Don't** let navigation labels, tags, ARM IDs, or inspector content clip silently.
- **Don't** make the topology webview fetch Azure or query SQLite directly; it renders typed snapshot data from the Tauri boundary.
