---
name: Binbin
description: Compact file-sharing tasks using shadcn/ui Base UI and the approved OKLCH theme.
colors:
  background: "oklch(1 0 0)"
  foreground: "oklch(0.145 0 0)"
  card: "oklch(1 0 0)"
  card-foreground: "oklch(0.145 0 0)"
  popover: "oklch(1 0 0)"
  popover-foreground: "oklch(0.145 0 0)"
  primary: "oklch(0.508 0.118 165.612)"
  primary-foreground: "oklch(0.979 0.021 166.113)"
  secondary: "oklch(0.97 0 0)"
  secondary-foreground: "oklch(0.205 0 0)"
  muted: "oklch(0.97 0 0)"
  muted-foreground: "oklch(0.556 0 0)"
  accent: "oklch(0.97 0 0)"
  accent-foreground: "oklch(0.205 0 0)"
  destructive: "oklch(0.577 0.245 27.325)"
  border: "oklch(0.922 0 0)"
  input: "oklch(0.922 0 0)"
  ring: "oklch(0.708 0 0)"
  chart-1: "oklch(0.87 0 0)"
  chart-2: "oklch(0.556 0 0)"
  chart-3: "oklch(0.439 0 0)"
  chart-4: "oklch(0.371 0 0)"
  chart-5: "oklch(0.269 0 0)"
  sidebar: "oklch(0.985 0 0)"
  sidebar-foreground: "oklch(0.145 0 0)"
  sidebar-primary: "oklch(0.596 0.145 163.225)"
  sidebar-primary-foreground: "oklch(0.979 0.021 166.113)"
  sidebar-accent: "oklch(0.97 0 0)"
  sidebar-accent-foreground: "oklch(0.205 0 0)"
  sidebar-border: "oklch(0.922 0 0)"
  sidebar-ring: "oklch(0.708 0 0)"
  dark-background: "oklch(0.145 0 0)"
  dark-foreground: "oklch(0.985 0 0)"
  dark-card: "oklch(0.205 0 0)"
  dark-card-foreground: "oklch(0.985 0 0)"
  dark-popover: "oklch(0.205 0 0)"
  dark-popover-foreground: "oklch(0.985 0 0)"
  dark-primary: "oklch(0.432 0.095 166.913)"
  dark-primary-foreground: "oklch(0.979 0.021 166.113)"
  dark-secondary: "oklch(0.269 0 0)"
  dark-secondary-foreground: "oklch(0.985 0 0)"
  dark-muted: "oklch(0.269 0 0)"
  dark-muted-foreground: "oklch(0.708 0 0)"
  dark-accent: "oklch(0.269 0 0)"
  dark-accent-foreground: "oklch(0.985 0 0)"
  dark-destructive: "oklch(0.704 0.191 22.216)"
  dark-border: "oklch(1 0 0 / 10%)"
  dark-input: "oklch(1 0 0 / 15%)"
  dark-ring: "oklch(0.556 0 0)"
  dark-chart-1: "oklch(0.87 0 0)"
  dark-chart-2: "oklch(0.556 0 0)"
  dark-chart-3: "oklch(0.439 0 0)"
  dark-chart-4: "oklch(0.371 0 0)"
  dark-chart-5: "oklch(0.269 0 0)"
  dark-sidebar: "oklch(0.205 0 0)"
  dark-sidebar-foreground: "oklch(0.985 0 0)"
  dark-sidebar-primary: "oklch(0.696 0.17 162.48)"
  dark-sidebar-primary-foreground: "oklch(0.262 0.051 172.552)"
  dark-sidebar-accent: "oklch(0.269 0 0)"
  dark-sidebar-accent-foreground: "oklch(0.985 0 0)"
  dark-sidebar-border: "oklch(1 0 0 / 10%)"
  dark-sidebar-ring: "oklch(0.556 0 0)"
typography:
  headline:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "24px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.025em"
  title:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "16px"
    fontWeight: 500
    lineHeight: 1.5
  body:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "14px"
    fontWeight: 400
    lineHeight: "20px"
  label:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "14px"
    fontWeight: 500
    lineHeight: "20px"
  field-label:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "14px"
    fontWeight: 500
    lineHeight: 1.375
  metadata:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "12px"
    fontWeight: 400
    lineHeight: "16px"
  badge:
    fontFamily: 'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: "12px"
    fontWeight: 500
    lineHeight: "16px"
  mono:
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace'
    fontSize: "12px"
    fontWeight: 400
    lineHeight: "16px"
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "14px"
  checkbox: "4px"
  badge: "32px"
