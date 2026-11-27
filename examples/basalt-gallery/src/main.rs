//! Every Basalt component on one scrolling page, grouped by type, live: switch the theme,
//! density and accent from the first section and everything restyles.
//!
//! `cargo run -p basalt-gallery`. Ctrl+K opens the command palette. Headless:
//! `BASALT_OPEN=palette|menu|dialog|toast|select|menubar` opens that state first, and
//! `BASALT_THEME=light`, `BASALT_DENSITY=0|1|2`, `BASALT_ACCENT=0..4` pick the look:
//! `cargo run -p basalt-gallery -- --snap out.png --size 1280x7600 --settle`.

#![forbid(unsafe_code)]

use std::time::Duration;

use rune::{App, AppSpec, Color, Cx, El, KeyEvent, Mods, ScrollSpec, TextEdit, TextStyle, Tx, WindowOptions, col, el, row, text};
use rune_basalt::app::{self, Cell, Column, ComboState, ColorState, Command, Date, DateState, Dock, Kind, Level, Menu, MenuBarState, MenuItem, Node, PaletteState, PaneSize, ReorderState, SelectState, Table, TableState, TagState, ToastQueue, TreeState};
use rune_basalt::icons::oc;
use rune_basalt::plot::{self, Plot, PlotState, Series};
use rune_basalt::{Theme, set_theme, tokens, ty};

const SIZE: (f32, f32) = (1280.0, 860.0);
const W: f32 = 1120.0;
const ACCENTS: [&str; 5] = ["#ff3e30", "#2563eb", "#16a34a", "#d97706", "#9333ea"];

#[derive(Clone)]
struct DataRow {
    id: String,
    name: String,
    value: u32,
    corr: f32,
}

struct State {
    clock: f64,
    // theme
    dark: bool,
    density: usize,
    accent: usize,
    // actions
    toggled: bool,
    more_open: bool,
    view: usize,
    clicks: u32,
    // inputs
    name: TextEdit,
    number: TextEdit,
    number_value: f64,
    notes: TextEdit,
    model: SelectState,
    model_choice: usize,
    combo: ComboState,
    combo_pick: Option<usize>,
    tags: TagState,
    date: DateState,
    color: ColorState,
    file: Option<String>,
    level: f32,
    stepper: i32,
    check: bool,
    sw: bool,
    radio: usize,
    // structure
    menubar: MenuBarState,
    last_menu: String,
    tree: TreeState,
    tab: usize,
    crumbs: usize,
    side: PaneSize,
    top: PaneSize,
    dock_collapsed: bool,
    dock_right: bool,
    dock_open: bool,
    // data
    rows: Vec<DataRow>,
    table: TableState,
    ctx: Option<String>,
    order: Vec<String>,
    reorder: ReorderState,
    filter: usize,
    // charts
    trace_a: Vec<f32>,
    trace_b: Vec<f32>,
    decay: Vec<f32>,
    points: Vec<(f32, f32)>,
    heat: Vec<f32>,
    plot: PlotState,
    plot_log: PlotState,
    // overlays
    palette: bool,
    pal: PaletteState,
    dialog: bool,
    toasts: ToastQueue,
}

const COLS: [Column; 5] = [Column::new("Key byte", 150.0), Column::new("Trend", 110.0), Column::new("Guess", 90.0).right(), Column::new("Correlation", 0.0), Column::new("Status", 120.0)];

fn noise(i: u32, seed: u32) -> f32 {
    let mut h = i.wrapping_mul(0x9E37_79B1) ^ seed.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 32767.5 - 1.0
}

fn trace(seed: u32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            0.15 + (t * 160.0).sin() * 0.05 + (0..10).map(|r| (-((t - 0.1 - r as f32 * 0.085) * 70.0).powi(2)).exp() * 0.45).sum::<f32>() + noise(i as u32, seed) * 0.04
        })
        .collect()
}

fn init() -> State {
    let open = std::env::var("BASALT_OPEN").unwrap_or_default();
    let rows: Vec<DataRow> = (0..1200)
        .map(|i| {
            let corr = (0.5 + noise(i, 3) * 0.5).clamp(0.02, 0.99);
            DataRow { id: format!("k{i:04}"), name: format!("k[{:02}] · 0x{:02x}", i % 16, (noise(i, 9) * 127.0 + 128.0) as u8), value: (noise(i, 5) * 127.0 + 128.0) as u32, corr }
        })
        .collect();
    let mut table = TableState::new(&COLS);
    table.selected = vec!["k0002".into()];
    table.anchor = Some("k0002".into());
    State {
        clock: 0.0,
        dark: std::env::var("BASALT_THEME").unwrap_or_default() != "light",
        density: std::env::var("BASALT_DENSITY").ok().and_then(|d| d.parse().ok()).unwrap_or(1).min(2),
        accent: std::env::var("BASALT_ACCENT").ok().and_then(|d| d.parse().ok()).unwrap_or(0).min(ACCENTS.len() - 1),
        toggled: true,
        more_open: false,
        view: 0,
        clicks: 0,
        name: TextEdit::new("campaign-07"),
        number: TextEdit::new("29.5"),
        number_value: 29.5,
        notes: TextEdit::new(""),
        model: SelectState { open: open == "select", highlight: 0 },
        model_choice: 0,
        combo: ComboState::new(""),
        combo_pick: None,
        tags: TagState::new(&["aes", "stm32"]),
        date: DateState::new(Date::new(2026, 11, 27)),
        color: ColorState::new(4.0, 0.81, 1.0),
        file: None,
        level: 0.62,
        stepper: 12,
        check: true,
        sw: true,
        radio: 1,
        menubar: MenuBarState { open: (open == "menubar").then_some(1), highlight: 1 },
        last_menu: String::new(),
        tree: TreeState { expanded: vec!["c7".into()], selected: Some("a".into()) },
        tab: 0,
        crumbs: 2,
        side: PaneSize::new(180.0, 110.0, 340.0),
        top: PaneSize::new(90.0, 50.0, 170.0),
        dock_collapsed: false,
        dock_right: false,
        dock_open: true,
        rows,
        table,
        ctx: (open == "menu").then(|| "k0002".to_owned()),
        order: ["Align traces", "Filter noise", "Normalise", "Run CPA", "Export report"].map(String::from).to_vec(),
        reorder: ReorderState::default(),
        filter: 0,
        trace_a: trace(1, 4000),
        trace_b: trace(2, 4000),
        decay: (0..400).map(|i| 1.0e4 * (-(i as f32) / 60.0).exp() + 3.0 + noise(i as u32, 4).abs() * 2.0).collect(),
        points: (0..500).map(|i| (noise(i, 1) * 0.5 + 0.5, (noise(i, 2) * 0.35 + 0.5 + (noise(i, 1) * 0.15)).clamp(0.0, 1.0))).collect(),
        heat: (0..16 * 16).map(|i| { let (r, c) = ((i / 16) as f32, (i % 16) as f32); (-(((r - 9.0).powi(2) + (c - 5.0).powi(2)) / 18.0)).exp() + noise(i as u32, 8).abs() * 0.12 }).collect(),
        plot: PlotState { cursor: Some(0.42), ..PlotState::default() },
        plot_log: PlotState::default(),
        palette: open == "palette",
        pal: PaletteState::new(),
        dialog: open == "dialog",
        toasts: {
            let mut q = ToastQueue::default();
            if open == "toast" {
                q.push(0.0, Level::Ok, "Attack finished", "14 of 16 key bytes recovered in 3.2 s.", Some("Open report"));
                q.push(0.0, Level::Err, "Byte 11 did not converge", "Peak correlation 0.21 is below the 0.40 threshold.", None);
            }
            q
        },
    }
}

fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    s.clock = now;
    let changed = s.toasts.expire(now);
    if let Some(due) = s.toasts.next_expiry() {
        tx.wake_at(due);
    }
    changed
}

fn on_key(s: &mut State, k: &KeyEvent, _: &mut Tx) -> bool {
    if k.is_command('k') {
        s.palette = !s.palette;
        s.pal = PaletteState::new();
        return true;
    }
    if k.key == rune::Key::Escape && s.palette {
        s.palette = false;
        return true;
    }
    false
}

// ---- page scaffolding -------------------------------------------------------------------------

fn section(n: usize, name: &str, note: &str, body: El) -> El {
    let s = tokens();
    col().w(W)
        .gap(28.0)
        .pt(64.0)
        .child(
            row()
                .w_full()
                .pb(14.0)
                .gap(16.0)
                .items_end()
                .border_sides([0.0, 0.0, 1.0, 0.0], s.line)
                .child(text(format!("{n:02}")).font(ty::META).color(s.dim))
                .child(text(name).font(ty::H3).color(s.text))
                .child(el().grow(1.0))
                .child(text(note).font(ty::META).color(s.muted)),
        )
        .child(body)
}

fn spec(label: &str, body: El) -> El {
    col().gap(12.0).child(text(label.to_uppercase()).font(ty::STATUS.tracking(1.1)).color(tokens().dim)).child(body)
}

fn framed(label: &str, w: f32, body: El) -> El {
    let s = tokens();
    spec(label, col().w(w).radius(12.0).border(1.0, s.line).clip_rounded(12.0).fill(s.panel).child(body))
}

fn grid() -> El {
    row().w_full().gap(40.0).flex_wrap().items_start()
}

fn w300(e: El) -> El {
    col().w(300.0).child(e)
}

// ---- sections ---------------------------------------------------------------------------------

fn foundations(s: &State) -> El {
    let tk = tokens();
    let swatch = |name: &str, c: Color| col().w(92.0).gap(8.0).child(el().size(92.0, 52.0).radius(10.0).fill(c).border(1.0, tk.hair)).child(text(name).font(app::TINY).color(tk.muted));
    let colors = row()
        .gap(12.0)
        .flex_wrap()
        .child(swatch("bg", tk.bg))
        .child(swatch("panel", tk.panel))
        .child(swatch("float", tk.float))
        .child(swatch("line", tk.line))
        .child(swatch("dim", tk.dim))
        .child(swatch("muted", tk.muted))
        .child(swatch("text", tk.text))
        .child(swatch("accent", tk.accent))
        .child(swatch("ok", tk.ok))
        .child(swatch("warn", tk.warn));
    let mut accents = row().gap(8.0).items_center();
    for (i, hex) in ACCENTS.iter().enumerate() {
        let c = rune::color::hex(hex).unwrap();
        let mut b = el().size(22.0, 22.0).radius(11.0).fill(c).cursor(rune::Cursor::Pointer).role("radio").label(*hex).checked(s.accent == i).focusable(true).on_press(move |s: &mut State| s.accent = i);
        if s.accent == i {
            b = b.border(3.0, tk.panel).box_shadow(rune::Shadow { x: 0.0, y: 0.0, blur: 0.0, spread: 2.0, color: c, inset: false });
        }
        accents = accents.child(b);
    }
    let controls = row()
        .gap(32.0)
        .flex_wrap()
        .items_end()
        .child(spec("Theme", app::segmented(&["Dark", "Light"], if s.dark { 0 } else { 1 }, |s: &mut State, i| s.dark = i == 0)))
        .child(spec("Density", app::segmented(&["Compact", "Regular", "Roomy"], s.density, |s: &mut State, i| s.density = i)))
        .child(spec("Accent", accents));
    let t = |label: &str, style: TextStyle, sample: &str| row().w_full().gap(24.0).items_center().py(8.0).border_sides([0.0, 0.0, 1.0, 0.0], tk.line).child(text(label).font(app::TINY).color(tk.dim).w(170.0)).child(text(sample).font(style).color(tk.text));
    let types = col()
        .w_full()
        .child(t("FIGURE · grotesk 22", app::FIGURE, "0.912"))
        .child(t("TITLE · grotesk 14", app::TITLE, "Key candidates"))
        .child(t("UI_STRONG · mono 12.5", app::UI_STRONG, "Run attack"))
        .child(t("UI · mono 12.5", app::UI, "Hamming weight"))
        .child(t("TINY · mono 11", app::TINY, "14:02:11  fs 29.5 MS/s"))
        .child(t("CAPTION · mono 10.5 caps", app::CAPTION, "PREPROCESSING"));
    col().gap(32.0).child(controls).child(spec("Colours", colors)).child(spec("Type", types))
}

