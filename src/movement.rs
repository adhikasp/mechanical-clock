//! The going train and its lever escapement.
//!
//! Nothing here draws. This module is the clock *mechanism*: the tooth counts, the arbor
//! positions derived from those counts, and the angles every wheel turns through at a given
//! moment. The view reads it once a frame.
//!
//! # Why the numbers are these numbers
//!
//! Every wheel meshes its neighbour's pinion, and a wheel that turns `n` times drives the
//! arbor it meshes with `teeth/leaves` times faster — no fudge factors. From the centre wheel
//! (which carries the minute hand, one turn an hour) the train falls out as:
//!
//! | arbor  | wheel | pinion | turns                         |
//! |--------|-------|--------|-------------------------------|
//! | barrel | 80    | -      | 1 / 8 h (mainspring drum)     |
//! | centre | 60    | 10     | 1 / hour                      |
//! | third  | 64    | 8      | 1 / 8 min                     |
//! | fourth | 80    | 8      | 1 / min (seconds hand)        |
//! | escape | 15    | 4      | 1 / 3 s                       |
//!
//! The escape wheel has 15 teeth and turns once every 3 s, so the lever releases one tooth
//! every 3/15 s — five times a second, 18 000 alternations an hour, the classic pocket-watch
//! beat. The balance makes one full swing per two beats (2.5 Hz).
//!
//! The hands follow the *escapement*, not the wall clock arithmetic: the whole train creeps
//! forward during each beat's impulse and then stands locked until the next one. Over any
//! interval the average is exact, so the clock neither gains nor loses.

use std::f64::consts::TAU as TAU64;

use crate::geom::{Pt, polar, rad, TAU};

/// Design units of pitch diameter per tooth: every gear in the movement shares it, so pitch
/// radii add up to centre distances and the teeth genuinely mesh.
pub const MODULE: f32 = 2.35;

/// Beats per second at the lever. Derived from the train, asserted in the tests.
pub const BEAT_HZ: f64 = 5.0;
/// Fraction of a beat the train spends moving before the escapement locks again.
pub const IMPULSE: f64 = 0.34;
pub const BALANCE_HZ: f64 = BEAT_HZ / 2.0;
/// The dial's centre, in design units (the 1000x1000 space the whole clock is authored in).
pub const CENTER: Pt = [500.0, 500.0];
pub const BALANCE_R: f32 = 50.0;
/// How far the balance turns either way (~140 degrees).
pub const BALANCE_AMP: f32 = 2.45;
/// How far the pallet fork swings either way.
pub const FORK_AMP: f32 = 0.20;

#[derive(Clone, Debug)]
pub struct Arbor {
    pub name: &'static str,
    pub center: Pt,
    /// Teeth on the wheel this arbor carries.
    pub wheel: u32,
    /// Leaves on the pinion the previous arbor drives, 0 for the barrel.
    pub leaves: u32,
    pub spokes: u32,
    pub module: f32,
    /// Revolutions per second.
    pub revs: f64,
    /// +1 clockwise on screen, -1 anti-clockwise (neighbouring wheels counter-rotate).
    pub sign: f32,
    /// Rotation at time zero.
    pub phase: f32,
    pub steel: bool,
}

impl Arbor {
    pub fn pitch_r(&self) -> f32 {
        self.module * self.wheel as f32 * 0.5
    }

    pub fn pinion_r(&self) -> f32 {
        self.module * self.leaves as f32 * 0.5
    }

    /// This arbor's rotation at `tod_q` seconds into the day.
    pub fn angle(&self, tod_q: f64) -> f32 {
        let turns = (self.revs * tod_q).rem_euclid(1.0);
        (self.phase as f64 + self.sign as f64 * turns * TAU64) as f32
    }
}

/// A wheel meeting a pinion, kept so the meshing can be asserted in a test.
#[derive(Clone, Copy, Debug)]
pub struct Mesh {
    /// Index into [`Train::arbors`] of the driving wheel.
    pub driver: usize,
    /// Index of the arbor whose pinion it drives.
    pub driven: usize,
    pub driver_teeth: u32,
    pub driven_teeth: u32,
    /// Direction from the driver's arbor to the driven one, radians.
    pub dir: f32,
}

