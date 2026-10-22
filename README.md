# burrow

Persistent Claude Code sessions on your machines, shown as bots in a room.

Each machine is a room. Every Claude Code session running on it is a little robot. The robot's place in the
room tells you what it is doing; there are no notifications, badges or counters.

- **busy**: it sits at its desk and types, and its screen scrolls in its project's colour
- **idle**: it gets up and walks around its project's rug, now and then sitting back down
- **idle for 10 minutes**: it falls asleep at its desk
- **talking**: a bot that sends another one a message walks over to it, across the room if it is in
  another project, and stands next to it for a moment

Sessions started in the same directory share a rug: one rug per project, one desk per bot, labelled
with the folder name. The empty desk at the end of a rug (`+`) starts another bot in that project.

Click a bot and its session opens next to the room. That pane is a real terminal running
`claude attach <id>`, so you are using Claude Code itself. Close it (✕, or click the bot again)
and the bot keeps working. Close burrow altogether and every bot keeps working too.

## How it works

burrow owns nothing. The sessions are Claude Code's own **background sessions**:

| burrow                    | runs on the machine (over ssh for a remote one)      |
|---------------------------|------------------------------------------------------|
| who is there, busy/idle   | `claude agents --json`, every 3 s (15 s when burrow is in the background) |
| start a bot               | `cd <dir> && claude --bg [-n name]`                   |
| open a bot                | `claude attach <id>` in a pty                         |
| stop a bot                | `claude stop <id>`                                    |
| who talks to whom         | messages between sessions, from the transcripts written in the last minute (same poll) |
| drop a file on the session | `scp` to `~/.cache/burrow/drop/<stamp>/`, then its path is typed into the prompt |

Nothing is installed on the machines except Claude Code. Remote machines are reached with your own `ssh`,
so `~/.ssh/config`, agents and tailscale names all work. One multiplexed connection per machine is
shared through `ControlMaster` (sockets in `~/.cache/burrow/`).

Sessions you started yourself in some other terminal also appear, greyed out: they are on the machine
and burrow shows them, but they belong to that terminal, so they don't open here.

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

`sshPort` and `sshIdentityFile` are used too. A desk on another machine is labelled `machine:project`, as scp
writes it; the other machines are lettered on the wall (in red when burrow can't reach one: hover it for why).

A desk is there while bots work at it, its name on a paper note: click the note to keep the desk (its name on
a plate), the x by its screen to let it go. A session in a git worktree sits at its repository's desk; the
worktree is a laptop on the cabinet beside the project's screen, plugged into it. burrow itself keeps only the
desks you keep and where the furniture stands, in `~/.config/burrow/room.json`.

The Rust app has a room per machine instead, in `~/.config/burrow/rooms.json`, written by its **+ machine** button:

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

The browser version (`python3 web/server.py`) keeps pair's robots and lettering but furnishes the room as
pixel art on a 16-dot grid, seen from above at a slant, after [Pixel Agents](https://github.com/pixel-agents-hq/pixel-agents):
its plants, whiteboard, armchair, meeting table, stools and mug are Pixel Agents' own sprites (MIT,
`web/sprites/`), the rest is drawn in `web/sprites.js` in the same manner.

A new directory has to be trusted by Claude Code once (run `claude` there by hand) before a background
session can start in it; burrow shows Claude's message if it isn't.

Test hooks: `BURROW_OPEN=<id>` opens that bot as soon as it shows up in the first room;
`BURROW_SPAWN=<dir>` starts a bot there at launch.
`BURROW_DEMO=1` fills the rooms with made-up bots (nothing real is touched); `BURROW_SHOT=<file.png>` makes
burrow save a screenshot of itself every second; `BURROW_ADD_DIR=1` opens the add-a-project dialog.
