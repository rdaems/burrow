# burrow

Persistent Claude Code sessions on your machines, shown as bots in a room.

One room for all your machines. Every Claude Code session running on them is a little robot. The robot's
place in the room tells you what it is doing; there are no notifications, badges or counters.

- **busy**: it sits at its desk and types, and its screen scrolls in its project's colour
- **idle**: it gets up and takes a break: coffee, the printer, the cooler, ping-pong, chess, the plants, the
  window, the armchair, a meeting at the whiteboard
- **asleep**: a session whose process Claude Code has stopped sleeps on a little mat it rolled out near its desk; opening it wakes it
- **needs input**: it sits at its desk with its hand up (a question for you, a permission prompt)
- **talking**: a bot that sends another one a message beams it across the room, antenna to antenna
- **over** (done, failed or stopped, its process gone: agent view's *Completed*): it sits switched off on
  the bench, the newest at the right end, a light on its chest for how it ended (green done, red failed);
  the bench's ends open the whole list. Click one and Claude Code restarts it: it gets up and goes to its
  desk. Hover one and its desk's screen lights up

Sessions started in the same directory share a desk, labelled with the folder name; a session in a git
worktree sits at its repository's desk, the worktree a laptop on the cabinet. The `+` beside a desk's
screen starts another bot there; the empty cabinet adds a project.

Click a bot and its session opens next to the room, as a tile: a real terminal running `claude attach <id>`,
so you are using Claude Code itself. Shift-click adds a tile beside the open ones; drag a tile by its band
onto another to arrange them. Close one (✕, or click the floor) and the bot keeps working. Close burrow
altogether and every bot keeps working too. Typing `exit` in a session detaches, as in Claude Code: the
tile closes, the session runs on. To stop a session, carry its bot onto the bench or to its machine's door:
a sign over it counts five seconds down (a click on the sign, or carrying the bot away, is the undo), then
it sits down on the bench, switched off, where it can be opened again. Without a bench in the room, the
sessions that are over wander about, switched off; the closet has a bench for them, or keep it that way
and tidy up with the fire. To delete a session from the list for good (`claude rm`; its transcript stays
for `claude --resume`), carry the bot, running or over, into the fire: the same countdown, then it goes up
in flames. A bot started from burrow gets a name the way pair names its agents (`dapper-dan`, `merry-moe`).

A click on a project's screen (or a worktree's laptop) opens your own terminal there: one login shell per
directory, in a tile like a session's. It is for the odd command or look, the bots do the work. Close the
tile and the shell keeps running (the screen shows its prompt, and a line running while it prints); click
the screen again to get back to it, with its recent output. `exit` ends it. Shells live as long as
`server.py` does.

Everything else happens in the room as well, and what needs more than the room opens as a 16-bit panel
beside it, never a dialog:

- drag furniture where you want it; the closet on the wall holds more of every kind (drag a piece into
  the room); the fire burns what is dragged into it, a desk, a piece of furniture or a bot
- the coffee pot and the cooler's bottle show what is left of your usage limits; a click on either opens
  the whole picture, every limit `/usage` reports and what is using it up
- the printer prints an office report: who is there, and the statistics of the shift (coffee fetched,
  water cooler talks, naps, games, meetings, messages beamed), row by row on a sheet that comes out of it
