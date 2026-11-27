//! Select, combobox, number field, text area, tag input, date picker, colour picker, file field.
//!
//! Each keeps a `…State` the app owns and reaches through a route
//! (`|s: &mut App| &mut s.model`), so it opens, highlights and moves by itself.

use std::path::PathBuf;
use std::rc::Rc;

use rune::{Align, Color, Cursor, Drag, El, FileDialogOptions, Key, KeyEvent, Shape, Side, TextEdit, Tx, col, el, icon, row, text, text_input};

use super::controls::field_box;
use super::{Popped, R, Route, TINY, UI, UI_STRONG, act, d, floating, h, is_disabled, matches, route};
use crate::tokens;

/// A shared "an option was picked" handler.
type Pick<S> = Rc<dyn Fn(&mut S, usize)>;
/// A shared "a number was committed" handler.
type Commit<S> = Rc<dyn Fn(&mut S, f64)>;

// ---- select -----------------------------------------------------------------------------------

/// A select's state: open or not, and the keyboard highlight while open.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectState {
    pub open: bool,
    pub highlight: usize,
}

fn chevron() -> El {
    icon(crate::kit().glyph.chevron_down, 11.0).color(tokens().muted)
}

/// A list of options anchored under `key`; `highlight` is drawn lit, `selected` checked.
fn option_list<S: 'static>(key: &str, options: &[&str], selected: Option<usize>, highlight: usize, width: f32, pick: Pick<S>, map: &[usize]) -> El {
    let s = tokens();
    let mut list = floating(col().w(width).padding(5.0).gap(1.0)).role("listbox").max_h(280.0);
    if map.is_empty() {
        list = list.child(row().h(28.0).px(9.0).items_center().child(text("No match").font(UI).color(s.dim)));
    }
    for (pos, &i) in map.iter().enumerate() {
        let f = pick.clone();
        let mut r = row().h(28.0).px(9.0).gap(8.0).items_center().radius(6.0).role("option").checked(selected == Some(i)).cursor(Cursor::Pointer).hover_fill(s.tint).on_press(move |st: &mut S| f(st, i));
        if pos == highlight {
            r = r.fill(s.tint);
        }
        r = r.child(text(options[i]).font(UI).color(s.text).grow(1.0));
        if selected == Some(i) {
            r = r.child(icon(crate::kit().glyph.check, 11.0).color(s.text));
        }
        list = list.child(r);
    }
    list.anchored(key, Side::Bottom, Align::Start, 4.0)
}

/// A select: a field showing the choice; open, a list under it. Up / Down move the highlight
/// (or the choice while closed), Enter / Space pick, Escape closes.
pub fn select<S: 'static>(key: &str, options: &[&str], selected: usize, state: &SelectState, to: impl for<'a> Fn(&'a mut S) -> &'a mut SelectState + 'static, on_pick: impl Fn(&mut S, usize) + 'static) -> Popped {
    let s = tokens();
    let to: Route<S, SelectState> = route(to);
    let pick: Pick<S> = {
        let to = to.clone();
        Rc::new(move |st: &mut S, i: usize| {
            on_pick(st, i);
            *to(st) = SelectState::default();
        })
    };
    let n = options.len();
    let (open, hl) = (state.open, state.highlight.min(n.saturating_sub(1)));
    let mut trigger = field_box(row().key(key).w_full().h(h()).px(d(10.0)).gap(8.0).items_center())
        .role("combobox")
        .expanded(open)
        .child(text(options.get(selected).copied().unwrap_or("")).font(UI).color(s.text).grow(1.0))
        .child(chevron());
    if !is_disabled() {
        let (t1, t2, p) = (to.clone(), to.clone(), pick.clone());
        trigger = trigger
            .focusable(true)
            .cursor(Cursor::Pointer)
            .on_press(move |st: &mut S| {
                let o = t1(st);
                *o = SelectState { open: !o.open, highlight: selected };
            })
            .on_key(move |st: &mut S, k: &KeyEvent, _: &mut Tx| {
                let o = *t2(st);
                match k.key {
                    Key::Down | Key::Up if n > 0 => {
                        let dir = if k.key == Key::Down { 1 } else { n - 1 };
                        if o.open {
                            t2(st).highlight = (o.highlight + dir) % n;
                        } else {
                            p(st, (selected + dir) % n);
                        }
                    }
                    Key::Enter | Key::Text(_) if o.open && (k.key == Key::Enter || k.key == Key::Text(" ".into())) => p(st, o.highlight.min(n - 1)),
                    Key::Enter => *t2(st) = SelectState { open: true, highlight: selected },
                    Key::Escape if o.open => *t2(st) = SelectState::default(),
                    _ => return false,
                }
                true
            });
    }
    let mut layers = Vec::new();
    if open {
        let t = to.clone();
        layers.push(rune_kit::scrim(format!("{key}-scrim"), move |st: &mut S| *t(st) = SelectState::default()));
        let map: Vec<usize> = (0..n).collect();
        layers.push(option_list(key, options, Some(selected), hl, 220.0, pick, &map));
    }
    Popped { trigger, layers }
}

