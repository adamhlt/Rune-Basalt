//! Command palette, dialog, toasts and tooltips. (The context menu is in [`super::chrome`].)

use std::rc::Rc;

use rune::{Align, El, Key, KeyEvent, Side, TextEdit, TextStyle, Tx, col, el, icon, row, text, text_input};

use super::{Level, Route, TINY, UI, UI_STRONG, act, caption, dot, floating, is_disabled, kbd, matches, route};
use crate::{MONO, SANS, tokens};

// ---- command palette --------------------------------------------------------------------------

/// A command-palette entry.
pub struct Command<'a> {
    pub glyph: &'static rune::Icon,
    pub label: &'a str,
    pub group: &'a str,
    pub shortcut: Option<&'a str>,
}

/// The palette's state: the query and the highlighted match.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteState {
    pub edit: TextEdit,
    pub highlight: usize,
}

impl PaletteState {
    pub fn new() -> PaletteState {
        PaletteState { edit: TextEdit::new(""), highlight: 0 }
    }
    /// The indices into `commands` that match the query, in order.
    pub fn matches(&self, commands: &[Command]) -> Vec<usize> {
        (0..commands.len()).filter(|&i| matches(commands[i].label, &self.edit.text)).collect()
    }
}

impl Default for PaletteState {
    fn default() -> Self {
        Self::new()
    }
}

/// The command palette (Ctrl K) over a dimmed window: a search field over grouped commands.
/// Up / Down move the highlight, Enter runs it (`on_run` gets the command's index), Escape or a
/// press outside closes. Returns the layers to mount.
pub fn command_palette<S: 'static>(commands: &[Command], state: &PaletteState, to: impl for<'a> Fn(&'a mut S) -> &'a mut PaletteState + 'static, on_run: impl Fn(&mut S, usize) + 'static, on_close: impl Fn(&mut S) + 'static) -> Vec<El> {
    let s = tokens();
    let close = Rc::new(on_close);
    let c2 = close.clone();
    let panel = palette_panel(commands, state, true, to, on_run, move |st: &mut S| close(st));
    vec![col().key("palette-backdrop").absolute(0.0, 0.0).w_full().h_full().fill(s.scrim).items_center().pt(110.0).child(panel.modal(true, move |st: &mut S| c2(st))).on_background_press(|_: &mut S| {})]
}

/// The palette's panel alone (for documentation pages); `autofocus` puts the caret in the field.
pub fn palette_panel<S: 'static>(commands: &[Command], state: &PaletteState, autofocus: bool, to: impl for<'a> Fn(&'a mut S) -> &'a mut PaletteState + 'static, on_run: impl Fn(&mut S, usize) + 'static, on_close: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let to: Route<S, PaletteState> = route(to);
    let run = Rc::new(on_run);
    let close = Rc::new(on_close);
    let shown = state.matches(commands);
    let n = shown.len();
    let hl = state.highlight.min(n.saturating_sub(1));

    let mut input = text_input(&state.edit, TextStyle::new(15.0, 400.0).family(MONO)).single_line().placeholder("Type a command or search…", s.dim).caret(s.text, s.text.alpha(0.2)).color(s.text).grow(1.0);
    if autofocus {
        input = input.autofocus();
    }
    let (t1, t2, t3, run_now, c1, order) = (to.clone(), to.clone(), to.clone(), run.clone(), close.clone(), shown.clone());
    input = input
        .on_edit(move |st: &mut S| &mut t1(st).edit)
        // Return in a one-line field submits: it runs the highlighted command (`hl` is current:
        // the view is rebuilt on every change).
        .on_submit(move |st: &mut S, _: &mut Tx| {
            if let Some(&i) = order.get(hl) {
                run_now(st, i);
            }
        })
        .on_input_change(move |st: &mut S, _: &mut Tx| t2(st).highlight = 0)
        .on_input_key(move |st: &mut S, k: &KeyEvent, _: &mut Tx| {
            let p = t3(st);
            match k.key {
                Key::Down if n > 0 => p.highlight = (hl + 1) % n,
                Key::Up if n > 0 => p.highlight = (hl + n - 1) % n,
                Key::Escape => c1(st),
                _ => return false,
            }
            true
        });

    let mut list = col().w_full().padding(6.0).gap(1.0);
    let mut last_group = "";
    for (pos, &i) in shown.iter().enumerate() {
        let c = &commands[i];
        if c.group != last_group {
            list = list.child(row().px(10.0).pt(if pos == 0 { 4.0 } else { 10.0 }).pb(4.0).child(caption(c.group)));
            last_group = c.group;
        }
        let on = pos == hl;
        let f = run.clone();
        let mut r = row().w_full().h(34.0).px(10.0).gap(12.0).items_center().radius(8.0).role("option").checked(on).cursor(rune::Cursor::Pointer).on_press(move |st: &mut S| f(st, i));
        r = if on { r.fill(s.tint) } else { r.hover_fill(s.tint.alpha(0.5)) };
        r = r.child(icon(c.glyph, 15.0).color(if on { s.text } else { s.muted })).child(text(c.label).font(UI).color(s.text).grow(1.0));
        if let Some(k) = c.shortcut {
            r = r.child(kbd(k));
        }
        list = list.child(r);
    }
    if n == 0 {
        list = list.child(row().w_full().h(64.0).center().child(text("No matching command").font(UI).color(s.dim)));
    }
    let k = crate::kit();
    floating(col().w(560.0))
        .role("dialog")
        .label("Command palette")
        .child(row().w_full().h(52.0).px(16.0).gap(12.0).items_center().border_sides([0.0, 0.0, 1.0, 0.0], s.line).child(icon(k.glyph.search, 16.0).color(s.muted)).child(input).child(kbd("Esc")))
        .child(col().key("palette-list").w_full().max_h(360.0).scroll(rune::ScrollSpec::default()).child(list))
        .child(row().w_full().h(34.0).px(16.0).gap(14.0).items_center().border_sides([1.0, 0.0, 0.0, 0.0], s.line).child(row().gap(6.0).items_center().child(kbd("↑↓")).child(text("navigate").font(TINY).color(s.dim))).child(row().gap(6.0).items_center().child(kbd("↵")).child(text("run").font(TINY).color(s.dim))))
}