spacing:
  xs: "4px"
  sm: "8px"
  control: "10px"
  md: "12px"
  lg: "16px"
  field: "20px"
  xl: "24px"
  section: "28px"
  header: "32px"
  page: "40px"
  empty: "64px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-outline:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.secondary-foreground}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.foreground}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-destructive:
    backgroundColor: "color-mix(in oklab, var(--destructive) 10%, transparent)"
    textColor: "{colors.destructive}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-link:
    backgroundColor: "transparent"
    textColor: "{colors.primary}"
    typography: "{typography.label}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  input:
    backgroundColor: "transparent"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "4px 10px"
    height: "32px"
    width: "100%"
  native-select:
    backgroundColor: "transparent"
    textColor: "{colors.foreground}"
    typography: "{typography.body}"
    rounded: "{rounded.lg}"
    padding: "4px 32px 4px 10px"
    height: "32px"
  badge-available:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.badge}"
    rounded: "{rounded.badge}"
    padding: "2px 8px"
    height: "20px"
  badge-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.secondary-foreground}"
    typography: "{typography.badge}"
    rounded: "{rounded.badge}"
    padding: "2px 8px"
    height: "20px"
  card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.card-foreground}"
    rounded: "{rounded.xl}"
    padding: "16px"
  alert:
    backgroundColor: "{colors.card}"
    textColor: "{colors.card-foreground}"
    rounded: "{rounded.lg}"
    padding: "8px 10px"
---

# Design System: Binbin

## Overview

**Creative North Star: "The compact task workspace"**

The compact task workspace gives files, settings, progress, and the next action a familiar shadcn/ui Base UI structure. Neutral surfaces in the shadcn Nova palette carry the content; green marks primary actions and available shares, while destructive treatments identify errors and irreversible actions. Native system typography keeps working screens compact.

This records the approved replacement of the former strictly neutral interface. The implemented world is the official base-nova component family, composed for Binbin’s upload, management, and recipient tasks. Application styles arrange those components; their variant APIs own primitive appearance and interaction states.

**Key Characteristics:**

- Compact task layouts with responsive file rows and settings panels.
- Exact supplied light/dark OKLCH semantics with green primary actions.
- Bordered controls, softly rounded cards, explicit feedback, and keyboard focus.

The normative implementation is [tokens.css](../web/styles/tokens.css), with semantic mappings and the font stack in [main.css](../web/styles/main.css). The frontmatter records light colors under their CSS names and dark colors under `dark-` names; the application uses the same CSS custom-property names in each theme. Keep the exact supplied values, including alpha values. [theme.json](../web/tests/e2e/support/theme.json) independently records the expected theme. This file remains `docs/design.md`; do not create a competing root or differently cased design file.

Authority: [product commitments](product.md), [lifecycle specification](specification.md), and the exact tokens in `web/styles/tokens.css`. Layout and interaction rules below describe the implemented interface; browser coverage and the build workflow are documented in [development instructions](../CONTRIBUTING.md).

## Colors

Cool white and dark surfaces support a green action accent and semantic destructive red. Frontmatter contains the complete approved theme, preserving the original OKLCH notation without conversion. The schema-v2 [sidecar](../.impeccable/design.json) extends it with stateful component previews, motion, breakpoints, and narrative. Its eight-step color ramps are generated swatch metadata, not additional shipped theme values; their interpolated stops do not replace the supplied chart sequence. Component previews translate the shipped primitives to standalone HTML/CSS and do not implement application workflows.

### Primary

- **Forest green action** (`primary` / `dark-primary`): primary buttons, upload indicators, the mark, selected checkboxes, links, and the Available badge. Pair with the supplied primary foreground.
- **Destructive red** (`destructive` / `dark-destructive`): errors, invalid states, revoke/delete controls, and their confirmation. The component variants use translucent destructive fills rather than introducing a separate warning palette.

### Secondary

- **Soft neutral selection** (`secondary` / `dark-secondary`): current navigation and non-active status badges. This is a neutral supporting role, not another chromatic accent.
- **Muted neutral surface** (`muted`, `accent`, and their dark counterparts): row hover, drop zones, subtle code backgrounds, and supporting surfaces. Muted foreground carries metadata and help.