// ---- combobox ---------------------------------------------------------------------------------

/// A combobox's state: the typed text, open, highlight among the matches.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboState {
    pub edit: TextEdit,
    pub open: bool,
    pub highlight: usize,
}

impl ComboState {
    pub fn new(text: &str) -> ComboState {
        ComboState { edit: TextEdit::new(text), open: false, highlight: 0 }
    }
}

/// A text field that filters `options` as you type: Up / Down move through the matches, Enter
/// picks, Escape closes. `on_pick` gets the option's index; the field takes its text.
pub fn combobox<S: 'static>(key: &str, options: &[&str], placeholder: &str, state: &ComboState, to: impl for<'a> Fn(&'a mut S) -> &'a mut ComboState + 'static, on_pick: impl Fn(&mut S, usize) + 'static) -> Popped {
    let s = tokens();
    let to: Route<S, ComboState> = route(to);
    let map: Vec<usize> = (0..options.len()).filter(|&i| matches(options[i], &state.edit.text)).collect();
    let owned: Rc<Vec<String>> = Rc::new(options.iter().map(|o| (*o).to_owned()).collect());
    let pick: Pick<S> = {
        let (to, owned) = (to.clone(), owned.clone());
        Rc::new(move |st: &mut S, i: usize| {
            on_pick(st, i);
            let c = to(st);
            c.edit = TextEdit::new(owned[i].as_str());
            c.open = false;
            c.highlight = 0;
        })
    };
    let mut input = text_input(&state.edit, UI).key(format!("{key}-input")).single_line().placeholder(placeholder, s.dim).caret(s.text, s.text.alpha(0.2)).color(s.text).grow(1.0);
    if !is_disabled() {
        let (t1, t2, t3, t4, p2, m, m2) = (to.clone(), to.clone(), to.clone(), to.clone(), pick.clone(), map.clone(), map.clone());
        input = input
            .on_edit(move |st: &mut S| &mut t1(st).edit)
            .on_input_change(move |st: &mut S, _: &mut Tx| {
                let c = t2(st);
                c.open = true;
                c.highlight = 0;
            })
            .on_input_key(move |st: &mut S, k: &KeyEvent, _: &mut Tx| {
                let c = t3(st);
                let n = m.len();
                match k.key {
                    Key::Down if n > 0 => {
                        c.highlight = if c.open { (c.highlight + 1) % n } else { 0 };
                        c.open = true;
                    }
                    Key::Up if n > 0 => c.highlight = (c.highlight + n - 1) % n,
                    Key::Escape if c.open => c.open = false,
                    _ => return false,
                }
                true
            })
            // Return in a one-line field submits; it picks the highlighted match.
            .on_submit(move |st: &mut S, _: &mut Tx| {
                let (open, hl) = {
                    let c = t4(st);
                    (c.open, c.highlight)
                };
                if open && !m2.is_empty() {
                    p2(st, m2[hl.min(m2.len() - 1)]);
                }
            });
    }
    let t = to.clone();
    let toggle = row().size(22.0, 22.0).center().radius(5.0).hover_fill(s.tint).cursor(Cursor::Pointer).role("button").label("Show options").child(chevron());
    let toggle = act(toggle, move |st: &mut S| {
        let c = t(st);
        c.open = !c.open;
    });
    let trigger = field_box(row().key(key).w_full().h(h()).pl(d(10.0)).pr(4.0).gap(6.0).items_center()).role("combobox").expanded(state.open).child(input).child(toggle);
    let mut layers = Vec::new();
    if state.open {
        let t = to.clone();
        layers.push(rune_kit::scrim(format!("{key}-scrim"), move |st: &mut S| t(st).open = false));
        layers.push(option_list(key, options, None, state.highlight, 240.0, pick, &map));
    }
    Popped { trigger, layers }
}

