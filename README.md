# Basalt

A dark, mono-and-grotesk design system for desktop software built with
[Rune](https://github.com/anomalyco/rune): colour tokens, a type scale, software components and
charts.

![the Basalt gallery](snapshots/basalt-gallery.png)

```rust
use rune_basalt::app::{self, Kind, Level};
use rune_basalt::icons::oc;

app::button("Run attack", Kind::Primary, Some(&oc::FLASK), Some("Ctrl R"), |s: &mut State| s.run());
app::badge("ready", Level::Ok);
```

Fonts: see below.

## Getting Rune: import or connect

The crate doesn't carry Rune. It takes `rune`, `rune-kit` and `rune-theme-opencode` from the
workspace that builds it (`{ workspace = true }`), so you choose where Rune comes from, with
`cargo rune` (a small Rust tool in `tools/rune-link`, the same on Windows, macOS and Linux):

| | Command | What it does |
| --- | --- | --- |
| **Import** | `cargo rune import [--rev <rev>]` | This repo pulls Rune itself from git, pinned to one revision: it writes a root `Cargo.toml` (workspace, Rune dependencies and Rune's `[patch.crates-io]` list) and `rust-toolchain.toml`. Then `cargo run -p basalt-gallery` here. |
| | `cargo rune unimport` | Removes those files (and `Cargo.lock`): the repo has no Rune again. |
| **Connect** | `cargo rune connect <workspace>` | Hosts Basalt in an existing Rune workspace (a Rune checkout, or your app's workspace): links `<workspace>/basalt/{themes,examples}` to this repo (junctions on Windows, symlinks elsewhere; edits here are live there), adds them to its `members`, adds `rune-basalt` to its `[workspace.dependencies]`, and `rune-theme-opencode` from the same source and revision as its `rune` when it lacks it. Basalt then builds against **that** workspace's Rune: no second copy, no pin to keep in step. |
| | `cargo rune disconnect <workspace>` | Removes exactly what `connect` added (it keeps a record in `<workspace>/basalt/rune-link.toml`), and the links, never the files they point to. The host's manifest keeps its formatting and line endings. |
| | `cargo rune status` | Says which mode the repo is in. |

In a connected workspace, an app uses Basalt with `rune-basalt = { workspace = true }`.
`--name <dir>` on connect/disconnect changes the `basalt` folder name.

The generated root files are git-ignored: each checkout picks its own mode.

## What's in it

**Theme.** `Theme::dark()` / `Theme::light()`, then `.accent(color)` and `.density(d)`; apply it with
`set_theme(theme)` (per thread). `tokens()` reads the colours (`bg`, `panel`, `float`, `text`,
`muted`, `dim`, `line`, `hair`, `tint`, `hover`, `accent`, `danger`, `ok`, `warn`, `solid`). `accent`
is for highlights (chart peaks, windows); `danger` stays red whatever the accent is and colours
errors and destructive actions. `ty` is the type scale (Space Grotesk headings and JetBrains Mono
text, both bundled under the SIL OFL in `assets/fonts/`; register them with
`AppSpec { fonts: rune_basalt::FONTS, .. }`), `kit()` a rune-kit `Kit` in the same look. Text and
muted colours are tested for contrast (7:1 and 4.5:1) in both themes.

**Scopes.** `disabled(cond, || ..)` and `invalid(cond, || ..)` wrap any controls built inside them.

`app`, the software components:

| Group | Components |
| --- | --- |
| Chrome | `menu_bar` and `context_menu`, `nav_item`, keyboard `tree`, `doc_tabs`, `breadcrumbs`, `pane_header`, collapsible `group`, `splitter` (drag), `dock_panel`, `scroll_view`, `status_bar`, `progress`, `spinner`, `busy` |
| Controls | `button` (primary, secondary, ghost, danger), `icon_button`, `toolbar` and `overflow_toolbar` (extra items go to a "..." menu), `segmented`, `text_field`, `stepper`, `slider` (drag and keys), `checkbox`, `switch`, `radio`, `field` with `field_msg` errors |
| Inputs | `select`, `combobox`, `number_field`, `textarea`, `tag_input`, `date_picker`, `color_picker`, `file_field` |
| Data | `Table` (sticky header, virtualized rows, sort, resizable columns, multi-selection, checkboxes, text/number/bar/badge cells), `log_view`, `reorder_list`, `prop`, `readout`, `badge`, `dot`, `kbd`, `empty_state` |
| Overlays | `command_palette` (Ctrl K), `dialog` (modal, returns focus), `toast_stack` with `ToastQueue` (errors persist), `tooltip`, and their panels alone |

`plot`: `Plot` (multi-series, log axis, legend, wheel zoom, Shift-wheel pan, double press resets,
draggable cursor, min/max decimation, state in `PlotState`), `scatter`, `heatmap`, `sparkline`,
`bars`. `icons::oc` is OpenCode's icon set from the Rune repository.

Overlays that open over the window return their layers; mount them with
`rune_kit::root(base, layers)`.

## Gallery

`cargo run -p basalt-gallery` shows every component, with live Theme, Density and Accent controls.
Ctrl K opens the palette. Environment variables set the starting state, for snapshots:
`BASALT_THEME=light`, `BASALT_DENSITY=0|1|2`, `BASALT_ACCENT=0..4`,
`BASALT_OPEN=palette|menu|menubar|select|toast|dialog`. Snapshot:
`basalt-gallery --snap out.png --size 1280x7600 --settle`.

## Tests

`cargo test --workspace` runs unit tests, doctests and interaction tests that drive Rune's headless
`Runtime` (clicks, keys, wheel, drags) on the components and the gallery.

## Limits

- Rune is a pre-release and not on crates.io, so Basalt can't be published there either.
- Interaction is tested headlessly; there is no real-window mouse test, and column-resize and
  splitter drags are not covered.
- The overflow toolbar estimates item widths. Ctrl/Shift-click in tables needs
  `App::modifier_keys()` and `on_modifiers` (the gallery wires them).
- No CI; macOS and Linux are untested.

## Layout

| Path | What |
| --- | --- |
| `themes/basalt` | The `rune-basalt` crate |
| `examples/basalt-gallery` | Every component on one scrolling page, grouped by type (`cargo run -p basalt-gallery`) |
| `tools/rune-link` | `cargo rune`: import or connect Rune |
| `.cargo/config.toml` | The `cargo rune` alias |

## License

GPL-3.0-or-later, see [LICENSE](LICENSE). The bundled fonts keep their own SIL OFL licences in
`themes/basalt/assets/fonts/`.
