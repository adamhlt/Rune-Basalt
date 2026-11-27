//! Buttons, toolbar, segmented control, fields, stepper, slider and toggles.

use std::rc::Rc;

use rune::{Color, Cursor, Drag, El, Icon, Key, TextEdit, Tx, col, el, icon, row, text, text_input};

use super::{CAPTION, Level, R, TINY, UI, UI_STRONG, act, d, h, inert, is_disabled, is_invalid};
use crate::tokens;

/// Button emphasis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `solid`: the one main action of a view.
    Primary,
    /// `tint`: ordinary actions.
    Secondary,
    /// No fill until hovered: toolbar-weight actions.
    Ghost,
    /// The accent: destructive actions.
    Danger,
}

/// A push button with an optional leading icon and trailing shortcut.
pub fn button<S: 'static>(label: &str, kind: Kind, glyph: Option<&'static Icon>, shortcut: Option<&str>, on_press: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let (fill, hover, ink) = match kind {
        Kind::Primary => (Some(s.solid), if s.dark { Color::WHITE } else { s.text.alpha(0.85) }, s.solid_fg),
        Kind::Secondary => (Some(s.tint), s.text.alpha(0.14), s.text),
        Kind::Ghost => (None, s.tint, s.text),
        Kind::Danger => (Some(s.danger.alpha(0.14)), s.danger.alpha(0.22), s.danger),
    };
    let mut b = row()
        .h(h())
        .px(d(12.0))
        .gap(8.0)
        .items_center()
        .radius(R)
        .hover_fill(hover)
        .role("button")
        .label(label)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .focus_ring(1.5, 2.0, s.text.alpha(0.6));
    if let Some(f) = fill {
        b = b.fill(f);
    }
    if let Some(g) = glyph {
        b = b.child(icon(g, 14.0).color(ink));
    }
    b = b.child(text(label).font(UI_STRONG).color(ink));
    if let Some(k) = shortcut {
        b = b.child(text(k).font(TINY).color(ink.alpha(0.5)));
    }
    act(b, on_press)
}

/// A square icon-only button; `active` keeps it lit (a toggled tool). Its label is its tooltip.
pub fn icon_button<S: 'static>(glyph: &'static Icon, label: &str, active: bool, on_press: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let b = row().size(h(), h()).center().radius(R).role("button").label(label).checked(active).focusable(true).cursor(Cursor::Pointer).help(label);
    let b = if active { b.fill(s.tint) } else { b.hover_fill(s.tint) };
    act(b.child(icon(glyph, 15.0).color(if active { s.text } else { s.muted })), on_press)
}

/// A toolbar button: icon and label, lit when `active`.
pub fn tool<S: 'static>(glyph: &'static Icon, label: &str, active: bool, on_press: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let b = row().h(h()).px(d(10.0)).gap(7.0).items_center().radius(R).role("button").label(label).checked(active).focusable(true).cursor(Cursor::Pointer);
    let b = if active { b.fill(s.tint) } else { b.hover_fill(s.tint) };
    let ink = if active { s.text } else { s.muted };
    act(b.child(icon(glyph, 14.0).color(ink)).child(text(label).font(UI).color(ink)), on_press)
}

/// A vertical hairline between toolbar groups.
pub fn tool_sep() -> El {
    el().w(1.0).h(18.0).mx(4.0).fill(tokens().line)
}

/// A toolbar: its items in a row, arrow keys moving between them.
pub fn toolbar(items: Vec<El>) -> El {
    let mut r = row().gap(4.0).items_center().role("toolbar").focus_group();
    for i in items {
        r = r.child(i);
    }
    r
}

/// A shared action on the app's state.
type Action<S> = Rc<dyn Fn(&mut S)>;

/// An item of an [`overflow_toolbar`]: its icon, label, estimated width in points and action.
pub struct ToolItem<S> {
    pub glyph: &'static Icon,
    pub label: &'static str,
    /// Width the button takes (about 34 + 7 per character).
    pub width: f32,
    pub active: bool,
    pub on_press: Action<S>,
}

