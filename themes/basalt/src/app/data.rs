//! The table, log lines and the reorderable list.

use std::rc::Rc;

use rune::{Cursor, Drag, El, GrowthSpec, Key, KeyEvent, Mods, ScrollSpec, Tx, col, el, icon, row, text};

use super::{Level, R, Route, TINY, UI, UI_STRONG, caption, d, route};
use crate::tokens;

// ---- table ------------------------------------------------------------------------------------

/// A table cell.
pub enum Cell {
    Text(String),
    /// Right-aligned mono figure.
    Num(String),
    /// A 0–1 bar with its value.
    Bar(f32),
    Badge(String, Level),
    /// Any element (a sparkline, a button, an icon with text…).
    Custom(El),
}

/// A column: title, width in points (0 = grows), right-aligned.
#[derive(Clone, Copy, Debug)]
pub struct Column {
    pub title: &'static str,
    pub width: f32,
    pub right: bool,
}

impl Column {
    pub const fn new(title: &'static str, width: f32) -> Column {
        Column { title, width, right: false }
    }
    pub const fn right(mut self) -> Column {
        self.right = true;
        self
    }
}

/// A table's state: the selected row ids, sort, resized column widths.
#[derive(Clone, Debug, PartialEq)]
pub struct TableState {
    pub selected: Vec<String>,
    /// The row a Shift range starts from.
    pub anchor: Option<String>,
    /// The sorted column and whether it is descending; the app sorts its rows by it.
    pub sort: (usize, bool),
    /// Column widths as dragged (0 where the column still grows).
    pub widths: Vec<f32>,
    /// Modifiers held, for Ctrl / Shift-click: set it from `App::on_modifiers` (optional).
    pub mods: Mods,
    base: f32,
}

impl TableState {
    pub fn new(columns: &[Column]) -> TableState {
        TableState { selected: Vec::new(), anchor: None, sort: (0, false), widths: columns.iter().map(|c| c.width).collect(), mods: Mods::default(), base: 0.0 }
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|s| s == id)
    }

    /// A plain click selects one row; Ctrl toggles it; Shift extends from the anchor.
    pub fn click(&mut self, id: &str, order: &[String]) {
        let (shift, cmd) = (self.mods.shift, self.mods.cmd);
        self.select(id, order, shift, cmd);
    }

    /// The selection rules behind clicks and keys.
    pub fn select(&mut self, id: &str, order: &[String], shift: bool, cmd: bool) {
        if shift && let Some(a) = self.anchor.clone().filter(|a| order.contains(a)) {
            let (i, j) = (order.iter().position(|x| *x == a).unwrap(), order.iter().position(|x| x == id).unwrap_or(0));
            self.selected = order[i.min(j)..=i.max(j)].to_vec();
        } else if cmd {
            match self.selected.iter().position(|x| x == id) {
                Some(p) => {
                    self.selected.remove(p);
                }
                None => self.selected.push(id.to_owned()),
            }
            self.anchor = Some(id.to_owned());
        } else {
            self.selected = vec![id.to_owned()];
            self.anchor = Some(id.to_owned());
        }
    }

    /// Selects every row.
    pub fn select_all(&mut self, order: &[String]) {
        self.selected = order.to_vec();
    }

    /// Sorts by `column`: a second press flips the direction.
    pub fn sort_by(&mut self, column: usize) {
        self.sort = if self.sort.0 == column { (column, !self.sort.1) } else { (column, false) };
    }

    /// Moves the selection by `by` rows (clamped), extending it with `shift`.
    pub fn step(&mut self, by: isize, order: &[String], shift: bool) {
        if order.is_empty() {
            return;
        }
        let cur = self.selected.last().and_then(|id| order.iter().position(|x| x == id)).map_or(if by >= 0 { -1 } else { order.len() as isize }, |p| p as isize);
        let next = (cur + by).clamp(0, order.len() as isize - 1) as usize;
        let id = order[next].clone();
        self.select(&id, order, shift, false);
    }
}

/// A data table: sortable, column-resizable, selectable (rows, checkboxes, Shift / Ctrl, keyboard)
/// with right-click and an optional fixed height that scrolls under a sticky header.
pub struct Table<'a> {
    pub columns: &'a [Column],
    /// `(id, cells)`, in display order; ids are unique.
    pub rows: Vec<(String, Vec<Cell>)>,
    /// A checkbox column to the left.
    pub checkboxes: bool,
    /// `Some(h)`: the body scrolls in `h` points under a header that stays; rows are windowed so
    /// tens of thousands stay cheap. `None`: as tall as its rows.
    pub height: Option<f32>,
    pub row_height: f32,
}

