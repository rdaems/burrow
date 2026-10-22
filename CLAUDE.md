# burrow — notes for Claude sessions

Rembert's own replacement for Orca (too bloated). Read README.md first; this file is the why.

## What he asked for (2026-10-20)

- Persistent Claude Code sessions: closing the app must never stop them; reopening reconnects.
- Machines over ssh; one **room** per machine. More rooms later.
- Bots run around in the room; their chats are hidden by default — he does not want to follow them.
- **State is shown only by where a bot is and what it does.** No notifications, no badges, no counts,
  no sounds. The room *is* the interface, not decoration.
- Clicking a bot opens its session as a real terminal (the actual `claude` TUI — keeps us on Claude Code's
  own footing, nothing re-implemented).
- Drag files onto the session, like Orca.
- Same style as his pair prompting room (rdaems/pair-prompting: `app/pixel.js`, `app/floor.js`).
- Keep it simple. Start with busy/idle only (no "has a question" state yet).
- **Pure Rust** (his call over Tauri): eframe/egui 0.37 + alacritty_terminal 0.26.

## Load-bearing decisions

- Persistence is Claude Code's own background sessions: `claude --bg`, `claude agents --json`,
  `claude attach <id>`, `claude stop <id>`. Nothing installed on the machines, no tmux, no daemon.
- Project = the directory a session started in (Claude still keys sessions by cwd) → one rug per cwd.
- `claude agents --json` costs ~0.5 s CPU on the machine (odyssey's CPU is precious): poll 3 s focused,
  15 s unfocused. Don't poll faster.
- Interactive (non-`--bg`) sessions are shown greyed and can't be opened.
- The terminal follows the machine's Claude `theme` (`~/.claude/settings.json`, read in the same poll).

## Gotchas already hit

- A just-started bot isn't in the feed until the next poll: keep a session while its attach is alive,
  or the pane closes one frame after spawning.
- `claude --bg` may print "Starting background service…" before the `backgrounded · <id> · …` line; an
  unnamed one prints `backgrounded · <id> (idle — …)` with no second `·` (2.1.306): take the first word.
- scp is SFTP mode now: the remote path is literal, don't shell-quote it.
- `claude attach` in a brand-new, untrusted directory fails until `claude` was run there once by hand.

## Talking (built 2026-10-20)

A bot that messages another walks over to it (`Room::talk`); he watched it work. Verified with real sessions, Claude Code 2.1.304:
- A delivered message is a `type:"user"` line in the **receiver's** transcript with
  `"origin":{"kind":"peer","from":"uds:/tmp/cc-socks/<pid>.sock","verifiedPeerPid":<pid>,"msg_id":…,"name":<sender>,…,"body":…}`.
  `verifiedPeerPid` = the sender's `pid` in `claude agents --json`; falls back to the name.
- The sender's `tool_use` `SendMessage` has `to` as either a name or a `uds:` address — not used.
- Messages between sessions in different permission modes are *held* (a `system` "Held peer message" line),
  never reach Claude, and are ignored. `claude --bg` sessions run in auto mode.
- The poll greps the last 4 MB of transcripts touched in the last minute: a receiver writes ~280 kB of
  context right after the message, 256 kB was too little. Messages already there at launch are not shown.

## Gotchas from testing

- `claude stop <name>` does not work, only the id.
- `screencapture` from the terminal triggers macOS's screen-recording prompt for the terminal app: don't.
- `claude agents --json` has `"status":"waiting","waitingFor":"permission prompt"` — the future
  "has a question" state, if he wants it.

## The room (redesigned 2026-10-20 night, after he called v1 "very ugly")

What he said: it must look like pair's room (port it, don't invent); no egui pills, no grey title bar
(the room runs behind the traffic lights, like Ghostty); no "+ bot" button; one **unit per working dir**,
the add-bot button lives at the unit, and later a file explorer / file viewer / diff editor will hang off
it; separate PCs per bot don't work. He hasn't decided the overall layout (terminal always on one side
vs floating windows) — don't decide it for him.

