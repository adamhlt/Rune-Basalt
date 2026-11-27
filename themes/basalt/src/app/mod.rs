//! Basalt's desktop-software components, re-exported flat (`app::button`, `app::Table`, …):
//!
//! - [`controls`]: buttons, toolbar, segmented control, fields, stepper, slider, toggles.
//! - [`inputs`]: select, combobox, number field, text area, tag input, date and colour pickers,
//!   file field.
//! - [`chrome`]: menu bar, panes, tree, tabs, splitters, docked panels, path bar, status bar,
//!   scroll view, spinner, progress, readouts, empty state.
//! - [`data`]: the table, log lines, reorderable list.
//! - [`overlays`]: command palette, context menu, dialog, toasts, tooltips.
//!
//! Stateful components keep their state in a `…State` value the app owns, and take a *route* to
//! it (`|s: &mut App| &mut s.select`), so they open, highlight and navigate by themselves.
//! Components that open over the window return a [`Popped`] (or layers) for
//! `rune_kit::root(base, layers)`. Build a control inside [`disabled`] to grey it out and drop
//! its handlers, inside [`invalid`] to mark it in error.

pub mod chrome;
pub mod controls;
pub mod data;
pub mod inputs;
pub mod overlays;

#[cfg(test)]
mod behaviour_tests;

pub use chrome::*;
pub use controls::*;
pub use data::*;
pub use inputs::*;
pub use overlays::*;

use std::cell::Cell as Flag;
use std::rc::Rc;

use rune::{Cursor, El, Shadow, TextStyle, el, row, text};

use crate::{MONO, SANS, density, tokens};

// ---- type and metrics -------------------------------------------------------------------------

/// Control text: mono 12.5.
pub const UI: TextStyle = TextStyle::new(12.5, 400.0).family(MONO).lh(18.0);
/// Emphasised control text.
pub const UI_STRONG: TextStyle = TextStyle::new(12.5, 500.0).family(MONO).lh(18.0);
/// Small print, keycaps, counts.
pub const TINY: TextStyle = TextStyle::new(11.0, 400.0).family(MONO).lh(14.0);
/// Section captions: mono 10.5, tracked caps.
pub const CAPTION: TextStyle = TextStyle::new(10.5, 500.0).family(MONO).lh(14.0).tracking(1.0);
/// Pane titles: grotesk 14.
pub const TITLE: TextStyle = TextStyle::new(14.0, 500.0).family(SANS).lh(18.0);
/// Big numbers in readouts: grotesk 22.
pub const FIGURE: TextStyle = TextStyle::new(22.0, 400.0).family(SANS).lh(26.0).tracking(-0.3);

/// Control corner radius.
pub const R: f32 = 8.0;

/// Control height at the current density (30 at 1.0).
pub fn h() -> f32 {
    (30.0 * density()).round()
}

/// `x` points scaled by the current density.
pub fn d(x: f32) -> f32 {
    (x * density()).round()
}

// ---- disabled and invalid scopes --------------------------------------------------------------

thread_local! {
    static DISABLED: Flag<bool> = const { Flag::new(false) };
    static INVALID: Flag<bool> = const { Flag::new(false) };
}

/// Builds `f`'s controls disabled when `cond`: dimmed, not focusable, their handlers dropped.
///
/// ```no_run
/// # use rune_basalt::app::{self, Kind};
/// # struct S;
/// let save = app::disabled(true, || app::button("Save", Kind::Primary, None, None, |_: &mut S| {}));
/// ```
pub fn disabled<R>(cond: bool, f: impl FnOnce() -> R) -> R {
    let prev = DISABLED.with(|d| d.replace(d.get() || cond));
    let r = f();
    DISABLED.with(|d| d.set(prev));
    r
}

/// Builds `f`'s fields marked invalid when `cond`: accent borders (pair with [`field_msg`]).
pub fn invalid<R>(cond: bool, f: impl FnOnce() -> R) -> R {
    let prev = INVALID.with(|d| d.replace(d.get() || cond));
    let r = f();
    INVALID.with(|d| d.set(prev));
    r
}

/// Whether controls being built now are disabled.
pub fn is_disabled() -> bool {
    DISABLED.with(Flag::get)
}

/// Whether fields being built now are invalid.
pub fn is_invalid() -> bool {
    INVALID.with(Flag::get)
}