fn actions(s: &State, layers: &mut Vec<El>) -> El {
    let tk = tokens();
    let buttons = row()
        .gap(10.0)
        .items_center()
        .child(app::button("Run attack", Kind::Primary, Some(&oc::FLASK), Some("Ctrl R"), |s: &mut State| s.clicks += 1))
        .child(app::button("Export", Kind::Secondary, Some(&oc::DOWNLOAD), None, |_: &mut State| {}))
        .child(app::button("Cancel", Kind::Ghost, None, Some("Esc"), |_: &mut State| {}))
        .child(app::button("Delete", Kind::Danger, Some(&oc::TRASH), None, |_: &mut State| {}));
    let off = app::disabled(true, || {
        row()
            .gap(10.0)
            .items_center()
            .child(app::button("Run attack", Kind::Primary, Some(&oc::FLASK), None, |_: &mut State| {}))
            .child(app::button("Export", Kind::Secondary, None, None, |_: &mut State| {}))
            .child(app::icon_button(&oc::COPY, "Copy", false, |_: &mut State| {}))
            .child(app::switch("Align", true, |_: &mut State| {}).w(120.0))
            .child(app::checkbox("Normalise", false, |_: &mut State| {}))
    });
    let icons = row()
        .gap(4.0)
        .child(app::icon_button(&oc::SIDEBAR, "Sidebar (toggled)", s.toggled, |s: &mut State| s.toggled = !s.toggled))
        .child(app::icon_button(&oc::CONSOLE, "Console", false, |_: &mut State| {}))
        .child(app::icon_button(&oc::COPY, "Copy", false, |_: &mut State| {}))
        .child(app::icon_button(&oc::SETTINGS_GEAR, "Settings", false, |_: &mut State| {}));
    let toolbar = app::toolbar(vec![
        app::tool(&oc::WAVEFORM, "Acquire", false, |_: &mut State| {}),
        app::tool(&oc::SLIDERS, "Align", s.toggled, |s: &mut State| s.toggled = !s.toggled),
        app::tool_sep(),
        app::segmented(&["Traces", "Correlation", "Both"], s.view, |s: &mut State, i| s.view = i),
    ]);
    let tools = vec![
        app::ToolItem::new(&oc::WAVEFORM, "Acquire", |_: &mut State| {}),
        app::ToolItem::new(&oc::SLIDERS, "Align", |s: &mut State| s.toggled = !s.toggled).active(s.toggled),
        app::ToolItem::new(&oc::OUTLINE_STEER, "Filter", |_: &mut State| {}),
        app::ToolItem::new(&oc::FLASK, "Attack", |s: &mut State| s.clicks += 1),
        app::ToolItem::new(&oc::DOWNLOAD, "Export", |_: &mut State| {}),
        app::ToolItem::new(&oc::SETTINGS_GEAR, "Settings", |_: &mut State| {}),
    ];
    let overflow = framed("", 400.0, row().padding(6.0).child(app::overflow_toolbar("tools", tools, 388.0, s.more_open, |s: &mut State| &mut s.more_open).mount(layers)));
    let keys = row()
        .gap(6.0)
        .items_center()
        .child(app::kbd("Ctrl"))
        .child(app::kbd("K"))
        .child(el().w(10.0))
        .child(app::kbd("Esc"))
        .child(app::kbd("↵"))
        .child(app::kbd("↑↓"))
        .child(el().w(10.0))
        .child(app::shortcut("Ctrl+O"));
    let _ = tk;
    grid()
        .child(spec(&format!("Buttons · primary, secondary, ghost, danger  ({} presses)", s.clicks), buttons))
        .child(spec("Disabled · app::disabled(true, || …)", off))
        .child(spec("Icon buttons · first one toggled · hover for tips", icons))
        .child(spec("Toolbar · arrow keys move between tools", toolbar))
        .child(spec("Toolbar that overflows into a menu when narrow", overflow))
        .child(spec("Keycaps", keys))
}

fn inputs(s: &State, layers: &mut Vec<El>) -> El {
    let model = app::select("model", &["Hamming weight", "Hamming distance", "Identity", "Bit 0"], s.model_choice, &s.model, |s: &mut State| &mut s.model, |s: &mut State, i| s.model_choice = i).mount(layers);
    let combo = app::combobox("fruit", &["Apple", "Apricot", "Banana", "Cherry", "Durian", "Elderberry"], "Type to filter…", &s.combo, |s: &mut State| &mut s.combo, |s: &mut State, i| s.combo_pick = Some(i)).mount(layers);
    let date = app::date_picker("date", &s.date, |s: &mut State| &mut s.date).mount(layers);
    let color = app::color_picker("color", &s.color, |s: &mut State| &mut s.color).mount(layers);
    let bad = s.name.text.trim().is_empty();
    let name = app::invalid(bad, || app::text_field(&s.name, "Name", None, None, |s: &mut State| &mut s.name));
    let off_field = app::disabled(true, || app::text_field(&s.name, "Name", None, None, |s: &mut State| &mut s.name));
    let off_select = app::disabled(true, || app::select("model-off", &["Hamming weight"], 0, &SelectState::default(), |s: &mut State| &mut s.model, |_: &mut State, _| {}).trigger);
    let range = s.number.text.trim().parse::<f64>().ok();
    grid()
        .child(spec("Text field · error state when empty", w300(app::field_msg("Campaign", name, Some("Letters, digits and dashes."), bad.then_some("A name is required.")))))
        .child(spec("Error state · app::invalid(true, || …)", w300(app::field_msg("Sample rate", app::invalid(true, || app::text_field(&TextEdit::new("-3"), "0", None, Some("MS/s"), |s: &mut State| &mut s.name)), None, Some("Must be between 1 and 100.")))))
        .child(spec("Disabled field and select", w300(col().gap(10.0).child(app::field("Campaign", off_field)).child(app::field("Model", off_select)))))
        .child(spec("Number field · type, ↑↓ or ±, clamps to 1–100", w300(app::field("Sample rate", app::number_field(&s.number, "MS/s", 1.0, 100.0, 0.5, |s: &mut State| &mut s.number, |s: &mut State, v| s.number_value = v)))))
        .child(spec("Select · ↑↓ Enter Esc", w300(app::field("Leakage model", model))))
        .child(spec(&format!("Combobox · filters as you type ({})", s.combo_pick.map_or("none".to_owned(), |i| format!("#{i}"))), w300(app::field("Fruit", combo))))
        .child(spec("Tag input · Enter or comma adds, Backspace removes", w300(app::field("Tags", app::tag_input("tags", "Add a tag…", &s.tags, |s: &mut State| &mut s.tags)))))
        .child(spec("Date picker", w300(app::field("Date", date))))
        .child(spec("Colour picker · drag the square and the hue bar", w300(app::field("Colour", color))))
        .child(spec("File field · opens the system dialog", w300(app::field("Trace file", app::file_field(s.file.as_deref(), "No file chosen", |s: &mut State, paths| s.file = paths.first().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()))))))
        .child(spec("Text area", w300(app::field("Notes", app::textarea(&s.notes, "Notes about this campaign…", 3, |s: &mut State| &mut s.notes)))))
        .child(spec("Stepper and slider · focus + arrow keys", col().w(300.0).gap(14.0).child(app::field("Traces used", app::stepper(&format!("{},000", s.stepper), "traces", |s: &mut State, d| s.stepper = (s.stepper + d).clamp(1, 50)))).child(app::field("Gain", app::slider(s.level, &format!("{:.0} dB", s.level * 50.0), 300.0, |s: &mut State, v| s.level = v)))))
        .child(spec(
            "Checkbox · switch · radio",
            col()
                .w(300.0)
                .gap(12.0)
                .child(app::checkbox("Normalise per trace", s.check, |s: &mut State| s.check = !s.check))
                .child(app::switch("Align traces", s.sw, |s: &mut State| s.sw = !s.sw))
                .child(app::radio(&["None", "Mean of 4", "Median of 4"], s.radio, |s: &mut State, i| s.radio = i))
                .child(text(format!("number field holds {:?}", range)).font(app::TINY).color(tokens().dim)),
        ))
}

