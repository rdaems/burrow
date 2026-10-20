# burrow

Persistent Claude Code sessions on your machines, shown as bots in a room.

Each machine is a room. Every Claude Code session running on it is a little robot. The robot's place in the
room tells you what it is doing; there are no notifications, badges or counters.

- **busy**: it sits at its desk and types, and its screen scrolls in its project's colour
- **idle**: it gets up and walks around its project's rug, now and then sitting back down
- **idle for 10 minutes**: it falls asleep at its desk

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
| drop a file on the session | `scp` to `~/.cache/burrow/drop/<stamp>/`, then its path is typed into the prompt |

Nothing is installed on the machines except Claude Code. Remote machines are reached with your own `ssh`,
so `~/.ssh/config`, agents and tailscale names all work. One multiplexed connection per machine is
shared through `ControlMaster` (sockets in `~/.cache/burrow/`).

Sessions you started yourself in some other terminal also appear, greyed out: they are on the machine
and burrow shows them, but they belong to that terminal, so they don't open here.

The terminal is light or dark to match the `theme` in Claude Code's settings on that machine.

## Machines

`~/.config/burrow/rooms.json`, written by the **+ machine** button:

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

Rust only: egui/eframe for the window, alacritty's `alacritty_terminal` for the terminal. The room is
drawn dot by dot after [pair](https://github.com/rdaems/pair-prompting)'s office: same robots, same
palette, Super Atlas's bitmap font (`src/glyphs.rs`). JetBrains Mono (OFL, `assets/`) for the terminal.

A new directory has to be trusted by Claude Code once (run `claude` there by hand) before a background
session can start in it; burrow shows Claude's message if it isn't.

Test hooks: `BURROW_OPEN=<id>` opens that bot as soon as it shows up in the first room;
`BURROW_SPAWN=<dir>` starts a bot there at launch.