// ---- dialog -----------------------------------------------------------------------------------

/// A modal dialog over a dimmed window: focus stays inside it, Escape or a press outside
/// dismisses it. Chain `.focus_return("trigger-key")` on the result to hand focus back to the
/// control that opened it.
pub fn dialog<S: 'static>(title: &str, body: &str, actions: El, on_close: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let close = Rc::new(on_close);
    let c2 = close.clone();
    el().key("dialog-backdrop")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .fill(s.scrim)
        .center()
        .child(dialog_panel(title, body, actions).modal(true, move |st: &mut S| c2(st)))
        .on_background_press(move |st: &mut S| close(st))
}

/// A dialog's panel: title, body, actions on the right.
pub fn dialog_panel(title: &str, body: &str, actions: El) -> El {
    let s = tokens();
    floating(col().w(420.0).padding(22.0).gap(10.0))
        .role("alertdialog")
        .label(title)
        .child(text(title).font(TextStyle::new(18.0, 500.0).family(SANS)).color(s.text))
        .child(text(body).font(UI).color(s.muted).wrap())
        .child(row().w_full().pt(14.0).gap(8.0).justify_end().child(actions))
}

// ---- toasts -----------------------------------------------------------------------------------

/// One toast.
#[derive(Clone, Debug, PartialEq)]
pub struct ToastItem {
    pub id: u64,
    pub level: Level,
    pub title: String,
    pub body: String,
    pub action: Option<String>,
    /// When it appeared, in the app's clock (seconds).
    pub born: f64,
}

/// The toasts on screen, kept by the app. Errors stay until dismissed; others expire.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToastQueue {
    pub items: Vec<ToastItem>,
    next: u64,
}

impl ToastQueue {
    /// Seconds a non-error toast stays.
    pub const TTL: f64 = 5.0;
    /// Most toasts shown at once; older ones go first.
    pub const MAX: usize = 4;

    pub fn push(&mut self, now: f64, level: Level, title: &str, body: &str, action: Option<&str>) -> u64 {
        self.next += 1;
        self.items.push(ToastItem { id: self.next, level, title: title.into(), body: body.into(), action: action.map(str::to_owned), born: now });
        while self.items.len() > Self::MAX {
            self.items.remove(0);
        }
        self.next
    }

    pub fn dismiss(&mut self, id: u64) {
        self.items.retain(|t| t.id != id);
    }

    /// Drops the toasts past their time; true if any went (the view changed).
    pub fn expire(&mut self, now: f64) -> bool {
        let before = self.items.len();
        self.items.retain(|t| t.level == Level::Err || now - t.born < Self::TTL);
        before != self.items.len()
    }

    /// When the next toast expires, if any will (for `Tx::wake_at`).
    pub fn next_expiry(&self) -> Option<f64> {
        self.items.iter().filter(|t| t.level != Level::Err).map(|t| t.born + Self::TTL).reduce(f64::min)
    }
}