### Neutral

- **Canvas and reading ink** (`background`, `foreground`): the page and primary text.
- **Raised surface and ink** (`card`, `card-foreground`): cards and alerts. Light cards share the canvas color; dark cards separate through their lighter tone.
- **Control edges and focus** (`border`, `input`, `ring`): dividers, input strokes, and focus feedback. Dark borders and inputs retain their supplied white alpha values.
- **Reserved theme roles** (`popover`, `chart-1` through `chart-5`, and `sidebar-*`, with their foregrounds and dark pairs): preserve these supplied values for theme fidelity. The shipped pages have a horizontal header and no chart, popover, or sidebar widget. Their presence is not a requirement to add one. The current neutral chart series is a supplied sequence, not a newly invented data palette.

**The Supplied Theme Rule.** Use the supplied semantic values in both themes; derive hover, invalid, and destructive treatments through the existing component variants.

Theme preference is `system`, `light`, or `dark`, persisted under `binbin-theme` when storage is available. System mode tracks OS changes; explicit modes override them. The root `.dark` class changes tokens, browser color-scheme, and the Tailwind dark variant together. Storage failure leaves theme selection usable for the current session.

## Typography

**Display and body font:** native system sans, as recorded in frontmatter; no separately loaded display face. **Mono font:** the Tailwind native monospace stack for code.

- **Headline:** page titles (24px, weight 600, line-height 1.3, tight tracking). Narrow page headings use 23px; recipient titles use 22px, then 21px at narrow widths.
- **Title:** panel headings (16px, weight 500). Card titles use snug line-height; embedded panel `h2` headings follow their base heading line-height.
- **Body:** working text (14px, 20px line-height). Paragraphs are limited to 72ch. Share rows and filenames commonly use 13px; strong text uses weight 500.
- **Label:** controls and field labels (14px, weight 500). Standard button text remains sentence case. Field labels use snug line-height (1.375); button labels use 20px line-height.
- **Metadata:** dates, sizes, counts, and support text (12px); fine print uses line-height 1.6. Quantities in row metadata and definition lists use tabular numerals.
- **Mark:** the existing Binbin wordmark uses 21px, weight 650, and −0.035em tracking. Keep its SVG mark; interface icons are Lucide SVGs.

Input text is 16px below the Tailwind medium breakpoint (48rem), then 14px. At application widths up to 480px, inputs and native selects have a 40px minimum height and 16px text; the theme selector uses icon buttons. Do not apply oversized display typography to routine working screens.

## Layout

The header, main workspace, and footer share one centered maximum width (1080px), with matching horizontal padding: 24px on desktop, 20px below 760px, and 16px below 480px. The header is sticky with a 56px minimum height and a translucent blurred background, and aligns mark, navigation, and account controls in three shrinking columns. Main desktop padding is 32px 24px 64px. The footer shows Binbin and the workspace package version from Cargo.toml.

Upload, share details, and administration use a flexible working panel plus a settings track (340px), with a 24px gap. Sign-in centers a card capped at 400px; recipient content uses a centered card capped at 544px inside the shared workspace. These task-specific patterns belong to their corresponding screens rather than every future page.

Spacing follows the frontmatter scale: 12px between related fields/actions, 16px row padding, 20px between fields, 24px panel gaps and working-card padding, and a 24px heading-to-content interval. The primitive card default is 16px padding; its compact size is 12px. Application working and settings cards use 20px, login and recipient cards 24px, reducing to 16px on narrow screens.

- **At 1000px and below:** hide the deadline column, reduce paired-layout settings tracks to 310px and gaps to 20px, and stack paired fields within those layouts.
- **At 900px and below:** navigation becomes a second header row.
- **At 760px and below:** paired panels stack, and main horizontal padding becomes 20px. Field pairs can return to two columns when the stacked panel has room. Signed-out and recipient shells omit navigation.
- **At 480px and below:** navigation uses a two-column grid and page headings clamp to three lines with their full title retained; horizontal padding becomes 16px, paired fields and inline forms stack, and account/user actions wrap or stack. Share rows use two columns, omit their desktop header, and place status beside the filename with size/count metadata below. Controls follow the mobile sizing described above.
- **Validation widths:** desktop 1440px, tablet 768px, mobile 390px, and narrow 320px, each in light and dark themes. These are evidence sizes, not additional CSS breakpoints.