What it is now (src/room.rs, a port of pair's app/floor.js):
- A unit = a cabinet with the project's screen and name at the left end, a low desktop to the right,
  its bots in chairs facing the screen, an empty chair with + at the end (starts a bot). The screen is
  the project's shared object (where the file explorer will hang). It scrolls only while a bot sits
  there busy. An empty wooden cabinet adds a project; projects persist in rooms.json `dirs`.
- Busy = seated, typing. Idle = beside its cabinet (one per project) or on one of pair's breaks
  (coffee, printer, cooler chat, pong, plants, window, armchair, whiteboard meeting). Idle 10 min =
  charging on a mat. A message = walks over, stands behind the receiver's chair; bubbles have dots,
  never words.
- Scale: dots per point = round(width/280) in 2..4 so the room is 220–330 dots wide, as dense as pair.
  Lettering is pair's 3x5 font, upper case. Colour only as accents (no rugs or tinted areas).
- Desks in aligned columns; rows and the lounge (meeting table, pong, mats) spread with pair's pad.

How to work on it without bothering him (he can't be your eyes):
- `BURROW_DEMO=1` made-up bots (sleep after 40 s), `BURROW_SHOT=<png>` the window saves itself every
  second (no screen-recording permission; it stays on top while it runs, a covered window isn't drawn),
  `BURROW_OPEN=demo0003` opens a pane, `BURROW_ADD_DIR=1` the dialog. Shrink with `sips -Z 1400`.
- A second model (Fable) reviewed the screenshots three times; its advice was good — ask again for
  a critique when stuck on looks, but weigh it.

## The browser version (web/, started 2026-10-21)

He asked for an HTML/CSS/JS version, "focus completely on the browser now", decide later whether to port
back to Rust. `python3 web/server.py` (stdlib only: polls, ptys, a hand-written websocket, SSE) serves
http://localhost:7777; `BURROW_DEMO=1 BURROW_PORT=7778` for made-up bots. `web/pixel.js` and the fonts are
pair's, copied as they are; `web/room.js` is floor.js plus the burrow parts of src/room.rs; xterm.js from
jsdelivr. Look at it with `node web/look.mjs URL 1280 800 steps.json` (headless Chrome over CDP: waits,
clicks, typed text, JS, screenshots). Chrome's own `--screenshot` doesn't run requestAnimationFrame: useless.

### What he asked for next (2026-10-21) and how it's done

He decided the layout: the terminal is fixed on the right, always there (an empty screen when no session
is open); the room keeps the same size whatever happens. It must stay consistent with dragging furniture
by hand, so the room is **not** laid out for the window: it is a fixed 256x256 dots, scaled by whole
pixels to fit (`size()` in room.js). Furniture stands where it was dragged, kept in room.json `layout`
(`{key: [x, y]}`; desks are `u:<unit>`, see One room); a new desk gets the first free place and keeps it.

His principle, in his words: "It's ultimately a frontend for the functionalities of Claude Code! Any action
you take is maximally embedded in this room representation, and any state of the Claude Code system is also
embedded in this representation, in a maximally coherent and consistent way." So far:
- usage limits (Claude Code's own `claude -p /usage --no-session-persistence`, every 5 min, ~2 s, no
  transcript left behind): the coffee pot = what's left of the five-hour session, the cooler's bottle =
  the week. An empty pot gives no coffee. Hover either for the numbers.
- waiting for a permission answer (`status: "waiting"` in `claude agents --json`): seated, hand up, the
  screen's cursor blinking; never falls asleep.
- stop: carry the bot to the door, click the "stop …?" sign. A stopped bot walks out through the door.
- a file dropped on a bot opens its session and types the path (uploaded first for ssh machines).
- hovering a bot lights up its session pane's edge; hovering the pane (or a project's screen) puts a halo
  on the bot(s).
- chess and ping-pong are for two idle bots; one idle bot alone may sit at the chess board waiting for an
  opponent, and the next idle one likely joins.

### Back to pair (2026-10-21, later)

He: "look at pair.daems.me, this was the basis for the UI, we've drifted, make it more like that, the
pixels are bigger here". So: two CSS pixels a dot as pair's floor (`W = gap / 2`), a room 256 dots wide
(pair's margin) and as tall as the window (dragged positions are clamped in; the bottom row stays at the
bottom); pair's wall (26), door, board, rug, mats, side-view chess table; pair's header (pixel wordmark
`pixel.woff2`, `.btn`s) and its starfield where nothing is open.

"There should be a bot idling to allow playing with them" meant *you* play the bots, as people do at
pair's tables: the chess table opens pair's board (`chessboard.js`, rules in `chess.py`, both copied) in
the pane against an idle bot of that room, which plays from server.py (two plies of material, not
Claude); the ping-pong table opens pair's `pong.js` with the bot as its ball machine. The bot sits or
stands at the room's table while you play; a bot that gets work leaves the game (you win). Bots still play
each other on their own breaks.

A bot and its seat are one thing for the UI: hovering either lights up both (and its session); a click
on either opens it; only dragging differs (the seat drags the desk).

### The furniture as pixel art (2026-10-22)

He: "improve the assets a bit, or get them online. see for ex. gather town (though i want to keep the 16
style), or the vs code pets plugin". Looked at: LimeZu's Modern Office (the Gather look on a 16 grid, but
its licence forbids redistributing it, and this repo is public), Kenney (CC0, too plain), vscode-pets
(animals under mixed licences; a pet would stand for nothing), and Pixel Agents
(pixel-agents-hq/pixel-agents, MIT): Claude Code agents in a 16x16 pixel office, the same idea as burrow.