impl<S: 'static> ToolItem<S> {
    pub fn new(glyph: &'static Icon, label: &'static str, on_press: impl Fn(&mut S) + 'static) -> ToolItem<S> {
        ToolItem { glyph, label, width: 34.0 + 7.0 * label.chars().count() as f32, active: false, on_press: Rc::new(on_press) }
    }
    pub fn active(mut self, a: bool) -> Self {
        self.active = a;
        self
    }
}

/// How many of `widths` fit in `available` points (4 apart) when the rest go to a 34-point
/// overflow button; all of them when they fit without it.
pub fn fit_count(widths: &[f32], available: f32) -> usize {
    let total: f32 = widths.iter().sum::<f32>() + 4.0 * widths.len().saturating_sub(1) as f32;
    if total <= available {
        return widths.len();
    }
    let mut used = 34.0;
    let mut n = 0;
    for w in widths {
        if used + w + 4.0 > available {
            break;
        }
        used += w + 4.0;
        n += 1;
    }
    n
}

/// A toolbar `width` points wide: the tools that fit are buttons, the rest sit in a "…" menu.
/// `open` / `to` keep whether that menu is open.
pub fn overflow_toolbar<S: 'static>(key: &str, items: Vec<ToolItem<S>>, width: f32, open: bool, to: impl for<'a> Fn(&'a mut S) -> &'a mut bool + 'static) -> super::Popped {
    let to: Rc<dyn for<'a> Fn(&'a mut S) -> &'a mut bool> = Rc::new(to);
    let widths: Vec<f32> = items.iter().map(|i| i.width).collect();
    let fits = fit_count(&widths, width);
    let mut bar = row().key(key).w(width).gap(4.0).items_center().role("toolbar").focus_group();
    let mut rest: Vec<(&'static str, Action<S>)> = Vec::new();
    for (i, it) in items.into_iter().enumerate() {
        if i < fits {
            let f = it.on_press.clone();
            bar = bar.child(tool(it.glyph, it.label, it.active, move |st: &mut S| f(st)));
        } else {
            rest.push((it.label, it.on_press));
        }
    }
    let mut layers = Vec::new();
    if !rest.is_empty() {
        let t = to.clone();
        bar = bar.child(el().grow(1.0)).child(icon_button(&crate::icons::oc::OUTLINE_DOTS, "More tools", open, move |st: &mut S| {
            let o = t(st);
            *o = !*o;
        }).key(format!("{key}-more")));
        if open {
            let t = to.clone();
            layers.push(rune_kit::scrim(format!("{key}-scrim"), move |st: &mut S| *t(st) = false));
            let entries: Vec<Option<super::MenuItem>> = rest.iter().map(|(label, _)| Some(super::MenuItem::new(label))).collect();
            let actions: Vec<Action<S>> = rest.into_iter().map(|(_, f)| f).collect();
            let t = to.clone();
            layers.push(super::menu_panel(&entries, None, move |st: &mut S, i: usize| {
                *t(st) = false;
                actions[i](st);
            }).anchored(format!("{key}-more"), rune::Side::Bottom, rune::Align::End, 4.0));
        }
    }
    super::Popped { trigger: bar, layers }
}

/// A segmented control: one of `options`, the chosen one raised; arrow keys move the choice.
pub fn segmented<S: 'static>(options: &[&str], selected: usize, on_pick: impl Fn(&mut S, usize) + 'static) -> El {
    let s = tokens();
    let pick = Rc::new(on_pick);
    let mut r = row().h(h()).padding(3.0).gap(2.0).radius(R).fill(s.bg).border(1.0, s.line).role("radiogroup").focus_group();
    for (i, o) in options.iter().enumerate() {
        let on = i == selected;
        let f = pick.clone();
        let mut b = row().h_full().px(d(10.0)).center().radius(R - 3.0).role("radio").label(*o).checked(on).focusable(true).cursor(Cursor::Pointer);
        b = if on { b.fill(s.tint).border(1.0, s.hair) } else { b.hover_fill(s.tint.alpha(0.5)) };
        r = r.child(act(b.child(text(*o).font(UI).color(if on { s.text } else { s.muted })), move |st: &mut S| f(st, i)));
    }
    r
}

// ---- fields -----------------------------------------------------------------------------------

/// A labelled field: caption above the control.
pub fn field(label: &str, control: El) -> El {
    field_msg(label, control, None, None)
}

