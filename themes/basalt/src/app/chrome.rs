//! App chrome and layout: menu bar, panes, tree, tabs, splitters, docked panels, path bar,
//! status bar, scroll view, spinner, progress, readouts, empty state.

use std::rc::Rc;

use rune::{Align, Cursor, Drag, El, Icon, Key, KeyEvent, ScrollSpec, Side, Tx, col, el, icon, row, text};

use super::controls::tool_sep;
use super::{CAPTION, Level, Popped, R, Route, TINY, TITLE, UI, UI_STRONG, act, caption, d, dot, floating, h, kbd, route};
use crate::{FIGURE_STYLE, tokens};

// ---- menu bar ---------------------------------------------------------------------------------

/// One entry of a menu-bar menu; `None` is a separator. Fields: label, shortcut, destructive,
/// disabled.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub danger: bool,
    pub disabled: bool,
}

impl MenuItem {
    pub fn new(label: &str) -> MenuItem {
        MenuItem { label: label.into(), shortcut: None, danger: false, disabled: false }
    }
    pub fn shortcut(mut self, k: &str) -> MenuItem {
        self.shortcut = Some(k.into());
        self
    }
    pub fn danger(mut self) -> MenuItem {
        self.danger = true;
        self
    }
    pub fn disabled(mut self) -> MenuItem {
        self.disabled = true;
        self
    }
}

/// A menu in the bar: its title and entries.
pub struct Menu {
    pub title: String,
    pub items: Vec<Option<MenuItem>>,
}

impl Menu {
    pub fn new(title: &str, items: Vec<Option<MenuItem>>) -> Menu {
        Menu { title: title.into(), items }
    }
}

/// A menu's panel: entries with shortcuts, separators, destructive items in the accent,
/// disabled ones dimmed. `highlight` (an index into `items`) is drawn lit.
pub fn menu_panel<S: 'static>(items: &[Option<MenuItem>], highlight: Option<usize>, on_pick: impl Fn(&mut S, usize) + 'static) -> El {
    let s = tokens();
    let pick = Rc::new(on_pick);
    let mut m = floating(col().w(240.0).padding(5.0).gap(1.0)).role("menu").focus_group();
    for (i, e) in items.iter().enumerate() {
        match e {
            None => m = m.child(el().w_full().h(1.0).my(4.0).fill(s.line)),
            Some(it) => {
                let f = pick.clone();
                let ink = if it.danger { s.danger } else { s.text };
                let mut r = row().w_full().h(28.0).px(9.0).items_center().radius(6.0).role("menuitem").label(it.label.as_str()).cursor(Cursor::Pointer);
                r = if highlight == Some(i) { r.fill(if it.danger { s.danger.alpha(0.12) } else { s.tint }) } else { r.hover_fill(if it.danger { s.danger.alpha(0.12) } else { s.tint }) };
                r = r.child(text(it.label.as_str()).font(UI).color(ink).grow(1.0));
                if let Some(k) = &it.shortcut {
                    r = r.child(text(k.as_str()).font(TINY).color(s.dim));
                }
                m = m.child(if it.disabled { super::inert(r) } else { r.focusable(true).on_press(move |st: &mut S| f(st, i)) });
            }
        }
    }
    m
}

/// A context menu anchored to a keyed element, closing on a press outside it. Returns the
/// layers to mount.
pub fn context_menu<S: 'static>(anchor: &str, items: &[Option<MenuItem>], on_pick: impl Fn(&mut S, usize) + 'static, on_close: impl Fn(&mut S) + 'static) -> Vec<El> {
    vec![rune_kit::scrim("menu-scrim", on_close), menu_panel(items, None, on_pick).anchored(anchor, Side::Bottom, Align::End, 2.0)]
}

/// A menu bar's state: which menu is open (`None` closed) and the highlighted entry.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuBarState {
    pub open: Option<usize>,
    pub highlight: usize,
}

