<div align="center">

# Basalt

**Dark / light design system for desktop software built with Rune: tokens, software components, charts and an import / connect tool**

![Language](https://img.shields.io/badge/Rust-16171C?logo=rust)
![Platform](https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-16171C)
![UI](https://img.shields.io/badge/Rune-16171C)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-16171C)](LICENSE)

<img src="snapshots/basalt-gallery.png" alt="The Basalt gallery" width="420">

</div>

---

Basalt is a design system for [Rune](https://github.com/anomalyco/rune), the Rust GPU UI framework, aimed at desktop software rather than web pages: panes, trees, tables, toolbars, inspectors, consoles, palettes and plots.

It is made of colour tokens (dark and light, with a swappable accent and three densities), a type scale (Space Grotesk and JetBrains Mono, bundled), about fifty components grouped by type, and a plot module. A gallery shows every component live, and `cargo rune` decides where Rune comes from: imported into this repository, or connected into an existing Rune workspace.

```rust
use rune_basalt::app::{self, Kind, Level};
use rune_basalt::icons::oc;

app::button("Run attack", Kind::Primary, Some(&oc::FLASK), Some("Ctrl R"), |s: &mut State| s.run());
app::badge("ready", Level::Ok);
```

## Features

**Theme**
- `Theme::dark()` and `Theme::light()`, then `.accent(color)` and `.density(d)`, applied with `set_theme`
- Colour tokens `bg`, `panel`, `float`, `text`, `muted`, `dim`, `line`, `hair`, `tint`, `hover`, `accent`, `danger`, `ok`, `warn`, `solid`
- `danger` stays red whatever the accent is; `accent` is for highlights such as chart peaks
- Text and muted colours are tested for contrast (7:1 and 4.5:1) in both themes
- `disabled(cond, || ..)` and `invalid(cond, || ..)` scopes wrap any control built inside them

**Components** (`rune_basalt::app`)

| Group | Components |
| --- | --- |
| Chrome | `menu_bar`, `context_menu`, `nav_item`, keyboard `tree`, `doc_tabs`, `breadcrumbs`, `pane_header`, collapsible `group`, `splitter`, `dock_panel`, `scroll_view`, `status_bar`, `progress`, `spinner`, `busy` |
| Controls | `button` (primary, secondary, ghost, danger), `icon_button`, `toolbar` and `overflow_toolbar`, `segmented`, `text_field`, `stepper`, `slider`, `checkbox`, `switch`, `radio`, `field` with `field_msg` |
| Inputs | `select`, `combobox`, `number_field`, `textarea`, `tag_input`, `date_picker`, `color_picker`, `file_field` |
| Data | `Table` (sticky header, virtualized rows, sort, resizable columns, multi-selection, checkboxes, text / number / bar / badge cells), `log_view`, `reorder_list`, `prop`, `readout`, `badge`, `dot`, `kbd`, `empty_state` |
| Overlays | `command_palette` (Ctrl K), `dialog` (modal, returns focus), `toast_stack` with `ToastQueue`, `tooltip`, and their panels alone |

**Plots** (`rune_basalt::plot`)
- `Plot`: multi-series lines, log axis, legend, wheel zoom, Shift-wheel pan, double press to reset, draggable cursor, min / max decimation
- `scatter`, `heatmap`, `sparkline`, `bars`
- `icons::oc` re-exports OpenCode's icon set from the Rune repository

## Getting Rune: import or connect

The crate does not carry Rune. It takes `rune`, `rune-kit` and `rune-theme-opencode` from the workspace that builds it (`{ workspace = true }`). `cargo rune` (a small Rust tool in `tools/rune-link`, identical on Windows, macOS and Linux) picks where they come from:

| Mode | Command | What it does |
| --- | --- | --- |
| **Import** | `cargo rune import [--rev <rev>]` | This repository pulls Rune from git, pinned to one revision: it writes a root `Cargo.toml` (workspace, Rune dependencies and Rune's `[patch.crates-io]` list) and `rust-toolchain.toml` |
| | `cargo rune unimport` | Removes those files and `Cargo.lock` |
| **Connect** | `cargo rune connect <workspace>` | Hosts Basalt in an existing Rune workspace: links `<workspace>/basalt/{themes,examples}` to this repository (junctions on Windows, symlinks elsewhere), adds them to its `members` and `rune-basalt` to its `[workspace.dependencies]`. Basalt then builds against **that** workspace's Rune |
| | `cargo rune disconnect <workspace>` | Removes exactly what `connect` added, using the record in `<workspace>/basalt/rune-link.toml`; the host manifest keeps its formatting and line endings |
| | `cargo rune status` | Tells which mode the repository is in |

`--name <dir>` changes the `basalt` folder name. In a connected workspace, an app uses Basalt with `rune-basalt = { workspace = true }`. The generated root files are git-ignored, so each checkout picks its own mode.

## Usage

### Gallery

```
cargo rune import
cargo run -p basalt-gallery
```

Every component on one scrolling page, with live Theme, Density and Accent controls. Ctrl K opens the palette. Environment variables set the starting state, which is handy for snapshots:

| Variable | Values |
| --- | --- |
| `BASALT_THEME` | `light` |
| `BASALT_DENSITY` | `0`, `1`, `2` |
| `BASALT_ACCENT` | `0` to `4` |
| `BASALT_OPEN` | `palette`, `menu`, `menubar`, `select`, `toast`, `dialog` |

```
basalt-gallery --snap out.png --size 1280x7600 --settle
```

### In an app

Register the bundled fonts, then build the view from the components. Overlays return their layers, which are mounted with `rune_kit::root(base, layers)`.

```rust
AppSpec { fonts: rune_basalt::FONTS, .. }
```

## Tests

```
cargo test --workspace
```

Unit tests, doctests and interaction tests that drive Rune's headless `Runtime` (clicks, keys, wheel, drags) on the components and on the gallery. `cargo test --manifest-path tools/rune-link/Cargo.toml` covers `cargo rune`.

## Known limitations

- Rune is a pre-release and not on crates.io, so Basalt cannot be published there either.
- Interaction is tested headlessly: there is no real-window mouse test, and column-resize and splitter drags are not covered.
- `overflow_toolbar` estimates item widths. Ctrl / Shift-click in tables needs `App::modifier_keys()` and `on_modifiers` (the gallery wires them).
- There is no CI; macOS and Linux are untested.

## Layout

| Path | What |
| --- | --- |
| `themes/basalt` | The `rune-basalt` crate |
| `examples/basalt-gallery` | The gallery |
| `tools/rune-link` | `cargo rune` |
| `.cargo/config.toml` | The `cargo rune` alias |

## License

GPL-3.0-or-later, see [LICENSE](LICENSE). The bundled fonts keep their own SIL OFL licences in `themes/basalt/assets/fonts/`.
