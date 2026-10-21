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
pixels to fit (`size()` in room.js). Furniture stands where it was dragged, kept per machine in
rooms.json `layout` (`{key: [x, y]}`; units are `u:<cwd>`); a new unit gets the first free place and keeps it.

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

## Next

- More of Claude Code in the room: the file explorer / diff hanging off a project's screen; maybe a
  session's context filling up, its model, its permission mode (all would need a coherent place first).
- A room that fills up: more than six projects overflow the default slots into any free spot.

`.app`: `macos/bundle.sh` (ad-hoc signed; icon from `burrow --icon-dots`).