// ---- number field -----------------------------------------------------------------------------

/// A number you can type, with − / + steppers and arrow keys (`step` per press). `value` is
/// what the field holds; `on_commit` gets the parsed, clamped number on Enter, blur or a step.
#[allow(clippy::too_many_arguments)]
pub fn number_field<S: 'static>(edit: &TextEdit, unit: &str, min: f64, max: f64, step: f64, to: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static, on_commit: impl Fn(&mut S, f64) + 'static) -> El {
    let s = tokens();
    let to: Route<S, TextEdit> = route(to);
    let commit: Commit<S> = Rc::new(on_commit);
    let current = edit.text.trim().parse::<f64>().ok();
    let invalid = current.is_none() || current.is_some_and(|v| v < min || v > max);
    let nudge = {
        let (to, commit) = (to.clone(), commit.clone());
        Rc::new(move |st: &mut S, dir: f64| {
            let e = to(st);
            let v = (e.text.trim().parse::<f64>().unwrap_or(min) + dir * step).clamp(min, max);
            *e = TextEdit::new(fmt_num(v));
            commit(st, v);
        })
    };
    let mut input = text_input(edit, UI_STRONG).single_line().caret(s.text, s.text.alpha(0.2)).color(s.text).grow(1.0);
    if !is_disabled() {
        let (t1, t3, c1, c2, n) = (to.clone(), to.clone(), commit.clone(), commit.clone(), nudge.clone());
        input = input
            .on_edit(move |st: &mut S| t1(st))
            .on_input_key(move |st: &mut S, k: &KeyEvent, _: &mut Tx| match k.key {
                Key::Up => {
                    n(st, 1.0);
                    true
                }
                Key::Down => {
                    n(st, -1.0);
                    true
                }
                _ => false,
            })
            .on_submit(move |st: &mut S, _: &mut Tx| {
                let v = t3(st).text.trim().parse::<f64>().unwrap_or(min).clamp(min, max);
                *t3(st) = TextEdit::new(fmt_num(v));
                c1(st, v);
            })
            .on_focus_change(move |st: &mut S, focused: bool, _: &mut Tx| {
                if !focused
                    && let Ok(v) = st_text(st, &to).trim().parse::<f64>()
                {
                    c2(st, v.clamp(min, max));
                }
            });
    }
    let side = |glyph: &str, dir: f64| {
        let n = nudge.clone();
        act(row().size(24.0, h() - 2.0).center().hover_fill(s.tint).cursor(Cursor::Pointer).role("button").label(if dir < 0.0 { "Decrease" } else { "Increase" }).child(text(glyph).font(UI_STRONG).color(s.muted)), move |st: &mut S| n(st, dir))
    };
    super::invalid(invalid, || field_box(row().h(h()).items_center().clip_rounded(R)))
        .role("spinbutton")
        .child(side("−", -1.0))
        .child(row().grow(1.0).h_full().px(8.0).gap(6.0).items_center().border_sides([0.0, 1.0, 0.0, 1.0], s.line).child(input).child(text(unit).font(TINY).color(s.dim)))
        .child(side("+", 1.0))
}

fn st_text<S>(st: &mut S, to: &Route<S, TextEdit>) -> String {
    to(st).text.clone()
}

/// `12`, `0.5`, `1.25`: no trailing zeros.
pub fn fmt_num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_owned() }
}

// ---- text area --------------------------------------------------------------------------------

