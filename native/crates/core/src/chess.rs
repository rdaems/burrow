//! The rules of chess for the room's chess table (web/chess.py, pair's rules): legal moves (castling, en
//! passant, promotion), check, mate and the draws, moves read and written in SAN (Nf3, exd5, O-O, e8=Q) or
//! as coordinates (g1f3). And the bots' play: two plies of material and a little chance, not Claude.
//!
//! Squares are 0..63, a1 = 0, h1 = 7, a8 = 56; white pieces are upper case, an empty square is 0.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

const FILES: &[u8; 8] = b"abcdefgh";
pub const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// a move: from, to, and the piece a pawn becomes (0, or one of b"qrbn")
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub promo: u8,
}

pub fn square(s: u8) -> String {
    format!("{}{}", FILES[(s % 8) as usize] as char, s / 8 + 1)
}

pub fn parse_square(name: &str) -> u8 {
    let b = name.as_bytes();
    (b[0] - b'a') + 8 * (b[1] - b'1')
}

pub fn uci(m: Move) -> String {
    let mut s = square(m.from) + &square(m.to);
    if m.promo != 0 {
        s.push(m.promo as char);
    }
    s
}

fn step(s: u8, df: i32, dr: i32) -> Option<u8> {
    let (f, r) = ((s % 8) as i32 + df, (s / 8) as i32 + dr);
    ((0..8).contains(&f) && (0..8).contains(&r)).then(|| (f + 8 * r) as u8)
}

fn jumps(s: u8, ds: &[(i32, i32)]) -> Vec<u8> {
    ds.iter().filter_map(|&(f, r)| step(s, f, r)).collect()
}

fn rays(s: u8, ds: &[(i32, i32)]) -> Vec<Vec<u8>> {
    ds.iter()
        .map(|&(df, dr)| {
            let (mut ray, mut t) = (vec![], step(s, df, dr));
            while let Some(x) = t {
                ray.push(x);
                t = step(x, df, dr);
            }
            ray
        })
        .collect()
}

struct Tables {
    knight: Vec<Vec<u8>>,
    king: Vec<Vec<u8>>,
    diag: Vec<Vec<Vec<u8>>>,
    line: Vec<Vec<Vec<u8>>>,
}

static T: LazyLock<Tables> = LazyLock::new(|| Tables {
    knight: (0..64).map(|s| jumps(s, &[(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)])).collect(),
    king: (0..64).map(|s| jumps(s, &[(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)])).collect(),
    diag: (0..64).map(|s| rays(s, &[(1, 1), (1, -1), (-1, 1), (-1, -1)])).collect(),
    line: (0..64).map(|s| rays(s, &[(1, 0), (-1, 0), (0, 1), (0, -1)])).collect(),
});

// castling: (right, king from, king to, squares that must be empty, squares not under attack, rook from)
const CASTLES: [(u8, u8, u8, &[u8], &[u8], u8); 4] = [
    (b'K', 4, 6, &[5, 6], &[4, 5, 6], 7),
    (b'Q', 4, 2, &[3, 2, 1], &[4, 3, 2], 0),
    (b'k', 60, 62, &[61, 62], &[60, 61, 62], 63),
    (b'q', 60, 58, &[59, 58, 57], &[60, 59, 58], 56),
];

/// a move from or onto one of these squares ends those rights
fn rights(s: u8) -> &'static str {
    match s {
        4 => "KQ",
        60 => "kq",
        0 => "Q",
        7 => "K",
        56 => "q",
        63 => "k",
        _ => "",
    }
}

fn upper(p: u8) -> u8 {
    p.to_ascii_uppercase()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub board: [u8; 64],
    pub white: bool,
    pub castle: String,
    pub ep: Option<u8>,
    pub half: u32,
    pub full: u32,
}

impl Position {
    pub fn fen(text: &str) -> Position {
        let mut f: Vec<&str> = text.split_whitespace().collect();
        f.extend(["-", "-", "0", "1"]);
        let mut p = Position { board: [0; 64], white: f[1] == "w", castle: String::new(), ep: None, half: 0, full: 1 };
        for (r, row) in f[0].split('/').enumerate() {
            let mut file = 0;
            for ch in row.bytes() {
                if ch.is_ascii_digit() {
                    file += (ch - b'0') as usize;
                } else {
                    p.board[(7 - r) * 8 + file] = ch;
                    file += 1;
                }
            }
        }
        p.castle = if f[2] == "-" { String::new() } else { f[2].to_string() };
        p.ep = if f[3] == "-" { None } else { Some(parse_square(f[3])) };
        p.half = f[4].parse().unwrap_or(0);
        p.full = f[5].parse().unwrap_or(1);
        p
    }