/// A labelled field with a hint under it, or an error (in the accent) instead.
pub fn field_msg(label: &str, control: El, hint: Option<&str>, error: Option<&str>) -> El {
    let s = tokens();
    let mut c = col().gap(6.0).child(text(label).font(TINY).color(s.muted)).child(control);
    match (error, hint) {
        (Some(e), _) => c = c.child(row().gap(6.0).items_center().child(super::dot(Level::Err)).child(text(e).font(TINY).color(s.danger))),
        (None, Some(h)) => c = c.child(text(h).font(TINY).color(s.dim)),
        _ => {}
    }
    c
}

/// The box fields share: fill, border (accent when invalid), focus ring.
pub(crate) fn field_box(e: El) -> El {
    let s = tokens();
    let border = if is_invalid() { s.danger } else { s.line };
    let e = e.radius(R).fill(s.bg).border(1.0, border).focus_within_ring(1.5, 0.0, if is_invalid() { s.danger } else { s.text.alpha(0.5) });
    if is_disabled() { inert(e) } else { e.hover_border(if is_invalid() { s.danger } else { s.hair }) }
}

/// A one-line text field with an optional leading icon and trailing unit.
pub fn text_field<S: 'static>(edit: &TextEdit, placeholder: &str, glyph: Option<&'static Icon>, unit: Option<&str>, route: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static) -> El {
    let s = tokens();
    let mut input = text_input(edit, UI).single_line().placeholder(placeholder, s.dim).caret(s.text, s.text.alpha(0.2)).color(s.text).grow(1.0);
    if !is_disabled() {
        input = input.on_edit(route);
    }
    let mut b = field_box(row().h(h()).px(d(10.0)).gap(8.0).items_center());
    if let Some(g) = glyph {
        b = b.child(icon(g, 13.0).color(s.dim));
    }
    b = b.child(input);
    if let Some(u) = unit {
        b = b.child(text(u).font(TINY).color(s.dim));
    }
    b
}

/// A number with − and + steppers; `on_step` gets −1 or +1 (display only: see
/// [`crate::app::number_field`] for one you can type in).
pub fn stepper<S: 'static>(value: &str, unit: &str, on_step: impl Fn(&mut S, i32) + 'static) -> El {
    let s = tokens();
    let step = Rc::new(on_step);
    let side = |glyph: &str, dir: i32| {
        let f = step.clone();
        let b = row().size(26.0, h() - 2.0).center().hover_fill(s.tint).cursor(Cursor::Pointer).role("button").label(if dir < 0 { "Decrease" } else { "Increase" });
        act(b.child(text(glyph).font(UI_STRONG).color(s.muted)), move |st: &mut S| f(st, dir))
    };
    let f = step.clone();
    let keys = move |st: &mut S, k: &rune::KeyEvent, _: &mut Tx| match k.key {
        Key::Up | Key::Right => {
            f(st, 1);
            true
        }
        Key::Down | Key::Left => {
            f(st, -1);
            true
        }
        _ => false,
    };
    let mut e = field_box(row().h(h()).items_center().clip_rounded(R))
        .role("spinbutton")
        .a11y_value(format!("{value} {unit}"))
        .child(side("−", -1))
        .child(row().grow(1.0).h_full().center().gap(4.0).border_sides([0.0, 1.0, 0.0, 1.0], s.line).child(text(value).font(UI_STRONG).color(s.text)).child(text(unit).font(TINY).color(s.dim)))
        .child(side("+", 1));
    if !is_disabled() {
        e = e.focusable(true).on_key(keys);
    }
    e
}