/// Attaches `f` as the press handler, or gives `e` the disabled look without it.
pub(crate) fn act<S: 'static>(e: El, f: impl Fn(&mut S) + 'static) -> El {
    if is_disabled() { inert(e) } else { e.on_press(f) }
}

/// The disabled look: dimmed, not focusable, announced as disabled.
pub(crate) fn inert(e: El) -> El {
    e.dim(0.4).focusable(false).aria_disabled(true).cursor(Cursor::NotAllowed)
}

/// A shared route to a component's state inside the app's state.
pub type Route<S, T> = Rc<dyn for<'a> Fn(&'a mut S) -> &'a mut T>;

pub(crate) fn route<S, T>(f: impl for<'a> Fn(&'a mut S) -> &'a mut T + 'static) -> Route<S, T> {
    Rc::new(f)
}

/// A trigger in place plus the layers to mount over the window while it is open.
pub struct Popped {
    pub trigger: El,
    pub layers: Vec<El>,
}

impl Popped {
    /// Moves the layers into `sink` and returns the trigger.
    pub fn mount(self, sink: &mut Vec<El>) -> El {
        sink.extend(self.layers);
        self.trigger
    }
}

// ---- small shared pieces ----------------------------------------------------------------------

pub(crate) fn shadow() -> Shadow {
    let s = tokens();
    Shadow { x: 0.0, y: 16.0, blur: 40.0, spread: -12.0, color: rune::Color::BLACK.alpha(s.shadow), inset: false }
}

/// A raised floating surface (menus, palette, dialogs, toasts).
pub fn floating(e: El) -> El {
    let s = tokens();
    e.fill(s.float).border(1.0, s.hair).radius(12.0).box_shadow(shadow())
}

/// Caps caption over a group.
pub fn caption(label: &str) -> El {
    text(label.to_uppercase()).font(CAPTION).color(tokens().dim)
}

/// Severity / state of a badge, dot, toast or log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Neutral,
    Ok,
    Warn,
    Err,
    Accent,
}

impl Level {
    pub fn color(self) -> rune::Color {
        let s = tokens();
        match self {
            Level::Neutral => s.muted,
            Level::Ok => s.ok,
            Level::Warn => s.warn,
            Level::Err => s.danger,
            Level::Accent => s.text,
        }
    }
    pub fn tag(self) -> &'static str {
        match self {
            Level::Neutral => "INFO",
            Level::Ok => "OK",
            Level::Warn => "WARN",
            Level::Err => "ERR",
            Level::Accent => "NOTE",
        }
    }
}

/// A small dot in the level's colour.
pub fn dot(level: Level) -> El {
    let c = level.color();
    el().size(6.0, 6.0).radius(3.0).fill(c).box_shadow(Shadow { x: 0.0, y: 0.0, blur: 6.0, spread: 0.0, color: c.alpha(0.5), inset: false })
}

/// A pill badge: level colour text on its tint.
pub fn badge(label: &str, level: Level) -> El {
    let c = level.color();
    row().h(20.0).px(7.0).center().radius(6.0).fill(c.alpha(0.12)).child(text(label).font(TINY).color(c))
}

/// A keycap (`Ctrl K`, `Esc`).
pub fn kbd(keys: &str) -> El {
    let s = tokens();
    row().h(20.0).min_w(20.0).px(5.0).center().radius(5.0).fill(s.tint).border_sides([1.0, 1.0, 2.0, 1.0], s.hair).child(text(keys).font(TINY).color(s.muted))
}

/// `true` when `label` contains `query`, ignoring case (palette and combobox filtering).
pub fn matches(label: &str, query: &str) -> bool {
    query.is_empty() || label.to_lowercase().contains(&query.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_nest_and_restore() {
        assert!(!is_disabled());
        disabled(true, || {
            assert!(is_disabled());
            disabled(false, || assert!(is_disabled(), "an inner false keeps the outer true"));
        });
        assert!(!is_disabled());
        invalid(true, || assert!(is_invalid()));
        assert!(!is_invalid());
    }

    #[test]
    fn filtering_ignores_case() {
        assert!(matches("Run CPA attack", "cpa"));
        assert!(matches("anything", ""));
        assert!(!matches("Align", "zz"));
    }
}