/// Multi-line text, `lines` tall; Enter breaks the line.
pub fn textarea<S: 'static>(edit: &TextEdit, placeholder: &str, lines: usize, to: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static) -> El {
    let s = tokens();
    let mut input = text_input(edit, UI).placeholder(placeholder, s.dim).caret(s.text, s.text.alpha(0.2)).color(s.text).w_full();
    if !is_disabled() {
        let t = route(to);
        let t2 = t.clone();
        input = input.on_edit(move |st: &mut S| t(st)).on_submit(move |st: &mut S, _: &mut Tx| t2(st).insert("\n"));
    }
    field_box(col().w_full().h(18.0 * lines as f32 + 16.0).px(10.0).py(8.0)).child(input)
}

// ---- tag input --------------------------------------------------------------------------------

/// A tag input's state: the tags and the text being typed.
#[derive(Clone, Debug, PartialEq)]
pub struct TagState {
    pub tags: Vec<String>,
    pub edit: TextEdit,
}

impl TagState {
    pub fn new(tags: &[&str]) -> TagState {
        TagState { tags: tags.iter().map(|t| (*t).to_owned()).collect(), edit: TextEdit::new("") }
    }

    /// Adds the typed text as tags (split on commas), skipping blanks and duplicates.
    pub fn commit(&mut self) {
        for t in self.edit.text.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            if !self.tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
                self.tags.push(t.to_owned());
            }
        }
        self.edit = TextEdit::new("");
    }
}

/// Tags as removable chips, then a field: Enter or a comma adds, Backspace on an empty field
/// removes the last.
pub fn tag_input<S: 'static>(key: &str, placeholder: &str, state: &TagState, to: impl for<'a> Fn(&'a mut S) -> &'a mut TagState + 'static) -> El {
    let s = tokens();
    let to: Route<S, TagState> = route(to);
    let mut wrap = row().gap(6.0).flex_wrap().items_center().grow(1.0);
    for (i, t) in state.tags.iter().enumerate() {
        let r = to.clone();
        let x = act(row().size(14.0, 14.0).center().radius(4.0).hover_fill(s.tint).cursor(Cursor::Pointer).role("button").label(format!("Remove {t}")).child(icon(crate::kit().glyph.close, 8.0).color(s.muted)), move |st: &mut S| {
            r(st).tags.remove(i);
        });
        wrap = wrap.child(row().h(22.0).pl(8.0).pr(4.0).gap(4.0).items_center().radius(6.0).fill(s.tint).child(text(t.as_str()).font(TINY).color(s.text)).child(x));
    }
    let input_key = format!("{key}-input");
    let mut input = text_input(&state.edit, UI).key(input_key.clone()).single_line().placeholder(if state.tags.is_empty() { placeholder } else { "" }, s.dim).caret(s.text, s.text.alpha(0.2)).color(s.text).min_w(80.0).grow(1.0);
    if !is_disabled() {
        let (t1, t2, t3, t4) = (to.clone(), to.clone(), to.clone(), to.clone());
        input = input
            .on_edit(move |st: &mut S| &mut t1(st).edit)
            // Return adds the tag and keeps typing in the field.
            .on_submit(move |st: &mut S, tx: &mut Tx| {
                t4(st).commit();
                tx.focus(input_key.clone(), false);
            })
            .on_input_change(move |st: &mut S, _: &mut Tx| {
                let c = t2(st);
                if c.edit.text.contains(',') {
                    c.commit();
                }
            })
            .on_input_key(move |st: &mut S, k: &KeyEvent, _: &mut Tx| {
                let c = t3(st);
                match k.key {
                    Key::Backspace if c.edit.text.is_empty() => {
                        c.tags.pop();
                    }
                    _ => return false,
                }
                true
            });
    }
    field_box(row().key(key).w_full().min_h(h()).px(6.0).py(4.0).items_center()).child(wrap.child(input))
}

// ---- date picker ------------------------------------------------------------------------------

/// Days in `month` (1–12) of `year`.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 31,
    }
}