The share index uses aligned five-column rows at desktop, four when the deadline is hidden, and two on narrow screens. Rows have a 78px minimum height and a muted hover surface. Long file titles wrap within shrinking tracks and clamp to two lines; transfer feedback truncates its filename to one line with the full name in a title attribute. Other long text and code wrap within their available width.

**The Content Fits Rule.** Use shrinking tracks, wrapping, and the existing bounded filename treatments so working content fits without horizontal page scrolling.

## Elevation & Depth

Depth comes primarily from semantic surface tones, borders, and fine outlines. Cards use the primitive’s thin foreground-at-10% ring rather than an ambient drop shadow. Lists and dividers use a single border owned by the row; inline password-reset forms, upload progress, and draft actions use spacing instead of adding a second adjacent rule. Do not turn these into floating marketing panels.

- **Notification shadow:** `0 8px 24px oklch(0 0 0 / 10%)`, used by the fixed notification only.
- **Primitive focus:** ring-colored border plus a 3px ring at 50% ring color. Invalid controls use destructive border/ring treatments, with the stronger dark-theme alpha values retained from the primitives.
- **Other focusable elements:** a visible 2px ring-colored outline with a 3px offset. The skip link appears on focus and moves focus to the main content.

State transitions are brief: the file-row hover background changes over 120ms; primitive color/all-property transitions use Tailwind’s default timing (150ms). Buttons depress by 1px while active unless acting as a popup trigger. Progress transitions follow actual committed bytes. Reduced-motion preference removes transitions, animations, and smooth scrolling throughout.

## Shapes

The supplied base radius is 0.625rem (10px at the default root size). Its mapped small/medium/large/extra-large sizes are 6/8/10/14px. Controls, alerts, lists, drop zones, and the small mark tile use the 10px form; cards use 14px. Small buttons use the mapped medium radius with the primitive’s cap. Checkboxes have 4px corners, status badges use the pill-like 32px radius, and progress tracks are fully rounded. Keep these differences; a card and a badge do not share one silhouette.

Controls and list dividers use thin strokes (1px). The file picker’s dashed edge distinguishes a selection area, with a primary edge and accent surface on drag or hover. Its focus-within outline keeps the invisible native file input discoverable to keyboard users.

## Components

**The Primitive Ownership Rule.** Use the shipped shadcn/ui Base UI primitives and their variants; keep application CSS concerned with composition and responsive task layout.

The registry selection and aliases live in [components.json](../web/components.json) (`base-nova`). [ui primitives](../web/src/components/ui/) use `@base-ui/react` where behavior is needed; cards and alerts remain semantic presentation components, and selects stay native. The web package uses React, TypeScript, Vite, and Tailwind CSS 4 with no Radix dependency. All npm manifests and build/test/lint/format configuration live under `web/`; see [web README](../web/README.md). Composition wrappers in [common.tsx](../web/src/components/common.tsx) and [forms.tsx](../web/src/components/forms.tsx) provide panel and field structure.

### Buttons

Compact, explicit actions. The default height is 32px with 10px horizontal padding; extra-small/small/large heights are 24/28/36px. Icon sizes follow their corresponding square dimensions. Inline SVGs use the primitive’s icon sizing.

- **Primary:** green with primary foreground; hover uses primary at 80% opacity.
- **Outline:** canvas surface and border; hover uses muted. Dark mode uses input-at-30% fill, input border, and input-at-50% hover.
- **Secondary:** secondary surface and foreground; hover mixes in 5% foreground in OKLCH.
- **Ghost:** transparent until muted hover; dark hover uses muted at 50%.
- **Destructive:** destructive text over destructive-at-10% fill (20% in dark), increasing to 20% on hover (30% in dark).
- **Link:** primary text with an underline on hover. `LinkButton` preserves anchor semantics while reusing button variants for navigation.

Focus, invalid, active, and disabled states belong to the primitive. Disabled buttons block pointer interaction and use 50% opacity. Pending mutations disable affected submission controls. Use a clear primary action for each task; retain named secondary and cancel actions.

### Inputs / Fields