impl<'a> Table<'a> {
    pub fn new(columns: &'a [Column], rows: Vec<(String, Vec<Cell>)>) -> Table<'a> {
        Table { columns, rows, checkboxes: false, height: None, row_height: 30.0 }
    }
    pub fn checkboxes(mut self) -> Self {
        self.checkboxes = true;
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = Some(h);
        self
    }

    /// `to` reaches the [`TableState`]; `on_menu(row id)` runs on a right-click (the row is
    /// keyed `row-<id>`, to anchor a context menu to).
    pub fn build<S: 'static>(self, key: &str, state: &TableState, to: impl for<'x> Fn(&'x mut S) -> &'x mut TableState + 'static, on_menu: impl Fn(&mut S, &str) + 'static) -> El {
        let s = tokens();
        let k = crate::kit();
        let to: Route<S, TableState> = route(to);
        let menu = Rc::new(on_menu);
        let order: Rc<Vec<String>> = Rc::new(self.rows.iter().map(|(id, _)| id.clone()).collect());
        let width_of = |i: usize| state.widths.get(i).copied().unwrap_or(self.columns[i].width);
        let cell_box = |i: usize| {
            let c = &self.columns[i];
            let r = row().h_full().px(12.0).items_center();
            let r = if width_of(i) > 0.0 { r.w(width_of(i)) } else { r.grow(1.0).min_w(60.0) };
            if c.right { r.justify_end() } else { r }
        };

        // Header.
        let mut head = row().w_full().h(32.0).fill(s.bg).border_sides([0.0, 0.0, 1.0, 0.0], s.line).role("row");
        if self.checkboxes {
            let (t, o) = (to.clone(), order.clone());
            let all = !order.is_empty() && order.iter().all(|id| state.is_selected(id));
            head = head.child(check_cell(all, "Select all", move |st: &mut S| {
                let ts = t(st);
                if all { ts.selected.clear() } else { ts.select_all(&o) }
            }));
        }
        for i in 0..self.columns.len() {
            let t = to.clone();
            let mut hcell = cell_box(i).gap(4.0).cursor(Cursor::Pointer).hover_fill(s.tint.alpha(0.4)).role("columnheader").focusable(true).child(caption(self.columns[i].title));
            if state.sort.0 == i {
                hcell = hcell.child(icon(k.glyph.chevron_down, 9.0).color(s.muted).rotate(if state.sort.1 { std::f32::consts::PI } else { 0.0 }));
            }
            head = head.child(hcell.on_press(move |st: &mut S| t(st).sort_by(i)));
            // A drag handle on the right edge of every fixed-width column.
            if width_of(i) > 0.0 && i + 1 < self.columns.len() {
                let (t, w0) = (to.clone(), width_of(i));
                head = head.child(
                    el().w(5.0).h_full().mx(-3.0).cursor(Cursor::ResizeColumn).hover_fill(s.hair).role("separator").on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| {
                        let ts = t(st);
                        if dr.phase == rune::Phase::Down {
                            ts.base = w0;
                        }
                        ts.widths[i] = (ts.base + dr.dx).clamp(48.0, 600.0);
                    }),
                );
            }
        }

        // Body.
        let rh = self.row_height;
        let mut body = col().key(format!("{key}-rows")).w_full();
        for (id, cells) in self.rows {
            let on = state.is_selected(&id);
            let (t1, t2, o1, m, id1, id2) = (to.clone(), to.clone(), order.clone(), menu.clone(), id.clone(), id.clone());
            let mut line = row()
                .key(format!("row-{id}"))
                .w_full()
                .h(rh)
                .border_sides([0.0, 0.0, 1.0, 0.0], s.line.alpha(0.6))
                .role("row")
                .checked(on)
                .label(id.as_str())
                .cursor(Cursor::Pointer)
                .on_press(move |st: &mut S| t1(st).click(&id1, &o1))
                .on_secondary_press(move |st: &mut S, _: rune::PointerPress, _: &mut Tx| {
                    if !t2(st).is_selected(&id2) {
                        t2(st).selected = vec![id2.clone()];
                    }
                    m(st, &id2);
                });
            line = if on { line.fill(s.tint) } else { line.hover_fill(s.hover) };
            if self.checkboxes {
                let (t, id3) = (to.clone(), id.clone());
                line = line.child(check_cell(on, "Select row", move |st: &mut S| {
                    let ts = t(st);
                    ts.select(&id3, &[], false, true);
                }));
            }
            for (i, cell) in cells.into_iter().enumerate().take(self.columns.len()) {
                let content = match cell {
                    Cell::Text(t) => text(t).font(UI).color(if on { s.text } else { s.muted }),
                    Cell::Num(t) => text(t).font(UI).color(s.text),
                    Cell::Badge(t, l) => super::badge(&t, l),
                    Cell::Bar(v) => {
                        let v = v.clamp(0.0, 1.0);
                        row().gap(8.0)
                            .items_center()
                            .child(el().w(70.0).h(4.0).radius(2.0).fill(s.line).child(el().w(70.0 * v).h(4.0).radius(2.0).fill(if v > 0.8 { s.accent } else { s.text })))
                            .child(text(format!("{v:.3}")).font(UI).color(s.text))
                    }
                    Cell::Custom(e) => e,
                };
                line = line.child(cell_box(i).child(content));
            }
            body = body.child(line);
        }

        // Keyboard: arrows move (Shift extends), Home / End jump, Ctrl+A selects all.
        let (t, o) = (to.clone(), order.clone());
        let keys = move |st: &mut S, kev: &KeyEvent, _: &mut Tx| {
            let ts = t(st);
            let shift = kev.mods.shift;
            match kev.key {
                Key::Down => ts.step(1, &o, shift),
                Key::Up => ts.step(-1, &o, shift),
                Key::Home => ts.step(-(o.len() as isize), &o, shift),
                Key::End => ts.step(o.len() as isize, &o, shift),
                _ if kev.is_command('a') => ts.select_all(&o),
                _ => return false,
            }
            true
        };
        let body = match self.height {
            Some(h) => {
                let rows = body.growth(GrowthSpec::default()).virtualize(rh);
                col().key(format!("{key}-scroll")).w_full().h(h - 32.0).scroll(ScrollSpec::default()).child(rows)
            }
            None => body,
        };
        col().key(key).w_full().role("table").focusable(true).on_key(keys).child(head).child(body)
    }
}

