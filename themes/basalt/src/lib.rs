//! Basalt: a mono-and-grotesk design system for desktop software built with Rune. Colour tokens
//! in a dark and a light theme ([`Tokens`], [`Theme`]), a type scale ([`ty`]), software
//! components ([`app`]) and charts ([`plot`]).
//!
//! ```no_run
//! use rune_basalt::app::{self, Kind};
//! # struct State { runs: u32 }
//! # fn view(s: &State) -> rune::El {
//! rune_basalt::set_theme(rune_basalt::Theme::light().accent(rune::color::hex("#2563eb").unwrap()));
//! app::button("Run", Kind::Primary, None, Some("Ctrl R"), |s: &mut State| s.runs += 1)
//! # }
//! ```
//!
//! Components read the current theme ([`theme`]) as they build, so [`set_theme`] before
//! building a view restyles everything in it. Register [`FONTS`] with `AppSpec::fonts`: Space
//! Grotesk (headings) and JetBrains Mono (everything else), both bundled under the SIL OFL.

#![forbid(unsafe_code)]

pub mod app;
pub mod icons;
pub mod plot;

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use rune::{Color, Family, TextStyle};
use rune_kit::{Kit, PaletteFocus, PaletteSeed};

/// Basalt's colour tokens for one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    /// Whether this is a dark theme.
    pub dark: bool,
    /// The deepest background: title bars, sidebars, wells, field fills.
    pub bg: Color,
    /// The window background.
    pub panel: Color,
    /// Floating surfaces: menus, the palette, dialogs, toasts.
    pub float: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text and idle icons.
    pub muted: Color,
    /// Tertiary text, placeholders, captions.
    pub dim: Color,
    /// Opaque rules and control borders.
    pub line: Color,
    /// `text` at low alpha: hairline borders on floating surfaces.
    pub hair: Color,
    /// `text` at low alpha: selected rows, toggled tools, tiles.
    pub tint: Color,
    /// Hovered rows.
    pub hover: Color,
    /// The accent: highlights, such as a chart's peak or window. Not used for errors.
    pub accent: Color,
    /// Errors and destructive actions: stays red whatever the accent is.
    pub danger: Color,
    /// Success.
    pub ok: Color,
    /// Warnings.
    pub warn: Color,
    /// The primary button's fill and ink.
    pub solid: Color,
    pub solid_fg: Color,
    /// The veil behind dialogs and the palette.
    pub scrim: Color,
    /// Drop-shadow strength of floating surfaces (0–1).
    pub shadow: f32,
}

fn hex(s: &str) -> Color {
    rune::color::hex(s).unwrap_or_else(|| panic!("bad colour {s}"))
}

impl Tokens {
    /// The dark theme.
    pub fn dark() -> Tokens {
        let text = hex("#ececec");
        Tokens {
            dark: true,
            bg: hex("#060607"),
            panel: hex("#0a0a0c"),
            float: hex("#161618"),
            text,
            muted: hex("#7f7f87"),
            dim: hex("#4a4a50"),
            line: hex("#1c1c21"),
            hair: text.alpha(0.09),
            tint: text.alpha(0.08),
            hover: hex("#0f0f12"),
            accent: hex("#ff3e30"),
            danger: hex("#ff3e30"),
            ok: hex("#4ade80"),
            warn: hex("#f5b544"),
            solid: hex("#f2f2f2"),
            solid_fg: hex("#000000"),
            scrim: Color::BLACK.alpha(0.55),
            shadow: 0.6,
        }
    }

    /// The light theme.
    pub fn light() -> Tokens {
        let text = hex("#17171a");
        Tokens {
            dark: false,
            bg: hex("#f4f4f5"),
            panel: hex("#ffffff"),
            float: hex("#ffffff"),
            text,
            muted: hex("#62626b"),
            dim: hex("#a1a1aa"),
            line: hex("#e4e4e7"),
            hair: text.alpha(0.12),
            tint: text.alpha(0.06),
            hover: hex("#f7f7f8"),
            accent: hex("#e5372a"),
            danger: hex("#e5372a"),
            ok: hex("#16a34a"),
            warn: hex("#c2780a"),
            solid: hex("#17171a"),
            solid_fg: hex("#ffffff"),
            scrim: hex("#17171a").alpha(0.3),
            shadow: 0.16,
        }
    }
}

/// A theme: tokens and density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub tokens: Tokens,
    /// Scales control heights and paddings: 0.85 compact, 1 regular, 1.15 roomy.
    pub density: f32,
}

impl Theme {
    pub fn dark() -> Theme {
        Theme { tokens: Tokens::dark(), density: 1.0 }
    }
    pub fn light() -> Theme {
        Theme { tokens: Tokens::light(), density: 1.0 }
    }
    /// Replaces the accent colour (highlights); errors keep [`Tokens::danger`].
    pub fn accent(mut self, c: Color) -> Theme {
        self.tokens.accent = c;
        self
    }
    /// Sets the density (clamped to 0.7–1.4).
    pub fn density(mut self, d: f32) -> Theme {
        self.density = d.clamp(0.7, 1.4);
        self
    }
    /// Edits the tokens (any colour).
    pub fn tokens(mut self, f: impl FnOnce(&mut Tokens)) -> Theme {
        f(&mut self.tokens);
        self
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::dark()
    }
}

thread_local! {
    /// The theme views on this thread build with, and its generation. Views are built on the UI
    /// thread, so a theme is per thread: tests and apps with several UI threads don't share one.
    static CURRENT: Cell<Option<(Theme, u64)>> = const { Cell::new(None) };
}