fn structure(s: &State, layers: &mut Vec<El>) -> El {
    let tk = tokens();
    let bar = app::menu_bar(
        vec![
            Menu::new("File", vec![Some(MenuItem::new("New campaign").shortcut("Ctrl N")), Some(MenuItem::new("Open…").shortcut("Ctrl O")), Some(MenuItem::new("Save").shortcut("Ctrl S")), None, Some(MenuItem::new("Quit").danger())]),
            Menu::new("Edit", vec![Some(MenuItem::new("Undo").shortcut("Ctrl Z")), Some(MenuItem::new("Redo").shortcut("Ctrl Y").disabled()), None, Some(MenuItem::new("Copy").shortcut("Ctrl C")), Some(MenuItem::new("Paste").shortcut("Ctrl V"))]),
            Menu::new("View", vec![Some(MenuItem::new("Toggle sidebar").shortcut("Ctrl B")), Some(MenuItem::new("Toggle console").shortcut("Ctrl J")), Some(MenuItem::new("Zoom in")), Some(MenuItem::new("Zoom out"))]),
            Menu::new("Help", vec![Some(MenuItem::new("Documentation")), Some(MenuItem::new("About"))]),
        ],
        &s.menubar,
        |s: &mut State| &mut s.menubar,
        |s: &mut State, menu, entry| s.last_menu = format!("menu {menu}, entry {entry}"),
    )
    .mount(layers);
    let bar = row().w_full().h(40.0).px(10.0).items_center().fill(tk.bg).border_sides([0.0, 0.0, 1.0, 0.0], tk.line).child(bar).child(el().grow(1.0)).child(text(if s.last_menu.is_empty() { "pick an entry".to_owned() } else { s.last_menu.clone() }).font(app::TINY).color(tk.dim));

    let nodes = vec![
        Node::branch("c7", "campaign-07", vec![Node::leaf("a", "traces-a.npy").level(Level::Ok), Node::leaf("b", "traces-b.npy").level(Level::Ok), Node::leaf("g", "fault-glitch.npy").level(Level::Warn)]),
        Node::branch("c6", "campaign-06", vec![Node::leaf("e", "em-probe.npy").level(Level::Err)]),
    ];
    let tree = col().w(240.0).padding(8.0).child(app::tree(&nodes, &s.tree, |s: &mut State| &mut s.tree, |_: &mut State, _| {}));
    let tabs = app::doc_tabs(&[("traces-a.npy", true), ("cpa-byte-03", false), ("report.md", false)], s.tab, |s: &mut State, i| s.tab = i, |_: &mut State, _| {});
    let crumbs = app::breadcrumbs(&["campaign-07", "traces-a.npy", "round 1"][..=s.crumbs.min(2)], |s: &mut State, i| s.crumbs = i);

    let panes = {
        let right = col()
            .grow(1.0)
            .h_full()
            .child(col().w_full().h(s.top.size).padding(12.0).child(text(format!("top {:.0} pt", s.top.size)).font(app::UI).color(tk.muted)))
            .child(app::splitter_h(|s: &mut State, dy, start| s.top.drag(dy, start, 1.0)))
            .child(col().w_full().grow(1.0).padding(12.0).child(text("Drag either divider").font(app::UI).color(tk.muted)));
        row().w_full().h(210.0).child(col().w(s.side.size).h_full().fill(tk.bg).padding(12.0).child(text(format!("{:.0} pt", s.side.size)).font(app::UI).color(tk.muted))).child(app::splitter(|s: &mut State, dx, start| s.side.drag(dx, start, 1.0))).child(right)
    };

    let dock_body = col().padding(12.0).gap(8.0).child(app::prop("Target", app::value("STM32F415")).w_full()).child(app::prop("Clock", app::value("7.37 MHz")).w_full());
    let main = col().grow(1.0).h_full().padding(14.0).gap(8.0).child(text("Main area").font(app::TITLE).color(tk.text)).child(text("The panel docks left or right, collapses to its header, or closes.").font(app::UI).color(tk.muted).wrap());
    let dock_el = col().w(250.0).padding(8.0).child(app::dock_panel("Inspector", s.dock_collapsed, dock_body, |s: &mut State, a| match a {
        Dock::Collapse => s.dock_collapsed = !s.dock_collapsed,
        Dock::Flip => s.dock_right = !s.dock_right,
        Dock::Close => s.dock_open = false,
    }));
    let docked = if !s.dock_open {
        row().w_full().h(150.0).child(main).child(col().padding(14.0).child(app::button("Reopen inspector", Kind::Secondary, None, None, |s: &mut State| s.dock_open = true)))
    } else if s.dock_right {
        row().w_full().h(150.0).child(main).child(dock_el)
    } else {
        row().w_full().h(150.0).child(dock_el).child(main)
    };

    let lines = (1..=40).fold(col().gap(0.0).padding(10.0), |c, i| c.child(text(format!("scrollable line {i:02}")).font(app::UI).color(tk.muted)));
    let status = app::status_bar(
        vec![app::status_item(Some(Level::Ok), "ChipWhisperer connected"), app::status_item(None, "12,000 traces"), app::status_item(Some(Level::Warn), "CPA 42%")],
        vec![app::status_item(None, "sample 410"), app::status_item(None, "v0.4.2")],
    );
    let progress = col().w(320.0).gap(14.0).child(app::progress("Acquisition", 0.42)).child(app::busy("Capturing traces…"));
    let readouts = row().w(560.0).gap(10.0).child(app::readout("Traces", "12,000", Some(("+1,000 today", Level::Neutral)))).child(app::readout("SNR", "4.82 dB", Some(("↑ 0.6 dB", Level::Ok)))).child(app::readout("Key", "14 / 16", Some(("2 pending", Level::Warn))));
    let empty = col().w(360.0).h(200.0).child(app::empty_state(&oc::FILE_DOCUMENT, "No report yet", "Run an attack, then export its results as a report.", "Run attack", |_: &mut State| {}));
    grid()
        .child(framed("Menu bar · click, hover to switch, ←→↑↓ Enter Esc", W, bar))
        .child(framed("Tree · click, chevrons, ←→↑↓", 280.0, tree))
        .child(spec("Tabs and breadcrumbs", col().gap(14.0).child(framed("Document tabs", 520.0, tabs.child_pad())).child(crumbs)))
        .child(framed("Splitters · vertical and horizontal, clamped", W, panes))
        .child(framed("Docked panel · collapse, flip, close", W, docked))
        .child(framed("Scroll view with bar", 360.0, col().w_full().h(150.0).child(app::scroll_view("demo-scroll", lines))))
        .child(spec("Progress and busy", progress))
        .child(spec("Readout tiles", readouts))
        .child(framed("Empty state", 360.0, empty))
        .child(framed("Status bar", W, status))
}