/// A toast: level dot, title, body, optional action and a dismiss button.
pub fn toast<S: 'static>(level: Level, title: &str, body: &str, action: Option<&str>, on_action: impl Fn(&mut S) + 'static, on_dismiss: impl Fn(&mut S) + 'static) -> El {
    let s = tokens();
    let k = crate::kit();
    let mut texts = col().gap(3.0).grow(1.0).child(text(title).font(UI_STRONG).color(s.text)).child(text(body).font(TINY).color(s.muted).wrap());
    if let Some(a) = action {
        texts = texts.child(row().pt(6.0).child(act(text(a).font(UI_STRONG).color(s.text).border_sides([0.0, 0.0, 1.0, 0.0], s.hair).cursor(rune::Cursor::Pointer).role("button").focusable(true), on_action)));
    }
    floating(row().w(340.0).padding(14.0).gap(12.0).items_start())
        .role("status")
        .child(row().pt(5.0).child(dot(level)))
        .child(texts)
        .child(row().size(20.0, 20.0).center().radius(5.0).hover_fill(s.tint).cursor(rune::Cursor::Pointer).role("button").label("Dismiss").on_press(on_dismiss).child(icon(k.glyph.close, 10.0).color(s.muted)))
}

/// The queue's toasts stacked at the bottom right of a window of `win` points, `inset` above
/// its bottom edge (a status bar's height). `on_action(id)` runs a toast's action.
pub fn toast_stack<S: 'static>(queue: &ToastQueue, win: (f32, f32), inset: f32, to: impl for<'a> Fn(&'a mut S) -> &'a mut ToastQueue + 'static, on_action: impl Fn(&mut S, u64) + 'static) -> El {
    let to: Route<S, ToastQueue> = route(to);
    let act_fn = Rc::new(on_action);
    let mut c = col().gap(8.0).w(340.0).role("region").label("Notifications");
    for t in &queue.items {
        let (id, t1, t2, a) = (t.id, to.clone(), to.clone(), act_fn.clone());
        c = c.child(toast(
            t.level,
            &t.title,
            &t.body,
            t.action.as_deref(),
            move |st: &mut S| {
                a(st, id);
                t1(st).dismiss(id);
            },
            move |st: &mut S| t2(st).dismiss(id),
        ));
    }
    let h = queue.items.len() as f32 * 110.0;
    c.absolute(win.0 - 340.0 - 20.0, (win.1 - inset - 20.0 - h).max(0.0))
}

// ---- tooltips ---------------------------------------------------------------------------------

/// Gives `e` Rune's own tooltip (shown on hover and keyboard focus, placed and flipped by the
/// runtime).
pub fn with_tip(e: El, tip: &str) -> El {
    if is_disabled() { e } else { e.help(tip) }
}

/// A tooltip anchored under a keyed element (mount it as a layer), for tips you control.
pub fn tooltip(anchor: &str, tip: &str, shortcut: Option<&str>) -> El {
    tooltip_bubble(tip, shortcut).anchored(anchor, Side::Bottom, Align::Center, 6.0)
}

/// A tooltip's bubble: text and an optional shortcut.
pub fn tooltip_bubble(tip: &str, shortcut: Option<&str>) -> El {
    let s = tokens();
    let mut r = floating(row().px(9.0).py(5.0).gap(8.0).items_center()).radius(7.0).role("tooltip").child(text(tip).font(TINY).color(s.text));
    if let Some(k) = shortcut {
        r = r.child(text(k).font(TINY).color(s.dim));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_filters_and_keeps_order() {
        let g: &'static rune::Icon = &crate::icons::oc::FLASK;
        let cmds = [Command { glyph: g, label: "Run attack", group: "A", shortcut: None }, Command { glyph: g, label: "Align traces", group: "A", shortcut: None }, Command { glyph: g, label: "Toggle console", group: "V", shortcut: None }];
        let mut p = PaletteState::new();
        assert_eq!(p.matches(&cmds), [0, 1, 2]);
        p.edit = TextEdit::new("TRA");
        assert_eq!(p.matches(&cmds), [1], "case-insensitive substring");
        p.edit = TextEdit::new("o");
        assert_eq!(p.matches(&cmds), [2], "only \"Toggle console\" has an o");
        p.edit = TextEdit::new("zzz");
        assert!(p.matches(&cmds).is_empty());
    }

    #[test]
    fn toasts_expire_but_errors_stay() {
        let mut q = ToastQueue::default();
        let a = q.push(0.0, Level::Ok, "Saved", "", None);
        q.push(0.0, Level::Err, "Failed", "", None);
        assert_eq!(q.next_expiry(), Some(ToastQueue::TTL));
        assert!(!q.expire(1.0));
        assert!(q.expire(6.0));
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].level, Level::Err);
        assert_eq!(q.next_expiry(), None);
        q.dismiss(a);
        for i in 0..8 {
            q.push(0.0, Level::Neutral, &format!("{i}"), "", None);
        }
        assert_eq!(q.items.len(), ToastQueue::MAX, "the oldest go first");
    }
}
