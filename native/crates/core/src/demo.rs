//! BURROW_DEMO: made-up bots on made-up machines, coming and going, for working on the room (and for the
//! app's tests). Nothing of Claude Code is called.

use crate::hub::{Event, Feed, Hub, Poke};
use crate::util::{self, random};
use burrow_model::{Bot, Limit, Usage};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

impl Hub {
    /// a demo machine's bots, as `claude agents` would list them
    pub(crate) fn demo_bots_for(&self, mid: &str) -> Vec<Bot> {
        let i = self.lock().machines.get_index_of(mid).unwrap_or(0);
        let home = self.cfg.home_str();
        let mut list: Vec<(&str, &str)> = vec![
            ("burrow", "burrow"),
            ("burrow", "talk-ui"),
            ("burrow/.claude/worktrees/icons", "icons"),
            ("kite", "kite"),
            ("kite/.claude/worktrees/tails", "tails"),
            ("teapot", "teapot"),
            ("", "home"),
            ("lantern", "lantern"),
        ];
        if i == 1 {
            list = vec![("puffin", "puffin"), ("puffin/.claude/worktrees/eval", "eval"), ("", "home")];
        }
        let now = util::now_ms();
        let mut bots: Vec<Bot> = list
            .iter()
            .enumerate()
            .map(|(k, (d, n))| {
                let k = k as i64;
                Bot {
                    id: format!("demo{i}{k:03}"),
                    name: n.to_string(),
                    cwd: if d.is_empty() { home.clone() } else { format!("{home}/projects/{d}") },
                    busy: k % 3 == 0,
                    waiting: k % 5 == 1,
                    alive: true,
                    state: (if k % 3 == 0 { "working" } else if k % 5 == 1 { "blocked" } else { "done" }).into(),
                    started: now - 3600000 * (k + 1),
                    pid: 1000 + k,
                    session: format!("demo{k}"),
                    rc: if k % 2 == 0 { "cse_demo".into() } else { String::new() },
                    ..Bot::default()
                }
            })
            .collect();
        // a /loop between runs: its process is off
        let last = bots.last_mut().unwrap();
        last.alive = false;
        last.busy = true;
        last.state = "working".into();
        last.pid = 0;
        // and some that are over: done, failed, stopped, as agent view's Completed group
        let mut over = vec![
            ("burrow", "sunny-sam", "done", "the booth built and tested"),
            ("kite", "zany-zoe", "failed", "the string snapped in the wind test"),
            ("teapot", "posh-pam", "stopped", ""),
            ("burrow", "merry-max", "done", "README brought up to date"),
            ("lantern", "jolly-jo", "done", "the docs rebuilt"),
        ];
        if i == 1 {
            over = vec![("puffin", "nimble-ned", "done", "run 12 scored, the report in out/"), ("puffin", "witty-wes", "failed", "CUDA out of memory on the big batch")];
        }
        bots.extend(over.iter().enumerate().map(|(k, (d, n, st, det))| {
            let k = k as i64;
            Bot {
                id: format!("demo{i}over{k}"),
                name: n.to_string(),
                cwd: format!("{home}/projects/{d}"),
                state: st.to_string(),
                over: st.to_string(),
                ended_at: now - 1800000 * (k + 1),
                detail: det.to_string(),
                started: now - 1800000 * (k + 3),
                session: format!("demo{i}o{k}"),
                ..Bot::default()
            }
        }));
        bots
    }

    pub(crate) fn demo(&self, mid: &str, poke: Arc<Poke>) {
        if !self.demo_begin(mid) {
            return;
        }
        while self.demo_tick(mid) {
            poke.wait(Duration::from_secs(3));
            poke.clear();
        }
    }