fn data(s: &State, layers: &mut Vec<El>) -> El {
    let tk = tokens();
    let mut view: Vec<&DataRow> = s.rows.iter().collect();
    let (c, desc) = s.table.sort;
    view.sort_by(|a, b| {
        let o = match c {
            2 => a.value.cmp(&b.value),
            3 | 4 => a.corr.partial_cmp(&b.corr).unwrap(),
            _ => a.name.cmp(&b.name),
        };
        if desc { o.reverse() } else { o }
    });
    let rows: Vec<(String, Vec<Cell>)> = view
        .iter()
        .map(|r| {
            let (st, lv) = if r.corr > 0.6 { ("Recovered", Level::Ok) } else if r.corr > 0.4 { ("Ambiguous", Level::Warn) } else { ("Failed", Level::Err) };
            let spark: Vec<f32> = (0..12).map(|i| r.corr * (1.0 - (-(i as f32) / 4.0).exp()) + noise(i + r.value, 5) * 0.03).collect();
            (r.id.clone(), vec![Cell::Text(r.name.clone()), Cell::Custom(plot::sparkline(&spark, 80.0, 16.0, if r.corr > 0.6 { tk.text } else { tk.muted })), Cell::Num(format!("0x{:02x}", r.value)), Cell::Bar(r.corr), Cell::Badge(st.into(), lv)])
        })
        .collect();
    let table = Table::new(&COLS, rows).checkboxes().height(300.0).build("demo-table", &s.table, |s: &mut State| &mut s.table, |s: &mut State, id| s.ctx = Some(id.to_owned()));
    if let Some(id) = &s.ctx {
        layers.extend(app::context_menu(
            &format!("row-{id}"),
            &[Some(MenuItem::new("Copy key byte").shortcut("Ctrl C")), Some(MenuItem::new("Show correlation trace")), Some(MenuItem::new("Rerun on this byte").shortcut("Ctrl R")), None, Some(MenuItem::new("Mark as known").disabled()), Some(MenuItem::new("Delete…").danger())],
            |s: &mut State, i| {
                s.ctx = None;
                if i == 5 {
                    s.dialog = true;
                }
            },
            |s: &mut State| s.ctx = None,
        ));
    }
    let selected = s.table.selected.len();
    let mut log = vec![
        ("14:02:11".to_owned(), Level::Neutral, "Connected to ChipWhisperer-Lite, target STM32F415.".to_owned()),
        ("14:02:12".to_owned(), Level::Ok, "Clock locked at 7.37 MHz.".to_owned()),
        ("14:03:41".to_owned(), Level::Warn, "37 traces clipped the ADC range and were dropped.".to_owned()),
        ("14:04:31".to_owned(), Level::Err, "Byte 11 did not converge: peak correlation 0.21.".to_owned()),
    ];
    for i in 0..60 {
        log.push((format!("14:05:{:02}", i % 60), Level::Neutral, format!("Captured batch {i}.")));
    }
    let f = s.filter;
    let console = col().w_full().h(170.0).fill(tk.bg).child(app::pane_header("Console · 64 lines, follows the bottom", app::segmented(&["All", "Warnings", "Errors"], s.filter, |s: &mut State, i| s.filter = i))).child(app::log_view("demo-log", &log, move |l| match f {
        1 => matches!(l, Level::Warn | Level::Err),
        2 => l == Level::Err,
        _ => true,
    }));
    let items: Vec<&str> = s.order.iter().map(String::as_str).collect();
    let reorder = col().w(300.0).child(app::reorder_list("demo-reorder", &items, &s.reorder, |s: &mut State| &mut s.reorder, |s: &mut State, from, to| app::moved(&mut s.order, from, to)));
    let props = col().w(320.0).child(app::prop("Target", app::value("STM32F415RG"))).child(app::prop("Clock", app::value("7.37 MHz"))).child(app::prop("Trigger", app::badge("armed", Level::Ok)));
    let badges = row().gap(8.0).child(app::badge("ready", Level::Ok)).child(app::badge("running", Level::Warn)).child(app::badge("failed", Level::Err)).child(app::badge("AES-128", Level::Neutral)).child(app::dot(Level::Ok));
    grid()
        .child(framed(&format!("Table · 1,200 rows, windowed · click, Ctrl/Shift-click, ↑↓, Shift+↑↓, Ctrl+A, right-click, sort, drag column edges · {selected} selected"), W, table))
        .child(framed("Log view", W, console))
        .child(spec("Reorderable list · drag a row", reorder))
        .child(spec("Property grid", props))
        .child(spec("Badges and dots", badges))
}

