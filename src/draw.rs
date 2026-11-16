//! Everything the eye sees: the case, the chapter ring, the wheels and the hands, built as
//! [`Shape`] polygons and strokes in design units (a 1000 x 1000 square) scaled by `k`.
//!
//! Shapes are rebuilt whenever the frame changes — that is what keeps the escapement honest:
//! the wheels' angles come from [`crate::movement`] each frame, never from an animation.

use rune::{Color, LineCap, Shape};

use crate::geom::{bezier, disc, place, polar, rad, skeleton, spiral, taper, toothed, Pt, TAU};
use crate::movement::{balance_angle, fork_angle, hand_angles, Arbor, Beat, Train, CENTER};

// ---------------------------------------------------------------- palette
//
// Rune paints in linear light and encodes to sRGB on output, so these are the *linear*
// values of the colours named in the comments — `(v/255)` pushed through the sRGB transfer
// function. Authoring them as if they were sRGB is what makes a dark dial come out grey.

pub const BRASS: Color = Color { r: 0.5070, g: 0.3580, b: 0.1180, a: 1.0 }; // #BC9E62
pub const BRASS_HI: Color = Color { r: 0.8300, g: 0.6900, b: 0.4000, a: 1.0 }; // #EBD59F
pub const BRASS_LO: Color = Color { r: 0.1119, g: 0.0685, b: 0.0160, a: 1.0 }; // #5E4A22
pub const RECESS: Color = Color { r: 0.0144, g: 0.0091, b: 0.0056, a: 1.0 }; // #201811
pub const DIAL: Color = Color { r: 0.0037, g: 0.0042, b: 0.0060, a: 1.0 }; // #0C0D12
pub const PLATE: Color = Color { r: 0.0152, g: 0.0160, b: 0.0205, a: 1.0 }; // #212329
pub const PLATE_HI: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.030 };
pub const STEEL: Color = Color { r: 0.5395, g: 0.5776, b: 0.6445, a: 1.0 }; // #C2C8D2
pub const STEEL_HI: Color = Color { r: 0.8228, g: 0.8469, b: 0.8879, a: 1.0 }; // #EAEDF2
pub const STEEL_LO: Color = Color { r: 0.0242, g: 0.0296, b: 0.0395, a: 1.0 }; // #2B3038
pub const BLUED: Color = Color { r: 0.0203, g: 0.0513, b: 0.2122, a: 1.0 }; // #27407F
pub const BLUED_HI: Color = Color { r: 0.1981, g: 0.3663, b: 0.8879, a: 1.0 }; // #7BA3F2
pub const RUBY: Color = Color { r: 0.1946, g: 0.0065, b: 0.0242, a: 1.0 }; // #7A132B
pub const RUBY_HI: Color = Color { r: 0.6038, g: 0.0545, b: 0.0931, a: 1.0 }; // #CC4256
pub const INK: Color = Color { r: 0.7991, g: 0.6445, b: 0.3813, a: 1.0 }; // #E7D2A6
pub const SPRING: Color = Color { r: 0.5972, g: 0.6654, b: 0.7529, a: 0.72 }; // #CBD5E1
pub const SHADOW: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 };

/// Design units to points.
fn d(p: Pt, k: f32) -> Pt {
    [p[0] * k, p[1] * k]
}