/// A menu bar (File / Edit / View…): press a title to open its menu, hover another to switch,
/// Left / Right change menu, Up / Down move, Enter picks, Escape closes. `on_pick` gets
/// `(menu, entry)`.
pub fn menu_bar<S: 'static>(menus: Vec<Menu>, state: &MenuBarState, to: impl for<'a> Fn(&'a mut S) -> &'a mut MenuBarState + 'static, on_pick: impl Fn(&mut S, usize, usize) + 'static) -> Popped {
    let s = tokens();
    let to: Route<S, MenuBarState> = route(to);
    let pick = Rc::new(on_pick);
    let n = menus.len();
    let mut bar = row().key("menubar").gap(2.0).items_center().role("menubar");
    let mut layers = Vec::new();
    for (i, m) in menus.iter().enumerate() {
        let on = state.open == Some(i);
        let (t1, t2, t3) = (to.clone(), to.clone(), to.clone());
        let mut b = row().key(format!("menubar-{i}")).h(h() - 4.0).px(d(10.0)).items_center().radius(6.0).role("menuitem").label(m.title.as_str()).expanded(on).focusable(true).cursor(Cursor::Pointer);
        b = if on { b.fill(s.tint) } else { b.hover_fill(s.tint.alpha(0.6)) };
        let count = m.items.len();
        bar = bar.child(
            b.child(text(m.title.as_str()).font(UI).color(if on { s.text } else { s.muted }))
                .on_press(move |st: &mut S| {
                    let c = t1(st);
                    *c = if c.open == Some(i) { MenuBarState::default() } else { MenuBarState { open: Some(i), highlight: 0 } };
                })
                // Moving along the bar while a menu is open switches to the one under the pointer.
                .on_hover_change(move |st: &mut S, over: bool, _: &mut Tx| {
                    let c = t2(st);
                    if over && c.open.is_some() && c.open != Some(i) {
                        *c = MenuBarState { open: Some(i), highlight: 0 };
                    }
                })
                .on_key(move |st: &mut S, k: &KeyEvent, _: &mut Tx| {
                    let c = t3(st);
                    let Some(o) = c.open else {
                        return if matches!(k.key, Key::Down | Key::Enter) {
                            *c = MenuBarState { open: Some(i), highlight: 0 };
                            true
                        } else {
                            false
                        };
                    };
                    match k.key {
                        Key::Left => *c = MenuBarState { open: Some((o + n - 1) % n), highlight: 0 },
                        Key::Right => *c = MenuBarState { open: Some((o + 1) % n), highlight: 0 },
                        Key::Down => c.highlight = (c.highlight + 1) % count.max(1),
                        Key::Up => c.highlight = (c.highlight + count.max(1) - 1) % count.max(1),
                        Key::Escape => *c = MenuBarState::default(),
                        _ => return false,
                    }
                    true
                }),
        );
        if on {
            let t = to.clone();
            layers.push(rune_kit::scrim("menubar-scrim", move |st: &mut S| *t(st) = MenuBarState::default()));
            let (t, p) = (to.clone(), pick.clone());
            layers.push(menu_panel(&m.items, Some(state.highlight), move |st: &mut S, entry: usize| {
                *t(st) = MenuBarState::default();
                p(st, i, entry);
            }).anchored(format!("menubar-{i}"), Side::Bottom, Align::Start, 4.0));
        }
    }
    Popped { trigger: bar, layers }
}

// ---- panes ------------------------------------------------------------------------------------

/// A pane's header row: title, then trailing actions.
pub fn pane_header(title: &str, actions: El) -> El {
    let s = tokens();
    row().w_full().h(40.0).px(14.0).items_center().border_sides([0.0, 0.0, 1.0, 0.0], s.line).child(text(title).font(TITLE).color(s.text).grow(1.0)).child(actions)
}

