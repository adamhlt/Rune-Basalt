//! Icons for app chrome: OpenCode's set (MIT, part of the Rune repository).
//!
//! ```no_run
//! use rune_basalt::app;
//! use rune_basalt::icons::oc;
//! # struct State { open: bool }
//! # fn view(open: bool) -> rune::El {
//! app::icon_button(&oc::SIDEBAR, "Toggle sidebar", open, |s: &mut State| s.open = !s.open)
//! # }
//! ```

/// OpenCode's full icon set (`oc::WAVEFORM`, `oc::SIDEBAR`, `oc::FLASK`, …).
pub use rune_theme_opencode::icons as oc;
