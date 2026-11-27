//! Charts for analysis tools: a line plot (multi-series, axes computed from the data, zoom and
//! pan, legend, linear or log scale, a highlighted window and a cursor), a scatter plot, a heat
//! map, a sparkline and a histogram. All drawn as vector shapes.

use rune::{Color, Cursor, Drag, El, Shape, Tx, WheelEvent, col, el, row, text};

use crate::app::{TINY, d};
use crate::tokens;

// ---- zoom and pan state -----------------------------------------------------------------------

/// What a [`Plot`] keeps between frames: the visible part of the x axis (0–1 of the data) and
/// the cursor (0–1 of the *data*).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotState {
    pub x0: f32,
    pub x1: f32,
    pub cursor: Option<f32>,
}

impl Default for PlotState {
    fn default() -> Self {
        PlotState { x0: 0.0, x1: 1.0, cursor: None }
    }
}

impl PlotState {
    pub fn span(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Zooms by `factor` (>1 in) keeping the point `at` (0–1 across the *view*) where it is.
    pub fn zoom(&mut self, at: f32, factor: f32) {
        let span = self.span();
        let new = (span / factor).clamp(0.005, 1.0);
        let world = self.x0 + at.clamp(0.0, 1.0) * span;
        self.x0 = (world - at.clamp(0.0, 1.0) * new).clamp(0.0, 1.0 - new);
        self.x1 = self.x0 + new;
    }

    /// Pans by `dx` of the *view*'s width (positive moves right).
    pub fn pan(&mut self, dx: f32) {
        let span = self.span();
        self.x0 = (self.x0 + dx * span).clamp(0.0, 1.0 - span);
        self.x1 = self.x0 + span;
    }

    pub fn reset(&mut self) {
        *self = PlotState { cursor: self.cursor, ..PlotState::default() };
    }

    pub fn is_zoomed(&self) -> bool {
        self.span() < 0.999
    }
}

// ---- ticks ------------------------------------------------------------------------------------

/// About `n` round tick values covering `lo..=hi` (1, 2, 5 × a power of ten steps).
pub fn nice_ticks(lo: f32, hi: f32, n: usize) -> Vec<f32> {
    if hi.partial_cmp(&lo) != Some(std::cmp::Ordering::Greater) || n == 0 {
        return vec![lo];
    }
    let raw = (hi - lo) / n as f32;
    let mag = 10f32.powf(raw.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    let first = (lo / step).ceil() * step;
    let mut out = Vec::new();
    let mut v = first;
    while v <= hi + step * 1e-3 {
        out.push(if v.abs() < step * 1e-4 { 0.0 } else { v });
        v += step;
    }
    out
}

/// `1200` → `1.2k`, `0.5` → `0.5`, no trailing zeros.
pub fn fmt_tick(v: f32) -> String {
    let a = v.abs();
    let (x, suffix) = if a >= 1.0e6 { (v / 1.0e6, "M") } else if a >= 1.0e4 { (v / 1.0e3, "k") } else { (v, "") };
    let s = if x.fract().abs() < 1e-4 { format!("{x:.0}") } else if a < 1.0 { format!("{x:.2}") } else { format!("{x:.1}") };
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_owned() } else { s };
    format!("{s}{suffix}")
}

// ---- line plot --------------------------------------------------------------------------------

/// One series: samples, colour, stroke width, legend label.
pub struct Series<'a> {
    pub values: &'a [f32],
    pub color: Color,
    pub width: f32,
    pub label: &'a str,
}

impl<'a> Series<'a> {
    pub fn new(label: &'a str, values: &'a [f32], color: Color) -> Series<'a> {
        Series { values, color, width: 1.3, label }
    }
}