/// A collapsible group in an inspector: caption with a chevron, then its body; Enter / Space
/// toggle it from the keyboard.
pub fn group<S: 'static>(title: &str, open: bool, on_toggle: impl Fn(&mut S) + 'static, body: El) -> El {
    let s = tokens();
    let k = crate::kit();
    let head = row()
        .w_full()
        .h(32.0)
        .px(14.0)
        .gap(8.0)
        .items_center()
        .cursor(Cursor::Pointer)
        .role("button")
        .expanded(open)
        .label(title)
        .focusable(true)
        .child(icon(k.glyph.chevron_down, 10.0).color(s.dim).rotate(if open { 0.0 } else { -std::f32::consts::FRAC_PI_2 }).motion(k.motion.disclosure))
        .child(caption(title));
    let mut c = col().w_full().border_sides([0.0, 0.0, 1.0, 0.0], s.line).child(act(head, on_toggle));
    if open {
        c = c.child(col().w_full().px(14.0).pb(14.0).gap(12.0).child(body));
    }
    c
}

/// A label / value row of a property grid.
pub fn prop(label: &str, value: El) -> El {
    let s = tokens();
    row().w_full().h(26.0).items_center().child(text(label).font(UI).color(s.muted).w(120.0)).child(row().grow(1.0).justify_end().child(value))
}

/// A mono value for [`prop`].
pub fn value(v: &str) -> El {
    text(v).font(UI).color(tokens().text)
}

/// A sidebar item: icon, label, optional count, lit when active.
pub fn nav_item<S: 'static>(glyph: &'static Icon, label: &str, count: Option<&str>, active: bool, on_press: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let mut r = row().w_full().h(h()).px(10.0).gap(10.0).items_center().radius(R).role("link").label(label).checked(active).focusable(true).cursor(Cursor::Pointer);
    r = if active { r.fill(s.tint) } else { r.hover_fill(s.tint.alpha(0.5)) };
    r = r.child(icon(glyph, 14.0).color(if active { s.text } else { s.muted })).child(text(label).font(UI).color(if active { s.text } else { s.muted }).grow(1.0));
    if let Some(c) = count {
        r = r.child(text(c).font(TINY).color(s.dim));
    }
    act(r, on_press)
}

// ---- tree -------------------------------------------------------------------------------------

/// A node of a [`tree`]: label, state dot, children, id (unique in the tree).
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub level: Option<Level>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn leaf(id: &str, label: &str) -> Node {
        Node { id: id.into(), label: label.into(), level: None, children: Vec::new() }
    }
    pub fn branch(id: &str, label: &str, children: Vec<Node>) -> Node {
        Node { id: id.into(), label: label.into(), level: None, children }
    }
    pub fn level(mut self, l: Level) -> Node {
        self.level = Some(l);
        self
    }
}

/// A tree's state: the expanded node ids and the selected one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeState {
    pub expanded: Vec<String>,
    pub selected: Option<String>,
}

impl TreeState {
    pub fn toggle(&mut self, id: &str) {
        match self.expanded.iter().position(|e| e == id) {
            Some(p) => {
                self.expanded.remove(p);
            }
            None => self.expanded.push(id.into()),
        }
    }
}

/// The rows currently visible: `(node, depth)`.
fn visible<'a>(nodes: &'a [Node], expanded: &[String], depth: usize, out: &mut Vec<(&'a Node, usize)>) {
    for n in nodes {
        out.push((n, depth));
        if !n.children.is_empty() && expanded.contains(&n.id) {
            visible(&n.children, expanded, depth + 1, out);
        }
    }
}