    /// the machine's made-up bots, put at their desks
    pub fn demo_begin(&self, mid: &str) -> bool {
        let Ok(m) = self.machine(mid) else { return false };
        let lim = |used, resets: &str| Some(Limit { label: String::new(), used, resets: resets.into() });
        self.lock().usage = Some(Usage { session: lim(64, "3:20pm"), week: lim(23, "Oct 24 at 9am"), ..Usage::default() });
        let mut bots = self.demo_bots_for(mid);
        let _ = self.fold(mid, &m, &mut bots);
        self.lock().demo_bots.insert(mid.to_string(), bots);
        true
    }

    /// one round of the demo: some bots change, now and then two talk; false once its door is gone
    pub fn demo_tick(&self, mid: &str) -> bool {
        let (bots, talk) = {
            let mut g = self.lock();
            let bots = g.demo_bots.entry(mid.to_string()).or_default();
            for b in bots.iter_mut() {
                if !b.over.is_empty() {
                    if random() < 1.0 / 400.0 {
                        // one that is over comes back (opened again)
                        b.over.clear();
                        b.busy = false;
                        b.waiting = false;
                        b.alive = true;
                        b.state = "done".into();
                        b.pid = 2000 + util::below(1000) as i64;
                    }
                    continue;
                }
                if !b.alive {
                    // between the runs of a /loop: the next run starts
                    if random() < 1.0 / 60.0 {
                        b.alive = true;
                        b.busy = true;
                        b.waiting = false;
                        b.state = "working".into();
                        b.pid = 1000 + util::below(1000) as i64;
                    }
                    continue;
                }
                if random() < 1.0 / 12.0 {
                    b.busy = !b.busy;
                    b.waiting = !b.busy && random() < 0.1;
                    b.state = (if b.busy { "working" } else if b.waiting { "blocked" } else { "done" }).into();
                } else if !b.busy && !b.waiting && random() < 1.0 / 300.0 {
                    // an idle one finishes
                    b.over = "done".into();
                    b.alive = false;
                    b.state = "done".into();
                    b.pid = 0;
                    b.ended_at = util::now_ms();
                    b.detail = "finished what it was doing".into();
                } else if !b.busy && !b.waiting && random() < 1.0 / 150.0 {
                    // an idle one's process is stopped (a /loop between runs)
                    b.alive = false;
                    b.busy = true;
                    b.state = "working".into();
                    b.pid = 0;
                }
            }
            let live: Vec<&Bot> = bots.iter().filter(|b| b.over.is_empty()).collect();
            let talk = if random() < 1.0 / 8.0 && live.len() >= 2 {
                let (a, b) = util::sample2(live.len());
                Some((live[a].id.clone(), live[b].id.clone()))
            } else {
                None
            };
            let bots = bots.clone();
            if !g.machines.contains_key(mid) {
                // its door taken out: gone with its bots
                g.feeds.shift_remove(mid);
                return false;
            }
            g.feeds.insert(mid.to_string(), Feed { bots: bots.clone(), light: false, error: None });
            (bots, talk)
        };
        if let Some((from, to)) = talk {
            self.push(Event::Talk { from, to });
        }
        self.fade(mid, util::now());
        self.chess_check();
        self.digest_watch(&bots);
        {
            let mut g = self.lock();
            let ids: HashSet<String> = g.feeds.values().flat_map(|f| f.bots.iter().map(|b| b.id.clone())).collect();
            g.removing.retain(|i, _| ids.contains(i));
        }
        self.push_state();
        true
    }

    /// the demo's stop: the bot is over (stopped), or gone with rm
    pub(crate) fn demo_stop(&self, mid: &str, bid: &str, rm: bool) {
        let mut g = self.lock();
        let Some(bots) = g.demo_bots.get_mut(mid) else { return };
        if rm {
            bots.retain(|b| b.id != bid);
        } else if let Some(b) = bots.iter_mut().find(|b| b.id == bid) {
            b.over = "stopped".into();
            b.busy = false;
            b.waiting = false;
            b.alive = false;
            b.state = "stopped".into();
            b.pid = 0;
            b.ended_at = util::now_ms();
        }
    }
}