fn charts(s: &State) -> El {
    let tk = tokens();
    let main = Plot {
        series: vec![Series::new("traces-a #482", &s.trace_b, tk.muted.alpha(0.6)), Series::new("mean", &s.trace_a, tk.text), Series { values: &s.decay[..0], color: tk.accent, width: 1.0, label: "" }],
        x_domain: (0.0, 3999.0),
        x_label: "sample",
        y_range: None,
        log_y: false,
        window: Some((0.36, 0.48)),
        legend: true,
    }
    .build(W - 28.0, 260.0, &s.plot, |s: &mut State| &mut s.plot);
    let log = Plot { series: vec![Series::new("leakage power", &s.decay, tk.accent)], x_domain: (0.0, 399.0), x_label: "trace index", y_range: None, log_y: true, window: None, legend: true }.build(540.0, 200.0, &s.plot_log, |s: &mut State| &mut s.plot_log);
    let scatter = plot::scatter(&s.points, &[3, 40, 77], (0.0, 1.0), (0.0, 1.0), 420.0, 200.0);
    let heat = plot::heatmap(&s.heat, 16, Some((9, 5)), 260.0, 200.0);
    let spark: Vec<f32> = (0..32).map(|i| 0.9 * (1.0 - (-(i as f32) / 7.0).exp()) + (i as f32 * 1.3).sin() * 0.03).collect();
    let hist: Vec<f32> = (0..32).map(|i| 0.15 + ((i * 37 % 17) as f32 / 17.0) * 0.4).collect();
    grid()
        .child(framed("Line plot · wheel zooms, Shift+wheel pans, drag moves the cursor, double-click resets · 4,000 samples", W, col().padding(14.0).child(main)))
        .child(framed("Log scale · axis from the data", 570.0, col().padding(14.0).child(log)))
        .child(framed("Scatter", 450.0, col().padding(14.0).child(scatter)))
        .child(framed("Heat map · peak in the accent", 290.0, col().padding(14.0).child(heat)))
        .child(spec("Sparklines", row().gap(24.0).child(plot::sparkline(&spark, 140.0, 28.0, tk.text)).child(plot::sparkline(&s.trace_a[..120], 140.0, 28.0, tk.muted)).child(plot::sparkline(&s.decay[..120], 140.0, 28.0, tk.accent))))
        .child(spec("Histogram", plot::bars(&hist, Some(11), 320.0, 72.0)))
}

const COMMANDS: [(&str, &str, Option<&str>); 6] = [("Toggle dark / light", "View", Some("Ctrl T")), ("Next accent colour", "View", None), ("Run CPA attack", "Attack", Some("Ctrl R")), ("Reset plot zoom", "Attack", None), ("Show a notification", "Help", None), ("Delete campaign", "File", None)];

fn commands() -> Vec<Command<'static>> {
    let glyphs: [&'static rune::Icon; 6] = [&oc::APPEARANCE, &oc::SLIDERS, &oc::FLASK, &oc::OUTLINE_MAGNIFIER_MINUS, &oc::NOTIFICATIONS, &oc::TRASH];
    COMMANDS.iter().zip(glyphs).map(|((label, group, shortcut), glyph)| Command { glyph, label, group, shortcut: *shortcut }).collect()
}

fn run_command(s: &mut State, i: usize) {
    s.palette = false;
    match i {
        0 => s.dark = !s.dark,
        1 => s.accent = (s.accent + 1) % ACCENTS.len(),
        2 => {
            let now = s.clock;
            s.toasts.push(now, Level::Ok, "Attack finished", "14 of 16 key bytes recovered in 3.2 s.", Some("Open report"));
        }
        3 => s.plot.reset(),
        4 => {
            let now = s.clock;
            s.toasts.push(now, Level::Neutral, "Heads up", "This one disappears after five seconds.", None);
        }
        _ => s.dialog = true,
    }
}

