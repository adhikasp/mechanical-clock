//! A wall clock you can hear with your eyes: the hands are not animated, they are *driven*.
//!
//! Each frame the view asks [`movement`] where the going train stands. The train only moves
//! during the impulse of each beat — five times a second, after the escape wheel is unlocked
//! — so the second hand creeps, the escape wheel snaps a tooth at a time and the balance
//! swings at 2.5 Hz between them. Over any interval the average is exact, because the
//! escapement is quantising the same clock the hands report.

mod draw;
mod geom;
mod movement;

use rune::{Align, AppSpec, Cx, El, Family, TextStyle, Tx, canvas, col, el, memo, row, text};

use movement::Train;

const WINDOW: (f32, f32) = (820.0, 900.0);
const CAPTION_ROOM: f32 = 148.0;

/// Semantic colours, kept in one place so the palette can be retuned without touching layout.
const BG_INNER: rune::Color = rune::Color { r: 0.086, g: 0.078, b: 0.070, a: 1.0 };
const BG_MID: rune::Color = rune::Color { r: 0.035, g: 0.032, b: 0.030, a: 1.0 };
const BG_OUTER: rune::Color = rune::Color { r: 0.012, g: 0.011, b: 0.010, a: 1.0 };

pub struct State {
    /// The window's own size in points, learned from the root's layout.
    win: (f32, f32),
    /// A fixed start time (seconds into the day), for snapshots and videos.
    fixed: Option<f64>,
    /// (app clock at the first poll, wall clock then), so the display follows real time
    /// without re-reading the system clock sixty times a second.
    anchor: Option<(f64, f64)>,
    /// The time of day the hands show, quantised onto the escapement's beats.
    tod: f64,
    /// The unquantised time of day, which is what swings the balance.
    real: f64,
    impulse: f64,
    beat: i64,
    /// Smoothed frames per second, measured from the app clock.
    fps: f64,
    last_t: f64,
}

impl Default for State {
    fn default() -> Self {
        State {
            win: WINDOW,
            fixed: None,
            anchor: None,
            tod: 0.0,
            real: 0.0,
            impulse: 0.0,
            beat: 0,
            fps: 0.0,
            last_t: f64::NAN,
        }
    }
}

impl State {
    fn at(mut self, tod: f64) -> Self {
        self.fixed = Some(tod);
        self.tod = tod;
        self.real = tod;
        self
    }
}

/// Seconds since local midnight, from the system clock (DST included).
fn local_tod() -> f64 {
    use chrono::Timelike;
    let now = chrono::Local::now();
    f64::from(now.num_seconds_from_midnight()) + f64::from(now.nanosecond()) * 1e-9
}

fn numeral_style(k: f32) -> TextStyle {
    TextStyle {
        size: 34.0 * k,
        weight: 500.0,
        family: Family::Named("Georgia, Times New Roman"),
        ..TextStyle::new(34.0 * k, 500.0)
    }
}

fn readout_style() -> TextStyle {
    TextStyle {
        size: 23.0,
        weight: 500.0,
        features: "'tnum' 1",
        tracking: 1.2,
        ..TextStyle::new(23.0, 500.0)
    }
}

fn fps_style() -> TextStyle {
    TextStyle {
        size: 11.5,
        weight: 600.0,
        tracking: 0.7,
        features: "'tnum' 1",
        ..TextStyle::new(11.5, 600.0)
    }
}

fn caption_style() -> TextStyle {
    TextStyle {
        size: 11.5,
        weight: 500.0,
        tracking: 0.7,
        ..TextStyle::new(11.5, 500.0)
    }
}

/// `HH:MM:SS.hh`, the time the *hands* show (so it also steps with the escapement).
fn readout(tod: f64) -> String {
    let h = (tod / 3600.0) as u32 % 24;
    let m = (tod / 60.0) as u32 % 60;
    let s = tod % 60.0;
    format!("{h:02}:{m:02}:{s:05.2}")
}