/// A tree view: a press selects, a press on the chevron (or Right / Left) opens and closes,
/// Up / Down move the selection. `on_select` gets the node id.
pub fn tree<S: 'static>(nodes: &[Node], state: &TreeState, to: impl for<'a> Fn(&'a mut S) -> &'a mut TreeState + 'static, on_select: impl Fn(&mut S, &str) + 'static) -> El {
    let s = tokens();
    let k = crate::kit();
    let to: Route<S, TreeState> = route(to);
    let on_select = Rc::new(on_select);
    let mut rows = Vec::new();
    visible(nodes, &state.expanded, 0, &mut rows);
    let ids: Rc<Vec<String>> = Rc::new(rows.iter().map(|(n, _)| n.id.clone()).collect());
    let has_children: Rc<Vec<String>> = Rc::new(rows.iter().filter(|(n, _)| !n.children.is_empty()).map(|(n, _)| n.id.clone()).collect());
    let mut c = col().w_full().gap(1.0).role("tree").focus_group();
    for (n, depth) in rows {
        let open = state.expanded.contains(&n.id);
        let on = state.selected.as_deref() == Some(n.id.as_str());
        let (t1, t2, t3, sel, ids2, has_children, id1, id2, id3) = (to.clone(), to.clone(), to.clone(), on_select.clone(), ids.clone(), has_children.clone(), n.id.clone(), n.id.clone(), n.id.clone());
        let branch = !n.children.is_empty();
        let mut r = row().key(format!("tree-{}", n.id)).w_full().h(26.0).pl(8.0 + depth as f32 * 14.0).pr(8.0).gap(6.0).items_center().radius(6.0).role("treeitem").label(n.label.as_str()).checked(on).expanded(open).focusable(true).cursor(Cursor::Pointer);
        r = if on { r.fill(s.tint) } else { r.hover_fill(s.tint.alpha(0.5)) };
        r = if branch {
            let chevron = row().size(14.0, 14.0).center().role("button").label(if open { "Collapse" } else { "Expand" }).child(icon(k.glyph.chevron_down, 9.0).color(s.dim).rotate(if open { 0.0 } else { -std::f32::consts::FRAC_PI_2 }));
            r.child(chevron.on_press(move |st: &mut S| t1(st).toggle(&id1)))
        } else {
            r.child(el().w(14.0))
        };
        if let Some(l) = n.level {
            r = r.child(dot(l));
        }
        r = r.child(text(n.label.as_str()).font(UI).color(if on { s.text } else { s.muted }));
        c = c.child(
            r.on_press(move |st: &mut S| {
                t2(st).selected = Some(id2.clone());
                sel(st, &id2);
            })
            .on_key(move |st: &mut S, kev: &KeyEvent, tx: &mut Tx| {
                let t = t3(st);
                // Act on the selected node, not the row holding focus: focus stays where the
                // last press put it while the selection moves with the keys.
                let cur = t.selected.clone().unwrap_or_else(|| id3.clone());
                let pos = ids2.iter().position(|i| *i == cur).unwrap_or(0);
                let is_branch = has_children.contains(&cur);
                let target = match kev.key {
                    Key::Right if is_branch && !t.expanded.contains(&cur) => {
                        t.expanded.push(cur.clone());
                        None
                    }
                    Key::Left if t.expanded.contains(&cur) => {
                        t.toggle(&cur);
                        None
                    }
                    Key::Down if pos + 1 < ids2.len() => Some(ids2[pos + 1].clone()),
                    Key::Up if pos > 0 => Some(ids2[pos - 1].clone()),
                    Key::Right | Key::Left | Key::Down | Key::Up => None,
                    _ => return false,
                };
                if let Some(next) = target {
                    t.selected = Some(next.clone());
                    tx.focus(format!("tree-{next}"), false);
                }
                true
            }),
        );
    }
    c
}