So the room is furnished in Pixel Agents' manner: seen from above at a slant, its wood lit along the front
edge and outlined in plum (#532e3d), metal and plastic outlined #3f3740, a row of 20% black shadow on the
floor; Pixel Agents' planks (#734728) and a navy wall (#28374b). Its own sprites (plants, whiteboard,
armchair, meeting table, stools, mug) are in `web/sprites/` as they are, with its licence; the rest is drawn
in `web/sprites.js` as letter grids in the same palette (`slab()` in room.js for the blocks of any width:
desks, cabinets, mats). What shows state is still painted in by room.js: screens, coffee pot, water bottle,
chess position, clock hands, the sky. The robots and the lettering stay pair's. This replaces pair's wall,
door, board, mats and side-view chess table, and its dark floor; pair's rug stays, muted.

Fable reviewed it once. Taken: a quieter floor, mats that read as charging (its bolt had been a plus: a
medical cross), desks on legs, a 1-dot monitor bezel without a fake LED, a pale free chair under its +.
Not taken: turning the chess board so white is at the bottom (the players sit left and right of it), and
dropping the rug.

## One room, Claude Code's machines (2026-10-22, evening)

He: "I don't want separate rooms after all"; the state is "the claude agents, locally + remote sessions";
desks "even without agents, persistent if the user wants it"; "can we see this tool as essentially a
frontend wrapper on claude code?" (yes: burrow keeps only what Claude Code has no notion of). Looked at
first: `claude agents --json` lists one machine only; Remote Control is Claude's only list across machines,
but it has no CLI or JSON (only the `ListAgents` tool inside a connected session), needs
`remoteControlAtStartup` everywhere and attaching still needs ssh; Desktop's saved environments (machine +
folders) sit in its private `ccd-session-presets.json`. He chose, and it is built:
- **Machines** = this one (id `""`) + `sshConfigs` in `~/.claude/settings.json` (documented; Desktop's SSH
  connections; `sshPort`, `sshIdentityFile` used). The local poll rereads it each round. The empty door frame writes an
  entry: `json.dumps(indent=2) + "\n"` round-trips his settings.json byte for byte; written in place.
- **One room.** A desk is a unit: `<dir>` here, `<machine id>:<dir>` elsewhere. Kept desks and the layout in
  `~/.config/burrow/room.json` (taken once from rooms.json: its local layout, its dirs minus worktrees; the Rust
  app still uses rooms.json). Every bot carries `m`, `unit`, `tree`; ids are assumed unique across machines.
- **Kept or passing**: a desk nobody keeps is there only while bots work at it, its name on a taped paper
  note; a click on the note keeps it (the old dark plate); the x by its screen lets it go. Before, every
  `--bg` cwd got pinned forever (that is how a worktree had a desk).
- **Worktrees**, "at the repo desk, but the repo desk is somehow connected, make it make sense": a session in a
  linked worktree (`git rev-parse --show-toplevel` ≠ the parent of `--git-common-dir`, asked once per new cwd
  per machine; `/.claude/worktrees/` in the path as fallback) sits at its repo's desk, after the main
  checkout's bots. The worktree is a laptop on the repo's cabinet beside the big screen, a cable between them
  (its own copy of the files, the repo's git). Each screen shows only its own checkout's bots; hovering a
  laptop lights its bots, a lit bot lights its laptop. (Laptops at the seats were hidden behind the bots;
  an annex desk per worktree made desks too long for the 256-dot room.)
- **Doors** (his: "each door is a device, the first one is local"): one per machine in the wall, then an empty
  door frame with + that adds one (one field, the ssh host). Bots come in and leave by their machine's door; carry
  one to its door to stop it. Hover a door: its name, its bots light up. Out of reach = boarded up, why on hover.
- **Desk names** (he: "the desks become very big, that's kind of stupid"; `arthroid:ct-annotation-reviewer` made a
  cabinet half the room): as a 16-bit game would, text stays out of the world. Another machine's door and desks
  are painted its colour (a palette swap, `PAINT` in room.js, in sshConfigs order; this machine is wood), so no
  machine prefix; the project's name is cut to 8 letters (`NAME`); the whole path is on the screen's hover.
- **Limits** are per account: one coffee pot and bottle, from this machine's `claude -p /usage`. That run is
  in `claude agents` for about half a second (interactive, cwd ~): it runs with `exec` and its pid is left out,
  or a grey bot popped up at the home desk now and then.
- Chess and ping-pong: any idle bot, one game for the room.

## Next

- More of Claude Code in the room: the file explorer / diff hanging off a project's screen; maybe a
  session's context filling up, its model, its permission mode (all would need a coherent place first).
- A room that fills up: more than six projects overflow the default slots into any free spot.

`.app`: `macos/bundle.sh` (ad-hoc signed; icon from `burrow --icon-dots`).
