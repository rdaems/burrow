# burrow

A room for your Claude Code sessions.

![burrow: the room, and a session open beside it](docs/burrow.png)

Every Claude Code session running in the background on your machines is a robot in one 16-bit office.
Where a robot is and what it does is what its session is doing; what you do to a robot happens to its
session. Click one and its session opens beside the room, in a real terminal.

## First principles

**burrow owns nothing.** The sessions are Claude Code's own background sessions (`claude --bg`), and burrow
is a front end for them: it runs `claude agents`, `attach`, `stop` and `rm`, here and over your own ssh. Close
burrow and every session keeps running; open it again and they are all there. Nothing is installed on a
machine but Claude Code.

**The room is the interface.** Every state is something in the room, every action something you do in it.
No notifications, badges or counters: a session that needs you raises its hand, you delete one by carrying
its robot into a booth, and your usage limits are the coffee pot and the water bottle. Even sound is a thing
in the room: a radio. Put one down and you hear what happens (a hand going up, a message beamed, a session
coming in, a robot leaving through the booth), never a background; click it for silence.

**The terminal is Claude Code's own.** Nothing of it is re-implemented, so whatever Claude Code can do, you
can do here.

**One room, all your machines.** Each machine is a door in the wall, each working directory a desk with its
sessions seated at it.

**It is an office.** Idle robots take breaks: coffee, chess, ping-pong, a duel on the hills outside the
window, a nap on a mat, painting at the easel. You furnish it, and keeping it tidy is yours: a full room
is a full room.

## The room

![the room: robots typing, one with a question, a message beamed across, a game of ping-pong](docs/room.png)

At its desk and typing, a robot is working; with its hand up and a `?` it needs you; anywhere else it is
idle. A finished session stays in the room, idle, until you delete it; opening it picks it up where it left
off. A message from one session to another is a beam between their antennas.

Click a robot to open its session (shift-click opens it beside the others), the `+` at a desk to start one,
a desk's screen for a shell of your own there. Drop a file on a robot or a terminal and it is pasted in, as in any terminal: an image becomes an image in the prompt.

Everything else is furniture, out of the closet: a booth that deletes a session (click it within five seconds
to take it back), a fire that burns the furniture you drag into it, a neuralyzer for `/clear`, a printer for the
office report, a switchboard for the settings, a radio for the sound, an easel the robots paint masterpieces on (with robots in them), a table for a
[pair](https://github.com/rdaems/pair-prompting) room, a dog, and much more. Switch on hardcore and the
booth is a gun closet, the office a first-person shooter in a small Doom engine, and in there a mysterious door
leads to Doom's first level ([Freedoom](https://freedoom.github.io)'s; put id's `DOOM1.WAD`, the free
shareware, or your `DOOM.WAD` in `~/.config/burrow/` for the real one).

An **office manager** sits at a reception. Pick one when you put it down:

- **Susan** is a Claude Code session that knows the office: ask her anything, she does it in the room.
- **Joan** is Susan on a round every half hour.
- **Linda** keeps notes on every session and what it needs from you, and you answer it right there.

<p><img src="docs/closet.png" width="49%" alt="the closet"> <img src="docs/limits.png" width="49%" alt="the limits"></p>

## How it works

| in the room                    | on the machine                                            |
|--------------------------------|-----------------------------------------------------------|
| who is there and what they do  | `claude agents --json --all`, every 3 s (15 s in the background) |
| open a session                 | `claude attach <id>` in a pty                             |
| start one                      | `claude --bg -n <name>` in the desk's directory           |
| delete one                     | `claude stop <id>`, then `claude rm <id>`                 |
| a message between sessions     | read from the transcripts                                 |
| the limits                     | `claude -p /usage`, every 5 min                           |

The machines are Claude Code's own ssh connections (`sshConfigs` in `~/.claude/settings.json`), reached with
your own `ssh`. burrow keeps only what Claude Code has no notion of, the desks, the furniture, your face and
the settings, in `~/.config/burrow/room.json`.

## Running it

```sh
python3 web/server.py          # http://localhost:7777, in a browser
cargo run --release            # or in a window of its own (it starts the server when none runs)
macos/bundle.sh                # or as burrow.app: cp -R target/release/burrow.app /Applications/
```

Python 3 (the standard library only), Claude Code, and a browser or Rust. A door from the closet adds a machine. A
new directory has to be trusted by Claude Code once (run `claude` there) before a session can start in it.

The window (`src/`) is the system's own webview around the same page, with what a browser tab can't have: a dock
icon, the keys a browser keeps (⌘W closes a tile), links in your browser, and Restart Server in its menu. The server
it starts is detached and keeps running when the window closes, as your sessions and terminals do.

## Working on it

`BURROW_DEMO=1 BURROW_PORT=7778 python3 web/server.py` fills the room with made-up sessions, and
`node web/look.mjs URL W H steps.json` drives it in headless Chrome. The pictures here were taken that way.

## Made of

The robots, the lettering and the 16-bit panels are [pair](https://github.com/rdaems/pair-prompting)'s. The
furniture is pixel art after [Pixel Agents](https://github.com/pixel-agents-hq/pixel-agents), some of it its
own sprites (MIT, `web/sprites/`). The terminal is xterm.js, its font JetBrains Mono (OFL).