fn check_cell<S: 'static>(on: bool, label: &str, f: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let mut b = el().size(16.0, 16.0).radius(4.5).center();
    b = if on { b.fill(s.solid).child(icon(crate::kit().glyph.check, 11.0).color(s.solid_fg)) } else { b.fill(s.bg).border(1.0, s.dim) };
    row().w(36.0).h_full().center().cursor(Cursor::Pointer).role("checkbox").label(label).checked(on).child(b).on_press(f)
}

// ---- log --------------------------------------------------------------------------------------

/// One log line: time, level tag, message.
pub fn log_line(time: &str, level: Level, msg: &str) -> El {
    let s = tokens();
    row().w_full()
        .px(14.0)
        .py(3.0)
        .gap(12.0)
        .items_start()
        .hover_fill(s.hover)
        .child(text(time).font(TINY).color(s.dim).w(64.0))
        .child(text(level.tag()).font(TINY).color(level.color()).w(34.0))
        .child(text(msg).font(TINY).color(if level == Level::Err { s.text } else { s.muted }).wrap().grow(1.0))
}

/// A log view: lines that follow the bottom as they arrive, windowed when long. `filter` keeps
/// the lines it returns true for.
pub fn log_view(key: &str, lines: &[(String, Level, String)], filter: impl Fn(Level) -> bool) -> El {
    let mut c = col().key(format!("{key}-lines")).w_full().py(6.0).growth(GrowthSpec::default()).virtualize(20.0);
    // Virtualised rows need stable keys: the line's index in the full log.
    for (i, (t, l, m)) in lines.iter().enumerate().filter(|(_, (_, l, _))| filter(*l)) {
        c = c.child(log_line(t, *l, m).key(format!("{key}-line-{i}")));
    }
    col().key(key).w_full().grow(1.0).scroll(ScrollSpec::stick_bottom()).child(c)
}

// ---- reorderable list -------------------------------------------------------------------------

/// A reorderable list's state: the row being dragged, where it would land, how far it moved.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReorderState {
    pub dragging: Option<usize>,
    pub target: usize,
    pub dy: f32,
}