pub struct Train {
    /// The going train, in drawing order (back to front).
    pub arbors: Vec<Arbor>,
    /// Motion work: cannon pinion, minute wheel and hour wheel, drawn over the centre wheel.
    pub motion: Vec<Arbor>,
    pub meshes: Vec<Mesh>,
    pub barrel: usize,
    pub third: usize,
    pub fourth: usize,
    pub escape: usize,
    /// Pallet fork pivot and the balance's arbor.
    pub fork: Pt,
    pub balance: Pt,
    pub balance_r: f32,
    pub beats_per_second: f64,
}

/// Fraction of a tooth a wheel's teeth sit from `toward`.
fn tooth_phase(phase: f32, teeth: u32, toward: f32) -> f32 {
    let x = (toward - phase) * teeth as f32 / TAU;
    x - x.floor()
}

/// The rotation a driven arbor needs so that, along the line of centres, a tooth of the
/// driver faces a gap of the driven wheel.
fn mesh_phase(driver_phase: f32, driver_teeth: u32, driven_teeth: u32, dir: f32) -> f32 {
    let p = tooth_phase(driver_phase, driver_teeth, dir);
    let q = 0.5 - p;
    (dir + std::f32::consts::PI - q * TAU / driven_teeth as f32).rem_euclid(TAU)
}

