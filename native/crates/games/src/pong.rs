//! pair's ping-pong (rdaems/pair-prompting, app/pong.js) as burrow plays it (web/pong.js): you against an
//! idle bot, pair's ball machine with a bot's face; or two bots' game to watch, both paddles machines. A bot
//! that gets work leaves the table (`left`). pair's live game between two pages is not used here and not
//! ported. The table is drawn from above into a 216 x 136 image; the card round it (who plays, the score,
//! how it ended) is the app's, from `card()`.

use crate::{Player, Rng, c, clamp, jround};
use pix::{Color, Image, css};

pub const FW: f64 = 200.;
pub const FH: f64 = 120.;
const FACE: f64 = 10.;
const HALF: f64 = 12.;
const R: f64 = 2.;
const TO: u32 = 11;
const M: f64 = 8.; // field units: paddle faces at x 10 and 190
const PAD: f64 = 125.; // burrow: your paddle's speed on the keys, field units a second (pair's 170 made it easy)
/// the canvas
pub const W: u32 = (FW + 2. * M) as u32;
pub const H: u32 = (FH + 2. * M) as u32;
/// you are always the left side
const SIDE: usize = 0;
const YOU: Color = Color::rgb(0xd97757);

/// the ball bounces off the long sides
pub fn fold(y: f64) -> f64 {
    let (lo, span) = (R, FH - 2. * R);
    let mut u = (y - lo) % (2. * span);
    if u < 0. {
        u += 2. * span;
    }
    lo + if u <= span { u } else { 2. * span - u }
}
fn face_x(s: usize) -> f64 {
    if s == 0 { FACE } else { FW - FACE }
}

/// a ball from where it was hit: where it is at any time follows from that (nothing integrated)
#[derive(Clone, Debug)]
pub struct Ball {
    pub t0: f64,
    pub x0: f64,
    pub y0: f64,
    pub vx: f64,
    pub vy: f64,
    /// a new one each serve or hit (the machines' err goes with the ball)
    n: u64,
    missed: Option<usize>,
}
impl Ball {
    /// when the ball reaches side s's paddle
    pub fn contact(&self, s: usize) -> f64 {
        self.t0 + (face_x(s) - self.x0) / self.vx
    }
    pub fn y_at(&self, t: f64) -> f64 {
        fold(self.y0 + self.vy * (t - self.t0))
    }
    /// the side it is heading for
    pub fn heading(&self) -> usize {
        if self.vx < 0. { 0 } else { 1 }
    }
}

