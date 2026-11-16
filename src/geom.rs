//! Small 2D-geometry helpers: discs, toothed rims, tapered profiles, spirals and the
//! rotate-then-translate transform every wheel is drawn through.

use std::f32::consts::PI;

pub const TAU: f32 = std::f32::consts::TAU;
pub type Pt = [f32; 2];

pub fn rad(deg: f32) -> f32 {
    deg * PI / 180.0
}

pub fn polar(c: Pt, r: f32, a: f32) -> Pt {
    [c[0] + r * a.cos(), c[1] + r * a.sin()]
}

/// A circle as a closed polygon.
pub fn disc(c: Pt, r: f32, n: usize) -> Vec<Pt> {
    (0..n)
        .map(|i| polar(c, r, i as f32 * TAU / n as f32))
        .collect()
}

/// A wheel's outline: `teeth` trapezoidal teeth around `r_pitch`, with the root circle filled
/// in, so one polygon is the whole disc plus its teeth (no holes to punch).
pub fn toothed(r_pitch: f32, teeth: u32, module: f32) -> Vec<Pt> {
    let teeth = teeth.max(4);
    let step = TAU / teeth as f32;
    let r_tip = r_pitch + 0.52 * module;
    let r_root = (r_pitch - 0.46 * module).max(0.3);
    let mut v = Vec::with_capacity(teeth as usize * 4);
    for i in 0..teeth {
        let a = i as f32 * step;
        v.push(polar([0.0, 0.0], r_root, a - 0.25 * step));
        v.push(polar([0.0, 0.0], r_tip, a - 0.15 * step));
        v.push(polar([0.0, 0.0], r_tip, a + 0.15 * step));
        v.push(polar([0.0, 0.0], r_root, a + 0.25 * step));
    }
    v
}

/// A skeleton wheel: the toothed outline above, an inner circle below, joined by a hairline
/// slit so one polygon can describe a ring with a hole in it (ear clipping needs a simple
/// polygon, and a ring isn't one without the slit).
pub fn skeleton(r_pitch: f32, teeth: u32, module: f32, r_inner: f32) -> Vec<Pt> {
    let mut v = toothed(r_pitch, teeth, module);
    let steps = 72;
    let eps = 0.006;
    for i in 0..=steps {
        let a = eps + (1.0 - i as f32 / steps as f32) * (TAU - 2.0 * eps);
        v.push(polar([0.0, 0.0], r_inner, a));
    }
    v
}

/// Rotates `pts` about the origin by `a`, then moves them to `c`.
pub fn place(pts: &[Pt], c: Pt, a: f32) -> Vec<Pt> {
    let (s, co) = a.sin_cos();
    pts.iter()
        .map(|p| [c[0] + p[0] * co - p[1] * s, c[1] + p[0] * s + p[1] * co])
        .collect()
}

/// Mirrors a `(y, half-width)` profile into a closed, symmetric outline.
pub fn taper(profile: &[(f32, f32)]) -> Vec<Pt> {
    let mut v = Vec::with_capacity(profile.len() * 2);
    for &(y, w) in profile {
        v.push([w, y]);
    }
    for &(y, w) in profile.iter().rev() {
        v.push([-w, y]);
    }
    v
}

/// An Archimedean spiral from `r0` to `r1` over `turns`, starting at angle 0.
pub fn spiral(r0: f32, r1: f32, turns: f32, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            polar([0.0, 0.0], r0 + (r1 - r0) * t, t * turns * TAU)
        })
        .collect()
}

/// A quadratic Bézier sampled into a polyline, for bridge arms and other curved metal.
pub fn bezier(p0: Pt, p1: Pt, p2: Pt, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            [
                u * u * p0[0] + 2.0 * u * t * p1[0] + t * t * p2[0],
                u * u * p0[1] + 2.0 * u * t * p1[1] + t * t * p2[1],
            ]
        })
        .collect()
}