/// A line plot. The x axis spans `x_domain` over the data (so ticks are real sample numbers,
/// times…), the y axis is `y_range` or the visible data's range, optionally logarithmic.
pub struct Plot<'a> {
    pub series: Vec<Series<'a>>,
    /// `(first, last)` x value of the data, for the ticks.
    pub x_domain: (f32, f32),
    pub x_label: &'a str,
    /// `None`: fit the visible data.
    pub y_range: Option<(f32, f32)>,
    pub log_y: bool,
    /// A highlighted x window, as a 0–1 fraction of the data.
    pub window: Option<(f32, f32)>,
    pub legend: bool,
}

impl<'a> Plot<'a> {
    pub fn new(series: Vec<Series<'a>>) -> Plot<'a> {
        let n = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        Plot { series, x_domain: (0.0, n.saturating_sub(1) as f32), x_label: "", y_range: None, log_y: false, window: None, legend: true }
    }

    /// `w` × `h` points. Wheel up zooms in around the pointer, Shift + wheel (or a sideways swipe)
    /// pans, a double press resets, a drag moves the cursor.
    pub fn build<S: 'static>(self, w: f32, h: f32, state: &PlotState, to: impl for<'x> Fn(&'x mut S) -> &'x mut PlotState + 'static) -> El {
        let s = tokens();
        let to: std::rc::Rc<dyn for<'x> Fn(&'x mut S) -> &'x mut PlotState> = std::rc::Rc::new(to);
        let legend_h = if self.legend && self.series.iter().any(|s| !s.label.is_empty()) { 24.0 } else { 0.0 };
        let gutter = 48.0;
        let foot = 22.0;
        let (pw, ph) = (w - gutter, h - foot - legend_h);
        let (x0, x1) = (state.x0, state.x1);
        let span = (x1 - x0).max(1e-6);
        let tr = |v: f32| if self.log_y { v.max(1e-9).log10() } else { v };

        // The visible slice of every series, and the y range.
        let slices: Vec<&[f32]> = self.series.iter().map(|sr| {
            let n = sr.values.len();
            let (i0, i1) = (((x0 * (n.max(1) - 1) as f32).floor() as usize).min(n), (((x1 * (n.max(1) - 1) as f32).ceil() as usize) + 1).min(n));
            &sr.values[i0..i1.max(i0)]
        }).collect();
        let (lo, hi) = match self.y_range {
            Some((a, b)) => (tr(a), tr(b)),
            None => {
                let (a, b) = slices.iter().flat_map(|v| v.iter()).fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(tr(*v)), b.max(tr(*v))));
                if a > b { (0.0, 1.0) } else { let pad = (b - a).max(1e-6) * 0.06; (a - pad, b + pad) }
            }
        };
        let yspan = (hi - lo).max(1e-9);
        let ypos = |v: f32| ph - ((tr(v) - lo) / yspan).clamp(0.0, 1.0) * ph;

        let mut shape = Shape::new();
        let yticks: Vec<f32> = if self.log_y {
            (lo.floor() as i32..=hi.ceil() as i32).map(|e| 10f32.powi(e)).filter(|v| v.log10() >= lo - 1e-3 && v.log10() <= hi + 1e-3).collect()
        } else {
            nice_ticks(lo, hi, 4)
        };
        for t in &yticks {
            let y = if self.log_y { ph - ((t.log10() - lo) / yspan) * ph } else { ph - ((t - lo) / yspan) * ph };
            shape = shape.stroke(vec![[0.0, y], [pw, y]], 1.0, s.line.alpha(0.7));
        }
        shape = shape.stroke(vec![[0.0, ph], [pw, ph]], 1.0, s.line);
        let (dom0, dom1) = self.x_domain;
        let xticks = nice_ticks(dom0 + x0 * (dom1 - dom0), dom0 + x1 * (dom1 - dom0), 6);
        let to_x = |v: f32| (((v - dom0) / (dom1 - dom0).max(1e-9)) - x0) / span * pw;
        for t in &xticks {
            let x = to_x(*t);
            if (0.0..=pw).contains(&x) {
                shape = shape.stroke(vec![[x, 0.0], [x, ph]], 1.0, s.line.alpha(0.35));
            }
        }
        if let Some((a, b)) = self.window {
            let (xa, xb) = (((a - x0) / span * pw).clamp(0.0, pw), ((b - x0) / span * pw).clamp(0.0, pw));
            if xb > xa {
                shape = shape.fill(vec![[xa, 0.0], [xb, 0.0], [xb, ph], [xa, ph]], s.accent.alpha(0.08));
                shape = shape.stroke(vec![[xa, 0.0], [xa, ph]], 1.0, s.accent.alpha(0.5)).stroke(vec![[xb, 0.0], [xb, ph]], 1.0, s.accent.alpha(0.5));
            }
        }
        for (sr, vis) in self.series.iter().zip(&slices) {
            let n = sr.values.len().max(2);
            let first = (x0 * (sr.values.len().max(1) - 1) as f32).floor() as usize;
            let at = |i: usize| ((first + i) as f32 / (n - 1) as f32 - x0) / span * pw;
            // Min / max per bucket once there are more samples than pixels, so peaks survive.
            let bucket = (vis.len() / (pw as usize).max(1)).max(1);
            let mut pts: Vec<[f32; 2]> = Vec::with_capacity(vis.len().min(pw as usize * 2) + 2);
            if bucket <= 1 {
                pts.extend(vis.iter().enumerate().map(|(i, v)| [at(i), ypos(*v)]));
            } else {
                for (b, chunk) in vis.chunks(bucket).enumerate() {
                    let (mut imin, mut imax) = (0, 0);
                    for (i, v) in chunk.iter().enumerate() {
                        if *v < chunk[imin] { imin = i }
                        if *v > chunk[imax] { imax = i }
                    }
                    let mut pair = [imin, imax];
                    pair.sort_unstable();
                    for i in pair {
                        pts.push([at(b * bucket + i), ypos(chunk[i])]);
                    }
                }
            }
            if pts.len() >= 2 {
                shape = shape.stroke(pts, sr.width, sr.color);
            }
        }
        if let Some(c) = state.cursor {
            let x = (c - x0) / span * pw;
            if (0.0..=pw).contains(&x) {
                shape = shape.stroke(vec![[x, 0.0], [x, ph]], 1.0, s.text.alpha(0.7));
            }
        }

        let (t1, t2, t3, t4) = (to.clone(), to.clone(), to.clone(), to.clone());
        let area = el()
            .key("plot-area")
            .size(pw, ph)
            .shape(shape)
            .cursor(Cursor::Text)
            .role("img")
            .label("Plot")
            .on_drag(move |st: &mut S, dr: &mut Drag, _: &mut Tx| {
                let p = t1(st);
                p.cursor = Some(p.x0 + (dr.x / dr.w.max(1.0)).clamp(0.0, 1.0) * p.span());
            })
            .on_wheel(move |st: &mut S, ev: WheelEvent, _: &mut Tx| {
                let p = t2(st);
                if ev.mods.shift || ev.dx.abs() > ev.dy.abs() {
                    p.pan(-(if ev.dx.abs() > ev.dy.abs() { ev.dx } else { ev.dy }) / pw.max(1.0));
                } else if ev.dy != 0.0 {
                    // Wheel up (negative dy) zooms in, as in every chart tool.
                    p.zoom((ev.x / pw.max(1.0)).clamp(0.0, 1.0), (1.0 - ev.dy * 0.01).clamp(0.5, 2.0));
                }
                true
            })
            .on_double_press(move |st: &mut S| t3(st).reset());

        let mut ylabels = el().size(gutter, ph);
        for t in &yticks {
            let y = if self.log_y { ph - ((t.log10() - lo) / yspan) * ph } else { ph - ((t - lo) / yspan) * ph };
            ylabels = ylabels.child(row().w(gutter - 8.0).justify_end().absolute(0.0, (y - 7.0).clamp(0.0, ph - 14.0)).child(text(fmt_tick(*t)).font(TINY).color(s.dim)));
        }
        let mut xlabels = el().size(pw, foot);
        for t in &xticks {
            let x = to_x(*t);
            if (0.0..=pw).contains(&x) {
                xlabels = xlabels.child(row().w(60.0).justify_center().absolute((x - 30.0).clamp(0.0, pw - 60.0), 4.0).child(text(fmt_tick(*t)).font(TINY).color(s.dim)));
            }
        }
        let mut c = col().key("plot").size(w, h);
        if legend_h > 0.0 {
            let mut lg = row().h(legend_h).pl(gutter).gap(16.0).items_center();
            for sr in self.series.iter().filter(|s| !s.label.is_empty()) {
                lg = lg.child(row().gap(6.0).items_center().child(el().size(10.0, 2.0).fill(sr.color)).child(text(sr.label).font(TINY).color(s.muted)));
            }
            if state.is_zoomed() {
                let t = t4;
                lg = lg.child(el().grow(1.0)).child(row().pr(8.0).child(text(format!("zoom ×{:.0} · double-click resets", 1.0 / state.span())).font(TINY).color(s.dim).role("button").cursor(Cursor::Pointer).on_press(move |st: &mut S| t(st).reset())));
            }
            c = c.child(lg);
        }
        c.child(row().child(ylabels).child(area)).child(row().child(el().w(gutter)).child(xlabels)).child(if self.x_label.is_empty() { el() } else { row().pl(gutter).child(text(self.x_label).font(TINY).color(s.dim)) })
    }
}