fn c_alpha(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

// ---------------------------------------------------------------- the case

/// Bezel, dial disc and the plate the movement sits on. Static, so the view memoises it.
pub fn case_shape(k: f32) -> Shape {
    let c = d(CENTER, k);
    let mut s = Shape::new();

    // Bezel: a dark outer edge, the brass band, and a lit inner lip.
    s = s.stroke_with(disc(c, 473.0 * k, 220), true, 14.0 * k, LineCap::Round, BRASS_LO);
    s = s.stroke_with(disc(c, 468.0 * k, 220), true, 12.0 * k, LineCap::Round, BRASS);
    s = s.stroke_with(disc(c, 462.0 * k, 220), true, 2.2 * k, LineCap::Round, BRASS_HI);
    s = s.stroke_with(disc(c, 455.0 * k, 220), true, 1.6 * k, LineCap::Round, c_alpha(BRASS_LO, 0.9));

    // Dial plate.
    s = s.fill(disc(c, 454.0 * k, 220), DIAL);

    // Engine-turned rings, barely there: they catch the light as the window moves.
    for (r, a) in [(449.0f32, 0.10f32), (446.0, 0.06)] {
        s = s.stroke_with(disc(c, r * k, 220), true, 1.1 * k, LineCap::Round, c_alpha(BRASS_HI, a));
    }

    // The window the movement is seen through, cut with a shadow and a brass rail.
    s = s.stroke_with(disc(c, 314.0 * k, 220), true, 11.0 * k, LineCap::Round, SHADOW);
    s = s.stroke_with(disc(c, 307.5 * k, 220), true, 2.6 * k, LineCap::Round, BRASS_LO);
    s = s.fill(disc(c, 305.0 * k, 220), PLATE);

    // Brushed plate: concentric hairlines.
    for r in [80.0f32, 140.0, 200.0, 255.0, 288.0] {
        s = s.stroke_with(disc(c, r * k, 160), true, 1.0 * k, LineCap::Round, PLATE_HI);
    }
    s
}

/// The chapter ring: minute ticks, five-minute batons and the two hairlines that frame them.
pub fn dial_marks(k: f32) -> Shape {
    let c = d(CENTER, k);
    let mut s = Shape::new();
    s = s.stroke_with(disc(c, 434.0 * k, 200), true, 1.4 * k, LineCap::Round, c_alpha(BRASS_HI, 0.30));
    s = s.stroke_with(disc(c, 372.0 * k, 200), true, 1.8 * k, LineCap::Round, c_alpha(BRASS_HI, 0.16));

    for i in 0..60 {
        let a = -TAU * 0.25 + i as f32 * TAU / 60.0;
        let baton = i % 5 == 0;
        let (r0, w) = if baton { (374.0, 5.6) } else { (400.0, 1.8) };
        s = s.line(
            polar(c, r0 * k, a),
            polar(c, 428.0 * k, a),
            w * k,
            LineCap::Butt,
            if baton { INK } else { c_alpha(INK, 0.5) },
        );
    }
    s
}

// ---------------------------------------------------------------- wheels

fn arbor_shape(mut s: Shape, k: f32, a: &Arbor, angle: f32) -> Shape {
    let c = d(a.center, k);
    let m = a.module * k;
    let (rim, hi, lo) = if a.steel { (STEEL, STEEL_HI, STEEL_LO) } else { (BRASS, BRASS_HI, BRASS_LO) };

    if a.wheel > 0 {
        let r = a.pitch_r() * k;
        let r_in = (r - 3.6 * m).max(r * 0.35);
        if a.steel {
            s = s.fill(disc(c, r + 2.0 * m, 90), SHADOW);
        }
        // The wheel is a ring: the plate (and whatever is behind it) shows through the web.
        s = s.fill(place(&skeleton(r, a.wheel, m, r_in), c, angle), rim);
        // Bevels: the inside of the rim catches the light, the outside rolls into shadow.
        s = s.stroke_with(disc(c, r_in, 110), true, 0.7 * m, LineCap::Round, hi);
        s = s.stroke_with(disc(c, r_in - 0.5 * m, 110), true, 0.5 * m, LineCap::Round, lo);
        // Crossings.
        let hub = (3.1 * m).min(r * 0.4);
        let spokes = a.spokes.max(3);
        for i in 0..spokes {
            let th = angle + i as f32 * TAU / spokes as f32;
            s = s.line(polar(c, hub, th), polar(c, r_in + 0.4 * m, th), 0.95 * m, LineCap::Round, hi);
        }
        s = s.fill(disc(c, hub * 1.05, 28), rim);
        s = s.stroke_with(disc(c, hub * 1.05, 28), true, 0.6 * m, LineCap::Round, hi);
        s = s.fill(disc(c, hub * 0.5, 18), RECESS);
    }

    if a.leaves > 0 {
        let pr = a.pinion_r() * k;
        s = s.fill(place(&toothed(pr, a.leaves, m), c, angle), STEEL);
        s = s.stroke_with(disc(c, pr * 0.92, 40), true, 0.5 * m, LineCap::Round, STEEL_HI);
        s = s.fill(disc(c, pr * 0.34, 18), STEEL_LO);
        s = s.fill(disc(c, pr * 0.13, 12), RECESS);
    }
    s
}

/// A bridge or cock: a rounded arm with a shadow under it and a lit top edge.
fn arm(mut s: Shape, k: f32, pts: &[Pt], width: f32) -> Shape {
    let line: Vec<Pt> = pts.iter().map(|p| d(*p, k)).collect();
    s = s.stroke(line.clone(), (width + 5.0) * k, SHADOW);
    s = s.stroke(line.clone(), width * k, BRASS);
    // Highlight down one side of the bridge.
    let lit: Vec<Pt> = pts.iter().map(|p| d([p[0] - 1.6, p[1] - 1.6], k)).collect();
    s.stroke(lit, width * 0.22 * k, c_alpha(BRASS_HI, 0.85))
}

fn screw(s: Shape, k: f32, at: Pt, r: f32, turn: f32) -> Shape {
    let c = d(at, k);
    let s = s.fill(disc(c, r * k, 28), BRASS_HI);
    let s = s.stroke_with(disc(c, r * k, 28), true, 1.1 * k, LineCap::Round, BRASS_LO);
    let a = turn;
    s.line(
        polar(c, r * 0.78 * k, a + std::f32::consts::PI),
        polar(c, r * 0.78 * k, a),
        1.5 * k,
        LineCap::Butt,
        RECESS,
    )
}

/// The ratchet wheel and its teeth on the barrel arbor, which is what the key winds.
fn ratchet(mut s: Shape, k: f32, at: Pt, teeth: u32, r: f32) -> Shape {
    let c = d(at, k);
    let m = 1.15 * k;
    s = s.fill(disc(c, r * k + 2.8 * k, 60), SHADOW);
    s = s.fill(place(&skeleton(r * k, teeth, m, r * k - 5.5 * k), c, rad(9.0)), STEEL);
    s = s.stroke_with(disc(c, r * k - 5.5 * k, 90), true, 1.2 * k, LineCap::Round, STEEL_HI);
    for i in 0..5 {
        let th = rad(9.0) + i as f32 * TAU / 5.0;
        s = s.line(polar(c, 6.0 * k, th), polar(c, r * k - 6.0 * k, th), 2.0 * k, LineCap::Round, STEEL_HI);
    }
    s = s.fill(disc(c, 7.0 * k, 24), STEEL);
    s = s.stroke_with(disc(c, 7.0 * k, 24), true, 1.0 * k, LineCap::Round, STEEL_HI);
    s = s.fill(disc(c, 3.0 * k, 16), STEEL_LO);
    s
}

fn jewel(s: Shape, k: f32, at: Pt, r: f32) -> Shape {
    let c = d(at, k);
    let s = s.fill(disc(c, r * k, 24), RUBY);
    let s = s.stroke_with(disc(c, r * k, 24), true, 1.0 * k, LineCap::Round, BRASS_HI);
    let s = s.fill(disc(c, r * 0.5 * k, 18), RUBY_HI);
    s.fill(disc([c[0] - r * 0.2 * k, c[1] - r * 0.24 * k], r * 0.17 * k, 10), STEEL_HI)
}

/// The whole movement: wheels, bridges, fork, balance.
pub fn train_shape(k: f32, t: &Train, b: Beat, tod: f64) -> Shape {
    let mut s = Shape::new();

    for a in &t.arbors {
        s = arbor_shape(s, k, a, a.angle(b.tod));
    }
    // Motion work goes over the centre wheel: the hour wheel is a ring round the arbor.
    for a in &t.motion {
        s = arbor_shape(s, k, a, a.angle(b.tod));
    }

    let barrel = t.arbors[t.barrel].center;
    let third = t.arbors[t.third].center;
    let fourth = t.arbors[t.fourth].center;
    let escape = t.arbors[t.escape].center;
    let minute = t.motion[1].center;

    // Barrel bridge: from the plate's edge, curving over the mainspring drum.
    let mut bridge = bezier([336.0, 352.0], [352.0, 424.0], barrel, 22);
    bridge.push(barrel);
    s = arm(s, k, &bridge, 25.0);
    s = screw(s, k, [336.0, 352.0], 7.6, rad(24.0));
    // The ratchet wheel on the barrel arbor, and the click that holds it.
    s = ratchet(s, k, barrel, 24, 30.0);
    s = arm(s, k, &[[452.0, 380.0], [444.0, 404.0], [428.0, 424.0]], 8.0);
    s = screw(s, k, [452.0, 380.0], 5.2, rad(-40.0));

    // Third and fourth arbor bridge, down the right side.
    let mut right = bezier([628.0, 470.0], [590.0, 512.0], third, 18);
    right.push(fourth);
    s = arm(s, k, &right, 22.0);
    s = screw(s, k, [628.0, 470.0], 7.6, rad(70.0));
    s = screw(s, k, fourth, 6.4, rad(12.0));

    // Escape bridge, so the escape arbor has something to turn in.
    s = arm(s, k, &[[448.0, 618.0], [478.0, 630.0], escape], 17.0);
    s = screw(s, k, [448.0, 618.0], 6.0, rad(-12.0));

    // A jewel over the minute wheel's arbor.
    s = jewel(s, k, [minute[0] + 10.0, minute[1] + 12.0], 4.2);

    // The pallet fork and the balance.
    s = fork_shape(s, k, t, b);
    s = balance_shape(s, k, t, tod);

    // Balance cock, reaching in from the plate's edge on the side away from the fork.
    let cock = bezier([362.0, 656.0], [392.0, 706.0], t.balance, 18);
    s = arm(s, k, &cock, 20.0);
    s = screw(s, k, [362.0, 656.0], 7.6, rad(38.0));
    s = jewel(s, k, t.balance, 6.6);
    s
}

fn fork_shape(mut s: Shape, k: f32, t: &Train, b: Beat) -> Shape {
    let pivot = t.fork;
    let target = t.balance;
    // The fork's tail points at the balance's roller; its arms reach up to the escape wheel.
    let base = (target[1] - pivot[1]).atan2(target[0] - pivot[0]) - std::f32::consts::FRAC_PI_2;
    let a = base + fork_angle(b);
    let p0 = d(pivot, k);
    // Local points, in design units, before the fork's own rotation.
    let lp = |p: Pt| d(p, k);
    let place_pts = |pts: &[Pt]| place(&pts.iter().map(|p| lp(*p)).collect::<Vec<_>>(), p0, a);

    // Arms up to the pallet stones, and the tail down to the roller.
    let arms: [&[Pt]; 2] = [&[[0.0, 10.0], [-20.0, -26.0]], &[[0.0, 10.0], [20.0, -26.0]]];
    for a_pts in arms {
        s = s.stroke(place_pts(a_pts), 6.0 * k, SHADOW);
    }
    for a_pts in arms {
        s = s.stroke(place_pts(a_pts), 4.0 * k, STEEL_HI);
    }
    let tail: Vec<Pt> = vec![[0.0, 0.0], [-3.0, 16.0], [-6.4, 33.0]];
    s = s.stroke(place_pts(&tail), 3.4 * k, STEEL_HI);
    // The fork's notch, which the balance's impulse jewel passes through.
    let prongs: [&[Pt]; 2] = [&[[-6.4, 33.0], [-11.0, 39.5]], &[[-6.4, 33.0], [-1.8, 39.5]]];
    for p_pts in prongs {
        s = s.stroke(place_pts(p_pts), 1.8 * k, STEEL);
    }
    // Pallet stones: the two rubies that lock the escape wheel.
    for stone in [-20.0f32, 20.0] {
        let quad: Vec<Pt> = vec![
            [stone - 2.4, -17.0],
            [stone + 2.4, -17.0],
            [stone + 2.4, -27.0],
            [stone - 2.4, -27.0],
        ];
        s = s.fill(place_pts(&quad), RUBY);
    }
    s.fill(disc(p0, 4.4 * k, 20), STEEL_HI)
}

fn balance_shape(mut s: Shape, k: f32, t: &Train, tod: f64) -> Shape {
    let c = d(t.balance, k);
    let r = t.balance_r * k;
    let a = balance_angle(tod);

    // Hairspring first, so the rim lies over it.
    s = s.fill(disc(c, r + 4.0 * k, 90), SHADOW);
    let spring = spiral(9.0 * k, r * 0.74, 5.0, 300);
    s = s.stroke(place(&spring, c, a * 0.2), 0.8 * k, c_alpha(SPRING, 0.8));

    // Rim, crossings, hub.
    s = s.stroke_with(disc(c, r, 150), true, 7.4 * k, LineCap::Round, STEEL);
    s = s.stroke_with(disc(c, r + 3.2 * k, 150), true, 1.1 * k, LineCap::Round, STEEL_HI);
    s = s.stroke_with(disc(c, r - 3.6 * k, 150), true, 1.3 * k, LineCap::Round, STEEL_LO);
    for i in 0..3 {
        let th = a + i as f32 * TAU / 3.0;
        s = s.line(polar(c, 8.0 * k, th), polar(c, r - 3.2 * k, th), 3.2 * k, LineCap::Round, STEEL);
    }
    s = s.fill(disc(c, 10.0 * k, 32), STEEL);
    s = s.stroke_with(disc(c, 10.0 * k, 32), true, 1.2 * k, LineCap::Round, STEEL_HI);
    s = s.fill(disc(c, 4.2 * k, 18), STEEL_LO);

    // Roller table with the impulse jewel that the fork's notch drives.
    let roller = polar(c, 9.0 * k, a + 2.2);
    s = s.fill(disc(roller, 4.6 * k, 20), STEEL_LO);
    s.fill(disc(polar(roller, 2.6 * k, a + 2.2), 1.7 * k, 12), RUBY_HI)
}

// ---------------------------------------------------------------- hands

const HOUR_HAND: &[(f32, f32)] = &[
    (64.0, 5.8),
    (0.0, 8.6),
    (-126.0, 9.4),
    (-200.0, 15.0),
    (-248.0, 12.4),
    (-282.0, 4.8),
    (-300.0, 1.1),
];
const MINUTE_HAND: &[(f32, f32)] = &[
    (76.0, 5.2),
    (0.0, 7.4),
    (-190.0, 8.0),
    (-268.0, 11.8),
    (-344.0, 6.2),
    (-392.0, 1.4),
];
const SECOND_HAND: &[(f32, f32)] = &[
    (92.0, 7.4),
    (38.0, 2.2),
    (-64.0, 2.4),
    (-352.0, 2.6),
    (-438.0, 0.9),
];

fn hand(mut s: Shape, k: f32, c: Pt, angle: f32, profile: &[(f32, f32)]) -> Shape {
    let local = taper(profile);
    let pts = place(&local.iter().map(|p| d(*p, k)).collect::<Vec<_>>(), c, angle);
    s = s.fill(pts, BLUED);
    // The long highlight down the middle of a polished blue hand.
    let tip = profile[profile.len() - 1].0;
    let up = angle - std::f32::consts::FRAC_PI_2;
    s.line(
        polar(c, 0.0, up),
        polar(c, -tip * k, up),
        1.3 * k,
        LineCap::Round,
        c_alpha(BLUED_HI, 0.75),
    )
}

pub fn hands_shape(k: f32, tod: f64) -> Shape {
    let c = d(CENTER, k);
    let [h, m, sec] = hand_angles(tod);
    let mut s = Shape::new();
    s = hand(s, k, c, h, HOUR_HAND);
    s = hand(s, k, c, m, MINUTE_HAND);

    // The seconds hand, with its counterweight.
    let local = taper(SECOND_HAND);
    let pts = place(&local.iter().map(|p| d(*p, k)).collect::<Vec<_>>(), c, sec);
    s = s.fill(pts, BLUED);
    let up = sec - std::f32::consts::FRAC_PI_2;
    s = s.line(
        polar(c, -SECOND_HAND[0].0 * k, up),
        polar(c, -SECOND_HAND[3].0 * k * 0.98, up),
        1.1 * k,
        LineCap::Round,
        c_alpha(BLUED_HI, 0.8),
    );
    s = s.fill(disc(polar(c, SECOND_HAND[0].0 * 0.66 * k, sec + std::f32::consts::FRAC_PI_2), 12.0 * k, 26), BLUED);

    // The boss over the three hands.
    s = s.fill(disc(c, 15.0 * k, 40), BRASS);
    s = s.stroke_with(disc(c, 15.0 * k, 40), true, 1.6 * k, LineCap::Round, BRASS_HI);
    s = s.fill(disc(c, 8.4 * k, 28), BLUED);
    s = s.fill(disc(c, 3.2 * k, 16), c_alpha(BLUED_HI, 0.9));
    s
}