Bordered, rounded, full-width fields use the input stroke, transparent light fill, and input-at-30% dark fill. The normal height is 32px; responsive text and minimum heights are described above. The native select follows the same control language with a chevron and native platform option rendering. Labels bind to generated or explicit IDs. Checkboxes use Base UI state and form submission semantics, primary fill when checked, and an SVG check indicator. Keep disabled/invalid treatments and field help adjacent to their controls; errors also include readable text.

### Cards / Containers

`Panel` composes Card, an optional CardHeader/title, and CardContent. Use it for genuine task groups and single-card login/recipient flows. Preserve the radius, subtle ring, and spacing rules above. Do not use a card for each file row: the index and file lists use aligned content. File rows have dividers only between items, with no trailing divider. Card footers, when used, have a border and muted-at-50% surface.

### Status badges

Pill-shaped, compact labels (20px height, 12px medium text, 8px horizontal padding). The Status wrapper uses the primary variant only for Available; Draft, Preparing, Expired, Allowance used, Removed, and Abandoned use the secondary variant. Labels carry the meaning alongside color. On share detail pages, the state sits directly beside the share name and matches its title typography, wrapping below it when necessary on narrow screens. Other list badges retain their compact size. These are state labels, not invented interactive filters.

### Navigation

Lucide icons accompany navigation and primary/destructive action labels throughout the app. Familiar actions such as copy, remove, preview, and pause use icon buttons with accessible names and shadcn tooltips. Signed-out and recipient pages show a single icon button that cycles System → Light → Dark → System; its monitor, sun, or moon icon shows the current preference, and its accessible name and tooltip state the current theme and the next choice. Signed-in users choose a theme in the account menu instead. A favicon reuses the existing Binbin mark.

The horizontal header has Shares and, for administrators, Administration as ghost links with a secondary current-page link (`aria-current="page"`); share detail keeps Shares current. At the right edge, a primary New share link sits beside the account menu: an outline button with the user’s initial, name and a chevron that opens a dropdown with the user and role, a System/Light/Dark radio group, and Sign out. Below 480px the name is hidden and only the initial and chevron remain. The existing header and recipient/signed-out behavior described under Layout apply at all routes. Back navigation is a semantic anchor; theme selection remains available on signed-out and recipient pages.

### File selection and upload progress

The real multiple-file input, drag/drop, and screenshot paste all append to the collection. File rows show a thumbnail or Lucide file-type icon, filename, size, and explicit queue state. Colliding names receive numbered suffixes with feedback. Rows support individual removal and eight-second Undo; Clear all uses a shadcn dialog. Supported raster images open in a shadcn dialog viewer with previous/next controls and arrow-key navigation. Authenticated previews remain available for completed uploads after reload.

The initial upload is explicit. Additions join a running queue; later additions wait for Upload new files. Failures remain attached to their rows while other files continue, with retry and original-file reselection actions. Publication stays disabled until all remaining files are uploaded. Its button sits in Share settings directly above Revoke and delete. A persistent upload indicator links to the draft and offers Pause while navigation remains usable. Local File handles survive route changes; closing or refreshing warns during active work. Only unfinished files need reselection after reload, and resumption verifies committed prefixes. Drafts remain editable until publication freezes the collection for ZIP preparation.

### Alerts, failures, and download feedback

Inline alerts use card surfaces, readable text, thin borders, and descriptive destructive treatment for errors. Generic page loading uses status text; management request failures use an inline destructive alert. Failed refreshes retain useful data with an explicit stale-data warning and a retry action. Recipient password challenges have a labeled password form. A temporary/network/server recipient failure says “Couldn’t load this share,” gives connection/server guidance, and offers Try again. Only a 404/410 result enters Share unavailable and checks for previously started transfers. Preserve that distinction.

If saved public transfer credentials still validate, replace Share unavailable with Resume your download. Use the existing file rows with a filename, Previously started transfer label, and Retry download link, plus an explanation that retries use the original transfer without another download charge. Show status text while checking saved transfers and a retry action if checking fails. Recovery validates and reuses the original credential without issuing a new one; it does not bypass deadlines, password changes, or revocation.

Notifications are dismissible fixed alerts at bottom right, bounded to 360px and the viewport, with reserved space for the dismiss control. They do not move the page layout. Errors use alert semantics; ordinary confirmation uses status semantics. Opening the share-deletion confirmation clears a previous notification before showing its warning.