/// The weekday of a date, 0 = Monday … 6 = Sunday.
pub fn weekday(year: i32, month: u32, day: u32) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let sunday_first = (y + y / 4 - y / 100 + y / 400 + T[month as usize - 1] + day as i32).rem_euclid(7);
    ((sunday_first + 6) % 7) as u32
}

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Date {
        let month = month.clamp(1, 12);
        Date { year, month, day: day.clamp(1, days_in_month(year, month)) }
    }
    /// `2026-11-27`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    /// `months` later (or earlier), the day clamped to the month.
    pub fn add_months(self, months: i32) -> Date {
        let idx = self.year * 12 + self.month as i32 - 1 + months;
        Date::new(idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32, self.day)
    }
}

/// A date picker's state: the chosen date, open, and the month shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DateState {
    pub date: Date,
    pub open: bool,
    pub shown: Date,
}

impl DateState {
    pub fn new(date: Date) -> DateState {
        DateState { date, open: false, shown: date }
    }
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// A field showing a date; open, a month grid under it.
pub fn date_picker<S: 'static>(key: &str, state: &DateState, to: impl for<'a> Fn(&'a mut S) -> &'a mut DateState + 'static) -> Popped {
    let s = tokens();
    let to: Route<S, DateState> = route(to);
    let t = to.clone();
    let trigger = act(
        field_box(row().key(key).w_full().h(h()).px(d(10.0)).gap(8.0).items_center())
            .role("button")
            .expanded(state.open)
            .focusable(true)
            .cursor(Cursor::Pointer)
            .child(text(state.date.iso()).font(UI).color(s.text).grow(1.0))
            .child(icon(&crate::icons::oc::OUTLINE_LIST_CHANGELOG, 13.0).color(s.muted)),
        move |st: &mut S| {
            let c = t(st);
            c.open = !c.open;
            c.shown = c.date;
        },
    );
    let mut layers = Vec::new();
    if state.open {
        let t = to.clone();
        layers.push(rune_kit::scrim(format!("{key}-scrim"), move |st: &mut S| t(st).open = false));
        layers.push(calendar(state, to).anchored(key, Side::Bottom, Align::Start, 4.0));
    }
    Popped { trigger, layers }
}

fn calendar<S: 'static>(state: &DateState, to: Route<S, DateState>) -> El {
    let s = tokens();
    let k = crate::kit();
    let shown = state.shown;
    let cell = 32.0;
    let nav = |label: &str, months: i32, flip: bool| {
        let t = to.clone();
        let g = icon(k.glyph.caret_right, 10.0).color(s.muted);
        let g = if flip { g.rotate(std::f32::consts::PI) } else { g };
        row().size(26.0, 26.0).center().radius(6.0).hover_fill(s.tint).cursor(Cursor::Pointer).role("button").label(label).child(g).on_press(move |st: &mut S| {
            let c = t(st);
            c.shown = c.shown.add_months(months);
        })
    };
    let head = row().w(cell * 7.0).items_center().child(nav("Previous month", -1, true)).child(row().grow(1.0).justify_center().child(text(format!("{} {}", MONTHS[shown.month as usize - 1], shown.year)).font(UI_STRONG).color(s.text))).child(nav("Next month", 1, false));
    let mut names = row();
    for n in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
        names = names.child(row().w(cell).h(24.0).center().child(text(n).font(TINY).color(s.dim)));
    }
    let first = weekday(shown.year, shown.month, 1);
    let days = days_in_month(shown.year, shown.month);
    let mut grid = col().role("grid");
    let mut day = 1u32;
    let mut slot = 0u32;
    while day <= days {
        let mut week = row();
        for _ in 0..7 {
            if slot < first || day > days {
                week = week.child(el().size(cell, cell));
            } else {
                let date = Date::new(shown.year, shown.month, day);
                let on = date == state.date;
                let t = to.clone();
                let mut b = row().size(cell - 4.0, cell - 4.0).center().radius(6.0).cursor(Cursor::Pointer).role("gridcell").checked(on).label(date.iso());
                b = if on { b.fill(s.solid) } else { b.hover_fill(s.tint) };
                b = b.child(text(format!("{day}")).font(UI).color(if on { s.solid_fg } else { s.text })).on_press(move |st: &mut S| {
                    let c = t(st);
                    c.date = date;
                    c.open = false;
                });
                week = week.child(row().size(cell, cell).center().child(b));
                day += 1;
            }
            slot += 1;
        }
        grid = grid.child(week);
    }
    floating(col().padding(10.0).gap(6.0)).role("dialog").label("Choose a date").child(head).child(names).child(grid)
}