    pub fn start() -> Position {
        Position::fen(START)
    }

    pub fn to_fen(&self) -> String {
        let mut rows = vec![];
        for r in (0..8).rev() {
            let (mut row, mut gap) = (String::new(), 0);
            for f in 0..8 {
                let ch = self.board[r * 8 + f];
                if ch != 0 {
                    if gap > 0 {
                        row += &gap.to_string();
                    }
                    row.push(ch as char);
                    gap = 0;
                } else {
                    gap += 1;
                }
            }
            if gap > 0 {
                row += &gap.to_string();
            }
            rows.push(row);
        }
        let ep = self.ep.map(square).unwrap_or_else(|| "-".into());
        let castle = if self.castle.is_empty() { "-" } else { &self.castle };
        format!("{} {} {} {} {} {}", rows.join("/"), if self.white { "w" } else { "b" }, castle, ep, self.half, self.full)
    }

    fn mine(&self, p: u8) -> bool {
        p != 0 && if self.white { p.is_ascii_uppercase() } else { p.is_ascii_lowercase() }
    }

    pub fn attacked(&self, s: u8, by_white: bool) -> bool {
        let b = &self.board;
        let side = |c: u8| if by_white { c } else { c.to_ascii_lowercase() };
        let (pawn, knight, bishop, rook, queen, king) = (side(b'P'), side(b'N'), side(b'B'), side(b'R'), side(b'Q'), side(b'K'));
        for df in [-1, 1] {
            // a pawn attacks one rank further on, diagonally
            if let Some(t) = step(s, df, if by_white { -1 } else { 1 }) {
                if b[t as usize] == pawn {
                    return true;
                }
            }
        }
        if T.knight[s as usize].iter().any(|&t| b[t as usize] == knight) || T.king[s as usize].iter().any(|&t| b[t as usize] == king) {
            return true;
        }
        for (rays, a) in [(&T.diag[s as usize], bishop), (&T.line[s as usize], rook)] {
            for ray in rays {
                for &t in ray {
                    let x = b[t as usize];
                    if x != 0 {
                        if x == a || x == queen {
                            return true;
                        }
                        break;
                    }
                }
            }
        }
        false
    }

    pub fn king(&self, white: bool) -> u8 {
        let k = if white { b'K' } else { b'k' };
        self.board.iter().position(|&p| p == k).expect("a king") as u8
    }

    pub fn in_check(&self) -> bool {
        self.attacked(self.king(self.white), !self.white)
    }