impl Train {
    pub fn build() -> Train {
        let mut meshes = Vec::new();

        // Barrel: the mainspring drum, upper left. Its phase is the one free choice; every
        // other arbor's phase follows from it through the mesh.
        let barrel_c = polar(CENTER, MODULE * (80.0 + 10.0) * 0.5, rad(210.0));
        let barrel = Arbor {
            name: "barrel",
            center: barrel_c,
            wheel: 80,
            leaves: 0,
            spokes: 6,
            module: MODULE,
            revs: 1.0 / (8.0 * 3600.0),
            sign: -1.0,
            phase: rad(11.0),
            steel: false,
        };

        // Centre wheel: carries the minute hand and the cannon pinion.
        let dir_bc = rad(30.0);
        let center_c = polar(barrel_c, MODULE * (80.0 + 10.0) * 0.5, dir_bc);
        let center = Arbor {
            name: "centre",
            center: center_c,
            wheel: 60,
            leaves: 10,
            spokes: 5,
            module: MODULE,
            revs: 1.0 / 3600.0,
            sign: 1.0,
            phase: mesh_phase(barrel.phase, 80, 10, dir_bc),
            steel: false,
        };
        meshes.push(Mesh { driver: 0, driven: 1, driver_teeth: 80, driven_teeth: 10, dir: dir_bc });

        // Third wheel, down and right of the centre.
        let dir_ct = rad(35.0);
        let third_c = polar(center_c, MODULE * (60.0 + 8.0) * 0.5, dir_ct);
        let third = Arbor {
            name: "third",
            center: third_c,
            wheel: 64,
            leaves: 8,
            spokes: 5,
            module: MODULE,
            revs: center.revs * 60.0 / 8.0,
            sign: -1.0,
            phase: mesh_phase(center.phase, 60, 8, dir_ct),
            steel: false,
        };
        meshes.push(Mesh { driver: 1, driven: 2, driver_teeth: 60, driven_teeth: 8, dir: dir_ct });

        // Fourth wheel, below the third: one turn a minute, so the seconds hand rides on it.
        let dir_tf = rad(80.0);
        let fourth_c = polar(third_c, MODULE * (64.0 + 8.0) * 0.5, dir_tf);
        let fourth = Arbor {
            name: "fourth",
            center: fourth_c,
            wheel: 80,
            leaves: 8,
            spokes: 6,
            module: MODULE,
            revs: third.revs * 64.0 / 8.0,
            sign: 1.0,
            phase: mesh_phase(third.phase, 64, 8, dir_tf),
            steel: false,
        };
        meshes.push(Mesh { driver: 2, driven: 3, driver_teeth: 64, driven_teeth: 8, dir: dir_tf });

        // Escape wheel: 15 teeth driven off the fourth wheel's 80:20 step up.
        let dir_fe = rad(120.0);
        let escape_c = polar(fourth_c, MODULE * (80.0 + 4.0) * 0.5, dir_fe);
        let escape = Arbor {
            name: "escape",
            center: escape_c,
            wheel: 15,
            leaves: 4,
            spokes: 4,
            module: MODULE,
            revs: fourth.revs * 80.0 / 4.0,
            sign: -1.0,
            phase: mesh_phase(fourth.phase, 80, 4, dir_fe),
            steel: true,
        };
        meshes.push(Mesh { driver: 3, driven: 4, driver_teeth: 80, driven_teeth: 4, dir: dir_fe });

        let beats_per_second = escape.revs * 15.0;

        // Motion work: the cannon pinion under the hands turns the minute wheel, whose pinion
        // turns the hour wheel — 12:1, and the hour hand goes round with the minute hand.
        // The two meshes run at slightly different modules, as they must: equal centre
        // distances with unequal tooth counts.
        let dir_cm = rad(-60.0);
        let motion_dist = MODULE * (12.0 + 36.0) * 0.5;
        let minute_c = polar(CENTER, motion_dist, dir_cm);
        let m_hour = 2.0 * motion_dist / (10.0 + 40.0);
        let cannon = Arbor {
            name: "cannon",
            center: CENTER,
            wheel: 12,
            leaves: 0,
            spokes: 0,
            module: MODULE,
            revs: center.revs,
            sign: 1.0,
            phase: center.phase,
            steel: true,
        };
        let minute = Arbor {
            name: "minute",
            center: minute_c,
            wheel: 36,
            leaves: 12,
            spokes: 4,
            module: MODULE,
            revs: cannon.revs * 12.0 / 36.0,
            sign: -1.0,
            phase: mesh_phase(cannon.phase, 12, 36, dir_cm),
            steel: false,
        };
        meshes.push(Mesh { driver: 5, driven: 6, driver_teeth: 12, driven_teeth: 36, dir: dir_cm });
        let hour = Arbor {
            name: "hour",
            center: CENTER,
            wheel: 40,
            leaves: 10,
            spokes: 5,
            module: m_hour,
            revs: minute.revs * 10.0 / 40.0,
            sign: 1.0,
            phase: mesh_phase(minute.phase, 10, 40, dir_cm + std::f32::consts::PI),
            steel: false,
        };

        meshes.push(Mesh {
            driver: 6,
            driven: 7,
            driver_teeth: 10,
            driven_teeth: 40,
            dir: dir_cm + std::f32::consts::PI,
        });

        // The escapement lives at the bottom of the movement: the fork under the escape wheel,
        // the balance beside it, both inside the dial's window.
        let s = MODULE / 2.05;
        let fork = [escape_c[0] - 22.0 * s, escape_c[1] + 30.0 * s];
        let balance = [escape_c[0] - 57.0 * s, escape_c[1] + 13.0 * s];

        Train {
            arbors: vec![barrel, center, third, fourth, escape],
            motion: vec![cannon, minute, hour],
            meshes,
            barrel: 0,
            third: 2,
            fourth: 3,
            escape: 4,
            fork,
            balance,
            balance_r: BALANCE_R * s,
            beats_per_second,
        }
    }

    /// Arbor by index, where `arbors.len()..` names the motion work.
    pub fn arbor_at(&self, i: usize) -> &Arbor {
        if i < self.arbors.len() { &self.arbors[i] } else { &self.motion[i - self.arbors.len()] }
    }

    /// The arbor with this name, in the going train or the motion work.
    #[cfg(test)]
    pub fn arbor(&self, name: &str) -> &Arbor {
        self.arbors
            .iter()
            .chain(self.motion.iter())
            .find(|a| a.name == name)
            .expect("arbor name")
    }

