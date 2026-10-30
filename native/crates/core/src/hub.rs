//! The feed: who is where on every machine, and what the room keeps (server.py's Hub). The app calls its
//! methods where the page posted to /api/*, and hears its `Event`s where the page had server-sent events.

use crate::chess::{self, Game};
use crate::config::Config;
use crate::parse;
use crate::room::{self, Room};
use crate::runner::{run, Kept, Mach, Runner};
use crate::scripts;
use crate::term::Shell;
use crate::util::{self, kind_of, split_unit, sq, unit};
use burrow_model::{self as model, Bot, State};
use indexmap::IndexMap;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, Weak};
use std::time::Duration;

pub type Res<T> = Result<T, String>;

/// what the hub pushes: the page's server-sent events
#[derive(Clone, Debug)]
pub enum Event {
    State(Box<State>),
    /// the office manager's notes
    Digest(DigestView),
    /// your terminals: started, ended, busy or quiet
    Shells(Vec<model::ShellView>),
    /// a message between two bots (ids; an office manager is mgr:<her booth>)
    Talk { from: String, to: String },
    /// a bot posting in a pair room ("say": a beam to that room's table) or reading it ("read": from the table)
    PairTalk { id: String, key: String, dir: String },
}

/// the notes, as the page had them: {sessions: {id: notes}, desks: {unit: paragraph}, writing, explaining, on}
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct DigestView {
    pub sessions: Map<String, Value>,
    pub desks: Map<String, Value>,
    pub writing: Option<String>,
    pub explaining: Option<String>,
    pub on: bool,
}

/// the events of one viewer (an open window): its receiver, and its being there (`viewers()`)
pub struct Events {
    pub rx: Receiver<Event>,
    _here: Arc<()>,
}

impl std::ops::Deref for Events {
    type Target = Receiver<Event>;
    fn deref(&self) -> &Receiver<Event> {
        &self.rx
    }
}

/// threading.Event: a poll woken early
#[derive(Default)]
pub struct Poke {
    flag: Mutex<bool>,
    cv: Condvar,
}