    pub fn pseudo(&self) -> Vec<Move> {
        let (b, w) = (&self.board, self.white);
        let mut out = vec![];
        let mv = |from: u8, to: u8, promo: u8| Move { from, to, promo };
        for s in 0..64u8 {
            let p = b[s as usize];
            if !self.mine(p) {
                continue;
            }
            match upper(p) {
                b'P' => {
                    let (d, last): (i32, u8) = if w { (8, 7) } else { (-8, 0) };
                    let mut moves = vec![];
                    let one = (s as i32 + d) as u8;
                    if b[one as usize] == 0 {
                        moves.push(one);
                        let two = (s as i32 + 2 * d) as u8;
                        if s / 8 == if w { 1 } else { 6 } && b[two as usize] == 0 {
                            out.push(mv(s, two, 0));
                        }
                    }
                    for df in [-1, 1] {
                        if let Some(t) = step(s, df, if w { 1 } else { -1 }) {
                            let x = b[t as usize];
                            if (x != 0 && !self.mine(x)) || Some(t) == self.ep {
                                moves.push(t);
                            }
                        }
                    }
                    for t in moves {
                        if t / 8 == last {
                            out.extend(b"qrbn".iter().map(|&pr| mv(s, t, pr)));
                        } else {
                            out.push(mv(s, t, 0));
                        }
                    }
                }
                k @ (b'N' | b'K') => {
                    let to = if k == b'N' { &T.knight[s as usize] } else { &T.king[s as usize] };
                    out.extend(to.iter().filter(|&&t| !self.mine(b[t as usize])).map(|&t| mv(s, t, 0)));
                }
                k => {
                    let rays: Vec<&Vec<u8>> = match k {
                        b'B' => T.diag[s as usize].iter().collect(),
                        b'R' => T.line[s as usize].iter().collect(),
                        _ => T.diag[s as usize].iter().chain(T.line[s as usize].iter()).collect(),
                    };
                    for ray in rays {
                        for &t in ray {
                            let x = b[t as usize];
                            if x == 0 {
                                out.push(mv(s, t, 0));
                            } else {
                                if !self.mine(x) {
                                    out.push(mv(s, t, 0));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        let rook = if w { b'R' } else { b'r' };
        for (right, ks, kt, empty, safe, rs) in CASTLES {
            if self.castle.as_bytes().contains(&right)
                && right.is_ascii_uppercase() == w
                && b[rs as usize] == rook
                && !empty.iter().any(|&x| b[x as usize] != 0)
                && !safe.iter().any(|&x| self.attacked(x, !w))
            {
                out.push(mv(ks, kt, 0));
            }
        }
        out
    }

    pub fn play(&self, m: Move) -> Position {
        let (s, t) = (m.from as usize, m.to as usize);
        let mut n = self.clone();
        let (p, mut captured) = (n.board[s], n.board[t]);
        let k = upper(p);
        n.board[t] = p;
        n.board[s] = 0;
        if k == b'P' && Some(m.to) == self.ep {
            // en passant: the pawn taken stands beside, not on the square
            n.board[if self.white { t - 8 } else { t + 8 }] = 0;
            captured = b'p';
        }
        if m.promo != 0 {
            n.board[t] = if self.white { m.promo.to_ascii_uppercase() } else { m.promo };
        }
        if k == b'K' && (t as i32 - s as i32).abs() == 2 {
            // castling: the rook jumps over
            let (rf, rt) = if t > s { (t + 1, t - 1) } else { (t - 2, t + 1) };
            n.board[rt] = n.board[rf];
            n.board[rf] = 0;
        }
        n.ep = (k == b'P' && (t as i32 - s as i32).abs() == 16).then(|| ((s + t) / 2) as u8);
        for x in [m.from, m.to] {
            let r = rights(x);
            if !r.is_empty() {
                n.castle = n.castle.chars().filter(|c| !r.contains(*c)).collect();
            }
        }
        n.half = if k == b'P' || captured != 0 { 0 } else { self.half + 1 };
        n.full = self.full + if self.white { 0 } else { 1 };
        n.white = !self.white;
        n
    }

    pub fn legal(&self) -> Vec<Move> {
        self.pseudo()
            .into_iter()
            .filter(|&m| {
                let n = self.play(m);
                !n.attacked(n.king(self.white), !self.white)
            })
            .collect()
    }

    pub fn san(&self, m: Move, legal: &[Move]) -> String {
        let (s, t) = (m.from, m.to);
        let b = &self.board;
        let p = upper(b[s as usize]);
        let mut out = if p == b'K' && (t as i32 - s as i32).abs() == 2 {
            if t > s { "O-O".to_string() } else { "O-O-O".to_string() }
        } else if p == b'P' {
            let cap = b[t as usize] != 0 || Some(t) == self.ep;
            let mut o = String::new();
            if cap {
                o.push(FILES[(s % 8) as usize] as char);
                o.push('x');
            }
            o += &square(t);
            if m.promo != 0 {
                o.push('=');
                o.push(m.promo.to_ascii_uppercase() as char);
            }
            o
        } else {
            let rivals: Vec<u8> = legal.iter().filter(|o| o.to == t && o.from != s && b[o.from as usize] == b[s as usize]).map(|o| o.from).collect();
            let mut dis = String::new();
            if !rivals.is_empty() {
                if rivals.iter().all(|o| o % 8 != s % 8) {
                    dis.push(FILES[(s % 8) as usize] as char);
                } else if rivals.iter().all(|o| o / 8 != s / 8) {
                    dis = (s / 8 + 1).to_string();
                } else {
                    dis = square(s);
                }
            }
            format!("{}{}{}{}", p as char, dis, if b[t as usize] != 0 { "x" } else { "" }, square(t))
        };
        let n = self.play(m);
        if n.in_check() {
            out.push(if n.legal().is_empty() { '#' } else { '+' });
        }
        out
    }

    /// what makes two positions the same for repetition: the en-passant square only counts when a pawn
    /// can actually take there
    pub fn key(&self, legal: &[Move]) -> String {
        let fen = self.to_fen();
        let mut k = fen.split(' ').take(3).collect::<Vec<_>>().join(" ");
        if let Some(ep) = self.ep {
            if legal.iter().any(|m| m.to == ep && upper(self.board[m.from as usize]) == b'P') {
                k = k + " " + &square(ep);
            }
        }
        k
    }

    pub fn insufficient(&self) -> bool {
        let rest: Vec<(usize, u8)> = self.board.iter().enumerate().filter(|(_, p)| **p != 0 && upper(**p) != b'K').map(|(s, p)| (s, *p)).collect();
        if rest.is_empty() || (rest.len() == 1 && matches!(upper(rest[0].1), b'N' | b'B')) {
            return true;
        }
        // only bishops left, all on squares of one colour
        rest.iter().all(|(_, p)| upper(*p) == b'B') && {
            let c = (rest[0].0 % 8 + rest[0].0 / 8) % 2;
            rest.iter().all(|(s, _)| (s % 8 + s / 8) % 2 == c)
        }
    }
}

fn norm(san: &str) -> String {
    san.replace(['=', 'x', '+', '#'], "")
}

static TRAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[+#!?\s]+$").unwrap());
static COORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[NBRQK]?([a-h][1-8])[-x:]?([a-h][1-8])=?([qrbnQRBN]?)$").unwrap());

/// a game replayed from its moves (coordinates), with everything a view needs
#[derive(Clone, Debug)]
pub struct Game {
    pub pos: Position,
    pub legal: Vec<Move>,
    pub moves: Vec<String>,
    pub sans: Vec<String>,
    seen: HashMap<String, u32>,
    repeated: u32,
}

impl Default for Game {
    fn default() -> Game {
        Game::new()
    }
}

impl Game {
    pub fn new() -> Game {
        let pos = Position::start();
        let legal = pos.legal();
        let seen = HashMap::from([(pos.key(&legal), 1)]);
        Game { pos, legal, moves: vec![], sans: vec![], seen, repeated: 0 }
    }

    pub fn from_moves(moves: &[&str]) -> Option<Game> {
        let mut g = Game::new();
        for u in moves {
            let m = g.find(u)?;
            g.push(m);
        }
        Some(g)
    }

    /// a legal move from what someone typed (SAN or coordinates), or None
    pub fn find(&self, text: &str) -> Option<Move> {
        let t = TRAIL.replace(text.trim(), "").replace("e.p.", "");
        let t = t.trim().replace("0-0-0", "O-O-O").replace("0-0", "O-O");
        if let Some(c) = COORD.captures(&t) {
            let (s, d) = (parse_square(&c[1]), parse_square(&c[2]));
            let pr = c[3].to_ascii_lowercase();
            let pr = pr.as_bytes().first().copied().unwrap_or(0);
            return self.legal.iter().copied().find(|m| m.from == s && m.to == d && if pr != 0 { m.promo == pr } else { m.promo == 0 || m.promo == b'q' });
        }
        let mut sans: Vec<(String, Move)> = vec![];
        for &m in &self.legal {
            let k = norm(&self.pos.san(m, &self.legal));
            match sans.iter_mut().find(|(x, _)| *x == k) {
                Some(e) => e.1 = m, // a dict: the later one stands
                None => sans.push((k, m)),
            }
        }
        let n = norm(&t);
        if let Some((_, m)) = sans.iter().find(|(k, _)| *k == n) {
            return Some(*m);
        }
        let loose: Vec<Move> = sans.iter().filter(|(k, _)| k.to_lowercase() == n.to_lowercase()).map(|(_, m)| *m).collect();
        (loose.len() == 1).then(|| loose[0])
    }

    pub fn push(&mut self, m: Move) {
        self.sans.push(self.pos.san(m, &self.legal));
        self.moves.push(uci(m));
        self.pos = self.pos.play(m);
        self.legal = self.pos.legal();
        let k = self.pos.key(&self.legal);
        let n = self.seen.entry(k).or_insert(0);
        *n += 1;
        self.repeated = *n;
    }

    /// (result, why) once the rules end the game, else None
    pub fn over(&self) -> Option<(&'static str, &'static str)> {
        if self.legal.is_empty() {
            if self.pos.in_check() {
                return Some((if self.pos.white { "0-1" } else { "1-0" }, "checkmate"));
            }
            return Some(("1/2-1/2", "stalemate"));
        }
        if !self.moves.is_empty() && self.repeated >= 3 {
            return Some(("1/2-1/2", "repetition"));
        }
        if self.pos.half >= 100 {
            return Some(("1/2-1/2", "fifty moves"));
        }
        if self.pos.insufficient() {
            return Some(("1/2-1/2", "material"));
        }
        None
    }

    /// the board as the state sends it: 64 squares, a1 first (server.py joins pos.board), "." for none
    pub fn board(&self) -> String {
        self.pos.board.iter().map(|&p| if p == 0 { '.' } else { p as char }).collect()
    }
}

fn value(p: u8) -> i32 {
    match p.to_ascii_lowercase() {
        b'p' => 1,
        b'n' | b'b' => 3,
        b'r' => 5,
        b'q' => 9,
        _ => 0,
    }
}

pub fn material(pos: &Position, white: bool) -> i32 {
    pos.board.iter().filter(|&&p| p != 0).map(|&p| value(p) * if p.is_ascii_uppercase() == white { 1 } else { -1 }).sum()
}

/// a bot's move: the one that leaves it best off in material after your best reply; mate if it sees one.
/// `random` gives 0..1 (the little chance)
pub fn bot_move(game: &Game, random: &mut dyn FnMut() -> f64) -> Move {
    let me = game.pos.white;
    let (mut best, mut pick): (Option<f64>, Move) = (None, game.legal[0]);
    for &m in &game.legal {
        let p = game.pos.play(m);
        let replies = p.legal();
        let mut s = if replies.is_empty() {
            if p.in_check() { 1000.0 } else { -0.5 }
        } else {
            replies.iter().map(|&r| material(&p.play(r), me)).min().unwrap() as f64 + if p.in_check() { 0.3 } else { 0.0 }
        };
        s += random() * 0.6;
        if best.is_none_or(|b| s > b) {
            best = Some(s);
            pick = m;
        }
    }
    pick
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perft(p: &Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        p.legal().into_iter().map(|m| perft(&p.play(m), depth - 1)).sum()
    }

    #[test]
    fn perft_from_the_start() {
        let p = Position::start();
        assert_eq!(perft(&p, 1), 20);
        assert_eq!(perft(&p, 2), 400);
        assert_eq!(perft(&p, 3), 8902);
    }

    #[test]
    fn perft_kiwipete_two_plies() {
        // castling both ways, en passant, promotions: the classic test position
        let p = Position::fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!(perft(&p, 1), 48);
        assert_eq!(perft(&p, 2), 2039);
    }

    #[test]
    fn fools_mate() {
        let g = Game::from_moves(&["f3", "e5", "g4", "Qh4"]).unwrap();
        assert_eq!(g.sans.last().unwrap(), "Qh4#");
        assert_eq!(g.over(), Some(("0-1", "checkmate")));
    }

    #[test]
    fn fen_round_trip_and_moves_read() {
        assert_eq!(Position::start().to_fen(), START);
        let g = Game::from_moves(&["e2e4", "e7e5", "Nf3", "Nc6", "Bb5", "a6", "0-0"]).unwrap();
        assert_eq!(g.sans, ["e4", "e5", "Nf3", "Nc6", "Bb5", "a6", "O-O"]);
        assert_eq!(g.moves.last().unwrap(), "e1g1");
        assert_eq!(g.pos.to_fen(), "r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 1 4"); // as chess.py has it
        assert!(g.find("nonsense").is_none());
    }

    #[test]
    fn repetition_ends_it() {
        let g = Game::from_moves(&["Nf3", "Nf6", "Ng1", "Ng8", "Nf3", "Nf6", "Ng1", "Ng8"]).unwrap();
        assert_eq!(g.over(), Some(("1/2-1/2", "repetition")));
    }

    #[test]
    fn the_bot_takes_a_mate_in_one() {
        // white mates with Qh7#? Scholar's: after e4 e5 Bc4 Nc6 Qh5 Nf6?? Qxf7#
        let g = Game::from_moves(&["e4", "e5", "Bc4", "Nc6", "Qh5", "Nf6"]).unwrap();
        let m = bot_move(&g, &mut || 0.5);
        assert_eq!(uci(m), "h5f7");
    }
}