// ---- colour picker ----------------------------------------------------------------------------

/// A colour picker's state: hue (0–360), saturation and value (0–1), open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorState {
    pub h: f32,
    pub s: f32,
    pub v: f32,
    pub open: bool,
}

impl ColorState {
    pub fn new(h: f32, s: f32, v: f32) -> ColorState {
        ColorState { h, s, v, open: false }
    }
    pub fn color(&self) -> Color {
        hsv(self.h, self.s, self.v)
    }
}

/// HSV (hue in degrees) to a colour.
pub fn hsv(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let to = |u: f32| rune::color::srgb_decode((u + m).clamp(0.0, 1.0));
    Color { r: to(r), g: to(g), b: to(b), a: 1.0 }
}

/// `#rrggbb` of a colour.
pub fn hex_of(c: Color) -> String {
    let e = |u: f32| (rune::color::srgb_encode(u.clamp(0.0, 1.0)) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", e(c.r), e(c.g), e(c.b))
}

/// A swatch and its hex; open, a saturation / value square and a hue bar (drag both).
pub fn color_picker<S: 'static>(key: &str, state: &ColorState, to: impl for<'a> Fn(&'a mut S) -> &'a mut ColorState + 'static) -> Popped {
    let s = tokens();
    let to: Route<S, ColorState> = route(to);
    let c = state.color();
    let t = to.clone();
    let trigger = act(
        field_box(row().key(key).w_full().h(h()).px(6.0).gap(8.0).items_center())
            .role("button")
            .expanded(state.open)
            .focusable(true)
            .cursor(Cursor::Pointer)
            .child(el().size(h() - 12.0, h() - 12.0).radius(5.0).fill(c).border(1.0, s.hair))
            .child(text(hex_of(c)).font(UI).color(s.text).grow(1.0)),
        move |st: &mut S| {
            let c = t(st);
            c.open = !c.open;
        },
    );
    let mut layers = Vec::new();
    if state.open {
        let t = to.clone();
        layers.push(rune_kit::scrim(format!("{key}-scrim"), move |st: &mut S| t(st).open = false));
        let (w, sq) = (220.0, 150.0);
        // Saturation across, value down, as a grid of cells.
        let n = 22;
        let mut shape = Shape::new();
        for i in 0..n {
            for j in 0..n {
                let (x0, y0) = (i as f32 / n as f32, j as f32 / n as f32);
                let col = hsv(state.h, (i as f32 + 0.5) / n as f32, 1.0 - (j as f32 + 0.5) / n as f32);
                let (x1, y1) = (x0 + 1.0 / n as f32 + 0.002, y0 + 1.0 / n as f32 + 0.002);
                shape = shape.fill(vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]], col);
            }
        }
        let t1 = to.clone();
        let square = el()
            .size(w, sq)
            .radius(8.0)
            .clip_rounded(8.0)
            .shape(shape.view_box(1.0, 1.0))
            .cursor(Cursor::Pointer)
            .on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| {
                let c = t1(st);
                c.s = (dr.x / dr.w.max(1.0)).clamp(0.0, 1.0);
                c.v = 1.0 - (dr.y / dr.h.max(1.0)).clamp(0.0, 1.0);
            })
            .child(el().size(12.0, 12.0).radius(6.0).border(2.0, Color::WHITE).absolute(state.s * w - 6.0, (1.0 - state.v) * sq - 6.0));
        let mut hues = Shape::new();
        for i in 0..36 {
            let (x0, x1) = (i as f32 / 36.0, (i + 1) as f32 / 36.0 + 0.002);
            hues = hues.fill(vec![[x0, 0.0], [x1, 0.0], [x1, 1.0], [x0, 1.0]], hsv(i as f32 * 10.0 + 5.0, 1.0, 1.0));
        }
        let t2 = to.clone();
        let bar = el()
            .size(w, 12.0)
            .radius(6.0)
            .clip_rounded(6.0)
            .shape(hues.view_box(1.0, 1.0))
            .cursor(Cursor::Pointer)
            .on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| t2(st).h = (dr.x / dr.w.max(1.0)).clamp(0.0, 1.0) * 360.0)
            .child(el().size(6.0, 16.0).radius(3.0).border(2.0, Color::WHITE).absolute(state.h / 360.0 * w - 3.0, -2.0));
        let mut swatches = row().gap(6.0);
        for (hh, ss, vv) in [(4.0, 0.81, 1.0), (32.0, 0.85, 0.96), (142.0, 0.7, 0.73), (217.0, 0.83, 0.92), (262.0, 0.65, 0.95), (0.0, 0.0, 0.93), (0.0, 0.0, 0.1)] {
            let t3 = to.clone();
            swatches = swatches.child(el().size(22.0, 22.0).radius(6.0).fill(hsv(hh, ss, vv)).border(1.0, s.hair).cursor(Cursor::Pointer).role("button").label(hex_of(hsv(hh, ss, vv))).on_press(move |st: &mut S| {
                let c = t3(st);
                (c.h, c.s, c.v) = (hh, ss, vv);
            }));
        }
        layers.push(floating(col().padding(12.0).gap(12.0)).role("dialog").label("Choose a colour").child(square).child(bar).child(swatches).anchored(key, Side::Bottom, Align::Start, 4.0));
    }
    Popped { trigger, layers }
}