impl Poke {
    pub fn set(&self) {
        *self.flag.lock().unwrap() = true;
        self.cv.notify_all();
    }
    /// true if it was set within `d`
    pub fn wait(&self, d: Duration) -> bool {
        let g = self.flag.lock().unwrap();
        let (g, _) = self.cv.wait_timeout_while(g, d, |set| !*set).unwrap();
        *g
    }
    pub fn clear(&self) {
        *self.flag.lock().unwrap() = false;
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Feed {
    pub bots: Vec<Bot>,
    pub light: bool,
    pub error: Option<String>,
}

/// a chess game: you against an idle bot, or two bots, both sides played here
pub(crate) struct GameSt {
    pub id: i64,
    pub white: model::Player,
    pub black: model::Player,
    pub bots: Vec<String>,
    pub bot: Option<String>,
    pub game: Game,
    pub result: Option<String>,
    pub why: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct Awake {
    pub mode: String,
    pub until: i64,
}

pub(crate) struct Inner {
    pub room: Room,
    /// id → machine, in the order of machines()
    pub machines: IndexMap<String, Mach>,
    pub feeds: IndexMap<String, Feed>,
    /// (machine id, cwd) → (desk directory, worktree or "")
    pub folds: HashMap<(String, String), (String, String)>,
    /// unit → since when a desk that came with its sessions has none (`fade`)
    pub absent: HashMap<String, f64>,
    /// (machine id, directory) → your terminal at that screen
    pub shells: IndexMap<(String, String), Arc<Shell>>,
    pub focused: bool,
    /// the account's limits, from Claude Code's own /usage here
    pub usage: Option<model::Usage>,
    /// its pids: `claude -p` shows up in `claude agents` while it runs
    pub own: VecDeque<u32>,
    pub game: Option<GameSt>,
    /// the demo's made-up bots, by machine (stopped and deleted from here)
    pub demo_bots: HashMap<String, Vec<Bot>>,
    pub demo_gone: HashSet<String>,
    pub demo_added: Vec<Mach>,
    pub awake: Awake,
    /// the office manager's notes: {sessions: {id: notes}, desks: {unit: paragraph}}
    pub digest: (Map<String, Value>, Map<String, Value>),
    /// what is being written up now: a session's id, or "u:<unit>" for a desk
    pub writing: Option<String>,
    pub explaining: Option<String>,
    /// id → (busy, waiting, over) at the last poll: what changed
    pub seen: HashMap<String, (bool, bool, String)>,
    /// the office managers' own sessions, as `claude agents` lists them (not bots in the room)
    pub mrows: IndexMap<String, Bot>,
    /// id → when: sessions she asked to remove, walked to the booth by her in the room
    pub removing: IndexMap<String, f64>,
    /// a pair agent's token → its room's name (from its join, `pair_table`)
    pub pair_rooms: HashMap<String, String>,
    /// token → when its join was last looked for in its whole transcript
    pub pair_asked: HashMap<String, f64>,
    /// the ones you took out of the booth again (she is told, and leaves them be)
    pub kept: Vec<String>,
}

pub struct Hub {
    pub cfg: Config,
    pub(crate) run: Arc<dyn Runner>,
    pub(crate) me: Weak<Hub>,
    pub(crate) inner: Mutex<Inner>,
    /// the sleep inhibitor while it runs: every machine's poll ticks it, one at a time
    pub(crate) caff: Mutex<Option<Box<dyn Kept>>>,
    pub(crate) pokes: Mutex<HashMap<String, Arc<Poke>>>,
    subs: Mutex<Vec<(Sender<Event>, Weak<()>)>>,
    /// the sessions waiting to be written up, in turn
    pub(crate) dq: Mutex<Vec<String>>,
    pub(crate) dcv: Condvar,
    pub(crate) me_round: std::sync::atomic::AtomicU64,
    pub(crate) port: std::sync::atomic::AtomicU16,
}

static BACKGROUNDED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*backgrounded · ([^\s·]+)").unwrap());
static NOT_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9]+").unwrap());
static SCRATCH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/\.claude/worktrees/|/scratch-workspaces/|^(/private)?/tmp/|^/var/folders/").unwrap());

const MIN: Duration = Duration::from_secs(60);

impl Inner {
    pub fn bots(&self) -> Vec<Bot> {
        let mut bs: Vec<Bot> = self.feeds.values().flat_map(|f| f.bots.iter().cloned()).collect();
        bs.sort_by_key(|b| b.started);
        bs
    }

    /// the desks: the ones you keep (a desk out of the closet, or kept at a desk's panel), which
    /// stay, then one for every other directory a session is in, in the order its first one started: it
    /// comes with its sessions and goes with the last of them (`fade`). So the room is as full as
    /// `claude agents --all` is: clearing out sessions is what frees it
    pub fn desks(&self) -> Vec<model::Desk> {
        let kept = room::desks(&self.room);
        let names = self.room.get("names");
        let (mut out, mut seen) = (vec![], HashSet::new());
        for u in kept.iter().cloned().chain(self.bots().into_iter().map(|b| b.unit)) {
            let (mid, d) = split_unit(&u);
            if self.machines.contains_key(&mid) && seen.insert(u.clone()) {
                let name = names.and_then(|n| n.get(&u)).and_then(Value::as_str).unwrap_or("").to_string();
                out.push(model::Desk { kept: kept.contains(&u), unit: u, m: mid, dir: d, name });
            }
        }
        out
    }

    pub fn booths(&self) -> Vec<String> {
        room::furniture(&self.room).into_iter().filter(|k| kind_of(k) == "manager").collect()
    }

    /// the one at that booth: room.json `managers[key]` (a booth from before the styles has Linda, the notes)
    pub fn manager(&self, key: &str) -> Map<String, Value> {
        let mut mg = self.room.get("managers").and_then(|m| m.get(key)).and_then(Value::as_object).cloned().unwrap_or_default();
        let style = mg.get("style").and_then(Value::as_str).filter(|s| crate::office::style(s).is_some()).unwrap_or("notes").to_string();
        mg.insert("style".into(), json!(style));
        mg
    }

    pub fn notes_on(&self) -> bool {
        self.booths().iter().any(|k| {
            let m = self.manager(k);
            m["style"] == "notes" && !m.get("vacant").is_some_and(util::truthy)
        })
    }

    pub fn shells_view(&self) -> Vec<model::ShellView> {
        self.shells.iter().filter(|(_, s)| s.alive()).map(|((m, d), s)| model::ShellView { m: m.clone(), dir: d.clone(), busy: s.busy() }).collect()
    }

    pub fn idle_bot(&self, bid: &str) -> Option<Bot> {
        self.feeds.values().flat_map(|f| f.bots.iter()).find(|b| b.id == bid && !b.busy && !b.waiting).cloned()
    }

    pub fn save(&self, cfg: &Config) {
        room::save_room(cfg, &self.room);
    }
}

impl Hub {
    /// the hub as room.json and digest.json have it; nothing runs until `start`
    pub fn new(cfg: Config, run: Arc<dyn Runner>) -> Arc<Hub> {
        let room = room::load_room(&cfg);
        let digest = crate::digest::load_digest(&cfg);
        let mode = room.get("awake").and_then(|a| a.get("mode")).and_then(Value::as_str).unwrap_or("off").to_string();
        Arc::new_cyclic(|me| Hub {
            run,
            me: me.clone(),
            inner: Mutex::new(Inner {
                room,
                machines: IndexMap::new(),
                feeds: IndexMap::new(),
                folds: HashMap::new(),
                absent: HashMap::new(),
                shells: IndexMap::new(),
                focused: true,
                usage: None,
                own: VecDeque::new(),
                game: None,
                demo_bots: HashMap::new(),
                demo_gone: HashSet::new(),
                demo_added: vec![],
                awake: Awake { mode, until: 0 },
                digest,
                writing: None,
                explaining: None,
                seen: HashMap::new(),
                mrows: IndexMap::new(),
                removing: IndexMap::new(),
                pair_rooms: HashMap::new(),
                pair_asked: HashMap::new(),
                kept: vec![],
            }),
            caff: Mutex::new(None),
            pokes: Mutex::new(HashMap::new()),
            subs: Mutex::new(vec![]),
            dq: Mutex::new(vec![]),
            dcv: Condvar::new(),
            me_round: Default::default(),
            port: std::sync::atomic::AtomicU16::new(cfg.port),
            cfg,
        })
    }

    /// the watchers of every machine, the notes' writer, the office managers' manuals
    pub fn start(&self) {
        let me = self.arc();
        let real_mgrs = self.cfg.real_office && self.lock().room.get("managers").and_then(Value::as_object).is_some_and(|ms| ms.values().any(|m| m.get("id").is_some_and(util::truthy)));
        if real_mgrs {
            // this burrow's port, their manuals as they are now (once the poll has seen where they run)
            let h = me.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(12));
                h.write_manuals();
            });
        }
        if self.cfg.demo && self.cfg.real_office {
            let h = me.clone();
            std::thread::spawn(move || h.demo_manager());
        }
        let h = me.clone();
        std::thread::spawn(move || h.digest_worker());
        self.set_machines(self.machines());
    }

    pub(crate) fn arc(&self) -> Arc<Hub> {
        self.me.upgrade().expect("the hub")
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn runner(&self) -> &dyn Runner {
        &*self.run
    }

    /// this computer, where `claude -p` and the office managers' sessions run (in the demo too)
    pub(crate) fn local(&self) -> Mach {
        Mach { id: String::new(), name: self.cfg.hostname.clone(), ..Mach::default() }
    }

    // ---- events

    /// a viewer's events, the state and the notes first
    pub fn subscribe(&self) -> Events {
        let (tx, rx) = channel();
        let here = Arc::new(());
        let _ = tx.send(Event::State(Box::new(self.state())));
        let _ = tx.send(Event::Digest(self.digest_view()));
        self.subs.lock().unwrap().push((tx, Arc::downgrade(&here)));
        Events { rx, _here: here }
    }

    /// how many viewers there are (someone has the room open)
    pub fn viewers(&self) -> usize {
        let mut s = self.subs.lock().unwrap();
        s.retain(|(_, w)| w.strong_count() > 0);
        s.len()
    }

    pub(crate) fn push(&self, e: Event) {
        let mut s = self.subs.lock().unwrap();
        s.retain(|(tx, w)| w.strong_count() > 0 && tx.send(e.clone()).is_ok());
    }

    pub(crate) fn push_state(&self) {
        self.push(Event::State(Box::new(self.state())));
    }

    pub(crate) fn push_shells(&self) {
        let v = self.lock().shells_view();
        self.push(Event::Shells(v));
    }

    // ---- the machines: this one, and Claude Code's own list of those it reaches over ssh

    pub(crate) fn claude_settings(&self) -> Value {
        std::fs::read_to_string(&self.cfg.claude_settings).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(json!({}))
    }

    /// this computer (id ""), then each of `sshConfigs` in ~/.claude/settings.json ({id, name, sshHost,
    /// sshPort?, sshIdentityFile?}, the Desktop app's SSH environments)
    pub fn machines(&self) -> Vec<Mach> {
        if self.cfg.demo {
            let g = self.lock();
            let mut ms = vec![
                Mach { id: "".into(), name: "mac".into(), ssh: "@demo".into(), ..Mach::default() },
                Mach { id: "lab".into(), name: "lab".into(), ssh: "@demo".into(), ..Mach::default() },
            ];
            ms.extend(g.demo_added.iter().cloned());
            return ms.into_iter().filter(|m| !g.demo_gone.contains(&m.id)).collect();
        }
        let mut ms = vec![self.local()];
        for c in self.claude_settings().get("sshConfigs").and_then(Value::as_array).into_iter().flatten() {
            if !c.is_object() || !c.get("id").is_some_and(util::truthy) || !c.get("sshHost").is_some_and(util::truthy) {
                continue;
            }
            let host = util::text(c.get("sshHost"));
            let name = util::text(c.get("name"));
            let opt = |k: &str| c.get(k).filter(|v| util::truthy(v)).map(|v| util::text(Some(v)));
            ms.push(Mach { id: room::id_str(c.get("id")), name: if name.is_empty() { host.clone() } else { name }, ssh: host, port: opt("sshPort"), key: opt("sshIdentityFile") });
        }
        ms
    }

    /// a watcher for each machine, from when it turns up in the settings until it goes
    pub fn set_machines(&self, ms: Vec<Mach>) {
        let mut new = vec![];
        {
            let mut g = self.lock();
            let ids: Vec<&String> = ms.iter().map(|m| &m.id).collect();
            g.machines.retain(|k, _| ids.contains(&k));
            for m in &ms {
                if !g.machines.contains_key(&m.id) {
                    new.push(m.id.clone());
                }
                g.machines.insert(m.id.clone(), m.clone());
            }
        }
        for mid in &new {
            self.start_machine(mid);
        }
        if !new.is_empty() {
            self.push_state();
        }
    }

    pub fn machine(&self, mid: &str) -> Res<Mach> {
        self.lock().machines.get(mid).cloned().ok_or_else(|| format!("no such machine: {mid}"))
    }

    pub(crate) fn poke(&self, mid: &str) {
        if let Some(p) = self.pokes.lock().unwrap().get(mid) {
            p.set();
        }
    }

    fn start_machine(&self, mid: &str) {
        let poke = Arc::new(Poke::default());
        self.pokes.lock().unwrap().insert(mid.to_string(), poke.clone());
        let (h, id) = (self.arc(), mid.to_string());
        if self.cfg.demo {
            std::thread::spawn(move || h.demo(&id, poke));
        } else {
            std::thread::spawn(move || h.watch(&id, poke));
        }
        if mid.is_empty() && !self.cfg.demo {
            let h = self.arc();
            std::thread::spawn(move || h.limits());
        }
    }

    /// the account's usage limits, every five minutes (they move slowly; it costs a second of CPU)
    fn limits(&self) {
        loop {
            if let Ok(m) = self.machine("") {
                let r = self.runner().exec(&m, "cd && exec claude -p /usage --no-session-persistence 2>&1", None, MIN, &mut |pid| self.own(pid));
                if let Some(u) = r.ok().filter(|o| o.code == 0 || !o.stdout.is_empty()).and_then(|o| parse::parse_usage(&String::from_utf8_lossy(&o.stdout))) {
                    self.lock().usage = Some(u);
                    self.push_state();
                }
            }
            std::thread::sleep(Duration::from_secs(300));
        }
    }

    pub(crate) fn own(&self, pid: u32) {
        let mut g = self.lock();
        g.own.push_back(pid);
        while g.own.len() > 4 {
            g.own.pop_front();
        }
    }

    fn watch(&self, mid: &str, poke: Arc<Poke>) {
        let (mut heard, mut first) = (HashSet::new(), true);
        loop {
            if mid.is_empty() {
                // Claude Code's settings say which machines there are
                self.set_machines(self.machines());
            }
            let gone = !self.lock().machines.contains_key(mid);
            let stale = self.pokes.lock().unwrap().get(mid).is_none_or(|p| !Arc::ptr_eq(p, &poke));
            if gone || stale {
                // gone from the settings (or back, with a new watcher)
                if gone {
                    self.lock().feeds.shift_remove(mid);
                    self.push_state();
                }
                return;
            }
            self.poll(mid, &mut heard, &mut first);
            self.push_state();
            let focused = self.lock().focused;
            poke.wait(Duration::from_secs(if focused { 3 } else { 15 }));
            poke.clear();
        }
    }

    /// one round of a machine's poll: its sessions, its theme, the messages between them, their records
    pub fn poll(&self, mid: &str, heard: &mut HashSet<String>, first: &mut bool) {
        if let Err(e) = self.poll_round(mid, heard, first) {
            let mut g = self.lock();
            let f = g.feeds.entry(mid.to_string()).or_default();
            f.error = Some(e.chars().take(300).collect());
        }
    }

    fn poll_round(&self, mid: &str, heard: &mut HashSet<String>, first: &mut bool) -> Res<()> {
        let m = self.machine(mid)?;
        let out = run(
            self.runner(),
            &m,
            &format!(
                "claude agents --json --all; echo '@@burrow@@'; cat ~/.claude/settings.json 2>/dev/null; echo '@@burrow@@'; {}; echo '@@burrow@@'; {}",
                scripts::PEERS,
                scripts::JOBS
            ),
            MIN,
        )?;
        let mut parts: Vec<&str> = out.splitn(4, "@@burrow@@").collect();
        parts.resize(4, "");
        let own: Vec<u32> = self.lock().own.iter().copied().collect();
        let mut bots: Vec<Bot> = parse::parse_agents(parts[0], &parse::parse_jobs(parts[3]))?.into_iter().filter(|b| !mid.is_empty() || !own.contains(&(b.pid as u32))).collect();
        if mid.is_empty() {
            bots = self.take_manager(bots);
        }
        let light = serde_json::from_str::<Value>(parts[1].trim()).ok().and_then(|s| s.get("theme").map(|t| util::text(Some(t)))).is_some_and(|t| t.starts_with("light"));
        let mut all = bots.clone();
        if mid.is_empty() {
            all.extend(self.lock().mrows.values().cloned());
        }
        let talks = parse::parse_peers(parts[2], &all);
        let (pairs, rooms) = parse::parse_pair(parts[2]);
        self.fold(mid, &m, &mut bots)?;
        self.lock().feeds.insert(mid.to_string(), Feed { bots: bots.clone(), light, error: None });
        self.fade(mid, util::now());
        self.chess_check();
        self.awake_tick();
        self.digest_watch(&bots);
        for (msg, a, b) in talks {
            if heard.insert(msg) && !*first {
                let (from, to) = (self.folk_of(&a), self.folk_of(&b));
                self.push(Event::Talk { from, to });
            }
        }
        // a bot talking in a pair room, or reading it: a beam to its table
        self.lock().pair_rooms.extend(rooms);
        for (eid, session, token, dir) in pairs {
            if !heard.insert(eid) || *first {
                continue;
            }
            let Some(b) = all.iter().find(|b| b.session == session) else { continue };
            if let Some(key) = self.pair_table(&m, &session, &token) {
                self.push(Event::PairTalk { id: self.folk_of(&b.id), key, dir: dir.into() });
            }
        }
        *first = false;
        // what she asked to remove and is gone (or was not taken in ten minutes)
        let mut g = self.lock();
        let ids: HashSet<String> = g.feeds.values().flat_map(|f| f.bots.iter().map(|b| b.id.clone())).collect();
        let now = util::now();
        g.removing.retain(|i, t| ids.contains(i) && now - *t <= 600.0);
        Ok(())
    }

    /// the pair table a bot's pair token belongs to: the one linked to its room, by the name pair gave in answer to
    /// its join (looked for once in its whole transcript when the last minutes did not have it); with the room
    /// unknown, the only table if there is one
    pub(crate) fn pair_table(&self, m: &Mach, session: &str, token: &str) -> Option<String> {
        static HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-fA-F-]+$").unwrap());
        let ask = {
            let mut g = self.lock();
            if g.room.get("pair").and_then(Value::as_object).is_none_or(|t| t.is_empty()) {
                return None;
            }
            let ask = !g.pair_rooms.contains_key(token) && util::now() - g.pair_asked.get(token).copied().unwrap_or(0.0) > 600.0 && HEX.is_match(session);
            if ask {
                g.pair_asked.insert(token.to_string(), util::now());
            }
            ask
        };
        if ask && let Ok(out) = run(self.runner(), m, &scripts::pair_join(session), MIN) {
            let rooms = parse::parse_pair(&out).1;
            self.lock().pair_rooms.extend(rooms);
        }
        let g = self.lock();
        let tables = g.room.get("pair").and_then(Value::as_object)?;
        match g.pair_rooms.get(token) {
            None => (tables.len() == 1).then(|| tables.keys().next().cloned()).flatten(),
            Some(name) => tables.iter().find(|(_, v)| v.get("room").and_then(Value::as_str) == Some(name.as_str())).map(|(k, _)| k.clone()),
        }
    }

    /// each bot's desk: the directory it started in, or its repo's for a worktree
    pub(crate) fn fold(&self, mid: &str, m: &Mach, bots: &mut [Bot]) -> Res<()> {
        let new: Vec<String> = {
            let g = self.lock();
            let mut n: Vec<String> = bots.iter().filter(|b| !b.cwd.is_empty() && !g.folds.contains_key(&(mid.to_string(), b.cwd.clone()))).map(|b| b.cwd.clone()).collect();
            n.sort();
            n.dedup();
            n
        };
        if !new.is_empty() {
            let lines: Vec<String> = if m.demo() { vec![] } else { run(self.runner(), m, &scripts::fold(&new), Duration::from_secs(20))?.split('\n').map(String::from).collect() };
            let mut g = self.lock();
            for (i, d) in new.iter().enumerate() {
                let line = lines.get(i).map(String::as_str).unwrap_or("");
                g.folds.insert((mid.to_string(), d.clone()), parse::fold(d, line));
            }
        }
        let g = self.lock();
        for b in bots.iter_mut() {
            let (d, tree) = g.folds.get(&(mid.to_string(), b.cwd.clone())).cloned().unwrap_or((b.cwd.clone(), String::new()));
            b.m = mid.to_string();
            b.unit = unit(mid, &d);
            b.tree = tree;
        }
        Ok(())
    }

    /// a desk that came with its sessions has gone with the last of them: two minutes on (a poll that
    /// misses them once doesn't count) its place is forgotten too, so it comes back at a free one
    pub fn fade(&self, mid: &str, now: f64) {
        let mut g = self.lock();
        let here: HashSet<String> = g.feeds.get(mid).map(|f| f.bots.iter().map(|b| b.unit.clone()).collect()).unwrap_or_default();
        let kept: HashSet<String> = room::desks(&g.room).into_iter().collect();
        let keys: Vec<String> = g.room.get("layout").and_then(Value::as_object).map(|l| l.keys().filter(|k| k.starts_with("u:") && split_unit(&k[2..]).0 == mid).cloned().collect()).unwrap_or_default();
        let mut gone = false;
        for k in keys {
            let u = k[2..].to_string();
            if kept.contains(&u) || here.contains(&u) {
                g.absent.remove(&u);
            } else if now - *g.absent.entry(u.clone()).or_insert(now) > 120.0 {
                room::layout(&mut g.room).shift_remove(&k);
                g.absent.remove(&u);
                gone = true;
            }
        }
        if gone {
            g.save(&self.cfg);
        }
    }

    pub fn bots(&self) -> Vec<Bot> {
        self.lock().bots()
    }

    pub fn desks(&self) -> Vec<model::Desk> {
        self.lock().desks()
    }

    // ---- the state the room is drawn from

    pub fn state(&self) -> State {
        let on = self.caff_on();
        let g = self.lock();
        let r = &g.room;
        let layout = r
            .get("layout")
            .and_then(Value::as_object)
            .map(|l| {
                l.iter()
                    .filter_map(|(k, v)| {
                        let a = v.as_array()?;
                        Some((k.clone(), [a.first()?.as_f64()? as i32, a.get(1)?.as_f64()? as i32]))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let pair = r
            .get("pair")
            .and_then(Value::as_object)
            .map(|p| p.iter().map(|(k, v)| (k.clone(), model::PairLink { url: util::text(v.get("url")), room: util::text(v.get("room")) })).collect())
            .unwrap_or_default();
        let paint = r.get("paint").and_then(Value::as_object).map(|p| p.iter().filter_map(|(k, v)| Some((k.clone(), v.as_f64()? as i32))).collect()).unwrap_or_default();
        let (name, seed) = room::you_of(r, &self.cfg.user);
        let color = r.get("you").and_then(|y| y.get("color")).and_then(Value::as_str).map(String::from);
        State {
            machines: g
                .machines
                .iter()
                .map(|(mid, m)| {
                    let f = g.feeds.get(mid);
                    model::Machine { id: mid.clone(), name: m.name.clone(), ssh: m.ssh.clone(), light: f.is_some_and(|f| f.light), error: f.and_then(|f| f.error.clone()) }
                })
                .collect(),
            desks: g.desks(),
            bots: g.bots(),
            layout,
            furniture: room::furniture(r),
            pair,
            paint,
            usage: g.usage.clone().unwrap_or_default(),
            chess: self.chess_view(&g),
            demo: self.cfg.demo,
            awake: model::Awake { mode: g.awake.mode.clone(), until: g.awake.until, on },
            width: r.get("width").and_then(Value::as_f64),
            shells: g.shells_view(),
            settings: room::options(r),
            managers: self.managers_view(&g),
            removing: g.removing.keys().cloned().collect(),
            you: model::You { name, seed, color, login: self.cfg.user.clone() },
            pair_at: self.cfg.pair.clone(),
        }
    }

    // ---- what the page asked for: a method each

    /// the window has the focus (polls every 3 s) or not (every 15 s)
    pub fn focus(&self, on: bool) {
        self.lock().focused = on;
        if on {
            for p in self.pokes.lock().unwrap().values() {
                p.set();
            }
        }
    }

    /// a background session in `d` there: its id. A free pair-style name if none is given; Remote Control
    /// and yolo as the switchboard says
    pub fn spawn_in(&self, m: &Mach, d: &str, name: &str, prompt: &str, extra: &str) -> Res<String> {
        let (name, o) = {
            let g = self.lock();
            let name = if name.is_empty() {
                let taken: HashSet<&String> = g.feeds.get(&m.id).map(|f| f.bots.iter().map(|b| &b.name).collect()).unwrap_or_default();
                let free: Vec<String> = scripts::BOT_NAMES.iter().filter(|n| !taken.contains(n)).cloned().collect();
                util::choice(if free.is_empty() { &scripts::BOT_NAMES } else { &free })
            } else {
                name.to_string()
            };
            (name, room::options(&g.room))
        };
        // in the app as well; yolo: never asks
        let flags = format!("{}{}{}", if o.rc { "--remote-control " } else { "" }, if o.yolo { "--dangerously-skip-permissions " } else { "" }, extra);
        let mut prompt = prompt.trim().to_string();
        if prompt.starts_with('-') {
            prompt = format!("Task: {prompt}");
        }
        let p = if prompt.is_empty() { String::new() } else { format!(" {}", sq(&prompt)) };
        let out = run(self.runner(), m, &format!("cd {} && claude --bg {flags}-n {}{p} 2>&1", sq(d), sq(&name)), MIN)?;
        // "backgrounded · 7ebbc505 · name (idle — …)" or, unnamed, "backgrounded · 7ebbc505 (idle — …)",
        // maybe after "Starting background service…"
        let id = BACKGROUNDED.captures(&out).map(|c| c[1].to_string()).ok_or_else(|| out.trim().to_string())?;
        self.poke(&m.id);
        Ok(id)
    }

    /// a new bot at a desk (/api/spawn): its id
    pub fn spawn(&self, mid: &str, dir: &str, name: &str) -> Res<String> {
        let m = self.machine(mid)?;
        if m.demo() {
            return Err("demo bots can't be started".into());
        }
        self.spawn_in(&m, dir, name.trim(), "", "")
    }

    /// stop a session: it stays listed, completed; with rm it is deleted from the list as well (agent view's
    /// second Ctrl+X, `claude rm`)
    pub fn stop(&self, mid: &str, id: &str, rm: bool) -> Res<()> {
        let m = self.machine(mid)?;
        if m.demo() {
            self.demo_stop(mid, id, rm);
        } else if rm {
            run(self.runner(), &m, &format!("claude stop {} >/dev/null 2>&1; claude rm {}", sq(id), sq(id)), MIN)?;
        } else {
            run(self.runner(), &m, &format!("claude stop {}", sq(id)), MIN)?;
        }
        self.poke(mid);
        Ok(())
    }

    /// the neuralyzer: /clear typed into the session, in the background
    pub fn clear(&self, mid: &str, id: &str) -> Res<()> {
        let m = self.machine(mid)?;
        if !m.demo() {
            let (h, id) = (self.arc(), id.to_string());
            std::thread::spawn(move || crate::term::clear_session(h.runner(), &m, &id));
        }
        Ok(())
    }

    /// the hosts of ~/.ssh/config, for the new door's field
    pub fn hosts(&self) -> Vec<String> {
        let doors: Vec<String> = self.claude_settings().get("sshConfigs").and_then(Value::as_array).into_iter().flatten().map(|c| util::text(c.get("sshHost"))).collect();
        parse::ssh_hosts(&self.cfg.ssh_config, &self.cfg.home, &doors)
    }

    /// directories there that start with what was typed so far
    pub fn complete(&self, mid: &str, dir: &str) -> Res<Vec<String>> {
        let m = self.machine(mid)?;
        let d = if dir.trim().is_empty() { "~/".to_string() } else { dir.trim().to_string() };
        if m.demo() {
            let (base, part) = d.rsplit_once('/').unwrap_or(("", &d));
            return Ok(["alpha", "beta", "gamma"].iter().filter(|x| x.starts_with(part)).map(|x| format!("{base}/{x}/")).collect());
        }
        // (the new desk's folder list: all of a folder's folders)
        let out = run(self.runner(), &m, &format!("ls -d {}*/ 2>/dev/null | head -300", sq(&d)), Duration::from_secs(15))?;
        Ok(out.lines().filter(|x| !x.is_empty()).map(String::from).collect())
    }

    /// the working directories there without a desk (a new desk's form): (its home, the directories)
    pub fn recent(&self, mid: &str) -> Res<(String, Vec<String>)> {
        let m = self.machine(mid)?;
        if m.demo() {
            let h = self.cfg.home_str();
            return Ok((h.clone(), self.off_floor(mid, &["alpha", "beta", "gamma"].map(|x| format!("{h}/projects/{x}")))));
        }
        let out = run(self.runner(), &m, scripts::RECENT, Duration::from_secs(20))?;
        let lines: Vec<String> = out.lines().map(String::from).collect();
        let dirs: Vec<String> = lines.iter().skip(1).filter(|x| x.starts_with('/')).cloned().collect();
        Ok((lines.first().cloned().unwrap_or_default(), self.off_floor(mid, &dirs)))
    }

    /// where Claude Code ran lately there that has no desk now, for a new desk's form
    fn off_floor(&self, mid: &str, recent: &[String]) -> Vec<String> {
        let desks: HashSet<String> = self.desks().into_iter().map(|d| d.unit).collect();
        let mut seen = HashSet::new();
        // not worktrees, the Desktop app's scratch chats, temporary dirs
        recent.iter().filter(|d| seen.insert(d.to_string()) && !desks.contains(&unit(mid, d)) && !SCRATCH.is_match(d)).cloned().collect()
    }

    /// a desk for a working directory there, kept: its unit
    pub fn dir(&self, mid: &str, dir: &str) -> Res<String> {
        self.dir_at(mid, dir, None)
    }

    /// the same, a desk out of the closet: it stands where it was let go (its place kept before the state that brings
    /// it goes out, or the room finds it a place of its own)
    pub fn dir_at(&self, mid: &str, dir: &str, at: Option<[i64; 2]>) -> Res<String> {
        let m = self.machine(mid)?;
        let d = dir.trim();
        let d = if m.demo() { d.replacen('~', &self.cfg.home_str(), 1) } else { run(self.runner(), &m, &format!("cd {} && pwd -P", sq(d)), Duration::from_secs(20))?.trim().to_string() };
        let u = unit(mid, &d);
        if let Some([x, y]) = at {
            room::layout(&mut self.lock().room).insert(format!("u:{u}"), json!([x, y]));
        }
        self.keep(&u, true);
        Ok(u)
    }

    /// a desk kept (by you), or taken away (burnt, its sessions gone first): gone with its place and name
    pub fn keep(&self, u: &str, on: bool) {
        {
            let mut g = self.lock();
            let mut ds: Vec<String> = room::desks(&g.room).into_iter().filter(|x| x != u).collect();
            if on {
                ds.push(u.to_string());
            }
            g.room.insert("desks".into(), json!(ds));
            if !on {
                room::layout(&mut g.room).shift_remove(&format!("u:{u}"));
                if let Some(n) = g.room.get_mut("names").and_then(Value::as_object_mut) {
                    n.shift_remove(u);
                }
            }
            g.save(&self.cfg);
        }
        self.push_state();
    }

    /// a desk's own name (its panel); empty: its folder's again
    pub fn rename(&self, u: &str, name: &str) -> Res<()> {
        {
            let mut g = self.lock();
            if !g.desks().iter().any(|d| d.unit == u) {
                return Err("no such desk".into());
            }
            let names = room::obj(&mut g.room, "names");
            if name.is_empty() {
                names.shift_remove(u);
            } else {
                names.insert(u.to_string(), json!(name.chars().take(40).collect::<String>()));
            }
            g.save(&self.cfg);
        }
        self.push_state();
        Ok(())
    }

    /// a desk's panel: renamed ({unit, name}, empty for its folder's name), or kept
    pub fn desk(&self, u: &str, name: Option<&str>, keep: bool) -> Res<()> {
        if keep {
            if !self.desks().iter().any(|d| d.unit == u) {
                return Err("no such desk".into());
            }
            self.keep(u, true);
            return Ok(());
        }
        self.rename(u, name.unwrap_or("").trim())
    }

    /// a desk burnt (its bots went first, through the booth or your gun)
    pub fn forget(&self, u: &str) {
        self.kill_shells(u);
        self.keep(u, false);
    }

    /// another machine, over ssh: an entry in Claude Code's sshConfigs; its door where it was let go
    pub fn add_machine(&self, ssh: &str, name: &str, at: Option<[i64; 2]>) -> Res<String> {
        let (ssh, name) = (ssh.trim(), name.trim());
        if ssh.is_empty() {
            return Err("where to ssh to".into());
        }
        let mid = if self.cfg.demo {
            // a made-up one (it never writes the settings)
            let mid = id_of(ssh);
            let mut g = self.lock();
            g.demo_gone.remove(&mid);
            if g.demo_added.iter().all(|m| m.id != mid) {
                g.demo_added.push(Mach { id: mid.clone(), name: if name.is_empty() { ssh } else { name }.to_string(), ssh: "@demo".into(), ..Mach::default() });
            }
            mid
        } else {
            run(self.runner(), &Mach { ssh: ssh.to_string(), ..Mach::default() }, "true", Duration::from_secs(20))?;
            add_ssh_config(&self.cfg.claude_settings, ssh, name)?
        };
        if let Some([x, y]) = at {
            let mut g = self.lock();
            room::layout(&mut g.room).insert(format!("door:{mid}"), json!([x, y]));
            g.save(&self.cfg);
        }
        self.set_machines(self.machines());
        Ok(mid)
    }

    /// a door taken out (the doors' panel): out of sshConfigs, and out of the room
    pub fn unmachine(&self, mid: &str) -> Res<()> {
        if mid.is_empty() {
            return Err("this computer's door stays".into());
        }
        if self.cfg.demo {
            self.lock().demo_gone.insert(mid.to_string());
        } else {
            remove_ssh_config(&self.cfg.claude_settings, mid)?;
        }
        let poke = self.pokes.lock().unwrap().get(mid).cloned();
        self.set_machines(self.machines());
        if let Some(p) = poke {
            p.set(); // its watcher sees it gone, drops its bots and stops
        }
        let shells: Vec<Arc<Shell>> = self.lock().shells.iter().filter(|(k, _)| k.0 == mid).map(|(_, s)| s.clone()).collect();
        for s in shells {
            s.kill(); // your terminals there
        }
        {
            // its door's place and paint, its desks and theirs
            let mut g = self.lock();
            let door = format!("door:{mid}");
            let pre = format!("u:{mid}:");
            room::layout(&mut g.room).retain(|k, _| *k != door && !k.starts_with(&pre));
            if let Some(p) = g.room.get_mut("paint").and_then(Value::as_object_mut) {
                p.shift_remove(mid);
            }
            let pre = format!("{mid}:");
            let ds: Vec<String> = room::desks(&g.room).into_iter().filter(|u| !u.starts_with(&pre)).collect();
            g.room.insert("desks".into(), json!(ds));
            g.save(&self.cfg);
        }
        self.push_state();
        Ok(())
    }

    /// where the furniture stands, moved by hand: {key: [x, y]}
    pub fn set_layout(&self, layout: &[(String, [f64; 2])]) {
        {
            let mut g = self.lock();
            let l: Map<String, Value> = layout.iter().map(|(k, [x, y])| (k.clone(), json!([*x as i64, *y as i64]))).collect();
            g.room.insert("layout".into(), Value::Object(l));
            g.save(&self.cfg);
        }
        self.push_state();
    }

    /// a piece out of the closet, put down at `at`: its key
    pub fn add_furniture(&self, kind: &str, at: [f64; 2]) -> Res<String> {
        if !room::KINDS.contains(&kind) {
            return Err("no such furniture".into());
        }
        if kind == "manager" {
            return Err("a reception comes with an office manager: pick one".into());
        }
        let key = {
            let mut g = self.lock();
            let mut fs = room::furniture(&g.room);
            let key = (0..1000).map(|i| format!("{kind}{i}")).find(|k| !fs.contains(k)).ok_or("no room")?;
            fs.push(key.clone());
            room::set_furniture(&mut g.room, fs);
            room::layout(&mut g.room).insert(key.clone(), json!([at[0] as i64, at[1] as i64]));
            if kind == "radio" {
                // a radio put down: the sound is on
                room::obj(&mut g.room, "settings").insert("sound".into(), json!(true));
            }
            if kind == "files" {
                // a filing cabinet put down: FILES on the bands
                room::obj(&mut g.room, "settings").insert("files".into(), json!(true));
            }
            g.save(&self.cfg);
            key
        };
        self.push_state();
        Ok(key)
    }

    /// a piece burnt in the fireplace
    pub fn remove_furniture(&self, key: &str) -> Res<()> {
        {
            let mut g = self.lock();
            let mut fs = room::furniture(&g.room);
            if room::FIXED.contains(&kind_of(key)) || !fs.iter().any(|k| k == key) {
                return Err("that stays".into());
            }
            fs.retain(|k| k != key);
            room::set_furniture(&mut g.room, fs);
            room::layout(&mut g.room).shift_remove(key);
            if let Some(p) = g.room.get_mut("pair").and_then(Value::as_object_mut) {
                p.shift_remove(key);
            }
            g.save(&self.cfg);
        }
        if kind_of(key) == "manager" {
            self.fire(key);
        }
        self.push_state();
        Ok(())
    }

    /// the room's width in dots (the divider, dragged)
    pub fn set_width(&self, w: f64) {
        let mut g = self.lock();
        let w = if w == 0.0 { 256 } else { w as i64 };
        g.room.insert("width".into(), json!(w.clamp(200, 1200)));
        g.save(&self.cfg);
    }

    /// a machine's door (and its desks) in another paint: -1 the wood, None its own again
    pub fn paint(&self, mid: &str, paint: Option<i64>) {
        {
            let mut g = self.lock();
            let p = room::obj(&mut g.room, "paint");
            match paint {
                None => {
                    p.shift_remove(mid);
                }
                Some(n) => {
                    p.insert(mid.to_string(), json!(n));
                }
            }
            g.save(&self.cfg);
        }
        self.push_state();
    }

    /// the switchboard: a switch set, for the options there are
    pub fn settings(&self, changes: &[(&str, bool)]) {
        {
            let mut g = self.lock();
            let st = room::obj(&mut g.room, "settings");
            for (k, on) in changes {
                if room::OPTIONS.iter().any(|(n, _)| n == k) {
                    st.insert(k.to_string(), json!(on));
                }
            }
            g.save(&self.cfg);
        }
        self.push_state();
        self.push_digest();
    }

    /// who you are: your name (empty: the login name), the face's seed, your colour; None leaves one as it
    /// is, an empty one takes it away. Every linked pair room hears, a moment later
    pub fn you(&self, name: Option<&str>, seed: Option<&str>, color: Option<&str>) -> (String, String) {
        static WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
        let r = {
            let mut g = self.lock();
            let you = room::obj(&mut g.room, "you");
            for (k, v) in [("name", name), ("seed", seed), ("color", color)] {
                let Some(v) = v else { continue };
                let mut v: String = WS.replace_all(v, " ").trim().chars().take(40).collect();
                if k == "seed" {
                    v = util::pair_seed(&v);
                }
                if v.is_empty() {
                    you.shift_remove(k);
                } else {
                    you.insert(k.into(), json!(v));
                }
            }
            g.save(&self.cfg);
            g.room.clone()
        };
        self.push_state();
        self.pair_me_all(&r, Duration::from_secs(1));
        room::you_of(&r, &self.cfg.user)
    }

    // ---- the clock keeps this computer awake, as Amphetamine does: when local bots work (the sessions
    // this computer runs; a remote machine's go on without it), or for some time, or not at all. The
    // "while bots work" choice is kept in room.json; a duration is not (a restart sleeps as usual)

    fn caff_on(&self) -> bool {
        self.caff.lock().unwrap().as_mut().is_some_and(|c| c.running())
    }

    pub fn awake_view(&self) -> model::Awake {
        let on = self.caff_on();
        let g = self.lock();
        model::Awake { mode: g.awake.mode.clone(), until: g.awake.until, on }
    }

    /// the clock: off, while bots work ("work"), or for `minutes` ("for")
    pub fn awake(&self, mode: &str, minutes: i64) -> Res<model::Awake> {
        if !["off", "work", "for"].contains(&mode) || (mode == "for" && minutes <= 0) {
            return Err("awake how?".into());
        }
        if mode != "off" && self.cfg.inhibitor.is_none() {
            return Err("no caffeinate or systemd-inhibit on this computer".into());
        }
        {
            let mut g = self.lock();
            g.awake = Awake { mode: mode.into(), until: if mode == "for" { util::now_ms() + minutes * 60000 } else { 0 } };
            g.room.insert("awake".into(), if mode == "work" { json!({"mode": "work"}) } else { json!({}) });
            g.save(&self.cfg);
        }
        self.awake_tick();
        self.push_state();
        Ok(self.awake_view())
    }

    /// the inhibitor runs exactly when the mode and the room say so (called on every poll, by every machine's
    /// thread: one at a time, or two would start one each and the one not kept would keep the Mac awake for good)
    pub fn awake_tick(&self) {
        let mut caff = self.caff.lock().unwrap();
        let want = {
            let mut g = self.lock();
            if g.awake.mode == "for" && util::now_ms() > g.awake.until {
                g.awake = Awake { mode: "off".into(), until: 0 };
            }
            // local bots: the ones this computer runs
            g.awake.mode == "for" || (g.awake.mode == "work" && g.feeds.get("").is_some_and(|f| f.bots.iter().any(|b| b.busy)))
        };
        let on = caff.as_mut().is_some_and(|c| c.running());
        if want && !on {
            if let Some(cmd) = &self.cfg.inhibitor {
                *caff = self.runner().keep(cmd).ok();
            }
        } else if on && !want
            && let Some(mut c) = caff.take() {
                c.stop();
            }
    }

    // ---- chess against a bot: pair's table (chess.rs has the rules), your opponent one of the idle bots.
    // It plays from here, not through Claude: two plies of material and a little chance. A bot that gets
    // work, or a question for you, gets up and leaves the game: you win.

    pub(crate) fn chess_view(&self, g: &Inner) -> Option<model::Chess> {
        let gs = g.game.as_ref()?;
        let game = &gs.game;
        let to_move = if game.pos.white { &gs.white } else { &gs.black }.id.clone();
        let mut v = model::Chess {
            id: gs.id,
            white: gs.white.clone(),
            black: gs.black.clone(),
            board: game.board(),
            sans: game.sans.clone(),
            last: game.moves.last().cloned(),
            check: game.pos.in_check(),
            result: gs.result.clone(),
            why: gs.why.clone(),
            free: gs.result.is_some(),
            legal: vec![],
            to_move,
        };
        if gs.result.is_none() && v.to_move == "you" {
            v.legal = game.legal.iter().map(|&m| chess::uci(m)).collect();
        }
        Some(v)
    }

    /// a player that is no longer idle (work, a question) has left the table: the other wins
    pub(crate) fn chess_check(&self) {
        let mut g = self.lock();
        let Some(gs) = g.game.as_ref().filter(|gs| gs.result.is_none()) else { return };
        let players = if gs.bots.is_empty() { gs.bot.iter().cloned().collect() } else { gs.bots.clone() };
        let white = gs.white.id.clone();
        if let Some(gone) = players.into_iter().find(|x| g.idle_bot(x).is_none()) {
            chess_end(g.game.as_mut().unwrap(), if white == gone { "0-1" } else { "1-0" }, "left");
        }
    }

    pub fn chess(&self, a: ChessAction) -> Res<()> {
        {
            let mut g = self.lock();
            let next = g.game.as_ref().map(|x| x.id + 1).unwrap_or(1);
            let going = g.game.as_ref().is_some_and(|x| x.result.is_none());
            let agent = |b: &Bot| model::Player { id: b.id.clone(), name: b.name.clone(), kind: "agent".into() };
            match a {
                ChessAction::Bots(x, y) => {
                    // two bots sat down at the board: a game between them, both sides played here
                    if going {
                        return Err("a game is going on".into());
                    }
                    let (Some(a), Some(c)) = (g.idle_bot(&x), g.idle_bot(&y)) else { return Err("both have to be idle to play".into()) };
                    g.game = Some(GameSt { id: next, white: agent(&a), black: agent(&c), bots: vec![a.id.clone(), c.id.clone()], bot: None, game: Game::new(), result: None, why: None });
                    drop(g);
                    self.chess_later(next, true);
                }
                ChessAction::Challenge(id) => {
                    let Some(bot) = g.idle_bot(&id) else { return Err("that bot is busy: only an idle one can play".into()) };
                    let (you, it) = (model::Player { id: "you".into(), name: "you".into(), kind: "human".into() }, agent(&bot));
                    let it_white = util::random() >= 0.5;
                    let (white, black) = if it_white { (it, you) } else { (you, it) };
                    g.game = Some(GameSt { id: next, white, black, bots: vec![], bot: Some(bot.id.clone()), game: Game::new(), result: None, why: None });
                    drop(g);
                    if it_white {
                        self.chess_later(next, false);
                    }
                }
                a => {
                    let Some(gs) = g.game.as_mut().filter(|x| x.result.is_none()) else { return Err("there is no game going on".into()) };
                    match a {
                        ChessAction::Leave(id) => {
                            // a bot leaves the bots' game (its break is over): the other wins
                            if gs.bots.contains(&id) {
                                let r = if gs.white.id == id { "0-1" } else { "1-0" };
                                chess_end(gs, r, "left");
                            }
                        }
                        ChessAction::Resign => {
                            let r = if gs.white.id == "you" { "0-1" } else { "1-0" };
                            chess_end(gs, r, "resignation");
                        }
                        ChessAction::Draw => {
                            // it takes a draw when it is behind on material
                            let it_white = gs.white.id != "you";
                            let it = if it_white { &gs.white } else { &gs.black };
                            if chess::material(&gs.game.pos, it_white) >= 0 {
                                return Err(format!("{} plays on", it.name));
                            }
                            chess_end(gs, "1/2-1/2", "agreement");
                        }
                        ChessAction::Move(t) => {
                            if if gs.game.pos.white { &gs.white } else { &gs.black }.id != "you" {
                                return Err("not your move".into());
                            }
                            let m = gs.game.find(&t).ok_or("not a legal move")?;
                            gs.game.push(m);
                            if let Some((r, why)) = gs.game.over() {
                                chess_end(gs, r, why);
                            } else {
                                let id = gs.id;
                                drop(g);
                                self.chess_later(id, false);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        self.push_state();
        Ok(())
    }

    fn chess_later(&self, id: i64, bots: bool) {
        let h = self.arc();
        let wait = if bots { util::uniform(2.0, 4.5) } else { util::uniform(1.2, 3.5) };
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs_f64(wait));
            {
                let mut g = h.lock();
                let Some(gs) = g.game.as_mut().filter(|x| x.id == id && x.result.is_none()) else { return };
                let m = chess::bot_move(&gs.game, &mut util::random);
                gs.game.push(m);
                if let Some((r, why)) = gs.game.over() {
                    chess_end(gs, r, why);
                } else if !gs.bots.is_empty() {
                    // the bots' own game: the other side moves next
                    drop(g);
                    h.chess_later(id, true);
                }
            }
            h.push_state();
        });
    }
}

fn chess_end(gs: &mut GameSt, result: &str, why: &str) {
    gs.result = Some(result.into());
    gs.why = Some(why.into());
}

/// what the chess table is asked: two bots sit down, you challenge one, a bot leaves, you resign, offer a
/// draw or move (SAN or coordinates)
#[derive(Clone, Debug)]
pub enum ChessAction {
    Bots(String, String),
    Challenge(String),
    Leave(String),
    Resign,
    Draw,
    Move(String),
}

/// a machine's id from its name or host
fn id_of(s: &str) -> String {
    let b = NOT_ID.replace_all(&s.to_lowercase(), "-").trim_matches('-').to_string();
    if b.is_empty() { "machine".into() } else { b }
}

/// Claude Code's settings written back in place (it may be a link into someone's dotfiles), as Python's
/// json.dumps(indent=2, ensure_ascii=False) writes them: his settings.json round-trips byte for byte
fn write_settings(path: &std::path::Path, v: &Value) -> Res<()> {
    let s = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    std::fs::write(path, s + "\n").map_err(|e| e.to_string())
}

/// a new entry in Claude Code's sshConfigs, the rest of its settings as they were: its id
pub fn add_ssh_config(path: &std::path::Path, ssh: &str, name: &str) -> Res<String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}".into(),
        Err(e) => return Err(e.to_string()),
    };
    let mut v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?; // one that doesn't parse is left alone
    let o = v.as_object_mut().ok_or("settings.json is not an object")?;
    let cs = o.entry("sshConfigs").or_insert(json!([])).as_array_mut().ok_or("sshConfigs is not a list")?;
    if cs.iter().any(|c| c.get("sshHost").and_then(Value::as_str) == Some(ssh)) {
        return Err(format!("{ssh} is there already"));
    }
    let base = id_of(if name.is_empty() { ssh } else { name });
    let (mut mid, mut k) = (base.clone(), 2);
    while cs.iter().any(|c| room::id_str(c.get("id")) == mid) {
        mid = format!("{base}-{k}");
        k += 1;
    }
    cs.push(json!({"id": mid, "name": if name.is_empty() { ssh } else { name }, "sshHost": ssh}));
    write_settings(path, &v)?;
    Ok(mid)
}

/// a machine's entry out of Claude Code's sshConfigs, the rest of its settings as they were (nothing on
/// the machine itself changes: its sessions go on there)
pub fn remove_ssh_config(path: &std::path::Path, mid: &str) -> Res<()> {
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let cs = v.get("sshConfigs").and_then(Value::as_array).cloned().unwrap_or_default();
    let left: Vec<Value> = cs.iter().filter(|c| !(c.is_object() && room::id_str(c.get("id")) == mid)).cloned().collect();
    if left.len() == cs.len() {
        return Err(format!("no such machine: {mid}"));
    }
    v.as_object_mut().ok_or("settings.json is not an object")?.insert("sshConfigs".into(), Value::Array(left));
    write_settings(path, &v)
}