// ---- scatter and heat map ---------------------------------------------------------------------

/// A scatter plot of `points` (x, y) over the given ranges; `hot` points are drawn in the accent.
pub fn scatter(points: &[(f32, f32)], hot: &[usize], x_range: (f32, f32), y_range: (f32, f32), w: f32, h: f32) -> El {
    let s = tokens();
    let (xs, ys) = ((x_range.1 - x_range.0).max(1e-9), (y_range.1 - y_range.0).max(1e-9));
    let mut shape = Shape::new();
    for t in nice_ticks(y_range.0, y_range.1, 4) {
        let y = h - (t - y_range.0) / ys * h;
        shape = shape.stroke(vec![[0.0, y], [w, y]], 1.0, s.line.alpha(0.6));
    }
    for t in nice_ticks(x_range.0, x_range.1, 6) {
        let x = (t - x_range.0) / xs * w;
        shape = shape.stroke(vec![[x, 0.0], [x, h]], 1.0, s.line.alpha(0.35));
    }
    let r = 2.2;
    for (i, (x, y)) in points.iter().enumerate() {
        let (px, py) = ((x - x_range.0) / xs * w, h - (y - y_range.0) / ys * h);
        if px < -r || px > w + r || py < -r || py > h + r {
            continue;
        }
        let c = if hot.contains(&i) { s.accent } else { s.text.alpha(0.7) };
        shape = shape.fill(vec![[px - r, py - r], [px + r, py - r], [px + r, py + r], [px - r, py + r]], c);
    }
    el().size(w, h).shape(shape).role("img").label("Scatter plot")
}