/// A single tree row, for sidebars that manage their own state: indent, disclosure chevron
/// (`None` for a leaf), dot, label.
pub fn tree_item<S: 'static>(label: &str, depth: usize, expanded: Option<bool>, selected: bool, level: Option<Level>, on_press: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let k = crate::kit();
    let mut r = row().w_full().h(26.0).pl(8.0 + depth as f32 * 14.0).pr(8.0).gap(6.0).items_center().radius(6.0).role("treeitem").label(label).checked(selected).focusable(true).cursor(Cursor::Pointer);
    r = if selected { r.fill(s.tint) } else { r.hover_fill(s.tint.alpha(0.5)) };
    r = match expanded {
        Some(open) => r.child(icon(k.glyph.chevron_down, 9.0).color(s.dim).rotate(if open { 0.0 } else { -std::f32::consts::FRAC_PI_2 })),
        None => r.child(el().w(9.0)),
    };
    if let Some(l) = level {
        r = r.child(dot(l));
    }
    act(r.child(text(label).font(UI).color(if selected { s.text } else { s.muted })), on_press)
}

// ---- tabs, breadcrumbs ------------------------------------------------------------------------

/// Document tabs: title, a dot while unsaved, a close button on the active one; Left / Right
/// move between them, middle-click closes.
pub fn doc_tabs<S: 'static>(tabs: &[(&str, bool)], active: usize, on_pick: impl Fn(&mut S, usize) + 'static, on_close: impl Fn(&mut S, usize) + 'static) -> El {
    let s = tokens();
    let k = crate::kit();
    let pick = Rc::new(on_pick);
    let close = Rc::new(on_close);
    let mut r = row().w_full().h(36.0).items_end().px(8.0).gap(2.0).border_sides([0.0, 0.0, 1.0, 0.0], s.line).role("tablist").focus_group();
    for (i, (title, dirty)) in tabs.iter().enumerate() {
        let on = i == active;
        let (f, c, c2) = (pick.clone(), close.clone(), close.clone());
        let mut t = row().h(32.0).px(12.0).gap(8.0).items_center().corners([R, R, 0.0, 0.0]).role("tab").label(*title).checked(on).focusable(true).cursor(Cursor::Pointer).on_middle_press(move |st: &mut S, _: &mut Tx| c2(st, i)).on_press(move |st: &mut S| f(st, i));
        t = if on { t.fill(s.panel).border_sides([1.0, 1.0, 0.0, 1.0], s.line) } else { t.hover_fill(s.tint.alpha(0.5)) };
        t = t.child(text(*title).font(UI).color(if on { s.text } else { s.muted }));
        if *dirty {
            t = t.child(el().size(6.0, 6.0).radius(3.0).fill(s.muted));
        }
        if on {
            t = t.child(row().size(18.0, 18.0).center().radius(5.0).hover_fill(s.tint).role("button").label("Close tab").on_press(move |st: &mut S| c(st, i)).child(icon(k.glyph.close, 10.0).color(s.muted)));
        }
        r = r.child(t);
    }
    r
}

/// A path bar: segments separated by chevrons, the last one current; `on_pick` gets the index.
pub fn breadcrumbs<S: 'static>(segments: &[&str], on_pick: impl Fn(&mut S, usize) + 'static) -> El {
    let s = tokens();
    let pick = Rc::new(on_pick);
    let n = segments.len();
    let mut r = row().gap(4.0).items_center().role("navigation");
    for (i, seg) in segments.iter().enumerate() {
        let last = i + 1 == n;
        let f = pick.clone();
        let t = text(*seg).font(UI).color(if last { s.text } else { s.muted });
        let item = row().h(24.0).px(6.0).items_center().radius(5.0).child(t);
        r = r.child(if last { item } else { act(item.hover_fill(s.tint).cursor(Cursor::Pointer).role("link").label(*seg).focusable(true), move |st: &mut S| f(st, i)) });
        if !last {
            r = r.child(icon(crate::kit().glyph.caret_right, 9.0).color(s.dim));
        }
    }
    r
}

// ---- splitters and docked panels --------------------------------------------------------------