/// Rows with a grip: drag one up or down to move it; a line shows where it will land and
/// `on_move(from, to)` runs on release (apply it to the app's own list with [`moved`]).
pub fn reorder_list<S: 'static>(key: &str, items: &[&str], state: &ReorderState, to: impl for<'x> Fn(&'x mut S) -> &'x mut ReorderState + 'static, on_move: impl Fn(&mut S, usize, usize) + 'static) -> El {
    let s = tokens();
    let to: Route<S, ReorderState> = route(to);
    let mv = Rc::new(on_move);
    let rh = d(34.0);
    let n = items.len();
    let mut c = col().key(key).w_full().gap(0.0).role("list");
    for (i, label) in items.iter().enumerate() {
        let (t, m) = (to.clone(), mv.clone());
        let lifted = state.dragging == Some(i);
        let mut r = row().w_full().h(rh).px(10.0).gap(10.0).items_center().radius(R).role("listitem").label(*label).cursor(Cursor::Grab);
        if lifted {
            r = r.fill(s.float).border(1.0, s.hair).offset(0.0, state.dy).box_shadow(super::shadow());
        } else {
            r = r.hover_fill(s.tint.alpha(0.5));
        }
        // The insertion line.
        if state.dragging.is_some() && state.dragging != Some(i) && state.target == i && state.dragging.is_some_and(|f| f > i) {
            r = r.border_sides([2.0, 0.0, 0.0, 0.0], s.accent);
        }
        if state.dragging.is_some() && state.dragging != Some(i) && state.target == i && state.dragging.is_some_and(|f| f < i) {
            r = r.border_sides([0.0, 0.0, 2.0, 0.0], s.accent);
        }
        c = c.child(
            r.child(text("⋮⋮").font(TINY).color(s.dim)).child(text(*label).font(UI).color(s.text).grow(1.0)).on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| {
                let rs = t(st);
                match dr.phase {
                    rune::Phase::Down | rune::Phase::Move => {
                        rs.dragging = Some(i);
                        rs.dy = dr.dy;
                        rs.target = (i as f32 + dr.dy / rh).round().clamp(0.0, n as f32 - 1.0) as usize;
                    }
                    rune::Phase::Up => {
                        let (from, target) = (i, rs.target);
                        *rs = ReorderState::default();
                        if from != target {
                            m(st, from, target);
                        }
                    }
                }
            }),
        );
    }
    c
}

/// Moves item `from` to position `to` in `list` (what `reorder_list`'s `on_move` applies).
pub fn moved<T>(list: &mut Vec<T>, from: usize, to: usize) {
    if from < list.len() && to < list.len() {
        let item = list.remove(from);
        list.insert(to, item);
    }
}

#[allow(dead_code)]
fn _style_used() -> rune::TextStyle {
    UI_STRONG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("r{i}")).collect()
    }

    #[test]
    fn click_ctrl_and_shift_select() {
        let order = ids(6);
        let cols = [Column::new("A", 0.0)];
        let mut t = TableState::new(&cols);
        t.select("r1", &order, false, false);
        assert_eq!(t.selected, ["r1"]);
        t.select("r3", &order, false, true);
        assert_eq!(t.selected, ["r1", "r3"]);
        t.select("r5", &order, true, false);
        assert_eq!(t.selected, ["r3", "r4", "r5"], "shift extends from the last anchor");
        t.select("r4", &order, false, true);
        assert_eq!(t.selected, ["r3", "r5"]);
    }

    #[test]
    fn keys_move_and_extend() {
        let order = ids(4);
        let cols = [Column::new("A", 0.0)];
        let mut t = TableState::new(&cols);
        t.step(1, &order, false);
        assert_eq!(t.selected, ["r0"], "from nothing, Down picks the first");
        t.step(1, &order, true);
        assert_eq!(t.selected, ["r0", "r1"]);
        t.step(100, &order, false);
        assert_eq!(t.selected, ["r3"], "clamped to the last row");
        t.sort_by(0);
        assert_eq!(t.sort, (0, true), "the same column flips direction");
        t.sort_by(2);
        assert_eq!(t.sort, (2, false));
    }

    #[test]
    fn reordering_moves_items() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        moved(&mut v, 0, 2);
        assert_eq!(v, ['b', 'c', 'a', 'd']);
        moved(&mut v, 3, 0);
        assert_eq!(v, ['d', 'b', 'c', 'a']);
        moved(&mut v, 9, 0);
        assert_eq!(v, ['d', 'b', 'c', 'a'], "out of range is ignored");
    }
}