fn numerals(k: f32) -> Vec<El> {
    const ROMAN: [&str; 12] = [
        "XII", "I", "II", "III", "IIII", "V", "VI", "VII", "VIII", "IX", "X", "XI",
    ];
    let c = [500.0 * k, 500.0 * k];
    let r = 352.0 * k;
    let (w, h) = (72.0 * k, 44.0 * k);
    (0..12)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * geom::TAU / 12.0;
            let p = geom::polar(c, r, a);
            el()
                .key(format!("num{i}"))
                .size(w, h)
                .absolute(p[0] - w * 0.5, p[1] - h * 0.5)
                .child(
                    text(ROMAN[i])
                        .font(numeral_style(k))
                        .color(draw::INK)
                        .w_full()
                        .align(Align::Center),
                )
        })
        .collect()
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let train: &Train = movement::train();
    let side = (s.win.0 - 64.0).min(s.win.1 - CAPTION_ROOM).clamp(200.0, 1600.0);
    let k = side / 1000.0;
    let beat = movement::Beat {
        tod: s.tod,
        impulse: s.impulse,
        beat: s.beat,
    };

    // The dial furniture never changes, so it is built once and cached as a texture: per frame
    // the GPU composites one quad instead of re-emitting a thousand ticks and rings.
    let static_face = memo("face", (side * 64.0) as u32, move || {
        let mut face = el()
            .key("face")
            .size(side, side)
            .absolute(0.0, 0.0)
            .child(canvas(draw::case_shape(k)).key("case").size(side, side).absolute(0.0, 0.0))
            .child(canvas(draw::dial_marks(k)).key("marks").size(side, side).absolute(0.0, 0.0));
        for n in numerals(k) {
            face = face.child(n);
        }
        face
    });

    let mut clock = el().key("clock").size(side, side).child(static_face);

    // The movement and the hands are rebuilt every frame: their angles are the simulation.
    clock = clock.child(canvas(draw::train_shape(k, train, beat, s.real)).key("train").size(side, side).absolute(0.0, 0.0));
    clock = clock.child(canvas(draw::hands_shape(k, s.tod)).key("hands").size(side, side).absolute(0.0, 0.0));

    let mut detail = row().gap(14.0).child(
        text("18 000 A/h · five beats a second · 15-tooth escape wheel · real time of day")
            .font(caption_style())
            .color(rune::Color { a: 0.45, ..draw::BRASS_HI }),
    );
    // Only when the loop has actually measured one: a headless still has no frame rate.
    if s.fps >= 1.0 {
        detail = detail.child(
            text(format!("{:.0} fps", s.fps))
                .font(fps_style())
                .color(rune::Color { a: 0.38, ..draw::BRASS_HI }),
        );
    }
    let caption = col()
        .key("caption")
        .gap(5.0)
        .child(text(readout(s.tod)).font(readout_style()).color(draw::INK))
        .child(detail);

    col()
        .key("root")
        .w_full()
        .h_full()
        .center()
        .gap(14.0)
        .fill(BG_OUTER)
        .radial_gradient(
            [0.5, 0.44],
            side * 1.05,
            [(0.0, BG_INNER), (0.55, BG_MID), (1.0, BG_OUTER)],
        )
        .on_layout_tx(|s: &mut State, r, _: &mut Tx| {
            if (s.win.0 - r[2]).abs() > 0.5 || (s.win.1 - r[3]).abs() > 0.5 {
                s.win = (r[2], r[3]);
            }
        })
        .child(clock)
        .child(caption)
}

fn poll(s: &mut State, t: f64, tx: &mut Tx) -> bool {
    // Ask for the next frame. Returning true only says the state changed; a poll that wants to
    // keep stepping has to schedule the wake itself, or the loop sleeps until the next input.
    // A wake due within a frame interval is held to one frame, so this paces at ~60 fps.
    tx.wake_after(std::time::Duration::ZERO);

    // Frame rate from the app clock, so the number under the dial is what the event loop is
    // really delivering rather than what the code hopes for.
    if s.last_t.is_finite() {
        let dt = t - s.last_t;
        if dt > 1e-6 {
            let inst = 1.0 / dt;
            s.fps = if s.fps <= 0.0 { inst } else { s.fps * 0.9 + inst * 0.1 };
        }
    }
    s.last_t = t;

    let tod = match s.fixed {
        Some(base) => base + t,
        None => {
            let (t0, tod0) = *s.anchor.get_or_insert((t, local_tod()));
            tod0 + (t - t0)
        }
    };
    s.real = tod.rem_euclid(86_400.0);
    let b = movement::beat(s.real);
    s.tod = b.tod;
    s.impulse = b.impulse;
    s.beat = b.beat;
    true
}

fn main() {
    // `--train` prints the movement and exits; everything else goes to Rune.
    if rune::tooling::extra_args().iter().any(|a| a == "--train") {
        print!("{}", movement::train().describe());
        return;
    }
    rune::run(AppSpec {
        poll: Some(poll),
        scenes: &[
            // 10:09:36 — the photographer's time, with the hands clear of each other.
            ("hero", |s| *s = State::default().at(10.0 * 3600.0 + 9.0 * 60.0 + 36.2)),
            ("midnight", |s| *s = State::default().at(0.0)),
            ("quarter-past", |s| *s = State::default().at(3.0 * 3600.0 + 15.0 * 60.0)),
        ],
        ..AppSpec::new("Mechanical Clock", WINDOW, State::default, view)
    });
}