fn overlays(s: &State, layers: &mut Vec<El>) -> El {
    let tk = tokens();
    let cmds = commands();
    let static_state = PaletteState { edit: TextEdit::new(""), highlight: 0 };
    let palette = app::palette_panel(&cmds, &static_state, false, |s: &mut State| &mut s.pal, |_: &mut State, _| {}, |_: &mut State| {});
    let items = [Some(MenuItem::new("Copy key byte").shortcut("Ctrl C")), Some(MenuItem::new("Show correlation")), None, Some(MenuItem::new("Mark as known").disabled()), Some(MenuItem::new("Delete…").danger())];
    let menu = app::menu_panel(&items, Some(0), |_: &mut State, _| {});
    let dialog = app::dialog_panel("Delete campaign-07?", "This deletes 12,000 traces and every attack result. It can't be undone.", row().gap(8.0).child(app::button("Cancel", Kind::Ghost, None, None, |_: &mut State| {})).child(app::button("Delete", Kind::Danger, Some(&oc::TRASH), None, |_: &mut State| {})));
    let toast = app::toast(Level::Ok, "Attack finished", "14 of 16 key bytes recovered in 3.2 s.", Some("Open report"), |_: &mut State| {}, |_: &mut State| {});
    let tip = row().gap(12.0).items_center().child(app::icon_button(&oc::SIDEBAR, "Hover me for Rune's own tooltip", false, |_: &mut State| {})).child(app::tooltip_bubble("Toggle sidebar", Some("Ctrl B")));
    let live = row()
        .gap(10.0)
        .flex_wrap()
        .child(row().key("open-palette").child(app::button("Open palette", Kind::Secondary, Some(&oc::MAGNIFYING_GLASS), Some("Ctrl K"), |s: &mut State| {
            s.palette = true;
            s.pal = PaletteState::new();
        })))
        .child(row().key("open-dialog").child(app::button("Open dialog", Kind::Secondary, None, None, |s: &mut State| s.dialog = true)))
        .child(app::button("Show toast", Kind::Secondary, None, None, |s: &mut State| {
            let now = s.clock;
            s.toasts.push(now, Level::Ok, "Saved", "campaign-07 saved. Goes away on its own.", None);
        }))
        .child(app::button("Show error toast", Kind::Secondary, None, None, |s: &mut State| {
            let now = s.clock;
            s.toasts.push(now, Level::Err, "Byte 11 did not converge", "Errors stay until dismissed.", None);
        }));
    if s.palette {
        layers.extend(app::command_palette(&cmds, &s.pal, |s: &mut State| &mut s.pal, run_command, |s: &mut State| s.palette = false));
    }
    if s.dialog {
        layers.push(
            app::dialog(
                "Delete campaign-07?",
                "This deletes 12,000 traces and every attack result. It can't be undone. Focus stays inside; Esc cancels and returns focus to the button.",
                row().gap(8.0).child(app::button("Cancel", Kind::Ghost, None, Some("Esc"), |s: &mut State| s.dialog = false)).child(app::button("Delete", Kind::Danger, Some(&oc::TRASH), None, |s: &mut State| s.dialog = false)),
                |s: &mut State| s.dialog = false,
            )
            .focus_return("open-dialog"),
        );
    }
    if !s.toasts.items.is_empty() {
        layers.push(app::toast_stack(&s.toasts, SIZE, 0.0, |s: &mut State| &mut s.toasts, |s: &mut State, _| s.tab = 2));
    }
    let _ = tk;
    grid()
        .child(spec("Open them for real · or press Ctrl K", live))
        .child(spec("Command palette · type, ↑↓, Enter, Esc", palette))
        .child(spec("Context menu · also: right-click a table row", menu))
        .child(spec("Dialog · modal, focus trapped", dialog))
        .child(spec("Toast", toast))
        .child(spec("Tooltip", tip))
}

// ---- view -------------------------------------------------------------------------------------

fn view(s: &State, _: &mut Cx<State>) -> El {
    let mut theme = if s.dark { Theme::dark() } else { Theme::light() };
    theme = theme.density([0.85, 1.0, 1.15][s.density]).accent(rune::color::hex(ACCENTS[s.accent]).unwrap());
    set_theme(theme);
    let tk = tokens();
    let mut layers = Vec::new();
    let intro = col()
        .w(W)
        .gap(10.0)
        .pt(48.0)
        .child(text("BASALT").font(ty::STATUS.tracking(1.3)).color(tk.muted))
        .child(text("Basalt components").font(ty::H2).color(tk.text))
        .child(text("Every control, pane, data view, chart and overlay, grouped by type. Live: they respond to the mouse and the keyboard.").font(ty::BODY).color(tk.muted).wrap());
    let page = col()
        .w_full()
        .items_center()
        .pb(80.0)
        .child(intro)
        .child(section(1, "Foundations", "theme, density, accent, colours, type", foundations(s)))
        .child(section(2, "Actions", "buttons, toolbar, keycaps, disabled", actions(s, &mut layers)))
        .child(section(3, "Inputs", "fields, pickers, toggles, error and disabled", inputs(s, &mut layers)))
        .child(section(4, "Structure", "menu bar, tree, tabs, panes, docks, scroll", structure(s, &mut layers)))
        .child(section(5, "Data", "table, log, reorderable list, properties", data(s, &mut layers)))
        .child(section(6, "Charts", "plots, zoom, log scale, scatter, heat map", charts(s)))
        .child(section(7, "Overlays", "palette, menus, dialog, toasts, tooltips", overlays(s, &mut layers)));
    let base = col().w_full().h_full().fill(tk.panel).child(col().key("page").w_full().grow(1.0).scroll(ScrollSpec::default()).child(page));
    rune_kit::root(base, layers)
}

fn on_modifiers(s: &mut State, m: Mods, _: &mut Tx) {
    s.table.mods = m;
}

fn main() {
    App::new(AppSpec { min_size: Some(SIZE), fonts: rune_basalt::FONTS, poll: Some(poll), on_key: Some(on_key), ..AppSpec::new("Basalt · components", SIZE, init, view) })
        .modifier_keys()
        .on_modifiers(on_modifiers)
        .window_options(WindowOptions { resizable: false, min_size: Some(SIZE), max_size: Some(SIZE), ..WindowOptions::default() })
        .run();
}

trait ChildPad {
    fn child_pad(self) -> El;
}

impl ChildPad for El {
    /// A tab strip needs a little body under it to read as a frame.
    fn child_pad(self) -> El {
        col().child(self).child(el().h(0.0))
    }
}

#[allow(dead_code)]
fn _duration() -> Duration {
    Duration::from_secs(0)
}

#[cfg(test)]
mod tests {
    //! The gallery driven through Rune's runtime: real clicks, keys and wheel events, then
    //! the app's state checked. The window is made tall so everything is on screen.

    use super::*;
    use rune::{Env, Key, Runtime};

    fn lab() -> Runtime<State> {
        let mut rt = Runtime::new(init(), view, (1280.0, 9600.0), Env { system_dark: true, scale: 1.0 });
        settle(&mut rt);
        rt
    }

    fn settle(rt: &mut Runtime<State>) {
        for _ in 0..4 {
            rt.frame(1.0 / 60.0);
        }
    }

    fn key(rt: &mut Runtime<State>, key: Key, shift: bool, cmd: bool) {
        rt.key(KeyEvent { key, mods: Mods { shift, cmd, ctrl: cmd, ..Mods::default() } });
        settle(rt);
    }

    fn typed(rt: &mut Runtime<State>, s: &str) {
        for ch in s.chars() {
            key(rt, Key::Text(ch.to_string()), false, false);
        }
    }