/// A 0–1 slider with a numeric readout; drag it, or focus it and use the arrow keys (5% a step).
pub fn slider<S: 'static>(value: f32, readout: &str, width: f32, on_change: impl Fn(&mut S, f32) + 'static) -> El {
    let s = tokens();
    let v = value.clamp(0.0, 1.0);
    let track_w = width - 56.0;
    let knob = 14.0;
    let change = Rc::new(on_change);
    let mut track = el()
        .size(track_w, 20.0)
        .cursor(Cursor::Pointer)
        .role("slider")
        .a11y_value(readout)
        .child(el().size(track_w, 4.0).radius(2.0).fill(s.line).absolute(0.0, 8.0))
        .child(el().size(v * track_w, 4.0).radius(2.0).fill(s.text).absolute(0.0, 8.0))
        .child(el().size(knob, knob).radius(knob * 0.5).fill(s.solid).border(3.0, s.panel).absolute(v * (track_w - knob), 3.0));
    if is_disabled() {
        track = inert(track);
    } else {
        let (f, g) = (change.clone(), change.clone());
        track = track
            .focusable(true)
            .on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| f(st, (dr.x / dr.w.max(1.0)).clamp(0.0, 1.0)))
            .on_key(move |st: &mut S, k: &rune::KeyEvent, _: &mut Tx| {
                let step = match k.key {
                    Key::Right | Key::Up => 0.05,
                    Key::Left | Key::Down => -0.05,
                    Key::Home => -1.0,
                    Key::End => 1.0,
                    _ => return false,
                };
                g(st, (v + step).clamp(0.0, 1.0));
                true
            });
    }
    row().w(width).gap(10.0).items_center().child(track).child(row().w(46.0).justify_end().child(text(readout).font(UI).color(s.text)))
}

/// A checkbox with its label.
pub fn checkbox<S: 'static>(label: &str, on: bool, on_toggle: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let mut b = el().size(16.0, 16.0).radius(4.5).center();
    b = if on { b.fill(s.solid).child(icon(crate::kit().glyph.check, 11.0).color(s.solid_fg)) } else { b.fill(s.bg).border(1.0, if is_invalid() { s.danger } else { s.dim }) };
    act(row().gap(9.0).items_center().role("checkbox").label(label).checked(on).focusable(true).cursor(Cursor::Pointer).child(b).child(text(label).font(UI).color(s.text)), on_toggle)
}

/// An on/off switch with its label on the left, taking the row's width.
pub fn switch<S: 'static>(label: &str, on: bool, on_toggle: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let (w, th) = (30.0, 18.0);
    let knob = el().size(th - 6.0, th - 6.0).radius((th - 6.0) * 0.5).fill(if on { s.solid_fg } else { s.muted }).absolute(if on { w - th + 3.0 } else { 3.0 }, 3.0).motion(crate::kit().motion.control);
    let track = el().size(w, th).radius(th * 0.5).fill(if on { s.solid } else { s.line }).child(knob);
    act(row().w_full().gap(10.0).items_center().justify_between().role("switch").label(label).checked(on).focusable(true).cursor(Cursor::Pointer).child(text(label).font(UI).color(s.text)).child(track), on_toggle)
}

/// Radio buttons, stacked; arrow keys move between them.
pub fn radio<S: 'static>(options: &[&str], selected: usize, on_pick: impl Fn(&mut S, usize) + 'static) -> El {
    let s = tokens();
    let pick = Rc::new(on_pick);
    let mut c = col().gap(8.0).role("radiogroup").focus_group();
    for (i, o) in options.iter().enumerate() {
        let on = i == selected;
        let f = pick.clone();
        let mut ring = el().size(16.0, 16.0).radius(8.0).center();
        ring = if on { ring.border(5.0, s.solid) } else { ring.fill(s.bg).border(1.0, s.dim) };
        let item = row().gap(9.0).items_center().role("radio").label(*o).checked(on).focusable(true).cursor(Cursor::Pointer).child(ring).child(text(*o).font(UI).color(if on { s.text } else { s.muted }));
        c = c.child(act(item, move |st: &mut S| f(st, i)));
    }
    c
}

/// Caps caption text in `CAPTION` style, for callers that want it inline.
pub fn caption_text(label: &str) -> El {
    text(label.to_uppercase()).font(CAPTION).color(tokens().dim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_fit_or_overflow() {
        let w = [60.0, 60.0, 60.0, 60.0];
        assert_eq!(fit_count(&w, 1000.0), 4, "everything fits without the overflow button");
        assert_eq!(fit_count(&w, 252.0), 4, "exactly full: 4×60 + 3×4");
        // 200 wide: the overflow button (34) leaves 166: two tools (60+4 each) fit, a third doesn't.
        assert_eq!(fit_count(&w, 200.0), 2);
        assert_eq!(fit_count(&w, 20.0), 0, "narrower than the overflow button: all overflow");
        assert_eq!(fit_count(&[], 100.0), 0);
    }
}