/// Generations are unique across threads, so the kit cache can key on them alone.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Makes `t` the theme components build with from now on (on this thread).
pub fn set_theme(t: Theme) {
    CURRENT.with(|c| {
        let generation = match c.get() {
            Some((old, g)) if old == t => g,
            _ => NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
        };
        c.set(Some((t, generation)));
    });
}

/// The current theme (dark until [`set_theme`]).
pub fn theme() -> Theme {
    current().0
}

fn current() -> (Theme, u64) {
    CURRENT.with(Cell::get).unwrap_or((Theme::dark(), 0))
}

/// The current theme's tokens.
pub fn tokens() -> Tokens {
    theme().tokens
}

/// The current theme's density.
pub fn density() -> f32 {
    theme().density
}

/// Space Grotesk, the headings' face, bundled.
pub const SANS: Family = Family::Named("Space Grotesk");
/// JetBrains Mono, the controls' and body's face, bundled.
pub const MONO: Family = Family::Named("JetBrains Mono");

/// The bundled fonts, for `AppSpec::fonts`: Space Grotesk and JetBrains Mono (both variable
/// weight, SIL OFL 1.1: `assets/fonts/*-OFL.txt`).
pub const FONTS: &[&[u8]] = &[include_bytes!("../assets/fonts/SpaceGrotesk-Variable.ttf"), include_bytes!("../assets/fonts/JetBrainsMono-Variable.ttf")];

/// Basalt's type scale for content. Controls use [`app::UI`] and its siblings.
pub mod ty {
    use super::*;

    const fn sans(size: f32, line: f32, tracking: f32) -> TextStyle {
        TextStyle::new(size, 400.0).family(SANS).lh(line).tracking(tracking)
    }
    const fn mono(size: f32, line: f32) -> TextStyle {
        TextStyle::new(size, 400.0).family(MONO).lh(line)
    }

    /// Display: grotesk 72 / 73, −1.8 tracking.
    pub const DISPLAY: TextStyle = sans(72.0, 73.0, -1.8);
    /// Heading 2: grotesk 40, −0.4 tracking.
    pub const H2: TextStyle = sans(40.0, 48.0, -0.4);
    /// Heading 3: grotesk 25 / 30, −0.25 tracking.
    pub const H3: TextStyle = sans(25.0, 30.0, -0.25);
    /// Heading 4: grotesk 19 / 26.
    pub const H4: TextStyle = sans(19.0, 26.0, 0.0);
    /// Lead paragraph: mono 16 / 29.6.
    pub const LEAD: TextStyle = mono(16.0, 29.6);
    /// Body: mono 14 / 23.8.
    pub const BODY: TextStyle = mono(14.0, 23.8);
    /// Small copy: mono 13 / 22.
    pub const SMALL: TextStyle = mono(13.0, 22.0);
    /// Meta lines: mono 12 / 20.
    pub const META: TextStyle = mono(12.0, 20.0);
    /// Tags: mono 11.5.
    pub const TAG: TextStyle = mono(11.5, 18.0);
    /// Status lines: mono 11.
    pub const STATUS: TextStyle = mono(11.0, 16.0);
}

pub(crate) use app::FIGURE as FIGURE_STYLE;

/// A rune-kit kit in the current theme's colours and type, so rune-kit components sit beside
/// Basalt's. Built once per theme and window blend mode (`rune::theme::blends_srgb`).
pub fn kit() -> &'static Kit {
    static KITS: Mutex<Option<HashMap<(u64, bool), &'static Kit>>> = Mutex::new(None);
    let (t, generation) = current();
    let srgb = rune::theme::blends_srgb();
    let mut kits = KITS.lock().unwrap();
    let kits = kits.get_or_insert_with(HashMap::new);
    kits.entry((generation, srgb)).or_insert_with(|| {
        let s = t.tokens;
        let k = rune::theme::with_blend(srgb, || {
            let mut k = Kit::from_palette_focus(PaletteSeed { bg: s.panel, fg: s.text, accent: s.solid }, PaletteFocus::Ink);
            k.c.page = s.panel;
            k.c.text_muted = s.muted;
            k.c.text_faint = s.dim;
            k.c.border_base = s.hair;
            k.c.danger.icon = s.danger;
            k.c.success.icon = s.ok;
            k.c.warning.icon = s.warn;
            let m = |t: TextStyle| t.family(MONO);
            k.ty.ui = m(k.ty.ui);
            k.ty.ui_medium = m(k.ty.ui_medium);
            k.ty.small = m(k.ty.small);
            k.ty.body = m(k.ty.body);
            k.radius.sm = 8.0;
            k.radius.md = 14.0;
            k.radius.lg = 26.0;
            k.radius.xl = 28.0;
            k
        });
        // One per theme change: a handful over an app's life.
        Box::leak(Box::new(k))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: Color, b: Color) -> f32 {
        let (a, b) = (a.luminance(), b.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn both_themes_keep_text_readable() {
        for t in [Tokens::dark(), Tokens::light()] {
            assert!(contrast(t.text, t.panel) >= 7.0, "text on panel");
            assert!(contrast(t.muted, t.panel) >= 4.5, "muted on panel");
            assert!(contrast(t.solid_fg, t.solid) >= 7.0, "primary button");
        }
    }

    #[test]
    fn set_theme_changes_tokens_and_kit() {
        set_theme(Theme::light().density(2.0));
        assert!(!tokens().dark);
        assert_eq!(density(), 1.4, "density is clamped");
        let light_kit = kit() as *const Kit;
        set_theme(Theme::dark().accent(hex("#2563eb")));
        assert!(tokens().dark);
        assert_eq!(tokens().accent, hex("#2563eb"));
        assert_eq!(tokens().danger, Tokens::dark().danger, "a new accent leaves errors red");
        assert_ne!(kit() as *const Kit, light_kit, "a new theme builds a new kit");
    }
}