#[derive(Clone, Debug)]
struct Mach {
    y: f64,
    ball: u64,
    err: f64,
}
impl Mach {
    fn new() -> Mach {
        Mach {
            y: FH / 2.,
            ball: 0,
            err: 0.,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    Points,
    Left,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Over {
    pub why: Why,
    pub winner: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
}

/// the game for the room: the bot you play (or the two you watch), whether a ball is in the air
#[derive(Clone, Debug, PartialEq)]
pub struct Playing {
    pub id: String,
    pub ids: Option<[String; 2]>,
    pub watch: bool,
    pub ball: bool,
}

/// what the card round the table says
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub players: [Option<Player>; 2],
    /// "serve" / "serves" beside the one to serve, while the ball waits
    pub serving: Option<(usize, &'static str)>,
    pub score: String,
    /// "↑ ↓ · to 11, two clear" under the table
    pub hint: bool,
    /// how it ended; then "play again" (`again`) and "someone else" (`choose`)
    pub over: Option<String>,
}

pub struct Pong {
    rng: Rng,
    t: f64,
    open: bool,
    playing: bool, // pong.js's mode "machine" (null: choosing)
    watching: bool,
    players: [Option<Player>; 2],
    opp: Option<Player>,
    ball: Option<Ball>,
    balls: u64,
    judged: bool,
    miss_at: f64,
    score: [u32; 2],
    serve_by: Option<usize>,
    serve_at: f64,
    over: Option<Over>,
    my_y: f64,
    keys: [bool; 2],
    mach: Mach,
    mach0: Option<Mach>,
    auto: Option<f64>,
    hits: u32,
}

impl Pong {
    pub fn new(seed: u64) -> Pong {
        Pong {
            rng: Rng::new(seed),
            t: 0.,
            open: false,
            playing: false,
            watching: false,
            players: [None, None],
            opp: None,
            ball: None,
            balls: 0,
            judged: false,
            miss_at: 0.,
            score: [0, 0],
            serve_by: None,
            serve_at: 0.,
            over: None,
            my_y: FH / 2.,
            keys: [false; 2],
            mach: Mach::new(),
            mach0: None,
            auto: None,
            hits: 0,
        }
    }

    // ---- the game itself
    fn serve(&mut self, s: usize, y: f64) {
        let a = (self.rng.next() - 0.5) * 0.6;
        let (sp, dir) = (115., if s == 0 { 1. } else { -1. });
        self.balls += 1;
        self.ball = Some(Ball {
            t0: self.t,
            x0: face_x(s),
            y0: y,
            vx: dir * sp * a.cos(),
            vy: sp * a.sin(),
            n: self.balls,
            missed: None,
        });
        self.judged = false;
        self.serve_by = None;
    }
    /// side s's paddle meets the ball at yb: harder, and angled by where it hit
    fn hit(&mut self, s: usize, yb: f64, tc: f64, pad: f64) {
        let b = self.ball.as_ref().unwrap();
        let sp = 300f64.min(b.vx.hypot(b.vy) * 1.06);
        let a = clamp((yb - pad) / (HALF + R), -1., 1.) * 0.9;
        self.balls += 1;
        self.ball = Some(Ball {
            t0: tc,
            x0: face_x(s),
            y0: yb,
            vx: (if s == 0 { 1. } else { -1. }) * sp * a.cos(),
            vy: sp * a.sin(),
            n: self.balls,
            missed: None,
        });
        self.judged = false;
        self.hits += 1;
    }
    /// the ball machine's game keeps its own score
    fn point(&mut self, winner: usize) {
        self.score[winner] += 1;
        self.ball = None;
        let sc = self.score;
        if sc[0].max(sc[1]) >= TO && sc[0].abs_diff(sc[1]) >= 2 {
            self.over = Some(Over {
                why: Why::Points,
                winner,
            });
            return;
        }
        self.serve_by = Some(server_of(sc));
        self.serve_at = self.t + 1.;
    }
    fn step(&mut self, dt: f64) {
        if self.watching && self.mach0.is_some() && self.over.is_none() {
            self.machine0(dt);
        }
        if self.keys[0] && !self.watching {
            self.my_y -= PAD * dt;
        }
        if self.keys[1] && !self.watching {
            self.my_y += PAD * dt;
        }
        if let (Some(o), Some(b)) = (self.auto, &self.ball) {
            if b.heading() == SIDE {
                self.my_y = b.y_at(b.contact(SIDE)) + o; // the test rig plays
            }
        }
        self.my_y = clamp(self.my_y, HALF, FH - HALF);
        let t = self.t;
        if self.playing && self.over.is_none() {
            self.machine(dt);
        }
        if self.over.is_some() || !self.playing {
            return;
        }
        if self.ball.is_none() && self.serve_by.is_some() && t >= self.serve_at {
            if self.serve_by == Some(SIDE) {
                self.serve(SIDE, self.my_y);
            } else {
                self.serve(1 - SIDE, self.mach.y);
            }
        }
        if let Some(b) = &self.ball {
            if !self.judged {
                let s = b.heading();
                let tc = b.contact(s);
                if t >= tc {
                    let yb = b.y_at(tc);
                    let pad = if s == SIDE { self.my_y } else { self.mach.y };
                    if (yb - pad).abs() <= HALF + R {
                        self.hit(s, yb, tc, pad);
                    } else {
                        self.judged = true;
                        self.miss_at = self.t + 0.3;
                        self.ball.as_mut().unwrap().missed = Some(s);
                    }
                }
            }
        }
        if self.judged && self.miss_at != 0. && self.t >= self.miss_at {
            self.miss_at = 0.;
            let s = self.ball.as_ref().and_then(|b| b.missed).unwrap_or(SIDE);
            self.point(1 - s);
        }
    }
    /// the ball machine follows the ball, a little late and a little off
    fn machine(&mut self, dt: f64) {
        let mut target = FH / 2. + (self.my_y - FH / 2.) * 0.25;
        if let Some(b) = &self.ball {
            if b.vx > 0. && !self.judged {
                if self.mach.ball != b.n {
                    self.mach.ball = b.n;
                    self.mach.err = (self.rng.next() - 0.5) * (14. + b.vx.hypot(b.vy) / 9.);
                }
                target = b.y_at(b.contact(1)) + self.mach.err;
            }
        }
        self.mach.y = clamp(
            self.mach.y + clamp(target - self.mach.y, -115. * dt, 115. * dt),
            HALF,
            FH - HALF,
        );
    }
    /// the left machine, the mirror of machine(): after the ball when it comes its way
    fn machine0(&mut self, dt: f64) {
        let mut target = FH / 2. + (self.mach.y - FH / 2.) * 0.25;
        let m0 = self.mach0.as_mut().unwrap();
        if let Some(b) = &self.ball {
            if b.vx < 0. && !self.judged {
                if m0.ball != b.n {
                    m0.ball = b.n;
                    m0.err = (self.rng.next() - 0.5) * (14. + b.vx.hypot(b.vy) / 9.);
                }
                target = b.y_at(b.contact(0)) + m0.err;
            }
        }
        m0.y = clamp(
            m0.y + clamp(target - m0.y, -115. * dt, 115. * dt),
            HALF,
            FH - HALF,
        );
        self.my_y = m0.y;
    }

    fn reset(&mut self) {
        self.playing = true;
        self.over = None;
        self.score = [0, 0];
        self.ball = None;
        self.judged = false;
        self.miss_at = 0.;
        self.mach = Mach::new();
        self.serve_by = Some(0);
        self.serve_at = self.t + 1.;
        self.open = true;
    }
    /// a game against this bot, right away (none: the same bot again)
    pub fn start(&mut self, bot: Option<Player>) {
        if let Some(b) = bot {
            self.opp = Some(b);
        }
        let Some(opp) = self.opp.clone() else { return };
        self.watching = false;
        self.mach0 = None;
        self.players = [Some(Player::you("You", YOU, None)), Some(opp)];
        self.reset();
    }
    /// two bots' game, to look on: the left paddle is a machine as well
    pub fn watch(&mut self, a: Player, b: Player) {
        self.opp = Some(b.clone());
        self.watching = true;
        self.players = [Some(a), Some(b)];
        self.reset();
        self.mach0 = Some(Mach::new());
    }
    /// "play again"
    pub fn again(&mut self) {
        self.start(None);
    }
    /// "someone else": back to choosing (the app lists the idle bots and calls `start`)
    pub fn choose(&mut self) {
        self.playing = false;
        self.over = None;
    }
    pub fn close(&mut self) {
        self.playing = false;
        self.over = None;
        self.ball = None;
        self.watching = false;
        self.open = false;
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    /// a bot that gets work leaves the table: the game is over
    pub fn left(&mut self, id: &str) {
        if self.playing && self.over.is_none() {
            let winner = if self.watching {
                if self.players[0].as_ref().is_some_and(|p| p.id == id) {
                    1
                } else {
                    0
                }
            } else {
                SIDE
            };
            self.over = Some(Over {
                why: Why::Left,
                winner,
            });
            self.ball = None;
        }
    }
    pub fn playing(&self) -> Option<Playing> {
        let opp = self.opp.as_ref()?;
        if !self.playing || self.over.is_some() {
            return None;
        }
        let ids = match (&self.players, self.watching) {
            ([Some(a), Some(b)], true) => Some([a.id.clone(), b.id.clone()]),
            _ => None,
        };
        Some(Playing {
            id: opp.id.clone(),
            ids,
            watch: self.watching,
            ball: self.ball.is_some(),
        })
    }

    /// the paddle goes with ↑ ↓ only (pair's mouse made it too easy); the app sends Esc to `close`
    pub fn key(&mut self, k: Key, down: bool) {
        if !self.open {
            return;
        }
        if down && !self.playing {
            return;
        }
        self.keys[k as usize] = down;
    }
    /// the window lost the keys
    pub fn blur(&mut self) {
        self.keys = [false; 2];
    }

    /// time goes by (the frame's dt, at most 0.05 s)
    pub fn tick(&mut self, dt: f64) {
        if !self.open {
            return;
        }
        let dt = dt.clamp(0., 0.05);
        self.t += dt;
        if self.playing {
            self.step(dt);
        }
    }

    pub fn card(&self) -> Card {
        let serving = match (self.over, self.playing, self.serve_by, &self.ball) {
            (None, true, Some(s), None) => Some((
                s,
                if s == SIDE && !self.watching {
                    "serve"
                } else {
                    "serves"
                },
            )),
            _ => None,
        };
        let name = |s: usize| match &self.players[s] {
            Some(p) if p.human => "You".to_string(),
            Some(p) => p.name.clone(),
            None => String::new(),
        };
        let over = self.over.map(|o| match o.why {
            Why::Points => {
                let you = self.players[o.winner].as_ref().is_some_and(|p| p.human);
                let (hi, lo) = (
                    self.score[0].max(self.score[1]),
                    self.score[0].min(self.score[1]),
                );
                format!(
                    "{} {}, {hi}–{lo}.",
                    name(o.winner),
                    if you { "win" } else { "wins" }
                )
            }
            Why::Left => format!(
                "{} left the table.",
                self.players[1 - o.winner].as_ref().map_or("", |p| &p.name)
            ),
        });
        Card {
            players: self.players.clone(),
            serving,
            score: if self.playing {
                format!("{} – {}", self.score[0], self.score[1])
            } else {
                String::new()
            },
            hint: self.playing && self.over.is_none() && !self.watching,
            over,
        }
    }

    /// the table from above, as paint() draws it
    pub fn draw(&self, img: &mut Image) {
        let t = self.t;
        let mut ctx = img.ctx();
        let (w, h) = (W as i32, H as i32);
        let m = M as i32;
        let (fw, fh) = (FW as i32, FH as i32);
        ctx.rect(0, 0, w, h, css("#130f1b"));
        let mut y = 3;
        while y < h {
            ctx.rect(0, y, w, 1, css("#19141f"));
            y += 6;
        }
        // the table from above: green top, white edges and centre line, the net across
        ctx.rect(m - 2, m - 2, fw + 4, fh + 4, c::INK);
        ctx.rect(m, m, fw, fh, c::SCRUB1);
        ctx.rect(m, m, fw, 1, c::STEEL4);
        ctx.rect(m, m + fh - 1, fw, 1, c::STEEL4);
        ctx.rect(m, m, 1, fh, c::STEEL4);
        ctx.rect(m + fw - 1, m, 1, fh, c::STEEL4);
        ctx.rect(m, m + fh / 2, fw, 1, css("rgba(244,246,251,.5)"));
        ctx.rect(m + fw / 2 + 1, m - 3, 2, fh + 6, css("rgba(0,0,0,.35)"));
        ctx.rect(m + fw / 2 - 1, m - 4, 2, fh + 8, c::STEEL3);
        ctx.rect(m + fw / 2 - 1, m - 5, 2, 2, c::INK2);
        ctx.rect(m + fw / 2 - 1, m + fh + 3, 2, 2, c::INK2);
        // the paddles, in their players' colours
        let mut ys = [0.; 2];
        ys[SIDE] = self.my_y;
        ys[1 - SIDE] = self.mach.y;
        for s in [0, 1] {
            let col = self.players[s].as_ref().map_or(c::STEEL2, |p| p.color);
            let x = M + if s == 0 { FACE - 4. } else { FW - FACE + 1. };
            let y = jround(M + ys[s] - HALF);
            ctx.fill_rect(x - 1., y - 1., 5., HALF * 2. + 2., c::INK);
            ctx.fill_rect(x, y, 3., HALF * 2., col);
            ctx.fill_rect(x, y, 1., HALF * 2., css("rgba(255,255,255,.35)"));
        }
        // the ball
        if let Some(b) = &self.ball {
            let x = b.x0 + b.vx * (t - b.t0);
            let y = b.y_at(t);
            if x > -6. && x < FW + 6. {
                let (bx, by) = (jround(M + x), jround(M + y));
                ctx.fill_rect(bx - 1., by + 2., 4., 2., css("rgba(0,0,0,.4)"));
                ctx.fill_rect(bx - 2., by - 2., 5., 5., c::INK);
                ctx.fill_rect(bx - 1., by - 1., 3., 3., c::WHITE);
            }
        } else if let (None, Some(s), true) = (self.over, self.serve_by, self.playing) {
            let y = jround(M + if s == SIDE { self.my_y } else { self.mach.y });
            if (t * 4.).floor() as i64 % 2 != 0 {
                ctx.fill_rect(
                    M + if s == 0 { FACE + 3. } else { FW - FACE - 5. },
                    y - 1.,
                    2.,
                    2.,
                    c::WHITE,
                );
            }
        }
    }

    // ---- for tests (window.__pong)
    pub fn score(&self) -> [u32; 2] {
        self.score
    }
    pub fn over(&self) -> Option<Over> {
        self.over
    }
    pub fn hits(&self) -> u32 {
        self.hits
    }
    pub fn ball(&self) -> Option<&Ball> {
        self.ball.as_ref()
    }
    pub fn my_y(&self) -> f64 {
        self.my_y
    }
    pub fn now(&self) -> f64 {
        self.t
    }
    /// your paddle meets every ball this far from its middle (none: the keys again)
    pub fn auto(&mut self, o: Option<f64>) {
        self.auto = o;
    }
}

/// who serves at a score: two each, one each from 10 all
pub fn server_of(sc: [u32; 2]) -> usize {
    let n = sc[0] + sc[1];
    (if sc[0].min(sc[1]) >= TO - 1 {
        n % 2
    } else {
        (n / 2) % 2
    }) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(id: &str) -> Player {
        Player::bot(id, id, css("#4a76b8"))
    }
    fn run(p: &mut Pong, secs: f64) {
        for _ in 0..(secs * 60.) as usize {
            p.tick(1. / 60.);
        }
    }

    #[test]
    fn the_ball_folds_off_the_long_sides() {
        assert_eq!(fold(60.), 60.);
        assert_eq!(fold(FH - R + 5.), FH - R - 5.);
        assert_eq!(fold(R - 5.), R + 5.);
        assert!((fold(60. + 2. * (FH - 2. * R)) - 60.).abs() < 1e-9);
        let b = Ball {
            t0: 1.,
            x0: FACE,
            y0: 60.,
            vx: 100.,
            vy: 0.,
            n: 0,
            missed: None,
        };
        assert!((b.contact(1) - (1. + (FW - 2. * FACE) / 100.)).abs() < 1e-12);
    }

    #[test]
    fn serving_goes_two_each_then_one_each_from_ten_all() {
        assert_eq!(server_of([0, 0]), 0);
        assert_eq!(server_of([1, 0]), 0);
        assert_eq!(server_of([1, 1]), 1);
        assert_eq!(server_of([3, 0]), 1);
        assert_eq!(server_of([10, 10]), 0);
        assert_eq!(server_of([11, 10]), 1);
    }

    #[test]
    fn a_hit_is_harder_and_angled_by_where_it_met() {
        let mut p = Pong::new(1);
        p.start(Some(bot("b")));
        p.ball = Some(Ball {
            t0: 0.,
            x0: FW - FACE,
            y0: 60.,
            vx: -200.,
            vy: 0.,
            n: 9,
            missed: None,
        });
        p.hit(0, 60. + HALF + R + 5., 1., 60.); // off the paddle's very end: the angle is held at 0.9
        let b = p.ball.as_ref().unwrap();
        assert!((b.vx.hypot(b.vy) - 212.).abs() < 1e-9);
        assert!((b.vy.atan2(b.vx) - 0.9).abs() < 1e-9);
        p.ball = Some(Ball {
            t0: 0.,
            x0: FW - FACE,
            y0: 60.,
            vx: -290.,
            vy: 0.,
            n: 9,
            missed: None,
        });
        p.hit(0, 60., 1., 60.);
        assert_eq!(p.ball.as_ref().unwrap().vx, 300.); // at most 300
    }

    #[test]
    fn a_miss_is_a_point_after_a_moment_and_eleven_two_clear_wins() {
        let mut p = Pong::new(7);
        p.start(Some(bot("b")));
        p.auto(Some(HALF + R + 8.)); // always beside the ball: every ball that comes to you is missed
        let mut was = 0;
        for _ in 0..60 * 120 {
            p.tick(1. / 60.);
            let s = p.score()[1];
            if s != was {
                assert_eq!(s, was + 1);
                was = s;
            }
            if p.over().is_some() {
                break;
            }
        }
        let o = p.over().expect("over");
        assert_eq!(
            o,
            Over {
                why: Why::Points,
                winner: 1
            }
        );
        assert!(p.score()[1] >= TO && p.score()[1] >= p.score()[0] + 2);
        assert_eq!(
            p.card().over.as_deref(),
            Some(format!("b wins, {}–{}.", p.score()[1], p.score()[0]).as_str())
        );
        assert!(p.playing().is_none());
    }

    #[test]
    fn the_rig_returns_every_ball() {
        let mut p = Pong::new(3);
        p.start(Some(bot("b")));
        p.auto(Some(0.));
        run(&mut p, 30.);
        assert!(p.hits() > 6, "{}", p.hits());
        assert_eq!(p.score()[1], 0);
    }

    #[test]
    fn watching_plays_on_its_own_and_a_bot_leaving_ends_it() {
        let mut p = Pong::new(11);
        p.watch(bot("a"), bot("b"));
        assert!(p.card().players[0].as_ref().unwrap().id == "a");
        p.key(Key::Up, true); // the keys move nothing while watching
        run(&mut p, 90.);
        assert!(p.hits() > 4, "{}", p.hits());
        assert!(p.score()[0] + p.score()[1] > 0 || p.over().is_some());
        let pl = p.playing();
        if p.over().is_none() {
            let pl = pl.unwrap();
            assert!(pl.watch);
            assert_eq!(pl.ids, Some(["a".into(), "b".into()]));
            p.left("a");
            assert_eq!(
                p.over(),
                Some(Over {
                    why: Why::Left,
                    winner: 1
                })
            );
            assert_eq!(p.card().over.as_deref(), Some("a left the table."));
        }
    }

    #[test]
    fn the_keys_move_your_paddle() {
        let mut p = Pong::new(2);
        p.start(Some(bot("b")));
        p.key(Key::Up, true);
        run(&mut p, 0.2);
        assert!(p.my_y() < FH / 2. - 20.);
        run(&mut p, 2.);
        assert_eq!(p.my_y(), HALF);
        p.key(Key::Up, false);
        p.left("b");
        assert_eq!(p.card().over.as_deref(), Some("b left the table."));
    }

    #[test]
    fn it_draws_the_table() {
        let mut p = Pong::new(5);
        p.start(Some(bot("b")));
        run(&mut p, 1.5);
        let mut im = Image::new(W, H);
        p.draw(&mut im);
        assert_eq!((W, H), (216, 136));
        assert_eq!(im.get(0, 0), css("#130f1b").0);
        assert_eq!(im.get(50, 50), c::SCRUB1.0);
        assert_eq!(im.get(8, 30), c::STEEL4.0); // the edge
        assert_eq!(im.get(M as i32 + 7, M as i32 + 60), YOU.0); // your paddle
        assert_eq!(im.inked(), (W * H) as usize);
    }
}