// ---- file field -------------------------------------------------------------------------------

/// A field showing a chosen file and a Browse button that opens the system's file dialog.
pub fn file_field<S: 'static>(chosen: Option<&str>, placeholder: &str, on_choose: impl Fn(&mut S, Vec<PathBuf>) + 'static) -> El {
    let s = tokens();
    let f = Rc::new(on_choose);
    let browse = row().h(h() - 8.0).px(10.0).center().radius(6.0).fill(s.tint).hover_fill(s.text.alpha(0.14)).cursor(Cursor::Pointer).role("button").label("Browse").child(text("Browse…").font(UI_STRONG).color(s.text));
    let browse = if is_disabled() {
        super::inert(browse)
    } else {
        browse.on_press_tx(move |_: &mut S, tx: &mut Tx| {
            let f = f.clone();
            tx.open_files(FileDialogOptions { title: "Choose a file".into(), ..Default::default() }, move |st: &mut S, result, _| {
                if let Ok(Some(paths)) = result {
                    f(st, paths)
                }
            });
        })
    };
    let name = match chosen {
        Some(n) => text(n).font(UI).color(s.text),
        None => text(placeholder).font(UI).color(s.dim),
    };
    field_box(row().w_full().h(h()).pl(d(10.0)).pr(4.0).gap(8.0).items_center()).child(icon(&crate::icons::oc::FILE_DOCUMENT, 13.0).color(s.muted)).child(name.grow(1.0)).child(browse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_maths() {
        assert_eq!(weekday(2026, 11, 27), 4, "a Friday");
        assert_eq!(weekday(2000, 1, 1), 5, "a Saturday");
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(Date::new(2026, 1, 31).add_months(1), Date::new(2026, 2, 28));
        assert_eq!(Date::new(2026, 1, 15).add_months(-2).iso(), "2025-11-15");
    }

    #[test]
    fn tags_commit_split_and_dedupe() {
        let mut t = TagState::new(&["fpga"]);
        t.edit = TextEdit::new("aes, FPGA, , side-channel");
        t.commit();
        assert_eq!(t.tags, ["fpga", "aes", "side-channel"]);
        assert!(t.edit.text.is_empty());
    }

    #[test]
    fn numbers_and_colours_round_trip() {
        assert_eq!(fmt_num(12.0), "12");
        assert_eq!(fmt_num(0.25), "0.25");
        assert_eq!(fmt_num(-0.0), "0");
        assert_eq!(hex_of(hsv(0.0, 1.0, 1.0)), "#ff0000");
        assert_eq!(hex_of(hsv(120.0, 1.0, 1.0)), "#00ff00");
        assert_eq!(hex_of(hsv(0.0, 0.0, 1.0)), "#ffffff");
    }
}
