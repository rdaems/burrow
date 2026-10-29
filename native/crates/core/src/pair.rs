//! A pair room (rdaems/pair-prompting, e.g. pair.daems.me): a chat where people and agents talk. One
//! table is one room: the piece holds your own page's link there (/h/<token>, the bookmark that is your
//! login); its `state` gives the room's name and your "bring" link, which makes a new agent of whatever
//! chat it is pasted into. A bot dropped on the piece gets that link typed into its session, and joins.
//! The person at the table is you, as the room draws you: a room opened from here (pair's POST
//! /api/rooms) or joined by its invite (/r/<invite>) gets your name and face, and so does the seat of a
//! page you link; a change of either here is sent to every linked room (pair's /h/<token>/me).

use crate::hub::{Hub, Res};
use crate::room::{self, Room};
use crate::util::{kind_of, text};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use std::time::Duration;

/// a pair room as its state says: your page, its name, your bring link
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct PairInfo {
    pub url: String,
    pub room: String,
    pub bring: String,
    pub invite: String,
    pub name: String,
    pub people: Vec<String>,
    /// what pair said when your name and face were put on the seat, if it refused
    pub warn: String,
}

static PAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(https?)://([^/?#]+)(/[hr]/[A-Za-z0-9_-]+/?)$").unwrap());

impl Hub {
    /// one call to pair: its JSON, or its error in its words
    pub(crate) fn pair_fetch(&self, url: &str, body: Option<Value>) -> Res<Value> {
        let (code, text_) = self.runner().http(url, body.as_ref()).map_err(|e| format!("pair is out of reach: {}", e.chars().take(80).collect::<String>()))?;
        if (200..300).contains(&code) {
            return serde_json::from_str(&text_).map_err(|e| format!("pair is out of reach: {}", e.to_string().chars().take(80).collect::<String>()));
        }
        let t = text_.trim();
        let err = serde_json::from_str::<Value>(t).ok().and_then(|v| v.get("error").filter(|e| crate::util::truthy(e)).map(|e| text(Some(e))));
        if let Some(e) = err {
            return Err(e);
        }
        if code == 404 {
            return Err("pair does not know that page".into());
        }
        // not pair's own words: whoever answered, as it said it
        let said = if t.is_empty() { String::new() } else { format!(" ({})", t.lines().next().unwrap_or("").chars().take(60).collect::<String>()) };
        Err(format!("pair answered {code}{said}"))
    }

    /// the link of your page in a room, or the room's invite: joined with your name and face, it gives the page
    pub(crate) fn pair_page(&self, url: &str) -> Res<String> {
        let url = url.split(['?', '#']).next().unwrap_or("");
        let c = PAGE.captures(url).ok_or("that is not a pair room: your page there (…/h/…) or its invite (…/r/…)")?;
        let base = format!("{}://{}{}", &c[1], &c[2], c[3].trim_end_matches('/'));
        if c[3].starts_with("/r/") {
            let (name, seed) = self.you_now();
            let r = self.pair_fetch(&base, Some(json!({"name": name, "avatar": seed})))?;
            return Ok(format!("{}://{}{}", &c[1], &c[2], text(r.get("url"))));
        }
        Ok(base)
    }

    fn you_now(&self) -> (String, String) {
        room::you_of(&self.lock().room, &self.cfg.user)
    }

    /// a new room at the pair server, opened by you: your page in it
    pub(crate) fn pair_new(&self, name: &str) -> Res<String> {
        let (you, seed) = self.you_now();
        let r = self.pair_fetch(&format!("{}/api/rooms", self.cfg.pair), Some(json!({"room": name, "name": you, "avatar": seed})))?;
        Ok(format!("{}{}", self.cfg.pair, text(r.get("url"))))
    }

    pub(crate) fn pair_state(&self, url: &str) -> Res<PairInfo> {
        let st = self.pair_fetch(&format!("{url}/state?since=0"), None)?;
        Ok(PairInfo {
            url: url.into(),
            room: text(st.get("room")),
            bring: text(st.get("bring")),
            invite: text(st.get("invite")),
            name: text(st.get("name")),
            people: st.get("people").and_then(Value::as_array).into_iter().flatten().map(|p| text(p.get("name"))).collect(),
            warn: String::new(),
        })
    }

    /// your name and face at the table, put on your seat in that room
    pub(crate) fn pair_me(&self, url: &str) -> Res<()> {
        let (name, seed) = self.you_now();
        self.pair_fetch(&format!("{url}/me"), Some(json!({"name": name, "avatar": seed}))).map(|_| ())
    }

    /// you changed: every linked room hears, a moment later (one round for a burst of clicks)
    pub(crate) fn pair_me_all(&self, room: &Room, delay: Duration) {
        let mine = self.me_round.fetch_add(1, Ordering::SeqCst) + 1;
        let urls: Vec<String> = room.get("pair").and_then(Value::as_object).map(|p| p.values().map(|x| text(x.get("url"))).filter(|u| !u.is_empty()).collect()).unwrap_or_default();
        if urls.is_empty() {
            return;
        }
        let h = self.arc();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            if mine != h.me_round.load(Ordering::SeqCst) {
                return;
            }
            for u in urls {
                if let Err(e) = h.pair_me(&u) {
                    eprintln!("pair {u}: {e}");
                }
            }
        });
    }

    /// a pair table: a new room opened (`new`: its name), a room linked (`url`: your page or an invite),
    /// or looked up (neither): the room and the bring link
    pub fn pair(&self, key: &str, new: Option<&str>, url: Option<&str>) -> Res<PairInfo> {
        if kind_of(key) != "pair" {
            return Err("not a pair piece".into());
        }
        let have = self.lock().room.get("pair").and_then(|p| p.get(key)).map(|h| text(h.get("url"))).filter(|u| !u.is_empty());
        let (new, url) = (new.map(str::trim).filter(|s| !s.is_empty()), url.map(str::trim).filter(|s| !s.is_empty()));
        let (fresh, link) = if let Some(n) = new {
            (true, Some(self.pair_new(&n.chars().take(80).collect::<String>())?))
        } else if let Some(u) = url {
            (true, Some(self.pair_page(u)?))
        } else {
            (false, have)
        };
        let link = link.ok_or("no room linked yet")?;
        let mut warn = String::new();
        if fresh {
            // the seat is yours: your name and face on it (a new room has them already)
            if let Err(e) = self.pair_me(&link) {
                warn = format!("pair: {e}");
            }
        }
        let mut st = self.pair_state(&link)?;
        {
            let mut g = self.lock();
            room::obj(&mut g.room, "pair").insert(key.into(), json!({"url": st.url, "room": st.room}));
            g.save(&self.cfg);
        }
        self.push_state();
        st.warn = warn;
        Ok(st)
    }
}