    fn click(rt: &mut Runtime<State>, id: &str) {
        let (x, y) = rt.find(id).unwrap_or_else(|| panic!("no element keyed {id:?}"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        settle(rt);
    }

    #[test]
    fn select_opens_moves_and_picks_with_the_keyboard() {
        let mut rt = lab();
        click(&mut rt, "model");
        assert!(rt.state.model.open, "a press opens the list");
        key(&mut rt, Key::Down, false, false);
        key(&mut rt, Key::Down, false, false);
        assert_eq!(rt.state.model.highlight, 2);
        key(&mut rt, Key::Enter, false, false);
        assert_eq!(rt.state.model_choice, 2, "Enter picks the highlighted option");
        assert!(!rt.state.model.open, "and closes the list");
    }

    #[test]
    fn menu_bar_switches_menus_and_closes() {
        let mut rt = lab();
        click(&mut rt, "menubar-0");
        assert_eq!(rt.state.menubar.open, Some(0));
        key(&mut rt, Key::Right, false, false);
        assert_eq!(rt.state.menubar.open, Some(1), "Right opens the next menu");
        key(&mut rt, Key::Down, false, false);
        assert_eq!(rt.state.menubar.highlight, 1);
        key(&mut rt, Key::Escape, false, false);
        assert_eq!(rt.state.menubar.open, None, "Escape closes it");
    }

    #[test]
    fn table_selects_with_mouse_and_keyboard() {
        let mut rt = lab();
        let mut sorted: Vec<&DataRow> = rt.state.rows.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let (first, second) = (sorted[0].id.clone(), sorted[1].id.clone());
        click(&mut rt, &format!("row-{first}"));
        assert_eq!(rt.state.table.selected, std::slice::from_ref(&first));
        key(&mut rt, Key::Down, false, false);
        assert_eq!(rt.state.table.selected, std::slice::from_ref(&second), "Down moves the selection");
        key(&mut rt, Key::Down, true, false);
        assert_eq!(rt.state.table.selected.len(), 2, "Shift+Down extends it");
        key(&mut rt, Key::Command('a'), false, true);
        assert_eq!(rt.state.table.selected.len(), 1200, "Ctrl+A selects every row");
    }

    #[test]
    fn table_header_sorts_and_flips() {
        let mut rt = lab();
        assert_eq!(rt.state.table.sort, (0, false));
        rt.state.table.sort_by(2);
        settle(&mut rt);
        assert_eq!(rt.state.table.sort, (2, false));
        rt.state.table.sort_by(2);
        assert_eq!(rt.state.table.sort, (2, true), "a second press flips the direction");
    }

    #[test]
    fn palette_filters_runs_and_shows_a_toast() {
        let mut rt = lab();
        click(&mut rt, "open-palette");
        assert!(rt.state.palette);
        typed(&mut rt, "cpa");
        assert_eq!(rt.state.pal.edit.text, "cpa", "the field took the focus and the keys");
        key(&mut rt, Key::Enter, false, false);
        assert!(!rt.state.palette, "running a command closes the palette");
        assert_eq!(rt.state.toasts.items.len(), 1, "Run CPA attack raised a toast");
        assert_eq!(rt.state.toasts.items[0].title, "Attack finished");
    }

    #[test]
    fn palette_escape_closes_it() {
        let mut rt = lab();
        click(&mut rt, "open-palette");
        key(&mut rt, Key::Escape, false, false);
        assert!(!rt.state.palette);
    }

    #[test]
    fn plot_wheel_zooms_and_a_double_press_resets() {
        let mut rt = lab();
        assert!(!rt.state.plot.is_zoomed());
        let (x, y) = rt.find("plot-area").expect("plot area");
        rt.wheel_xy(x, y, 0.0, -120.0);
        settle(&mut rt);
        assert!(rt.state.plot.is_zoomed(), "wheel up zooms in");
        let span = rt.state.plot.span();
        rt.wheel_xy(x, y, 0.0, -120.0);
        settle(&mut rt);
        assert!(rt.state.plot.span() < span, "and keeps zooming");
        rt.state.plot.reset();
        assert!(!rt.state.plot.is_zoomed());
    }

    #[test]
    fn reorder_list_moves_a_dragged_row() {
        let mut rt = lab();
        let before = rt.state.order.clone();
        let (x, y) = rt.find("demo-reorder").expect("list");
        // The list's centre is on its middle row (index 2); drag it one row down.
        rt.pointer_down(x, y);
        rt.pointer_move(x, y + 20.0);
        rt.pointer_move(x, y + 40.0);
        rt.pointer_up(x, y + 40.0);
        settle(&mut rt);
        assert_ne!(rt.state.order, before, "the order changed");
        let mut a = before.clone();
        let mut b = rt.state.order.clone();
        a.sort();
        b.sort();
        assert_eq!(a, b, "nothing was lost or duplicated");
    }

    #[test]
    fn tags_add_and_remove_from_the_keyboard() {
        let mut rt = lab();
        click(&mut rt, "tags-input");
        typed(&mut rt, "fpga");
        key(&mut rt, Key::Enter, false, false);
        assert_eq!(rt.state.tags.tags, ["aes", "stm32", "fpga"]);
        key(&mut rt, Key::Backspace, false, false);
        assert_eq!(rt.state.tags.tags, ["aes", "stm32"], "Backspace on an empty field removes the last tag");
    }

    #[test]
    fn combobox_filters_and_picks() {
        let mut rt = lab();
        click(&mut rt, "fruit-input");
        typed(&mut rt, "ap");
        assert!(rt.state.combo.open, "typing opens the list");
        key(&mut rt, Key::Down, false, false);
        key(&mut rt, Key::Enter, false, false);
        // "ap" matches Apple (0) and Apricot (1); Down moved to the second.
        assert_eq!(rt.state.combo_pick, Some(1));
        assert_eq!(rt.state.combo.edit.text, "Apricot");
        assert!(!rt.state.combo.open);
    }

    #[test]
    fn theme_switch_restyles_the_window() {
        let mut rt = lab();
        assert!(tokens().dark);
        rt.update(|s, _| s.dark = false);
        settle(&mut rt);
        assert!(!tokens().dark, "the view applied the light theme");
        rt.update(|s, _| s.dark = true);
        settle(&mut rt);
        assert!(tokens().dark);
    }

    #[test]
    fn dialog_opens_and_escape_dismisses_it() {
        let mut rt = lab();
        click(&mut rt, "open-dialog");
        assert!(rt.state.dialog);
        key(&mut rt, Key::Escape, false, false);
        assert!(!rt.state.dialog, "Escape dismisses the modal");
    }
}