/// A divider between panes that resizes them: `on_drag` gets the pointer's travel since the
/// press and whether this is the press itself. `vertical` is the line between side-by-side
/// panes (a column resize), otherwise the line between stacked panes.
pub fn splitter_axis<S: 'static>(vertical: bool, on_drag: impl Fn(&mut S, f32, bool) + 'static) -> El {
    let s = tokens();
    let e = if vertical { el().w(5.0).h_full().cursor(Cursor::ResizeColumn) } else { el().h(5.0).w_full().cursor(Cursor::ResizeRow) };
    let line = if vertical { el().w(1.0).h_full().absolute(2.0, 0.0) } else { el().h(1.0).w_full().absolute(0.0, 2.0) };
    e.role("separator")
        .hover_fill(s.hair)
        .on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| on_drag(st, if vertical { dr.dx } else { dr.dy }, dr.phase == rune::Phase::Down))
        .child(line.fill(s.line))
}

/// The vertical splitter (between side-by-side panes).
pub fn splitter<S: 'static>(on_drag: impl Fn(&mut S, f32, bool) + 'static) -> El {
    splitter_axis(true, on_drag)
}

/// The horizontal splitter (between stacked panes).
pub fn splitter_h<S: 'static>(on_drag: impl Fn(&mut S, f32, bool) + 'static) -> El {
    splitter_axis(false, on_drag)
}

/// A resizable size kept by the app: `size` now, `base` when the drag began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneSize {
    pub size: f32,
    base: f32,
    pub min: f32,
    pub max: f32,
}

impl PaneSize {
    pub fn new(size: f32, min: f32, max: f32) -> PaneSize {
        PaneSize { size, base: size, min, max }
    }
    /// Applies a splitter drag; `grow` is `-1.0` when the pane sits after the splitter (dragging
    /// right makes it smaller).
    pub fn drag(&mut self, travel: f32, start: bool, grow: f32) {
        if start {
            self.base = self.size;
        }
        self.size = (self.base + travel * grow).clamp(self.min, self.max);
    }
}

/// A pane with a header that can be docked left or right, collapsed to its header, or closed:
/// a docked panel as in an IDE. `on_action` gets `Dock::Collapse`, `Dock::Flip` or `Dock::Close`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dock {
    Collapse,
    Flip,
    Close,
}

pub fn dock_panel<S: 'static>(title: &str, collapsed: bool, body: El, on_action: impl Fn(&mut S, Dock) + 'static) -> El {
    let s = tokens();
    let act_fn = Rc::new(on_action);
    let btn = |glyph: &'static Icon, label: &str, what: Dock| {
        let f = act_fn.clone();
        row().size(22.0, 22.0).center().radius(5.0).hover_fill(s.tint).cursor(Cursor::Pointer).role("button").label(label).help(label).child(icon(glyph, 12.0).color(s.muted)).on_press(move |st: &mut S| f(st, what))
    };
    let head = row()
        .w_full()
        .h(34.0)
        .pl(12.0)
        .pr(6.0)
        .items_center()
        .gap(2.0)
        .child(text(title).font(UI_STRONG).color(s.text).grow(1.0))
        .child(btn(if collapsed { &crate::icons::oc::EXPAND } else { &crate::icons::oc::COLLAPSE }, if collapsed { "Expand" } else { "Collapse" }, Dock::Collapse))
        .child(btn(&crate::icons::oc::LAYOUT_RIGHT, "Dock to the other side", Dock::Flip))
        .child(btn(&crate::icons::oc::CLOSE, "Close", Dock::Close));
    let mut c = col().w_full().fill(s.bg).border(1.0, s.line).radius(10.0).clip_rounded(10.0).child(head);
    if !collapsed {
        c = c.child(el().w_full().h(1.0).fill(s.line)).child(body);
    }
    c
}

// ---- scroll, status bar -----------------------------------------------------------------------

/// A vertical scroll view with a slim scroll bar riding its right edge.
pub fn scroll_view(key: &str, content: El) -> El {
    rune_kit::Kit::scroll_bar(crate::kit(), col().key(key).w_full().h_full().scroll(ScrollSpec::default()).child(content))
}