    /// The movement, as text: what `--train` prints and what the README's table says.
    pub fn describe(&self) -> String {
        let mut out = String::from("arbor    wheel  pinion  seconds per turn\n");
        for a in self.arbors.iter().chain(self.motion.iter()) {
            let per = 1.0 / a.revs;
            out.push_str(&format!(
                "{:<8} {:>5}  {:>6}  {:>15.3}\n",
                a.name, a.wheel, a.leaves, per
            ));
        }
        out.push_str(&format!(
            "\n{:.0} beats a second, {:.0} A/h, balance {:.2} Hz\n",
            self.beats_per_second,
            self.beats_per_second * 3600.0,
            BALANCE_HZ
        ));
        out.push_str("\nmesh                        teeth  ratio  line\n");
        for m in &self.meshes {
            let driver = self.arbor_at(m.driver);
            let driven = self.arbor_at(m.driven);
            out.push_str(&format!(
                "{:<12} -> {:<12} {:>3}:{:<3} {:>6.2} {:>6.1}deg\n",
                driver.name,
                driven.name,
                m.driver_teeth,
                m.driven_teeth,
                f64::from(m.driver_teeth) / f64::from(m.driven_teeth),
                f32::to_degrees(m.dir)
            ));
        }
        out
    }
}

/// The train is constant, so build it once.
pub fn train() -> &'static Train {
    static TRAIN: std::sync::OnceLock<Train> = std::sync::OnceLock::new();
    TRAIN.get_or_init(Train::build)
}

/// `1 - (1 - u)^3`: the wheels start moving at the impulse, fast, and settle into the lock.
fn ease_out_cubic(u: f64) -> f64 {
    let v = 1.0 - u;
    1.0 - v * v * v
}

#[derive(Clone, Copy, Debug)]
pub struct Beat {
    /// The time the hands show: whole beats behind the wall clock, plus however far into the
    /// current beat's impulse the train has travelled.
    pub tod: f64,
    /// 0 at the unlock, 1 once the train has settled against the banking.
    pub impulse: f64,
    /// Beats since midnight, for the fork's alternating pallets.
    pub beat: i64,
}

/// Quantises a time of day onto the escapement's beat grid.
pub fn beat(tod: f64) -> Beat {
    let x = tod * BEAT_HZ;
    let whole = x.floor();
    let p = x - whole;
    let e = ease_out_cubic((p / IMPULSE).min(1.0));
    Beat {
        tod: (whole + e) / BEAT_HZ,
        impulse: e,
        beat: whole as i64,
    }
}

/// The balance's swing: a clean cosine, its extremes falling on the beats, where the lever
/// unlocks. (The escapement only *wants* to move there — the train does the moving.)
pub fn balance_angle(tod: f64) -> f32 {
    (BALANCE_AMP as f64 * (TAU64 * BALANCE_HZ * tod).cos()) as f32
}

/// Which way the pallet fork is thrown, `-1` to `+1` across the current beat.
pub fn fork_angle(b: Beat) -> f32 {
    let now = if b.beat.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
    let t = -1.0 + 2.0 * b.impulse;
    (FORK_AMP as f64 * now * t) as f32
}