/// A heat map of `values` (row-major, `cols` per row): low cells fade into the background, high
/// ones go to the text colour, the `peak` cell (row, col) to the accent.
pub fn heatmap(values: &[f32], cols: usize, peak: Option<(usize, usize)>, w: f32, h: f32) -> El {
    let s = tokens();
    let cols = cols.max(1);
    let rows = values.len().div_ceil(cols).max(1);
    let (lo, hi) = values.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let span = (hi - lo).max(1e-9);
    let (cw, ch) = (w / cols as f32, h / rows as f32);
    let mut shape = Shape::new();
    for (i, v) in values.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let t = ((v - lo) / span).clamp(0.0, 1.0);
        let col = if peak == Some((r, c)) { s.accent } else { s.text.alpha(0.06 + 0.9 * t * t) };
        let (x, y) = (c as f32 * cw, r as f32 * ch);
        shape = shape.fill(vec![[x + 0.5, y + 0.5], [x + cw - 0.5, y + 0.5], [x + cw - 0.5, y + ch - 0.5], [x + 0.5, y + ch - 0.5]], col);
    }
    el().size(w, h).shape(shape).role("img").label("Heat map")
}

// ---- sparkline and histogram ------------------------------------------------------------------

/// A sparkline: one series filling `w` × `h`.
pub fn sparkline(values: &[f32], w: f32, h: f32, color: Color) -> El {
    let (lo, hi) = values.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let span = (hi - lo).max(1e-6);
    let n = values.len().max(2);
    let pts: Vec<[f32; 2]> = values.iter().enumerate().map(|(i, v)| [w * i as f32 / (n - 1) as f32, h - 1.0 - (v - lo) / span * (h - 2.0)]).collect();
    el().size(w, h).shape(Shape::new().stroke(pts, 1.4, color)).a11y_hidden()
}