Published file lists share one component for owners and recipients: a header strip with the file count, total size and Download all as ZIP (omitted for a single-file recipient share, which shows a primary Download in its row), then one row per file with a type icon, name, size and an icon-only Download button. Clicking anywhere on a row opens the file in a new browser tab; the icon button saves an attachment. Both actions reuse the same transfer identity and allowance. Filenames use a subtle underline, visible focus, and an accessible name explaining the new tab. Preparing a transfer is a button action; recovered transfer URLs remain real links. Draft and unavailable files retain their existing noninteractive names.

Starting a download shows no message on the page: the browser’s own download UI is the feedback, and owner downloads raise no toast. Repeating a download reuses the transfer identity and allowance, and the saved transfer can still be retried from Resume your download after a reload. Never claim Download complete.

### Editing, deletion, and secrets

The inline administrator password-reset form names the target account in its heading and clears the password when a different account is selected. Disable target changes and submission while a mutation is pending. Keep the session-invalidation consequence explicit in the submit label, and separate the form from account rows with spacing as described above.

Share editing replaces the read-only settings summary and includes Cancel. The deletion flow opens a confirmation dialog titled with the share name, a destructive description (“This permanently removes access and files. There is no undo.”), Keep share, and Confirm deletion. Keep the whole warning visible and distinct from floating feedback; do not replace this with a routine toast. Active shares lead their settings panel with the share link and a primary Copy link action. Drafts show a sticky publish bar with the upload count and Publish share, and an outlined time-left badge beside the status.

## Delete after download

Use a labeled checkbox in Share details, unchecked by default. When checked, replace the numeric allowance with adjacent help: “Delete the files and share details after the first complete download.” Explain whole-share delivery and that publication locks the setting. Draft settings are editable; published edit forms state whether the policy is enabled and locked. Keep the existing publish action without another confirmation dialog.

Repeat “Delete after download” in draft summaries, publish feedback, published link blocks, share headings, and dashboard metadata. Dashboard allowance text becomes “One download”; an active or reserved transfer shows “Downloading”. Management file controls remain available, with a short explanation that owner/admin previews and downloads do not trigger deletion, while public links follow recipient rules. Deleted shares disappear from lists and history; a subscribed open detail page returns to Shares with a transient “Deleted after download” notice.

After unlocking any password gate, recipient pages show the policy badge and the inline warning “Download once. Then this share is deleted.” List filenames and sizes as static text. Offer one green “Download file once” or “Download ZIP once” action. Use the shared shadcn primitives, existing semantic theme, and responsive recipient card. No preview links or individual multi-file controls appear.

During delivery, show “Download in progress” and point to the browser's downloads without claiming client completion. On interruption, explain the 15-minute reservation, native browser resume, and Retry download. Other recipients see “Download already in progress” and Check again. If live status disconnects, explain that the browser download may still be running and offer Check again. The live deletion receipt shows “Deleted after download” with no filenames; fresh visits show the existing generic Share unavailable state. Temporary server/network failures retain their retry behavior. Copy link still copies only the URL, and administration gains no additional global toggle.

## Do's and Don'ts

### Do:

- Do keep the exact light/dark theme pairs and the System, Light, and Dark selector on every surface.
- Do use semantic links for navigation and downloads, and buttons for actions.
- Do give fields associated labels, icon-only controls accessible names, and keyboard users visible focus.
- Do pair status color with text, retain real upload progress and pause, and keep retry feedback tied to its resource.
- Do keep temporary request failures retryable and permanent unavailability explained; keep deletion warnings visible with a cancel action.
- Do keep screens quiet: omit helper text that restates behavior the interface already shows, such as storage totals, allowance footnotes, and expiry reminders on drafts.
- Do fit content at 1440, 768, 390, and 320px in both themes and respect reduced-motion preferences.

### Don't:

- Don’t restore the superseded strictly neutral palette or substitute new theme values.
- Don’t override primitive visuals with legacy application selectors or introduce Radix primitives into this Base UI system.
- Don’t hide horizontal overflow on the document to conceal a layout defect.
- Don’t turn temporary recipient failures into a claim that the share is permanently unavailable.
- Don’t let a stale notification obscure irreversible-action guidance or add duplicate feedback for repeated downloads of the same resource.
- Don’t imply that starting a download proves completion, or that a deleted share can be restored.