/// The three hands' angles at `tod`, clockwise from twelve.
pub fn hand_angles(tod: f64) -> [f32; 3] {
    let hand = |period: f64| ((tod / period).rem_euclid(1.0) * TAU64) as f32;
    [hand(43200.0), hand(3600.0), hand(60.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn the_train_multiplies_out_to_the_classic_beat() {
        let t = train();
        assert!(close(t.beats_per_second as f32, 5.0, 1e-6), "{}", t.beats_per_second);
        // One alternate every 0.2 s, 18 000 an hour.
        assert!(close((t.beats_per_second * 3600.0) as f32, 18_000.0, 1e-3));
        assert!(close(t.arbors[t.fourth].revs as f32, 1.0 / 60.0, 1e-9));
        assert!(close(t.arbor("centre").revs as f32, 1.0 / 3600.0, 1e-12));
    }

    #[test]
    fn the_hands_run_at_their_wheels_rates() {
        let t = train();
        let hour_wheel = t.arbor("hour");
        let wrap = |x: f32| (x + TAU * 0.5).rem_euclid(TAU) - TAU * 0.5;
        for (t0, t1) in [(0.0, 17.0), (3600.0, 43_266.0), (86_000.0, 86_123.0)] {
            let [h0, m0, s0] = hand_angles(t0);
            let [h1, m1, s1] = hand_angles(t1);
            let d = |a: f32, b: f32| wrap(a - b).abs();
            assert!(d(h1 - h0, hour_wheel.angle(t1) - hour_wheel.angle(t0)) < 1e-2);
            let centre = t.arbor("centre");
            assert!(d(m1 - m0, centre.angle(t1) - centre.angle(t0)) < 1e-2);
            assert!(d(s1 - s0, t.arbors[t.fourth].angle(t1) - t.arbors[t.fourth].angle(t0)) < 1e-2);
        }
    }

    #[test]
    fn the_hands_read_the_right_time() {
        let quarter = std::f32::consts::FRAC_PI_2;
        let [h, m, s] = hand_angles(0.0);
        assert_eq!((h, m, s), (0.0, 0.0, 0.0));
        // Three o'clock: the hour hand at 90 degrees, the minute hand on twelve.
        let [h, m, _] = hand_angles(3.0 * 3600.0);
        assert!(close(h, quarter, 1e-5), "{h}");
        assert!(close(m, 0.0, 1e-5));
        // A quarter past: the minute hand at 90 degrees, a quarter of a minute later the second
        // hand too.
        let [_, m, _] = hand_angles(15.0 * 60.0);
        assert!(close(m, quarter, 1e-5), "{m}");
        let [_, _, s] = hand_angles(15.0);
        assert!(close(s, quarter, 1e-5), "{s}");
        // 10:09: the classic photograph, hands clear of each other.
        let [h, m, _] = hand_angles(10.0 * 3600.0 + 9.0 * 60.0);
        assert!((h - 5.3145).abs() < 1e-3 && (m - 0.94248).abs() < 1e-3, "{h} {m}");
    }

    #[test]
    fn the_beat_quantiser_never_drifts() {
        // Within a beat the hands run up to one beat off the wall clock — the train travels its
        // tooth quickly, then stands locked — but whole beats are exact, so nothing drifts.
        for i in 0..2000 {
            let tod = 12.0 * 3600.0 + f64::from(i) * 0.037;
            let b = beat(tod);
            assert!((tod - b.tod).abs() <= 1.0 / BEAT_HZ + 1e-9, "{}", tod - b.tod);
            for n in 1..4 {
                let later = beat(tod + f64::from(n) / BEAT_HZ);
                assert!((later.tod - b.tod - f64::from(n) / BEAT_HZ).abs() < 1e-6);
            }
        }
        // Most of the beat is spent locked, so the second hand visibly rests between steps.
        let b = beat(1.0 + 0.9 / BEAT_HZ);
        assert!(b.impulse > 0.999);
        // And the balance's extremes land on the beats.
        assert!(balance_angle(0.0).abs() > BALANCE_AMP - 1e-3);
    }

    #[test]
    fn every_mesh_lines_a_tooth_up_with_a_gap() {
        let t = train();
        for m in &t.meshes {
            for tod in [0.0, 0.37, 1234.5, 43_200.0, 86_399.0] {
                let a = t.arbor_at(m.driver);
                let b = t.arbor_at(m.driven);
                let pa = tooth_phase(a.angle(tod), m.driver_teeth, m.dir);
                let pb = tooth_phase(b.angle(tod), m.driven_teeth, m.dir + std::f32::consts::PI);
                let sum = (pa + pb).rem_euclid(1.0);
                assert!((sum - 0.5).abs() < 5e-3, "{} -> {} at {tod}: {sum}", a.name, b.name);
            }
        }
    }
}
