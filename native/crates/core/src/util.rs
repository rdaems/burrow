//! Small things server.py has at the top level: quoting for a shell, units, keys, times, a random source.

use regex::Regex;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// single-quote for a POSIX shell; a leading ~/ stays a home directory
pub fn sq(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        return format!("\"$HOME\"/{}", sq(rest));
    }
    if s == "~" {
        return "\"$HOME\"".into();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// a desk: a directory here, or "<machine id>:<directory>" on another machine
pub fn unit(mid: &str, d: &str) -> String {
    if mid.is_empty() { d.to_string() } else { format!("{mid}:{d}") }
}

pub fn split_unit(u: &str) -> (String, String) {
    if u.starts_with('/') || u.starts_with('~') {
        return (String::new(), u.to_string());
    }
    match u.split_once(':') {
        Some((m, d)) => (m.to_string(), d.to_string()),
        None => (u.to_string(), String::new()),
    }
}

/// a piece's kind: its key without the number (plant0 → plant)
pub fn kind_of(key: &str) -> &str {
    key.trim_end_matches(|c: char| c.is_ascii_digit())
}

static NOT_SEED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9-]").unwrap());

/// a face's seed as pair keeps it (its clean_avatar): letters, digits and dashes, 24 at most
pub fn pair_seed(s: &str) -> String {
    NOT_SEED.replace_all(s, "").chars().take(24).collect()
}

pub fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

pub fn now_ms() -> i64 {
    (now() * 1000.0) as i64
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn gmtoff(t: i64) -> i64 {
    unsafe {
        let tt = t as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&tt, &mut tm);
        tm.tm_gmtoff
    }
}

static ISO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2})(?::?(\d{2})(?::?(\d{2})(?:[.,](\d+))?)?)?)?(Z|[+-]\d{2}(?::?\d{2})?)?$").unwrap()
});

/// an ISO time (as the records keep it) in ms since the epoch, 0 if none; one without a zone is local
pub fn when(iso: &str) -> i64 {
    let Some(c) = ISO.captures(iso.trim()) else { return 0 };
    let n = |i: usize| c.get(i).map(|m| m.as_str().parse::<i64>().unwrap_or(0)).unwrap_or(0);
    let (mo, d) = (n(2), n(3));
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return 0;
    }
    let mut t = days_from_civil(n(1), mo, d) * 86400 + n(4) * 3600 + n(5) * 60 + n(6);
    let frac = c.get(7).map(|m| format!("{:0<3}", &m.as_str()[..m.as_str().len().min(3)]).parse::<i64>().unwrap_or(0)).unwrap_or(0);
    match c.get(8).map(|m| m.as_str()) {
        Some("Z") => {}
        Some(z) => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let digits: String = z[1..].chars().filter(|c| c.is_ascii_digit()).collect();
            let (h, m) = (digits[..2].parse::<i64>().unwrap_or(0), digits.get(2..4).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0));
            t -= sign * (h * 3600 + m * 60);
        }
        None => t -= gmtoff(t),
    }
    t * 1000 + frac
}

/// ms since the epoch as "2026-10-24 13:05", local time (the office's view)
pub fn local_minute(ms: i64) -> String {
    unsafe {
        let tt = (ms / 1000) as libc::time_t;
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&tt, &mut tm);
        format!("{:04}-{:02}-{:02} {:02}:{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min)
    }
}

/// this computer's short name (`hostname -s`)
pub fn hostname() -> String {
    let mut buf = [0u8; 256];
    let ok = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) } == 0;
    let s = if ok { String::from_utf8_lossy(&buf[..buf.iter().position(|&b| b == 0).unwrap_or(0)]).to_string() } else { String::new() };
    let s = s.split('.').next().unwrap_or("").to_string();
    if s.is_empty() { "here".into() } else { s }
}

/// the login name (getpass.getuser)
pub fn login() -> String {
    for k in ["LOGNAME", "USER", "LNAME", "USERNAME"] {
        if let Ok(v) = std::env::var(k)
            && !v.is_empty() {
                return v;
            }
    }
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if !pw.is_null() && !(*pw).pw_name.is_null() {
            return std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned();
        }
    }
    "you".into()
}

/// a program on the PATH (shutil.which)
pub fn which(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

// ---- chance: a small generator of its own (splitmix64), seeded by the clock; tests may pin it
static RNG: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new((now() * 1e9) as u64 ^ ((std::process::id() as u64) << 32)));

pub fn seed(s: u64) {
    *RNG.lock().unwrap() = s;
}

fn next() -> u64 {
    let mut g = RNG.lock().unwrap();
    *g = g.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *g;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// 0 ≤ x < 1
pub fn random() -> f64 {
    (next() >> 11) as f64 / (1u64 << 53) as f64
}

pub fn uniform(a: f64, b: f64) -> f64 {
    a + (b - a) * random()
}

/// 0 ≤ n < k
pub fn below(k: usize) -> usize {
    ((random() * k as f64) as usize).min(k.saturating_sub(1))
}

pub fn choice<T: Clone>(xs: &[T]) -> T {
    xs[below(xs.len())].clone()
}

/// two different indices of 0..n (n ≥ 2)
pub fn sample2(n: usize) -> (usize, usize) {
    let a = below(n);
    let b = (a + 1 + below(n - 1)) % n;
    (a, b)
}

/// Python's truthiness of a JSON value (bool(v))
pub fn truthy(v: &serde_json::Value) -> bool {
    use serde_json::Value::*;
    match v {
        Null => false,
        Bool(b) => *b,
        Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        String(s) => !s.is_empty(),
        Array(a) => !a.is_empty(),
        Object(o) => !o.is_empty(),
    }
}

/// a JSON value as Python's str() would read where server.py takes `x or ""`
pub fn text(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(v) if truthy(v) => v.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(sq("a b"), "'a b'");
        assert_eq!(sq("it's"), "'it'\\''s'");
        assert_eq!(sq("~/p/x"), "\"$HOME\"/'p/x'");
        assert_eq!(sq("~"), "\"$HOME\"");
    }

    #[test]
    fn units_and_keys() {
        assert_eq!(unit("", "/a"), "/a");
        assert_eq!(unit("lab", "/a"), "lab:/a");
        assert_eq!(split_unit("lab:/a:b"), ("lab".into(), "/a:b".into()));
        assert_eq!(split_unit("~/x"), ("".into(), "~/x".into()));
        assert_eq!(kind_of("plant12"), "plant");
        assert_eq!(kind_of("meet"), "meet");
        assert_eq!(pair_seed("rembert:15"), "rembert15");
        assert_eq!(pair_seed("a-very-long-seed-of-thirty-chars"), "a-very-long-seed-of-thir");
    }

    #[test]
    fn times() {
        assert_eq!(when("2026-10-24T12:00:00.123Z"), 1792843200123);
        assert_eq!(when("2026-10-24T14:00:00+02:00"), 1792843200000);
        assert_eq!(when(""), 0);
        assert_eq!(when("yesterday"), 0);
    }

    #[test]
    fn chance_stays_in_range() {
        for _ in 0..1000 {
            let x = random();
            assert!((0.0..1.0).contains(&x));
            let (a, b) = sample2(3);
            assert!(a != b && a < 3 && b < 3);
        }
    }
}