- the whiteboard opens big and you draw on it; the host of a meeting draws on the same board
- the window opens the outside: the sky by the hour, the sun or the moon, stars and lit windows at night
- the chess and ping-pong tables are games against an idle bot, drawn by lot
- a pair room ([pair](https://github.com/rdaems/pair-prompting), e.g. pair.daems.me) is a table with a
  speakerphone: give it the link of your own page in the room, click it for the chat, drop a bot on it and
  the bot joins the chat as an agent
- drop yourself or a bot on a piece of furniture and it uses it: the armchair, the window, the
  coffee machine, a plant, the printer, a table
- a click on your own figure picks your face

## How it works

burrow owns nothing. The sessions are Claude Code's own **background sessions**:

| burrow                    | runs on the machine (over ssh for a remote one)      |
|---------------------------|------------------------------------------------------|
| who is there, busy/idle, over | `claude agents --json --all`, every 3 s (15 s when burrow is in the background); the one-line summaries and end times of the sessions that are over from `~/.claude/jobs/*/state.json` (same poll) |
| start a bot               | `cd <dir> && claude --bg [-n name]`                   |
| open a bot                | `claude attach <id>` in a pty                         |
| stop a bot                | `claude stop <id>` (it stays listed: the bench)       |
| delete a bot              | `claude stop <id>; claude rm <id>` (the fire)          |
| who talks to whom         | messages between sessions, from the transcripts written in the last minute (same poll) |
| drop a file on the session | `scp` to `~/.cache/burrow/drop/<stamp>/`, then its path is typed into the prompt |
| the limits                | `claude -p /usage --no-session-persistence` on this machine, every 5 min |
| join a pair room          | the room's bring link (from your page's `/state`) is typed into the session |

Nothing is installed on the machines except Claude Code. Remote machines are reached with your own `ssh`,
so `~/.ssh/config`, agents and tailscale names all work. One multiplexed connection per machine is
shared through `ControlMaster` (sockets in `~/.cache/burrow/`).

The bots are the sessions `claude agents --all` lists: background ones (`claude --bg`), the ones that are
over included, as agent view's Completed group has them. A `claude` you run in some other terminal belongs
to that terminal and cannot be attached to, so it is not in the room, as agent view leaves it out too. You
are in the room as well: beside the bot whose session is open.

The terminal is light or dark to match the `theme` in Claude Code's settings on that machine.

## Machines

The browser version (`python3 web/server.py`, then http://localhost:7777) has one room for all of them. Its
machines are Claude Code's own: this one, and each of `sshConfigs` in `~/.claude/settings.json`, the list the
Desktop app's SSH environments use too. The **+** on the wall adds one there:

```json
"sshConfigs": [
  { "id": "odyssey", "name": "odyssey", "sshHost": "odyssey" }
]
```

`sshPort` and `sshIdentityFile` are used too. Each machine is a door in the wall (the first one is this
machine); its desks and door are painted its colour. Bots come and go through their machine's door; a
machine burrow can't reach has its door boarded up (hover it for why). The empty door frame adds a machine.

Every directory a session was ever seen in has a desk until you burn it. burrow itself keeps only the
desks, the furniture and where it stands, your face, and the pair rooms' links, in `~/.config/burrow/room.json`.

The Rust app (`src/`) is the earlier, native version and has not followed the browser version since the
latter took over: one room per machine, in `~/.config/burrow/rooms.json`, written by its **+ machine** button:

```json
[
  { "name": "mac" },
  { "name": "odyssey", "ssh": "odyssey" }
]
```

An entry without `ssh` is the machine burrow runs on.

## Build

```sh
cargo run --release
```

On a Mac, `macos/bundle.sh` makes `target/release/burrow.app` (icon: one of the room's robots); copy it
to `/Applications`. Started from the Dock it finds `claude` in `~/.local/bin`, `~/.claude/local` or Homebrew.

Rust only: egui/eframe for the window, alacritty's `alacritty_terminal` for the terminal. The room is
drawn dot by dot after [pair](https://github.com/rdaems/pair-prompting)'s office (`app/floor.js`): same
robots, palette, furniture and lettering. JetBrains Mono (OFL, `assets/`) for the terminal.

The browser version (`python3 web/server.py`, stdlib only: it polls, holds the ptys, speaks websocket and
server-sent events itself) keeps pair's robots, lettering, palette and 16-bit panels, but furnishes the
room as pixel art on a 16-dot grid, seen from above at a slant, after
[Pixel Agents](https://github.com/pixel-agents-hq/pixel-agents): its plants, armchair, meeting table,
stools and mug are Pixel Agents' own sprites (MIT, `web/sprites/`), the rest is drawn in `web/sprites.js`
and `web/room.js` in the same manner. The terminal is xterm.js.

A new directory has to be trusted by Claude Code once (run `claude` there by hand) before a background
session can start in it; burrow shows Claude's message if it isn't.

Test hooks (Rust app): `BURROW_OPEN=<id>` opens that bot as soon as it shows up in the first room;
`BURROW_SPAWN=<dir>` starts a bot there at launch.
`BURROW_DEMO=1` fills the rooms with made-up bots (nothing real is touched); `BURROW_SHOT=<file.png>` makes
burrow save a screenshot of itself every second; `BURROW_ADD_DIR=1` opens the add-a-project dialog.
Browser version: `BURROW_DEMO=1 BURROW_PORT=7778 python3 web/server.py`, and `node web/look.mjs URL W H steps.json`
drives it in headless Chrome (clicks, drags, screenshots).