/// A horizontal scroll view (no bar; the wheel with Shift, or a trackpad, scrolls it).
pub fn scroll_x(key: &str, content: El) -> El {
    row().key(key).w_full().scroll_x().child(content)
}

/// The status bar: items on the left, items on the right.
pub fn status_bar(left: Vec<El>, right: Vec<El>) -> El {
    let s = tokens();
    let mut l = row().gap(16.0).items_center().grow(1.0);
    for e in left {
        l = l.child(e);
    }
    let mut r = row().gap(16.0).items_center();
    for e in right {
        r = r.child(e);
    }
    row().w_full().h(26.0).px(12.0).items_center().fill(s.bg).border_sides([1.0, 0.0, 0.0, 0.0], s.line).child(l).child(r)
}

/// A status bar item: optional dot, then mono text.
pub fn status_item(level: Option<Level>, label: &str) -> El {
    let s = tokens();
    let mut r = row().gap(6.0).items_center();
    if let Some(l) = level {
        r = r.child(dot(l));
    }
    r.child(text(label).font(TINY).color(s.muted))
}

// ---- progress, spinner, readouts, empty state -------------------------------------------------

/// A linear progress bar with a caption and percentage.
pub fn progress(label: &str, value: f32) -> El {
    let s = tokens();
    let v = value.clamp(0.0, 1.0);
    col().w_full()
        .gap(6.0)
        .child(row().w_full().child(text(label).font(TINY).color(s.muted).grow(1.0)).child(text(format!("{:.0}%", v * 100.0)).font(TINY).color(s.text)))
        .child(el().w_full().h(4.0).radius(2.0).fill(s.line).child(el().h(4.0).w_frac(v).radius(2.0).fill(s.text)))
        .role("progressbar")
        .a11y_value(format!("{:.0}%", v * 100.0))
}

/// A busy spinner of `size` points.
pub fn spinner(size: f32) -> El {
    rune_kit::Loader::new().size(size).color(tokens().muted).build(crate::kit())
}

/// A busy row: spinner and a label.
pub fn busy(label: &str) -> El {
    row().gap(10.0).items_center().child(spinner(14.0)).child(text(label).font(UI).color(tokens().muted)).role("status")
}

/// A big number with a caption and a delta (a readout tile).
pub fn readout(label: &str, figure: &str, delta: Option<(&str, Level)>) -> El {
    let s = tokens();
    let mut c = col().gap(4.0).padding(14.0).radius(12.0).fill(s.tint).grow(1.0).child(caption(label)).child(text(figure).font(FIGURE_STYLE).color(s.text));
    if let Some((d, l)) = delta {
        c = c.child(text(d).font(TINY).color(l.color()));
    }
    c
}

/// A centred message for an empty pane.
pub fn empty_state<S: 'static>(glyph: &'static Icon, title: &str, body: &str, action: &str, on_action: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    col().w_full()
        .h_full()
        .center()
        .gap(10.0)
        .child(row().size(44.0, 44.0).center().radius(12.0).fill(s.tint).child(icon(glyph, 20.0).color(s.muted)))
        .child(text(title).font(TITLE).color(s.text))
        .child(col().max_w(300.0).child(text(body).font(UI).color(s.muted).wrap().align(Align::Center)))
        .child(row().pt(6.0).child(super::button(action, super::Kind::Secondary, None, None, on_action)))
}

/// A keyboard-hint footer: `(keys, what)` pairs.
pub fn hints(items: &[(&str, &str)]) -> El {
    let s = tokens();
    let mut r = row().gap(14.0).items_center();
    for (k, what) in items {
        r = r.child(row().gap(6.0).items_center().child(kbd(k)).child(text(*what).font(TINY).color(s.dim)));
    }
    r
}

/// A thin vertical rule between groups in a row (toolbar separator).
pub fn rule() -> El {
    tool_sep()
}

/// The caption style, re-exported for layouts that build their own caps text.
pub const CAPTION_STYLE: rune::TextStyle = CAPTION;