/// A histogram: one bar per value (0–1), the `hot` one in the accent colour.
pub fn bars(values: &[f32], hot: Option<usize>, w: f32, h: f32) -> El {
    let s = tokens();
    let n = values.len().max(1);
    let gap = 2.0;
    let bw = (w - gap * (n as f32 - 1.0)) / n as f32;
    let mut shape = Shape::new();
    for (i, v) in values.iter().enumerate() {
        let bh = v.clamp(0.0, 1.0) * h;
        let x = i as f32 * (bw + gap);
        let c = if Some(i) == hot { s.accent } else { s.muted.alpha(0.55) };
        shape = shape.fill(vec![[x, h - bh], [x + bw, h - bh], [x + bw, h], [x, h]], c);
    }
    el().size(w, h).shape(shape).a11y_hidden()
}

#[allow(dead_code)]
fn _density_used() -> f32 {
    d(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_are_round_and_cover_the_range() {
        assert_eq!(nice_ticks(0.0, 1000.0, 4), [0.0, 500.0, 1000.0]);
        assert_eq!(nice_ticks(0.0, 1.0, 4), [0.0, 0.5, 1.0]);
        let t = nice_ticks(3.0, 47.0, 5);
        assert!(t.iter().all(|v| (3.0..=47.0).contains(v)) && t.len() >= 3, "{t:?}");
        assert!(t.windows(2).all(|w| w[1] > w[0]));
        assert_eq!(nice_ticks(5.0, 5.0, 4), [5.0], "a flat range has one tick");
    }

    #[test]
    fn ticks_follow_the_zoomed_domain() {
        // A bug before: the x labels were fixed at 0–1000 whatever was shown.
        let t = nice_ticks(2000.0, 2100.0, 5);
        assert!(t.first().is_some_and(|v| *v >= 2000.0) && t.last().is_some_and(|v| *v <= 2100.0), "{t:?}");
    }

    #[test]
    fn tick_labels_are_short() {
        assert_eq!(fmt_tick(0.0), "0");
        assert_eq!(fmt_tick(0.5), "0.5");
        assert_eq!(fmt_tick(250.0), "250");
        assert_eq!(fmt_tick(12_000.0), "12k");
        assert_eq!(fmt_tick(2_500_000.0), "2.5M");
    }

    #[test]
    fn zoom_keeps_the_pointer_still_and_pan_stays_inside() {
        let mut p = PlotState::default();
        p.zoom(0.5, 4.0);
        assert!((p.span() - 0.25).abs() < 1e-6);
        assert!((p.x0 + 0.125 - 0.5).abs() < 1e-5, "the middle stays in the middle: {p:?}");
        p.pan(100.0);
        assert!((p.x1 - 1.0).abs() < 1e-6, "pan stops at the end");
        p.pan(-100.0);
        assert!(p.x0.abs() < 1e-6, "and at the start");
        p.zoom(0.0, 1000.0);
        assert!(p.span() >= 0.005, "zoom is bounded");
        p.reset();
        assert!(!p.is_zoomed());
    }
}
