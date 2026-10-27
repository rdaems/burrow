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
  per machine; `/.claude/worktrees/` in the path as fallback) sits at its repo's desk (after the main
  checkout's bots then; in arrival order since 2026-10-26). The worktree is a laptop on the repo's cabinet beside the big screen, a cable between them
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

## A wrapper on `claude agents` (2026-10-23)

He asked what a grey "in a terminal" bot was for, then: "What's the point of showing the grey bots, if I
can't do anything with it? Let's approach it the same way as `claude agents` and not show them", and
"closely mapping the `claude agents` system, as a wrapper". Checked against the docs (agent-view.md) and
`claude agents --json` on arthroid:
- The room = `claude agents --json` (no `--all`): background sessions only. `kind: "interactive"` rows
  (a `claude` in someone's terminal) are listed but nothing can attach to them (`claude attach` by name
  or pid: "No job matching"), and agent view hides them; so does burrow now. Completed sessions
  (`--all`) stay out: 18 on the Mac alone.
- State as agent view groups it: `state: blocked` or `status: waiting` = **needs input** (hand up; before,
  only a permission prompt counted and a bot that had asked a question sat typing, because its process
  still said `busy`); `working` = typing; `done` with a live process = idle. A row without a pid (the
  supervisor stops idle processes after ~1 h; a sleeping `/loop`) is `alive: false`: it charges on a mat,
  its screen off, and opening it restarts it (Claude Code does that on attach).
- Messages between bots are a beam (up from the sender's antenna, across, down into the receiver's),
  not a walk: "let them beam something across the floor, like a laser beam".
- **You** are in the room, pair's human figure (`pixelAvatar(…, "human")`, `getpass.getuser()` as the
  seed, terracotta clothes): beside the bot whose session is open, a dots bubble while you type to it
  ("I would be standing next to you with a text balloon" — a speech bubble); at the chess/ping-pong
  table opposite the bot while you play; asleep on a mat once the window has been unfocused a minute
  (pair's away); otherwise on breaks like the bots. Not clickable, not draggable.
- The header is the wordmark only (a settings button may come); the pane has its own bar: the session's
  name, its machine, ✕. The stop button is gone (carry the bot to its door, or type `exit`).
- A plain click on bare floor or wall closes the pane. So does the attach ending: `exit` in an attached
  session does **not** end `claude attach`, it drops into Claude Code's agent view in the same pty (no
  terminal title is set, so it is recognised by its prompt text, "describe a task for a new session");
  Ctrl+Z ends the process. He has not decided whether he wants "normal" terminals in the pane.

## Tiles, and desks handled at their screen (2026-10-23, later)

He: "I want a tiling window behavior": each tile a border with a header that "is essentially a border, but
thicker", the buttons on the left (✕, then a pin that says the tile is kept when another opens, default off),
dragging panes "in the usual way"; "keep it minimalistic". And: no more + chair ("I don't like that
visibility, let's do all the interfacing with the main desk with the computer screen"), desks always stay
(pinning a desk was redundant), remove a desk by dragging it somewhere "in style".
- `web/tiles.js`: a split tree (`{dir, kids, sizes}` / `{leaf}`), tiles absolutely positioned, a new tile
  splits the focused one along its longer side, gaps are draggable, a tile dragged by its band onto a side
  of another goes there (a sibling if the split runs that way, else a nested split), onto the middle swaps.
  Nothing is persisted. The band is `--line` (the border, thicker), 22 px; ✕ and •/◦ on the left.
- index.html: a tile a session (its own xterm + websocket), one for chess, one for ping-pong. A click on a
  bot replaces the focused tile unless it is pinned or shift is held (then the old one is pinned and the
  new one splits it; the same for the games). A click on bare floor closes the unpinned tiles. The room's
  `open()` is the focused tile's bot (where you stand), `opens()` every tile's bot (their rings).
- Desks: every directory a session is seen in is added to room.json `desks` for good (`Hub.desks`);
  `kept`/the paper note/the x are gone. A `+` beside the screen starts a bot; the desk shows a chair a
  bot (one pale chair when empty). Carry a desk up against a door: a sign "remove NAME?" over it (click),
  or "NAME has bots" in red (refused, server side too). Removing drops its layout entry as well.

## His feedback round on the tiles (2026-10-23, night)

- You talk the whole time a session has the focus (a dots bubble while standing with the bot), not only
  while typing; the bot whose session is open sits down at its desk for you (`f.open`, in think()) and
  gets up when the tile closes; never falls asleep while open.
- The wall reaches the top bar (its dark top band read as a gap). Tiles have omarchy-like gaps: 10 px
  between and around (`G`, `OUT` in tiles.js); the band is 28 px, ✕ and • 17 px; the focused tile wears
  its bot's project colour (`--c` on the tile), the hovered one terracotta as before.
- "Allow resizing the office layout horizontally": `#divider` between room and pane, dragged; the room's
  width in dots is `W` (a `let` now), kept in room.json `width` (`/api/room`); default desk slots are
  columns every 124 dots as the width allows (`SLOTS()`). "When there's just one pane, allow restructuring
  it by dragging": read as dragging *into* it: a bot carried out of the room over the pane shows the tiles'
  drop zone (`app.carry`/`carried`, `tiles.preview`/`addAt`), a side splits, the middle replaces, an empty
  pane takes it whole.
- The path of a desk's screen now starts with ~ (the 3x5 font had no ~ glyph, so it was dropped).
- Removing a desk: he asked for "a container or a fire place, something that matches the 16bit theme".
  A fireplace on the wall (default right of the whiteboard, draggable like the board): drag a desk into it,
  the fire flares, "burn NAME" (or "NAME has bots", refused), let go and it is gone. The door sign is gone.
- A click on your avatar: the `you` dialog, face (seed `name:n`, ◀ ▶) and clothes colour (swatches), kept in
  room.json `you`, applied at once. The dir dialog completes paths as you type (`/api/complete`: the
  machine's `ls -d prefix*/`, a datalist).
- Games are not tiles any more: the chess or ping-pong table is clicked, an idle bot is drawn by lot, the
  game starts at once in a dialog over everything (`#gameDialog`, pair's panels inside); closing it resigns.
  pong.js got `start(bot)`. You and the bot walk to the room's table as before.

## His corrections (2026-10-23, late)

- The bubble is only while typing; standing next to the bot is the "session has focus" state.
- Tiles: 6 px gaps, a 2 px border, the focused tile terracotta like the hovered one (not the bot's colour);
  no pin. A click replaces the focused tile, shift-click adds one beside it, a click on the floor closes all.
- The fireplace stands on the floor, drawn like the rest. He: "bigger, and a 3d view (as the other furniture,
  it's flat now). Think like a midcentury fireplace in the middle of the room": so it is a Malm, a 1960s
  freestanding cone in burnt-orange enamel hung from its flue, open all round over a bowl on a tripod, on an
  oval pad of pebbles; 32 x 54 dots (`drawFire`, procedural, its palette `E` at the top; black enamel is a
  one-line change). Default left of the armchair; the mats moved left of it to make room.
- The `you` dialog is pair's door picker only: ← face →, nothing else, no text. No rounded corners anywhere.
- Dragging furniture snaps to whole dots while it moves ("a bit of jittering should be visible"), as pair.
- Lettering: he tried the terminal's font for all of it and did not like it ("still, try to make it a little
  smaller and/or readable", "or only use the other font for the popups"). Tried next: the canvas at the
  screen's resolution, the pixel font at 3 device pixels a letter pixel, the popups in the terminal's font;
  "revert the last changes on the fonts, it's worse now". So it is pair's 3x5 font, a letter pixel a dot,
  everywhere, and the canvas has two pixels a dot; leave the lettering alone.
- An empty desk shows no chair ("when there's no bot yet at a table, do not show a chair"); the first bot
  brings its chair. The desktop keeps its one-seat length so the desk does not jump when it comes.
- The clock is a wall piece like the board and the window (`clock` in `DEF`, `ON_WALL`), dragged along the
  wall; the wall's right end is no longer reserved for it.
- Layout (2026-10-23, night): the header is 36 px (6 px padding), no line under it; the room has the tiles'
  6 px margin around it (`#roomwrap` padding, `MARGIN` in room.js; the pane's own `OUT` makes the gap to the
  first tile), so room and tiles sit on one grid.
- The body wears what the portrait wears: pair's figure drew the torso in the person's colour whatever the
  portrait's outfit ("a white shirt in the selector doesn't show up on the full body"); `body()` now reads
  the torso colours off the portrait's shoulders (pixels (6,19) and (15,19)).

## The view and the closet (2026-10-23, late night)

He: furniture must be addable "integrated in the floor, ie., there's a closet where you can click that
opens a popup interface to drag new furniture in it"; "the popup is actually always on the star field on
the right. When the popup opens, the tiles all go"; "make sure the popup interfaces are all the same,
keep it simple, streamlined, minimalistic"; the games too; "you can also drag other furniture in the fire
place"; the fireplace "without the hood, and make the perspective more from above, similar to the ping
pong table and the chess table"; the top bar smaller still.
- **The view** (`#view` in index.html, `openView`/`closeView`): one element in the pane, framed like a
  focused tile (band, ✕, title), in the tiles' place; opening it closes every tile, opening a session
  closes it; a click on the floor or Esc closes it. Its panels: the closet, chess, ping-pong, a project
  (dir form), a machine (ssh form), you (the face picker). No `<dialog>` is left. `[hidden]` is
  `display:none !important` now (the panels' own display rules used to beat the attribute).
- **Furniture is a list** (room.json `furniture`, keys like `plant0`, `mat2`; `kind_of`/`KIND` strips the
  number; seeded from `FURNITURE` in server.py, the old `mats` layout entry split into `mat0..2`).
  room.js lays out what the list says (`layout()`: `by`, `one()`, `all()`); every `L.<piece>` may be null
  and `L.windows`, `L.chairs`, `L.plants`, `L.mats` are arrays; breaks pick a free one by key (`act.at`,
  `act.mat`). `/api/furniture` `{add: kind, at}` / `{remove: key}`. Kinds there can be many of: plant,
  armchair, mat, window; the rest one at most (greyed in the closet while in the room). The fire and
  the closet are `FIXED`: not burnable, not in the catalogue.
- **The closet** is a wardrobe on the floor (`closet` in `DEF`, default right of the meeting table); its
  right door stands open while its view is. The view shows every kind drawn as it stands in the room
  (`room.icon(kind)`, two pixels a dot, `drawKind`); drag one into the room: `room.carry` draws its ghost
  at the pointer, `room.place` gives the spot, the page posts the add. Pointer capture, not HTML drag.
- **The fireplace burns furniture too**: `inFire` for any piece but the fixed ones and the `add` cabinet;
  the sign says "burn the <name>" (`NAMES`); `act {kind:"burn", key}`. It is a **fire bowl** now
  (`drawFire`, 32 x 24): an orange enamel bowl seen from above with the fire as a blob inside (`ELL` rows
  for the ovals), a stand, a pebble pad; it flares with tongues when something is held over it.
- **Ping-pong was broken** since the dialog days: pong.js's `show()` starts its frame loop only if its
  card was hidden when `start()` ran, and the page unhid it first. The view hides `#pong` again right
  before `pongUI.start`. Chess did work.
- The header is 28 px.

## Interaction round (2026-10-23, evening)

What he asked: stop a bot by putting it on the fire too, with a brief undo instead of a confirm click;
the fireplace "too futuristic, use more flames going up", the danger zone highlighted on the fireplace
while something is dragged over it; the closet against the top wall (it cannot be burnt); the whiteboard
off the wall, on wheels on the floor; anything burnable, the project cabinet too (back from the closet);
every piece with an action, if only a hover; the printer prints a report; the view "should NOT have an
orange border and not fill the whole star screen. It's just a smaller rectangle in the center"; no limit
on how many of a kind; your own avatar draggable and dropped onto things to use them.
- **Stopping**: a bot dropped at its door or into the fire gets `act {kind:"doom", where, left:5}`: it
  stands there, a sign over it "NAME stops - undo" with a shrinking bar, a click on the sign undoes.
  At zero `claude stop`; through the door it walks out, in the fire it vanishes in a shower of sparks.
  `w.stopped` keeps a stopped id from re-entering while the feed still lists it (30 s at most).
- **Drops**: any figure (you too) dropped on a piece uses it (`useAt`): armchair = reads, mat = sleeps,
  window, coffee, plant, printer; chess/ping-pong = a game (for you) or waiting at the board (a bot);
  meeting table = sits a while. A working bot goes back to its desk.
- **Hover**: every piece under the pointer gets a terracotta outline (`outlined()`: the piece drawn
  into a canvas of its footprint and put with `halo`). Plants sway (the top of the sprite shifted a dot).
  The fire outlines red while something is held over it; the pointer on it stirs sparks (`w.sparks`);
  a burnt bot flares them (`w.flare`). The limits' tips are "63% 5h" / "77% 1w".
- **The fire pit** (`drawFire`, 32 x 28): a ring of stones from above, embers and logs, flames rising
  (taller while feeding), sparks (`burst` 0..1). The **closet** hangs on the wall (`ON_WALL`), drawn
  frontally like the doors; the **whiteboard on wheels** is a floor piece (`drawBoard`, anchor bottom
  centre, `INK_AREA` 26 x 11); the **project cabinet** is the `add` kind (`drawAdd`), burnable, in the
  catalogue; `FIXED` = fire, closet. Every key is kind + number now (`/api/furniture` numbers them);
  the old unnumbered keys stay valid. room.json `v: 2` marks the migration (add, board off the wall).
- **The printer**: a click opens the report view (`fillReport`: machines, sessions with state, start
  and directory, the limits) on a paper panel, and a page comes out of the printer (`w.print`).
- **The view** is a centred rectangle (880 px at most, 48 px from the pane's edges), plain band.
- look.mjs: `{dragTo: "js giving [[x,y],…]"}`; room.js exposes `room.at(id)` (a figure's place in
  page pixels) and `room.hovered()` for driving it.

## The pair room piece (2026-10-23, night)

He: "the names of the bots are the same as in the pair room. Add a furniture piece that represents a pair
room, hosted at pair.daems.me ie., this tool directly works as a frontend for that pair room server. If you
click it you open the chat, you can drag bots on it to connect them to that chat". pair (rdaems/pair-
prompting, read from GitHub; `pair_server.py` has no CORS and no frame headers): a room is its links; a
person's own page is `/h/<token>` (the bookmark is the login), its `/state` JSON has the room's name and
`bring`, a `/n/<token>` link; every chat that link is pasted into becomes a new agent, named by pair
(`BOT_NAMES`, random), with the person's colour; the agent then talks over `/a/<its token>` with curl.
- The piece (`pair` kind, from the closet): a round table with a speakerphone, its light on once a room
  is linked. room.json `pair: {key: {url, room}}`; `/api/pair {key, url}` sets the link (the server fetches
  `/state` to check it and for the name), `{key}` looks it up again and returns `bring`.
- A click: the view shows the chat itself, pair's page in an iframe (`#vPair`); unlinked, a one-field form
  for your page's link. A bot dropped on it: `joinPair` opens its session and types the bring link + Enter
  (so Claude fetches it and joins). You dropped on it: the chat.
- Not done: naming. pair names agents itself and `/n/<token>` takes no name, so a bot's pair handle is
  pair's, not its burrow name. If he wants them the same, that is a change in pair (a `?name=` on the
  bring link), not here.

## Review round (2026-10-23, late evening)

He: "do a code review of this project. Catch up on the rust version (keep them both). Go over the UI/UX,
there's many small bugs and inconsistencies"; "As a style reference, I like the UI of pair room more.
Though the assets are better here." A second session of his, `rust-vs-browser-drift-check`, was on the
Rust side at the same time, so the Rust app was only checked to build (`cargo check` clean) and left alone.
- Fixed: a wall piece kept the height it was put down at (his closet stood on the floor, drawn frontally):
  `clampAt` pins wall pieces to `WALL_Y`. The stop countdown was cancelled by the bot's work changing, its
  session opening or sleep (stale `"door"` act checks → `"doom"`). The fire's spark burst took the older
  of a stir and a burning. `/api/furniture` without add or remove raised an unbound-variable error.
  The closet's pictures are redrawn each time it opens (they were cached blank before the sprites loaded).
- The chrome after pair (burrow.css): tiles and the view are pair's panels: a 1 px line, 8 px corners,
  a band on `--box` with the name in terracotta (pair's panel h2) and the ✕ as pair's `.x`; the focused
  or hovered tile has a terracotta line and a tinted band (pair's own-message bubble), no solid band any
  more; the view is a panel on `--box` with pair's shadow; fields on `--code` with 6 px corners, `.btn`
  with 6 px corners, chips and cards rounded as pair's. His earlier "no rounded corners anywhere" was
  about the room's pixel art and the first tiles; pair's UI has them, so the HTML chrome has them now.
  The room itself (canvas) is unchanged. View titles read like pair's panel titles ("Add a project",
  "Furniture", "Report", "Pair room", "Your face").
- README brought up to date (one room, desks for good, tiles, the closet, the fire, the games, pair).

## Round of 2026-10-23, night: 16-bit popups, the fire's throw, the board you draw on

What he said: the orange outline only where a click does something ("we've overdone it a bit"), plants
keep swaying; animate what is thrown into the fire, "coherent with the flames, think about z-depth";
chess clicks on the board itself; popups narrower, and "never show the borders you use for the terminal.
We need to think about that UI aspect, make it more like 16 bit"; the closet without boxes; the whiteboard
opens big and you can draw on it, the bots as well; his avatar must stand still while dragged; a dragged
desk takes its seated bots along (he walks himself); the chess subtitle was redundant; bot names like
pair's; terminals a little lighter; a click on the coffee or the water: one popup with the limits "a lot
like the info you get in the settings of the claude app"; the printer in 16-bit style, big, the sheet and
the letters appearing row by row; and `exit` still feels weird (his question; see the reply in the log).
- **Outlines**: `CLICK` in room.js lists the kinds a click does something on (add, closet, chess, pong,
  printer, pair, board, coffee, cooler); only those get the terracotta outline; the fire's red one stays
  for danger. Chess: the table top only (`top`), the stools are not it.
- **The fire**: `toFire(r, img)` queues a picture of the thing (`snapshot()` of its footprint: a desk via
  `drawDesk`, a piece via `drawKind`, a bot its carried pose); `drawFire` tosses it in an arc (0.35 s) to
  the pit, then sinks it (0.8 s) clipped to the pit, drawn between the pit's floor and the flames, the
  flames taller meanwhile (`w.burning`).
- **The view is a 16-bit panel** (`#view.pn.window`, pair's door.css: the ink ring, two bevels, the
  dithered `--f-window` fill from pixel.js, clipped corners, the pixel font at 16/22, pair's steel buttons
  and fields), 640 px wide at most, centred; titles upper case. The tiles keep their chrome: they are
  terminals. The closet shows bare pictures with names. Chess and ping-pong cards keep the terminal font.
- **The printer** (`#vReport.print`): the printer sprite at six pixels a dot (`room.icon(kind, scale)`),
  the sheet under it growing as the lines print, letter by letter (`fillReport`'s `tick`).
- **The limits** (`#vLimits`, `fillLimits`): a bar per "Current …" line of `/usage` (`parse_usage` keeps
  them all as `bars`, and the rest of the text as `text`: what contributes), opened from the coffee
  machine or the cooler (`act limits`).
- **The whiteboard** is ink (`w.ink`, 130 x 55, five pixels a board dot; `inkLine`, `wipe`, `cells`): the
  meeting's host draws its boxes/arrows/dots there in its colour (`stroke(w, col)`), you draw big in the
  view (`#vBoard`, `room.board.line`) in your colour; the small board shows any inked dot. A board more
  than a third inked is wiped when a meeting starts.
- Your avatar stands while held (`!you.held`); seated bots ride a desk being dragged (desk act).
- **Names**: a bot started from here gets a free pair-style name (`BOT_NAMES` in server.py, `-n`).
- **`exit`**: the tile closes and the bot walks to its door with the five-second undo sign (`room.exit`).
  My view, given to him: hiding the terminal makes `exit` ambiguous (Claude Code means "detach"); in the
  room the honest reading of typing exit is "I am done with this one", so it becomes the stop flow with
  its undo, the same gesture as the door.
- Fixed on the way: `/api/furniture` remove was unreachable since the review round's guard (burning was
  broken on main for an hour); terminals on `--box` (#1a1626).

## Round of 2026-10-23, later: the report with humour, the outside, 16-bit terminals

He: the printer's output "is boring. Make it funny. It's about some statistics"; paths overflowed the
sheet; "Shall we try the same UI design on the terminals?"; `~` was a heart in the project form (the pixel
font has no tilde); the path completion dropdown in 16-bit style; plants seemed to sway only once; a
click on the window to look outside. And two things from before that were not right: seated bots must be
fixed to a dragged desk, and the danger highlight belongs on the dragged object, not on the fire.
- **Fixed to the desk**: `draw()` sets every seated bot of the desk being moved onto its seat each frame
  (the think tick alone lagged a frame behind the pointer, hence the jagging).
- **Danger**: `dangerNow` (something held over the fire) outlines the dragged desk or piece in red
  (`danger(key)` in `piece()` and the desk item) and rings a held bot red (`drawFigure`); the fire only
  flares. The hover for outlines and sway no longer depends on the previous frame's `hovered`.
- **Statistics** (`w.stats`, `stat()`, in `thinkOne`: acts begun by kind, seconds busy/idle/asleep, beams
  sent/got in `talk()`, `w.meetings`): `room.stats()`. The report (`fillReport`) is an office report with
  captions that depend on the counts (coffee, cooler, prints, plants, naps, window, armchair, games,
  meetings, beams, employee of the shift, most relaxed, the limits); projects by name, not path.
- **Outside** (`#vOutside`, `drawOutside`): the window opens a 130 x 70 scene by the clock: the sky's
  colour by hour (`SKY` keyframes), sun or moon on an arc, stars after dark, drifting clouds, hills, a few
  houses with lights on at night. `CLICK` has `window`.
- **Tiles as 16-bit windows** (a try, as he asked): the ink ring with a steel bevel, a dithered blue band
  in the pixel font, gold for the focused/hovered one, the ✕ as a steel button; the terminal inside on
  `--box`. The earlier pair-panel tile chrome is gone.
- **Fields** in the view are set in the terminal font (`#view .field`): paths and hosts, and a real `~`.
  The directory completion is a steel list under the field (`.suggest`, `#dirs`): ↑ ↓ pick, Tab/→/Enter
  take, click too; no datalist.
- Plants sway through four positions at 8 Hz (they did move, at two positions, which read as a jump).
- look.mjs: `{drag: …, hold: true}` then `{release: [x, y]}` for a mid-drag look; arrow keys in `press`.

## Round of 2026-10-24: the band dimmer, margins in the pane, the fire's warning, rescue by dragging

He: the gold on the active pane "a little bit too much, dim it a bit"; the text touched the pane's edges;
"a clear feedback when I drag something above the fire, the object I drag should get a red overlay or
something"; cancelling a burning robot by dragging it away from the fire.
- The focused (or hovered) tile's band is `--f-gold` (one line added to pixel.js), its bevel `--pale1`/
  `--pale0`; the bright `--f-hot` stays for buttons and the limits' bars. First made a darker amber
  (gold1 into gold0); he: "I actually wanted to say to make it more pastel", so it is #ffeeaf dithering
  into #f8cd8e, gold2 and gold1 washed with white.
- `.term` has 10/12/8/12 px padding; the fit addon takes it off the parent's size, so nothing is cut.
- Danger (`dangerNow`): what is held over the fire, or a bot held at its own door, is washed in red
  (`tinted`, source-atop) flashing five times a second (`flashA`) besides its red outline, and the sign
  says what the drop does ("burn the plant", "stop NAME"). Those signs had been set but never drawn: the
  tip was skipped while anything was held. A desk with bots is not in danger (it will not burn).
- A bot waiting to be stopped (`doom`) is hoverable again: its sign lights up, a click on the bot is the
  undo (not "open"), and picking it up cancels the stop (pointermove) so dragging it away rescues it;
  dropping it back on the fire or the door starts a fresh five seconds.
- He: "the bots really hang on my mouse pointer, so they should flash red when my mouse is a little
  higher than the fire"; the project cabinet did not flash. A held figure hangs 24 dots under the
  pointer, so every drop target is now found by where it stands (`spot`, `dropAt`: the fire pit and a
  little in front of it, the doorway, the pair table, the pieces `useAt` takes), not by the pointer.
  The cabinet is outlined and washed like the other pieces (it was drawn outside `piece()`).

## The bench: the sessions that are over (2026-10-24)

He: agents on arthroid were "not showing"; they had finished (`state: done`, their processes reaped, so the
plain `claude agents --json` no longer listed them), "the bot must've set himself as completed. We should
have a way of seeing the completed bots. As always, keep building consistently with the `claude agents`
interface". Agent view (docs/agent-view.md): a *Completed* group (done green, failed red, stopped grey),
`∙` for a process that has exited (attaching restarts it), the rows that don't fit fold into "… N more";
`Ctrl+X` stops a session, `Ctrl+X` again within two seconds deletes it from the list (`claude rm`; the
transcript stays for `claude --resume`). So:
- The poll is `claude agents --json --all`; a row with no pid in state done/failed/stopped is `over` (that
  state), with `endedAt` (its record's `updatedAt`) and `detail` (agent view's row summary) from one grep
  over `~/.claude/jobs/*/state.json` in the same ssh round (`JOBS`, `parse_jobs`). Over bots bring no desk
  back, don't play chess, don't hold a desk against burning.
- **The bench** (`bench` kind, from the closet; room.json `v: 3` adds `bench0` to older rooms; its place by
  `free()`, no default): the over bots sit on it switched off (`dim()`: greys, the ink kept; a 2-dot light
  on the chest, green done / red failed / none stopped), three a bench, the oldest left, the newest right
  (`BENCH_SEATS`, `benchSeat`); more benches seat more; the rest are only in the list. Hover a sitter: its
  name and how it ended, its desk's screen lit (`litDesk`); the desk's screen lights its sitters too. A
  click opens it (Claude Code restarts it): its figure gets up from the bench (`w.benchAt`) and walks to
  its desk. The bench's ends (`completed` act) open the **COMPLETED** panel (`#vDone`, `fillDone`): a row a
  session, newest first, state, name, machine · project, when and how long it ran, the summary; a click
  opens, ✕ twice (within 2.5 s) deletes. Over bots are not folk: the bench draws them (`drawSitter`); one
  picked up gets a figure (`lift`, `benched`) and goes back to the bench on its own when let go
  (`resume`), unless onto the pane (opens) or into the fire.
- **Stop** = the bench: a running bot dropped on the bench, at its door, or `exit` typed: the countdown
  sign, then `claude stop` and the figure, now grey, goes to sit at the free end of the last bench
  (`bench` act; `benchFill`); once the feed says `over` and it sits, the bench takes it over. A bot that
  goes over on its own (reaped after an hour idle, stopped from agent view) walks to the bench the same
  way. **The fire = delete**: `claude stop`
  then `claude rm` (`/api/stop {rm: true}`), running or over; the sign says "NAME is deleted - undo".
- The demo has over bots, bots that finish and come back, and `demo_stop` for the flows.
- His two decisions after it (2026-10-24): `exit` typed in a session only detaches, as in Claude Code (the
  tile closes, nothing else; `room.exit` is gone), stopping is the bench, the door and the fire. No bench
  in the room: the over bots wandered about as figures, switched off (superseded the same day, see
  "The bench is optional").

## Idle bots wander (2026-10-24)

He: "where do the 10 minutes come from, I think when the bot is idle it should start wandering right away.
The sleeping bots at their desks is not a nice view." The 10 minutes (`sleepAfter`, 600 s; 40 in the demo)
were burrow's own, from the first room (after pair's away state); Claude Code's only idle threshold is the
supervisor stopping an idle process after about an hour. Idle bots did wander at once; the nap came after
10 minutes, on a mat, or seated at the desk when no mat was free (you take one while the window is left
alone). Now a bot sleeps only while its process is off (`alive: false`: reaped after an hour, a /loop
between runs), so the mat means one thing, and opening it restarts it; the tip says "process off, opens
again". A process-off bot with no free mat still sleeps at its desk (add mats from the closet). The demo
has a bot between loop runs, and idle ones go off now and then.

## Your terminals (2026-10-24)

He: "we need normal terminals as well. Maybe if you click on the main screen of a project, a normal terminal
opens. But I want to keep it simple"; then: "there's just one terminal for each project, and the user is
encouraged to only use it ad-hoc. The agents should primarily run scripts and apps"; a `cd` to another
project inside it: "simply ignore that"; "that terminal is stored if you click it away. That's it".
- Nothing new on the floor: a shell has no agent behind it, its user is you. A click on a project's screen
  (or a worktree's laptop) opens a tile with a login shell in that directory on that machine, band "name on
  machine" like a session's (`openShell`, kind `shell`, keyed machine:dir). You walk to the desk and stand
  under the screen while the tile has the focus (`screen` act, bubble while typing). Files dropped on the
  tile or on the screen get their path typed in.
- **One per screen, kept by the server** (`Shell` in server.py, `hub.shells` by (machine id, dir)): a
  pty read by its own thread, the last 200 kB kept and replayed on attach; `/shell?m=&dir=` attaches,
  closing the tile only detaches, `exit` ends it (the tile closes with the socket). A burnt desk kills
  its shells (`kill_shells`, worktrees via `folds`). Not kept over a restart of server.py, by design
  (tmux was ruled out on day one).
- The screen shows it: a prompt on the last row with a blinking cursor while the shell exists, a grey
  line running while output comes (`busy`: output in the last 1.5 s, pushed as the `shells` event, or the
  open tile's own output). The bots' bars keep the rows above. Hover a shell's tile and its screen lights
  up; hover the screen and the tile does.

## Mats gone, more breaks, a click on the floor (2026-10-24)

He: "I think we can remove the sleeping mats, we have the bench now. Or rather, I'd like idle bots to
interact with the environment more, do funny stuff, play chess with each other, and they can roll out a
little mat wherever and go to sleep if they want. So remove the explicit charging mats from the system,
the idle bots will do that on their own"; and "when i click somewhere on the empty floor, my avatar moves
there".
- The `mat` kind is gone (closet, `CATALOGUE`, `KINDS`, `FURNITURE`; `furnish` drops `mat*` from older
  rooms, `v: 4`). A nap is an act: the figure takes a rolled-up mat along (`roll` arms), rolls it out on
  a clear bit of floor near where it is (`napSpot`: rings outwards from it, clear of every piece's
  footprint with a margin, of the other mats, inside the floor), lies down (the old sleeper drawing;
  `drawMat` is the mat without the charger), and when it gets up rolls it up and carries it a moment
  (`wake`). Forced, with no `left`, while a bot's process is off or while you are away; a break with a
  `left` for an idle bot. A piece put down on a sleeper wakes it.
- New breaks in `pickBreak`: the nap; hands at the fire (`warm`, one bot a side); watching a game of
  chess or ping-pong from behind the table (`watch`: turns with the play, arms up at a move); a stretch
  (`stretch`, arms up and down); a coffee for a busy colleague (`serve`: fetched at the machine, put on
  its desk by the keyboard, `w.mugs`, drawn by `drawDesk` for 90 s; kept if the colleague got up). A bot
  waiting at the chess board draws the next idle one more often (0.7).
- A click on bare floor still closes the tiles, and sends you there (`loiter`, 20–45 s, then breaks).

## Hosts for a new door, and Remote Control looked at (2026-10-24)

He: hearts instead of `~` in the machine form (the pixel font has no tilde); a completion list there, "we know
all the hosts from the config file", in the 16-bit style; "the color of the home desk on odyssey does not match
odyssey's door"; and whether his persistent Remote Control session on odyssey is compatible with burrow ("I'm
not saying it should! You should investigate if it makes sense").
- Paths in the view's prose are `<code>`, set in the terminal's font like the fields (`.vform code`): a real `~`.
- `completes(inp, list, choices)` in index.html is one completer for both fields (the steel list, ↑ ↓, Tab, a
  click): it opens as soon as the field has the focus (`ask()` dispatches a focus event as well: a window
  without the focus gets none) and as you type. The host field's choices: `/api/hosts` = `ssh_hosts()`, the
  `Host` names of `~/.ssh/config` and its `Include`s, in its order, without patterns and without the hosts
  that have a door already.
  A taken suggestion only fills the field (and lists its children), so both forms have an ADD → button (Enter with
  nothing picked does the same, a double click on a suggestion takes and adds it): a directory that has directories of its own
  could not be chosen with the mouse (reported 2026-10-26).
- The home desk: the only one is `arthroid:/home/rembert` (teal, arthroid's); odyssey has no desk, its one
  session there is over (over bots bring no desk back) and the app's sessions at its `~` are interactive rows,
  hidden. A door and a desk take their paint from `paintOf(mid)`, the index in `state.machines`, so they
  cannot disagree for one machine; the screen's hover says whose a desk is (`arthroid:~`).
- **Remote Control**, checked on odyssey (2.1.306) and here (2.1.310). His `claude remote-control` runs as a
  systemd user service (`claude-rc.service`, cwd `~`, same-dir mode): one `claude --print --sdk-url
  …/code/sessions/cse_… --session-id cse_…` child per chat from the app, all in `~`. `claude agents --json`
  lists them as `kind: "interactive"` with the app's chat names and their status; agent view hides them, so
  does burrow; `claude attach` refuses them ("No job matching") and there is no terminal behind `--print`
  anyway. Their links: the environment's `https://claude.ai/code?environment=env_…` (in the service's log),
  a chat's `https://claude.ai/code/session_<id without cse_>`. So they coexist with the room and only show in
  the coffee pot (one account's limits); nothing in Claude Code gives a terminal on them.
- The other way round works: `claude --bg --remote-control -n name` is a background session that is also in
  the app ("/remote-control is active · Continue here, on your phone, or at https://claude.ai/code/session_…";
  the job's state.json has `bridgeSessionId: cse_…` and `--remote-control` in `respawnFlags`; tested in
  football, stopped and removed). Not built: one flag on `/api/spawn` would make every bot started from the
  room reachable from his phone, and its link is known for a click. His call: it puts every burrow bot in
  the app's list too.

## README with pictures (2026-10-24)

He: "re-write the readme, explain the first principles, add some screenshots. Keep it simple, elegant". The
principles are his own from this file (owns nothing, the room is the interface, the terminal is Claude Code's,
one room for all machines, an office). The pictures in `docs/` come from the demo on :7778 through look.mjs at
1280x820 (the window the hero shows); demo bots cannot be attached to, so the hero's tile is the burrow desk's
shell with `claude` started in it and quit again; `room.png` is a crop of the desks at device pixels, the
panels are crops too. Redo them the same way when the room changes much.

## The bench is optional (2026-10-24, afternoon)

He came back to a bot he had set to work sitting on the bench: "I don't like that. When I would do the same
in orca, the session would still be there. Does this deviate from the claude agents idea?" Checked against
the docs (agent-view.md) and `claude agents --json` here: it does not. `done` is "ready for your next
prompt, whether or not its process is still alive"; the supervisor stops a finished session's process after
about an hour unattached, to free resources (`Ctrl+T` in agent view pins one; nothing outside agent view
can); done with no process is listed only with `--all`, "completed background sessions": agent view's
Completed group, i.e. the bench. Orca kept an interactive `claude` alive in a terminal: no supervisor, no
done state. Attaching restarts a completed session from its saved conversation, so it is there either way.
His decisions:
- **The bench is opt-in.** With a bench the over sessions sit on it as before. **With no bench they are
  not shown at all** (no wandering grey figures). The figure of a session that goes over while the room
  is watched walks out through its machine's door, switched off: in `draw()`, `ids` leaves the over bots
  out when there is no bench, so they leave like a session gone from the feed; the `bench` act with no
  bench just waits for `over`. "Many people (me included) would rather restart an old chat using
  `/resume`": `claude --resume`, `claude attach <id>` and agent view's `/resume` picker know them all.
  A new room still gets `bench0` (`FURNITURE`); burn it to opt out, the closet has one to opt back in
  (it was missing from the closet's `CATALOGUE` in index.html until the evening: his bench was burnt and
  could not come back; a new kind goes in `CATALOGUE` as well as `KINDS`).
- A brief tip on a sitter: "NAME - done - click to resume" (failed, stopped the same).
- The report says "On the bench" or "Out of the room" for the over sessions, by whether there is a bench.

## The switchboard, and bots in the app too (2026-10-24, afternoon)

He: "I like 'the other' direction. Let's do that. We're going to need some settings for things like this.
Let's remove the top bar with 'BURROW' and add a settings furniture thing!"
- **No header.** The room's wall reaches the top of the window; the wordmark is gone (`header`, `.top`,
  `.mark` removed). `size()` only ever looked at the room's parent, so nothing else moved.
- **The switchboard** (`settings` kind, key `settings0`, `FIXED` like the fire and the closet, `ON_WALL`,
  14 x 14, default left of the closet; `furnish` v 5 adds it to older rooms): a steel box on the wall, a
  toggle a setting with its lever up and its lamp lit while on (`drawSwitchboard`, `SWITCHES` in room.js
  names the keys drawn), its lid flipped up while its panel is open. A click: the SETTINGS view
  (`#vSettings`, `fillSettings`, `OPTIONS` in index.html: key, name, what it does), a steel ON/OFF button a
  line, ON in the pastel gold. Kept in room.json `settings`; `OPTIONS` in server.py has the keys and the
  defaults, `options(room)` the merge, `/api/settings {key: bool}` sets one and pushes the state (the
  panel and the lamp follow the push). The piece cannot be burnt and is not in the closet.
- **The first setting, `rc`, on by default**: `/api/spawn` runs `claude --bg --remote-control -n <name>`,
  so the session is in the Claude app and at claude.ai/code as well (tested in the previous round). The
  job's record carries `bridgeSessionId` (`cse_…`); the JOBS grep takes it along with updatedAt and
  detail, the bot has it as `rc`. A bot with `rc`: a white 1-dot light over its antenna, blinking slowly
  (1 s on, 1 s off; not while over or held), and its tile's band an APP button on the right
  (`appLink` in showWho) that opens `https://claude.ai/code/session_<id without cse_>` in a new tab. A
  session in which he types `/remote-control` himself gets the same (the record gets the id). A reaped
  session keeps the flag: `respawnFlags` has `--remote-control`, so a restart on attach is in the app again.
- The demo gives every other running bot an `rc`.
- Not decided: the app's own chats on odyssey (the `claude remote-control` service's children, interactive
  rows) stay out of the room, as before.

## You, and a pair table as one room (2026-10-24, afternoon)

He: the `you` popup "should shrink horizontally"; "Where did you get my name? This should also be a field in
this interface"; "When setting up a pair room table, allow a button to start a new one"; "My avatar will be
used for the pair room"; "One pair room table represents one pair room".
- The view's width is its kind's (`data-kind` on `#view`): YOU is as narrow as its face (`width:auto`). Under
  the face a name field; empty = the login name (`getpass.getuser()`, the only source before; it shows greyed
  as the placeholder). `state.you` is `{name, seed, login}` from `you_of()`; `/api/you` merges name, seed,
  colour (empty removes one) and answers with `you_of`.
- The face's seed is in pair's form, `<start>-<i>` (pair's `clean_avatar` keeps `[A-Za-z0-9-]`, 24 at most;
  burrow's `name:i` would have come out as another face there): `v: 6` rewrites `rembert:15` to `rembert-15`
  (v 5 is the other session's switchboard, landed meanwhile), so the face changed once. A first face is
  still the seed of the name.
- A pair table is one room. Unlinked, its panel opens a new room at `PAIR` (`BURROW_PAIR`, default
  https://pair.daems.me; `state.pairAt`) with pair's `POST /api/rooms {room, name, avatar}`: you are in it as
  the room draws you (the prose shows the small face and the name); or links a room by your own page (/h/) or
  its invite (/r/: pair's `POST /r/<invite> {name, avatar}` joins). Linking a page puts your name and face on
  that seat (`POST /h/<token>/me`); if pair refuses (the name is taken there) the link is kept and the reason
  goes on the wall (`warn`). A change of name or face here reaches every linked room a second later
  (`pair_me_all`: one round for a burst of clicks, `_me_round`).
- Tested against a local copy of pair's server (rdaems/pair-prompting `app/`, `PAIR_DATA`, `PORT`), not
  pair.daems.me: a room there is for good (pair has no delete). :7781 for a worktree's demo got killed by
  another session's test; :7783 was used.
- Not done: the colour. pair gives every human a colour by join order and nothing can set it; here you stay
  terracotta.

## Five small things (2026-10-24, afternoon)

He: "I should not be able to walk below the bottom edge"; "When I click the sofa, I will go sit on it";
the nap "too much like a piece of furniture now", wanted "a simple mat with the bot lying on it"; the
fireplace ~50% bigger; the focused pane's gold "should match the highlight (more orange color) on the
floor. I'd like something inbetween the two, slightly more towards the gold color".
- **The bottom edge**: a click on the floor was already clamped to the floor (`L.floor.y1`); what went past
  it was a *carried* figure: it hangs 24 dots under the pointer and followed it down to `H`, and let go
  near the bottom it stood there, below the floor. Now a held figure never hangs past `FLOORY - 4`, and
  one let go inside the room is clamped into the floor before the drop targets are looked at (the door
  rule, `y < WALL + 16`, still holds at `floor.y0`).
- **The armchair** (his "sofa") is in `CLICK`, tip "have a seat"; its hot action `sit` is handled inside
  room.js's pointerup (not sent to the page): you get the `chair` act with `chosen: true`, a bot sitting
  there gets up. The open-session logic (`with`, `screen`) leaves a `chosen` act alone, so you can sit
  while a session is open; the bubble comes to you there.
- **The nap**: `drawMat` is a flat mat, 32 x 16, outline and two thin rows (no slab bevel, no pillow);
  `drawSleeper` is the standing sprite turned a quarter left (head left, feet right, lying on its back),
  the z's at the head; `MATR` 34 x 19 for the clearance; the beam, bubble and visitor offsets for a lying
  figure follow.
- **The fire**: `FIRE = { w: 48, h: 42, S: 1.5 }`; `drawFire` draws its 32 x 28 original through `X(v) =
  round(v * S)` (ring, stones, pit, logs, embers, a flame tongue a dot with the same height profile,
  sparks, the toss and sink of what burns, its clip to the pit's floor at `y0 + X(23)`), so a later size
  is one number. Its default place moved to `W - 92` to clear the armchair.
- **The band**: `--f-gold` dithers #f0be8c into #ecab78 = 60% of the pastel gold (#ffeeaf/#f8cd8e) and 40%
  of the room's terracotta (#d97757); `--pale1`/`--pale0` the same, for the bevel.

## Busy is the state, not the process (2026-10-24, afternoon)

He opened my session (clever-cleo) and found it `done 1:38 PM · 1 shell still running` while the room had
me typing. The shell was a headless `claude agents` capture of mine from the morning that never exited (the
TUI ignores SIGTERM), carried as an in-flight `local_bash` task. Measured on myself: a background command
still running after the turn keeps `claude agents --json` at `state: working, status: busy` (the record's
`tempo: idle` is the only trace that nothing happens, and it is not in the JSON); between turns this morning
my state was `done` with the task in flight, and the process's `status` stays `busy` while a task runs. The
room counted a bot busy on `status busy` *or* `state working`, so a done session with a forgotten background
command typed away while agent view listed it under Completed. Now `busy` is `state == "working"`, and
`status` decides only for a row without a state (`parse_agents`); `status: waiting` still means needs input.

## Round of 2026-10-24, afternoon: the gravestone, the paper, the clock

He: a burnt bot "burned immediately. Then it shows a gravestone. When I click that, the bot returns. But
there's no progress/wait bar. The gravestone simply disappears after a while. After a few seconds, there's
an animation that makes it fade away"; test the burning of all furniture; the gold band back to the pastel
one, a little less saturated, and that gold on the floor too; the printer's report "like an old fashioned
plotter ... with the little square holes on the sides", pixel font, a margin, less verbose; plants stop
after two movements; a bigger plus; the clock "a control like amphetamine".
- **The fire for a bot** (`bury`, `unbury`, `drawGrave`, `GRAVE`): at once: its picture into the flames
  (`toFire`), a gravestone (`STONE`, 9 x 11) on a clear spot beside the pit (left, right, further out,
  never on another stone; `w.graves`), the name on hover, a click brings it back where the stone stood
  (`w.riseAt`); after 5 s the stone crumbles 1.5 s in an ordered dither (`BAYER`) and then `claude stop`
  + `claude rm`. The feed leaves a buried bot out while its stone stands. The door and the bench keep
  their countdown sign; `doom` is theirs only now.
- **Burning tested** (look.mjs, every key into the fire): all floor pieces, the project cabinet and an
  empty desk burn, a desk with bots is refused. The window and the clock could not: a wall piece was
  pinned to the wall while dragged. Now a burnable wall piece comes off the wall while carried (free x
  and y, in the room) and is hung back where it is let go; fixed ones (closet, switchboard) still slide
  along the wall. A bench with sitters moves by its ends (a press on a sitter lifts the sitter).
- **Gold**: `--f-gold` #ffe9a4 into #f6c784 (`--pale1`/`--pale0`; the first try, #fdf0bf into #f7d4a0, was
  "too bleak"), and `GOLD` (#f6c784) is the room's hover
  colour: outlines, halos, rings, the frame, the undo sign; `TERRACOTTA` is only your clothes now.
- **Plants** stir twice under the pointer (`w.sway`: when the pointer came; four positions at 125 ms),
  then rest until it comes back. **The plus** is a 5 x 5 cross with a bigger hot zone.
- **The report** (`fillReport`): fanfold paper (`.sheet::before/::after`: sprocket holes behind a dashed
  perforation), the pixel font 16/22; the font is proportional and a canvas measures a fallback, so a
  hidden `.ruler` span in the paper's font measures: lines cut to the paper's width (`cut`), staff columns
  padded to pixel stops (`row`, `padTo`), the date flush right (`two`). Shorter: staff, bench tally, break
  room, games, meetings, messages, the employee of the shift, limits.
- **The printer prints into the pane** (the next morning): no panel for the report (`#view[data-kind=
  "report"]` is frameless, no band, closed by the floor or Esc); the printer sprite at 16 px a dot
  (`room.icon(kind, S, pad)`, pad 1) stands at the bottom, the fanfold sheet (480 px) is anchored in its
  slot (`bottom` set from the image's height) and grows upward as the lines print, so the paper moves up.
- **Drops settle on clear floor** (`settle(L, key, at, u, kind)`): he dragged a new armchair out of the
  closet onto the whiteboard and it "got stuck": it landed behind the board (pieces draw and hit-test in
  one order, so the board covered it and took the pointer). One rule for every drop now, a moved piece,
  a moved desk, a piece out of the closet: the nearest spot (rings outward, two dots a step, up to sixty)
  whose footprint meets no other piece's; a wall piece slides along the wall. While a piece is carried
  in from the closet the room's `pointer` follows it (the canvas gets no events then) and no tips show.
- **The desk's path** on hover as a shell prompt shortens it (`abbrev`: fish, starship): `~/p/burrow`.
- **The whiteboard** is drawn on in black, by you and by the bots.
- **The printer in the pane** is a wide-carriage one drawn to the paper's width (`printerPicture(paper,
  S)`: 8 px a dot, three dots a side wider than the paper, the slot in row 1 one dot wider than the
  paper each side, a lamp, a front opening, a stand); it stands across the pane's bottom edge with its top
  twelve rows showing (`SHOW`), the sheet (520 px at most, set before the lines are measured) rises out
  of the slot. The room's small printer sprite is no longer scaled up for it.
- **A second switch, `yolo`**, off by default: `/api/spawn` adds `--dangerously-skip-permissions`, so a
  bot started from the room never waits for a permission answer (`OPTIONS` in server.py and index.html,
  `SWITCHES` in room.js draws its lever on the switchboard).
- **The Rust app** stays the earlier version (his "2." to port / freeze / webview): its crates are the
  newest, its features are Oct 21's; it builds. Not to be ported while the room changes daily.
- **The clock** is Amphetamine (`/api/awake {mode: off|work|for, minutes}`, `Hub.awake`, `awake_tick` on
  every poll, `inhibitor()`: `caffeinate -i -s -w <server pid>`, else `systemd-inhibit`): off, when local
  bots work (a busy bot on this machine; remote ones go on without it; kept in room.json `awake`), or
  1/3/8 h (not kept). The CLOCK panel
  (`#vClock`, `fillClock`, `AWAKE`), the clock's face yellowed (see "The clock's face", 2026-10-26), the tip says
  "awake 2h 13m more" / "awake, bots at work". The state carries `awake: {mode, until, on}`.

## Games for real, and the kickabout (2026-10-25)

He: "When I click a plant, my avatar walks there and waters the plant. Idle robots also water plants, start
playing football on the floor, playing other games, playing chess with each other, or pong. If I click those
games while the robots are playing, I can watch. They are really playing." Also: the pair room without the
blue panel ("we have the stars background already"), and a burnt desk "cropped at the right and left".
- **Water**: `plant` is in `CLICK`, a hot `water` on its footprint; the click gives you the `plant` act
  (`chosen`, so an open session does not pull you away), the can in hand, three seconds of watering.
- **Football** (`football(w, L, team)`, `footballPitch`, `pitchSpot`, `kickabout`, act `football`): two or
  three idle bots (a break at 0.44, after pong) stand round a clear patch of floor near the one who starts
  (rings outward, nine points checked with `L.free`), the ball (`w.ball`, 3 x 3 with a hop) flies from one
  to another in 0.7 s, waits a moment at the feet, the kicker's arms go forward; a player that gets work
  leaves, with one left the game is over. Drawn among the figures by its y.
- **Chess between bots is a real game**, played by the server for both sides (`/api/chess {bots: [a, b]}`:
  `chess_later` schedules the next move after each, 2 to 4.5 s; `{leave: id}` ends it, the other wins;
  `chess_check` ends it when either stops being idle). The room's make-believe board (`w.chess`,
  `chessMove`) is gone: the table shows the server's position, START with no game. White is the left
  seat; once both sit, white asks (`chessBots` act, once per 4 s). The act keeps `gameId`, the id of the
  game that started for the pair (an older finished game in the state is not theirs, which once made them
  get up after three seconds); they get up three seconds after its result, and a break that runs out
  mid-game is a `chessLeave`. The mover reaches out on each new ply (`w.chessAt`, `moved`), yours too.
- **Watching**: a click on the chess table while the bots' game runs opens the board as a spectator
  (`openGame`: chessboard.js shows a game you are not in without moves; the band says "A vs B"); Esc or
  the floor closes it without resigning (only your own game resigns). A click on the ping-pong table while
  two bots are at it (`room.playing("pong")`: both `there`) opens pong.js in `watch(a, b)`: both paddles
  machines (`mach0`, `machine0` mirrors `machine`), no pointer or keys, the hint hidden; `playing()` has
  `ids` and `watch`, the room's "the bot you play" ignores a watched game; a watched bot that gets work
  ends it (`left(id)`).
- **The pair room has no panel** once linked: `#view[data-kind="pair"]:has(.pair-room:not([hidden]))` is
  frameless and full height, pair's page (its own stars) fills the pane; the link form keeps the panel.
- **The fire**: what is wider than the pit shrinks to fit it as it flies in (`fit`), so a desk is no
  longer cropped by the pit's clip while it sinks.
- Test hooks on `window.__room`: `game(kind, a, b)` seats two bots at chess or pong, `football(ids)`.

## A ? for needs input (2026-10-26)

He, of a bot in `needs input`: "I would like him to have a text bubble with a question mark". So a bot that
waits for you (blocked, or a prompt open) has a bubble with pair's 3x5 `?` in it, steady (no animation,
reduced motion too), for as long as it waits, besides its hand up and its screen's blinking cursor. The
one exception to "dots, never words": a glyph, not words. 9 x 9 with the talk bubble's notch, its bottom a
dot over the antenna and right of it (a robot's portrait is empty above row 4: the talk bubble, placed for
a person's head, floats ~6 dots over a robot). It takes the place of the talk dots while it shows; not
while the stop sign counts down. To look at it in the demo: `Object.defineProperty(f, 'waiting',
{get: () => true, set() {}})` on a folk from `__room.world().folk` (think() resets `waiting` every frame).

## Dragging an armchair (2026-10-26)

He: "Draggin an armchair on the floor is still buggy, have you tested this? I've asked about it before."
It had not been tested past the first fix (settle). Tested now in the demo with look.mjs (hold, shots
mid-drag and after), three faults found and fixed:
- **`settle` gave up after 60 dots** and put the piece where it was let go, on top of what was there: a
  new armchair from the closet dropped on the whiteboard, with the old armchair, the meeting table, the
  desks and the bench around it, landed on the board (covered by it, the board taking the clicks). It
  searches the whole room now (two dots a step to 60, four beyond; about 7 ms in that corner).
- **What you saw was not where it landed**: a dragged piece was drawn at the pointer, even over another,
  and jumped to clear floor when let go. Now `moving.at` and the closet's ghost are settled while they
  move: it lands where it is shown. Not while over the fire (it shows there, red, to burn) and not a
  burnable wall piece off the wall.
- **A reader rode a moved armchair a frame late** (think's `sitAt`, a jump of 40 dots made it get up and
  walk): `draw()` sets whoever sits in it onto the seat each frame, as for a desk.
`__room.settle(key, at, u, kind)` is a test hook for it. A press on a figure in the armchair still lifts the
figure; the chair is dragged by its lamp then.

## The clock's face (2026-10-26)

He: "Show the state of the computer not being closed, on the clock. 1h, 3h, 8h, show that as yellowed
regions on the clock (pie's). And when local bots do not allow sleep/shutdown, the full dial is yellow ...
if setting is on but no local bots are working, the full dial is 'gearceerd'" (hatched). Built as he said;
the light dot it replaces is gone. `drawClock(ctx, x, y, awake)` paints the face's cells (`dialCells`: the
sprite's #ededed, found once) in Pixel Agents' yellow (`SP.Y`) before the hands:
- **For a while**: a wedge from the hour hand clockwise to the hour it ends, on the 12-hour dial (1 h = 30°),
  so it shrinks as the time runs out, as a kitchen timer. A cell is lit if the wedge reaches it at all
  (its half width in angle past the end), and the wedge is never under half an hour wide while time is left:
  the hands cover its start, and 20 minutes or less vanished under them (the clock looked off while on).
- **When local bots work**, one at work (the inhibitor runs): the whole face. None at work: hatched, a
  diagonal line every third cell.
At the room's size (12 x 10 dots) 1 h is three cells, 3 h a quarter, 8 h most of the face. The tip keeps the
exact minutes. Test hook `__room.clock(awake)`: the face as a data URL, for looking at every state at once.

## Copying out of a terminal; pair behind Cloudflare (2026-10-26)

He: "I cannot copy paste text from the terminal." An attached session always renders fullscreen and captures
the mouse (`claude` turns on modes 1000/1002/1003/1006, seen in a pty), so a drag selects inside Claude Code,
which copies on release itself: `pbcopy` on a local session, an OSC 52 sequence over ssh (docs: fullscreen,
"Keep native text selection"). xterm.js dropped the OSC 52 (no handler), so nothing from arthroid reached the
clipboard; and the terminal's own selection needs Option+drag on a Mac, which xterm.js only does with
`macOptionClickForcesSelection` (VS Code's terminal has the same setting). Now:
- `@xterm/addon-clipboard@0.1.0` (6 kB, js-base64 inside) handles OSC 52 with a provider of our own that
  writes only: a read request (`52;c;?`) gets an empty answer, never the clipboard.
- `macOptionClickForcesSelection: true`: Option+drag selects in the terminal, Cmd+C copies it.
Tested in the demo's shell tile (look.mjs `drag` takes `alt: true` now): with tracking on (written into the
terminal) a plain drag selects nothing, Option+drag selects; a `printf '\e]52;c;…'` lands on the clipboard.
Not tested: Claude Code's own copy in a real attached session (the demo's bots have none).

Jonas (his own burrow): linking an existing pair room to a pair table said "pair answered 403". pair.daems.me
is behind Cloudflare (public DNS), whose browser check answers Python's default user agent
(`Python-urllib/…`) with 403, "error code: 1010"; at Rembert's home the name resolves to 192.168.1.4, past
Cloudflare, so it never showed here. `pair_fetch` sends `User-Agent: burrow (+…)` now, and an error that
is not pair's own JSON carries the first line of what answered. Tested by pinning the name to Cloudflare's
address: main gave "pair answered 403", the fix reaches pair. Jonas needs to pull and restart his server.
## The notes on the sessions: the notice board, then the office manager (2026-10-25)

Asked: keep track of what the sessions are doing, which decisions are still open, what was built; "a dedicated
agent" with an up-to-date summary. Chosen: in the room, Claude writes it, refreshed when a session finishes or
on opening. And, on the plan: "The 16 bit room is perfect for an overview, notes should help the user really
understand in depth what is happening": so the notes are **not** a 16-bit `#view` panel but a tile at the
screen's resolution (system sans 15 px, headings, lists), only the band in the room's look.
- **The board** (`notes` kind, `notes0`, `ON_WALL`, 20 x 16, default `W - 118`; `furnish` v 7 adds it; in the
  closet, burnable): cork in a frame, a paper per desk with notes on its listed sessions, pinned in the desk's
  colour, six at most; a pencil wiggles on the paper being written (`drawNotes`). A click: `openNotes()`.
- **The tile** (`kind: "notes"`, one at most, `renderNotes`): desks that wait on you first, then the freshest; a
  desk's paragraph, then each session: name (a click opens it beside), state, goal, Now, Open decisions (question,
  context, options), Built, Next, how much of the transcript it read. Re-rendered only when the HTML changes (the
  scroll stays). `t.term` is absent on it: `closeTile` and the file drop check.
- **The server** (`digest_*` in Hub): `digest_watch` on every poll queues a session that leaves busy, starts
  waiting or goes over; `/api/digest {refresh}` (the tile opening) queues those whose record's `updatedAt` is newer
  than their notes' `src` (over ones of the last three days). One worker, one at a time: `SLICE` over ssh returns
  the job record and the transcript's tail (4 MB) only if its size changed; `condense()` keeps what the person
  and the session said, EDITED paths, commit/push commands, AskUserQuestion with the answer, plans; the newest
  80k characters. `summarize()` runs `claude -p` **here** (`--tools '' --strict-mcp-config --setting-sources ''`,
  `--json-schema`, `structured_output`; `BURROW_DIGEST_MODEL`, default claude-sonnet-5-5, about 3 s and a few cents
  a session), its pid in `self.own`. After the queue drains, the touched desks get their paragraph. Kept in
  `~/.config/burrow/digest.json`; notes older than two weeks are dropped. Pushed as the `digest` event.
- Tried on a real finished session (knee-id): three open decisions with options, its commits and tests, right.
- A switch `digest` on the switchboard (on): off, nothing is written. With `yolo` (landed meanwhile) that is three
  switches: the board is as tall as its levers need (`SB_H`: 14 dots for two, 18 for three).
- Then (2026-10-26), asked: "the notes feature should be represented by an office manager on the office floor, not
  by a notes board on the wall". The board is gone (`furnish` v 8 swaps `notes*` for `manager0`). The **office
  manager** is a robot like the bots, but she (asked: "make it a she", then "office manager should be a
  female robot") wears a bow on her head and a skirt in her pink (`body(…, femme)`; `MANAGER`: seed manager-16, one
  antenna, happy eyes, #f06b72); `id: "manager"`, `staff: true`, `human: false`, kept out of the bots' logic by `staff`. In the
  room while her desk (`manager` kind, a small writing desk with a stack of sheets as high as the sessions with
  notes, her chair on the right; `drawManagerDesk`, placed by `free()`) is. `thinkManager`: while `digest.writing`
  is a session she walks to its bot (below its chair, or beside it) with a sheet ("paper" arms) and writes; a
  desk's paragraph: at that desk's screen; else seated at her desk, filing now and then; with `digest` off she
  dozes (`doze`, the z's). A click on her or the desk: the notes tile; she cannot be picked up (`staff` is no
  `press.bot`); hovering the notes tile lights her (`app.hl` gives "manager") and her the tile. Burn the desk
  and she walks out of the door; the closet has it.
- The view, after four tries shown as screenshots (needs-you list, project cards, list + detail, status board),
  asked: "go with A combined with C". The tile is two panes (`renderNotes`): on the left **Needs you**, every
  open decision a line (question, project, session), those a session waits on first (`blocking`), the rest under
  "Also open, no hurry"; then Working, Waiting for your answer, Idle and done, a session a line with its
  `headline`; on the right the notes of what was clicked (the project's paragraph folded at the top), the decision
  picked lit up and scrolled to. Both panes scroll on their own and keep their place when notes come in; a narrow
  tile stacks them (`@container`). The notes got `headline` (twelve words: where it stands, what it waits on)
  and per decision `blocking`, most important first, five at most; notes without a headline are written again.
- Deciding in the notes (asked: "when there is something for me to decide ... I want to be able to decide right
  there. Give me an option to get an explainer about the decision to be made and to make the decision"). Under
  each open decision: **Explain this decision** (`/api/explain`, `Hub.explain`: `claude -p` with the decision, the
  notes and the transcript's story; `EXPLAIN_SCHEMA` what, why, options with means/pro/con, recommendation; kept in
  the notes' `explained` by question; the manager walks to the bot meanwhile, `digest.explaining`); the options as
  buttons that fill the answer; a text box; **Send to NAME** (`/api/answer`, `Hub.answer`): the answer is typed into
  the session as "About your question “…”: …". Claude Code has no command to message a background session, so
  `deliver()` attaches in a pty of its own (`claude attach`, 120x40), waits for the screen to settle (an attach
  restarts a process that was off), pastes the text (bracketed paste: new lines stay in it), Enter, reads a moment,
  lets go (SIGHUP; the session goes on). Tested on a throwaway session: asked APPLE or PEAR, got the answer as a
  plain user message, answered PEAR. A session with a prompt open (`status: waiting`, the bot's `dialog`) is not
  typed to: the notes send you to its session. Answered decisions leave Needs you and are listed as answered
  (`answered` in the notes; both `explained` and `answered` survive a rewrite). Drafts, a picked option, the focus
  and the caret survive the notes being redrawn.
  Merged onto "Tempo, not state" (Rembert's round): `waiting` there means a dialog is open, so `dialog` is that; in
  the notes a session needs you when a dialog is open or its tempo is `blocked` (`needsYou` in index.html), and the
  prompt for its notes says which.

## Tempo, not state; the completed ones roam (2026-10-26)

He asked why tidy-tess looked busy. It sat in a pair room with a 50-minute `curl …/wait` as a background
command: `claude agents` said `working` (its last message, "listening for replies"); an hour later the same
activity, worded "no replies yet, still listening", was `done`. `state` is Claude Code's classification of
how the last message ends (23 named branches in the 2.1.310 binary: trailing-q, wait-external,
will-check-back, result-marker, …); the job record (~/.claude/jobs/<id>/state.json) also has `tempo`: active
(a turn runs), blocked (it waits for you; `needs` says for what), idle. His decisions:
- **Tempo, not state.** `busy` = tempo active with a live process. `waiting` (hand up, the ?) = a dialog open,
  `status: "waiting"` in the list: tested with a throwaway Haiku session asking an AskUserQuestion (list:
  `status waiting, waitingFor "input needed"`; record: `tempo blocked, needs "answer: Which colour? (Red ·
  Blue)"`). `waitingFor` is "permission prompt", "dialog open", "input needed", "sandbox request" or "goal
  proposal". A last message that only reads as a question (tempo blocked, no dialog: "Shall I build it?")
  is idle, like done. No tempo (an old record): Claude Code's own fallback, busy→active, waiting→blocked.
  The JOBS grep takes `tempo` along; parse_agents keeps `state` and `tempo` in the bot for the record.
- **The completed ones roam**, idle like the rest. He: "is there any difference in the state of this bot [3 h
  after its task], than a bot that I ran 48 days ago?" None: the supervisor stops a finished process after
  about an hour, both are done with no process, both restart on attach. So every session `claude agents
  --all` lists is a figure (35 completed against 4 running when built); the fire (`claude rm`) is how one
  leaves for good. No more greys, no chest lights, no walking out, no nap forced on a stopped process (a nap
  is a break of its own; you still sleep when away). A chair at its desk only while its process runs (desks
  would grow a chair per completed session); opening one restarts it and its chair comes. Completed bots play
  chess too (`idle_bot` no longer excludes them). A figure with no chair at page load starts at `freeAt`.
- **The bench** is a bench: idle bots sit on it a while (`bench` act `{at, seat}`, `benchFree`), a bot dropped
  on it sits; a click on the bench still lists the completed sessions (COMPLETED, open or ✕✕ delete), its
  tip counts them. Dropping a bot on it no longer stops it; the door still does (until the booth comes).
Superseded: "The bench: the sessions that are over", "The bench is optional", "Busy is the state, not the
process", "Idle bots wander"'s process-off nap.

## Links in the terminal; the fire carried (2026-10-26)

He: "check the way we interact with the text, to make it consistent with a 'normal' terminal ... a link to an
artifact with a copy icon in Claude's output is not clickable. I think any hyperlink should be clickable".
xterm.js with no `linkHandler` answers a click on an OSC 8 link (how Claude Code prints its links) with a
`confirm()` dialog, and plain URLs in the text are no links at all without the web-links addon. Now
`linkHandler: {activate: openLink}` and `@xterm/addon-web-links@0.11.0`, both opening http(s) in a new tab on a
click (Cmd+click too). Tested in the demo's shell tile with `window.open`/`confirm` patched: a plain URL and
an OSC 8 link open on a click, with mouse tracking off and on (written into the terminal as Claude Code turns
it on), no confirm. Checked and already as a terminal: paste (Cmd+V), the wheel (to Claude Code), Option+drag
then Cmd+C, Claude Code's own drag-copy (OSC 52 handled since the morning).

He: "Dragging the fire around is weird". Seen mid-drag in the demo: the bots warming their hands at it stayed
put while it went (standing in the pit, or left in the open where it had been), and one stood in the flames
where it was put down. Now picking the fire up ends the `warm` acts (and no new one starts while it is
carried), and putting it down on a figure sends that one out below the pit (`loiter`). The pit still snaps to
clear floor while carried, as everything does since the morning.

## The booth and the neuralyzer; no more stopping at the door (2026-10-26)

He: "a more humane bot removal tool, like a futurama cabin? And a men in black mechanic for executing `/clear`
on a bot? When you add that, you can remove the mechanic by draggin over the door, that one's confusing".
- **The booth** (`booth` kind, `drawBooth`: a glass cabin on a plinth, a lit sign on its roof; two layers so a
  bot stands inside, behind the glass): a bot let go over it (`dropAt` → "booth", `boothAt`) is washed red
  while held ("bye NAME"), then stands inside (`doom` act `{at, left: 5}`), the roof sign red, a countdown
  sign over the booth ("bye NAME - undo"; a click on it or the bot, or carrying it out, undoes). At zero
  `claude stop` + `claude rm` (`/api/stop {rm}`), a soft light fills the cabin (`w.boothFlash`, 1.2 s) and it
  is gone (`w.stopped` keeps it out while the feed still lists it). Any bot, running or completed. The fire
  still deletes at once, with its gravestone.
- **The neuralyzer** (`neuralyzer` kind, `drawNeuralyzer`: a black pedestal, the silver flasher): an idle bot
  let go by it (`useAt`; a busy one is refused, "NAME is busy") steps up (`neuralyze` act), a flash (the room
  white 0.5 s at half strength, brightest at the flasher; only the glow with reduced motion), stars round its
  head while it stands dazed, and `/api/clear` runs `clear_session` in a thread: `claude attach` in a pty,
  read until the screen settles, `/clear` and Enter, read again, let go. Tested on throwaway Haiku sessions:
  the job and its process stay, the session id is new (a new conversation), 4.8 s.
- **The door** stops nothing any more (`dropAt` has no "door"; the door is no longer ajar for a doomed bot).
- Both pieces are in the closet; `furnish` v 9 puts one of each in older rooms, `FURNITURE` in new ones.

## Doors that move, their panel, the door frame as furniture (2026-10-26)

He: "Allow the doors to be dragged as the other things on the wall, and remove the empty plus door, that
should be a furniture object. Though my design principle is now flawed.. about the principle where things at
the top wall can not be deleted. ... But let's not be too strict. When I click a door, I get a popup with an
overview of all the doors where I can change colors and see the names."
- **A door is a piece** (`door:<machine id>`, not in the furniture list; `KIND` gives "door"): `ON_WALL`,
  `FIXED` (it slides along the wall and never burns: that would take the machine out of Claude Code's
  settings), its place in room.json `layout` under its key. `layout()` places the doors first (saved, or the
  old row `2 + i * DOOR`, else the nearest free stretch of wall: `wallSpot`), then what hangs on the wall;
  `L.doors` `{key, mid, m, x0}`; `doorX(mid)` reads them (bots come and go where the door is now).
  `wallFrom()` is 2: wall pieces go anywhere along the wall, settling off the others and the doors.
- **The door frame** (`frame` kind, `drawFrame` with a y now): the dotted frame with its + as a wall piece,
  in the closet, burnable; a click adds a machine (the old form). `furnish` v 10 puts one in older rooms
  (`frame0`), placed right after the doors.
- **The doors' panel** (a click on a door, `#vDoors`, `fillDoors`): a line a machine, its door in its paint
  (`room.door(i)`), its name, "this computer" or "over ssh, host", its sessions or its error, and the paints as
  swatches (wood and nine: the four there were, then mustard, lilac, sage, charcoal). `/api/paint {m, paint}`
  keeps it in room.json `paint` (-1 the wood); `paintOf` takes it before the order-based default, so the
  door and the machine's desks change together.
- Testing: his Chrome had a tab open on localhost:7783 (a demo port) from an earlier session; it reconnects
  to whatever demo runs there, with an old room.js, and posts its own layout (it put the frame on the floor).
  The demo for this ran on 7785.

## Outside the window: the other machines (2026-10-26)

He: "When I look out the window, I can see the other venues (instances), make it cosy." Read as the other
machines (each a door in the room; this one is the room itself). `drawOutside` (130 x 70, the view scales it):
the sky, sun or moon, stars, clouds as before; the anonymous roofs are gone. Now pines, the meadow, and a
cottage a machine (`state.machines` minus this one), spread evenly over the hills, in its door's paint
(`room.paint(mid)`): both windows lit and a plume of smoke from the chimney while its bots work (or wait for
you), one lamp while it has sessions idling, dark with none, planks across while it is out of reach; a warm
glow round lit windows after dark, fireflies over the meadow. Its name under it in the room's 3x5 lettering
(`room.tiny`, `room.tinyW`, cut to its share of the width). Looked at by day and at 22:15 (`getHours`
patched) with three machines put into the page's state (working, idle, out of reach).

## Dragging, once more: the fire's edge (2026-10-26)

He: "the draggin of new furniture still doesn't work". Tested in a copy of his room (`BURROW_DEMO_ROOM=<a copy
of room.json>` makes the demo start from it; still never saved): a piece out of the closet, the booth, the
neuralyzer and the door frame all moved and landed. What was wrong was his armchair, put down right under
the fire: `inFire` counted any overlap with the fire's footprint plus 4 dots below, and settle only keeps
footprints apart, so the chair rested 1 dot inside that margin. Picked up and slid sideways it was washed red
("burn the armchair") and let go it would have burnt. Now a piece is over the fire when the middle of its
footprint is (an edge touching does not count); and a piece or desk held over the fire is drawn over
everything (depth 1e9), so its red wash shows instead of being hidden behind the flames.

## The office manager clears out; hardcore (2026-10-26)

He: "I have to accept that the old bots should simply get cleaned up ... I would like to have an option to
have the office manager make a decision on it, she checks all the bots, and goes around with a shotgun killing
off all the ones we don't want around anymore. That's the hardcore setting (add this in settings), the normal
version is the suicide booth, she gently walks the bots towards it. So, in hardcore mode the booth is a gun
closet. If I click it, I get a cross hair and i can shoot bots."
- Two switches (server `OPTIONS`, the page's `OPTIONS`, room.js `SWITCHES`), both off by default: `cleanup`
  and `hardcore`. Five levers do not fit one column above the floor: three a column (`SB_COLS`, `SB_W`).
- **Her decision** (`app.cleanup()` in index.html): a session is let go when it is completed (`over`), her notes
  on it have no open decision (answered ones do not count) and nothing `next` (empty, or a line starting "Nothing
  left …", which the notes often write instead), and it ended over an hour ago;
  oldest first. Rules over her notes, no extra Claude call. No notes on it, no decision.
- **Her round** (`cleanStep`, before her notes in `thinkManager`; `w.clean`, `w.cleanRest` the pause between):
  normal: `fetch` (to the bot) → `walk` (it follows a step behind her: `escort` act) → `booth` (it stands in the
  booth, `doom`, its five seconds; she waits beside it). Hardcore: `arm` (a shotgun from the gun closet:
  `carry: "gun"`, the `gun` arms) → `hunt` → `aim` (0.9 s levelled) → the shot (`shoot`: a flash at the barrel,
  the bot down, a gravestone where it stood, the grave's five seconds then `claude stop` + `claude rm`) → `after`
  → the next, then `return` (the gun back). Taking a bot from her, the booth's undo, or a click on its stone puts
  it in `w.spared`: she leaves it alone for the rest of the page's life. Her tip says whom she is seeing out.
- **Hardcore** (`hardcore()`): the booth is drawn as a gun closet (`drawGunCloset`: olive steel, a red lamp, two
  shotguns behind glass, one gone while in someone's hands) and takes no bot in. A click on it takes a gun
  (`aiming`): the cursor is a pixel crosshair (red over a bot); a click shoots (a bot: `shoot`; the floor: a
  puff), a click on a gravestone still undoes, a click on the closet or Esc puts the gun back. (Superseded the
  same day: the gun closet opens the first-person view, see "the gun closet in the first person".)
- Tested in the demo with notes faked for its completed bots (its made-up notes always have a `next`):
  her normal round (fetched, walked, five seconds, gone), her hardcore round (armed, hunted across the room,
  shot, the next), and the crosshair (shot, the stone clicked: back and spared).

## The closet drag, for real this time (2026-10-26)

He, after three rounds of "fixed": "DRAGGING A BENCH STILL DOESN'T WORK. Why are you not fixing this?" His
room.json had not been written in 27 minutes: his drops never reached the server. The cause was not in the
room at all: `fillCloset` ran on every `state` push while the closet was open, rebuilding its items, and the
item being dragged (pointer capture, its own pointermove/pointerup) was thrown away mid-drag. In his room (40
bots, three machines, the notes) a push comes every second or two; the demo pushes rarely and look.mjs drags
took half a second, so every test passed. Reproduced with a 3.6 s drag over 4 pushes (nothing landed); fixed:
the closet is built when it opens and again on a push only while its pictures were still loading, never while
something is carried out of it (`closetCarry`); a cancelled drag clears its ghost; the pictures are not
draggable. The same flaw, smaller: `fillSettings` and `fillClock` rebuilt their buttons on every push, which
could swallow a click; they rebuild only when what they show changed.
Lesson for testing drags here: drag slowly (seconds, many steps) against a room that pushes often, e.g. his
room.json through `BURROW_DEMO_ROOM`; look.mjs's `{cdp: "Input.setInterceptDrags", params: {enabled: true}}`
reports a native browser drag if one starts.

## Remains instead of gravestones (2026-10-26)

He: "Add more gore when killing a bot, splash it's body in parts, with cog wheels etc. Idle bots come clean it up.
The remains act as the gravestone mechanic, so no gravestone". For every kill (the crosshair, her shotgun, the
fire) the gravestone code is gone (`GRAVE`, `drawGrave`, `w.graves`, `unbury`):
- `scatter(f, at, charred)`: the bot's own body picture cut into head, chest, arms and legs (`drawImage` of a
  part of it), plus cogs (`COG`, `COG7`), bolts and a spring; each with a velocity out and up. `stepRemains` (in
  `think`): gravity, two bounces at most (in twelfth-of-a-second steps a small hop never dies out by itself: the
  first version never let a piece land), then sliding to a stop. An oil puddle (`puddle`, shrinking as pieces go)
  and ~90 drops and streaks, under everything (depth -1e9). A burnt bot: its picture into the flames as before,
  six charred cogs and bolts beside the pit, no oil. Reduced motion: the pieces lie where they would land.
- The undo: a click on any of it (`unshoot`, also with the gun in hand) flies the pieces back in (0.45 s) and the
  bot stands up there, spared from the office manager. Her hunt skips what is in pieces (`w.remains`).
- The clean-up: once the pieces had 1.5 s to land, idle bots on an ordinary break (`SWEEPERS`) are sent over, two
  to a heap (`sweep` act): to the nearest landed piece, bent down 0.8 s, the piece gone with the oil round it.
  The last piece gone: `gone()` = `claude stop` + `claude rm`. Nobody to clear it: it rusts away after 30 s
  (`RUST`) and goes then. Tested in the demo: shot, landed within 2 s, two sweepers, cleared and deleted; the undo;
  the fire.

## Taking a door out (2026-10-26)

He: "Obviously, the doors interface should have a way of deleting a door". In the doors' panel every door but
this computer's has "remove", asked twice ("sure? remove" within 3 s, `doorArmed`, kept across the panel's
redraws). `/api/unmachine {m}`: `remove_machine` takes its entry out of Claude Code's sshConfigs in
~/.claude/settings.json the way `add_machine` writes (in place, `json.dumps(indent=2)` + newline: tested on a copy
of his settings.json, the rest byte for byte); the machine and its sessions are left as they are. Then
`set_machines` and a poke (its watcher sees it gone, drops its feed and stops; the demo's machine thread does the
same, `DEMO_GONE` stands in for the settings there), your terminals there killed, its door's place and paint, its
desks and their places dropped from room.json. Its figures walk out (through this computer's door: theirs is gone).
Adding it again brings the door back; its desks come back as sessions are seen there.

## Doors as wall pieces, no frame (2026-10-26)

He: "Removing the door doesn't work, and I asked you before there should not be a ghost + door, it should be in
furniture. Doors are entities that are equal to the other wall entities, I should be able to drag them wherever".
Removing did work in the demo; his :7777 had not been restarted, so `/api/unmachine` was not there yet.
- **No frame.** The `frame` kind is gone (`furnish` v 11 drops `frame*` from the furniture and the layout). The
  closet has a **door** instead (`door` in `CATALOGUE`, drawn in the paint the next machine gets): let go on the
  wall, it hangs there (`room.pend(at)`, drawn while its form is open) and the machine form opens; ADD posts
  `/api/machine {ssh, at}` and the server puts `door:<id>` at `at` in the layout before the state goes out (the
  pending door stays up until the state brings the real one, `pend(at, mid)`); ✕ or Esc puts it back.
- **A door is like the window and the clock**: not `FIXED` (`fixed(key)` = the fixed kinds, or this computer's
  `door:`), so it comes off the wall while carried and hangs again where it is let go (settled on clear wall).
  Carried into the fire it takes its machine out: `act unmachine` → `/api/unmachine`, the same as the panel's
  remove; the fire's tip says "take out NAME". This computer's door only slides along the wall and never burns.
- A wall piece being carried (door, window, clock) is drawn over everything (`aloft`, in `items` at 1e9) and
  washed red over the fire (`onTheWall`); before, a window over the fire was drawn under the flames, unwashed.
  `drawDoor` takes a y; the doors' own hover (a steel frame behind) is the room's gold outline now.
- The demo can add a machine (`DEMO_ADDED`, made-up bots). look.mjs: `releaseTo` (a JS expression) for a held
  drag. A figure standing in front of a door takes the press (you, just in through it, at load).

## Desks are yours; no desk, not in the room (2026-10-26)

He: "We do have a state in burrow that is not in claude agents: the presence of desks. How should we handle that?
Every bot is defined by its working dir, correct? Maybe we can hide bots that don't have a desk. And when you click
new desk, you get a list of the recent/existing working dirs that are not on the floor". Yes: a bot's place is its
unit (machine + directory, a worktree folded into its repo), and the desks are the one thing burrow owns, so they
are his choice now instead of a record of every directory a session was seen in.
- **No desk comes on its own** (`Hub.desks()` only lists room.json `desks`). `state()` carries only the sessions at
  a desk (`on_floor`); the rest stay in `hub.bots()` for everything else (the clock's keep-awake counts every local
  bot at work; the office manager writes notes only on the ones in the room: `digest_watch`, `digest_refresh`).
  Their figures walk out through their door when they leave the state, in when they come back.
- **The project cabinet's form** lists what is not on the floor (`/api/recent {m}` → `off_floor`): the units with
  sessions first, how many running and "waits for you" when one needs input (they are not in the room: this is
  where they show), the newest first; then where Claude Code ran lately on that machine (`RECENT`: the newest
  transcript of the 100 latest dirs of `~/.claude/projects`, its first `"cwd"`, if it still exists; ~0.1 s), not
  worktrees, not the Desktop app's scratch chats, not temporary dirs (/tmp, /var/folders). Typed: the ones that contain it, then `ls` completion. The
  completer (`completes`) takes `{value, note}` items, the note on the right. Asked once a form and machine.
- **A desk with bots burns** (it was refused: it would have come right back). First built as "its bots go on, out
  of the room, through their door"; he: "They should not walk out the door, we changed that! In normal mode, they go
  in the cabin, in hardcore mode my avatar takes a gun and shoots them", and "Likewise when I exit a bot from the
  bench UI". So both hand the bots to `condemn(ids, unit)` (room.js; `room.condemn` for the page):
  - normal: they line up by the booth (`queue` act, on its roomier side, 12 dots apart, five a row) and go in one
    at a time when the first has arrived (`condemnStep`), the booth's own five seconds each (`doom`);
  - hardcore: they stand where they are, hands up (`condemned`), and you (`huntStep`, before your `thinkOne`) walk
    to the gun closet, take a gun (one shows gone from it), go to the nearest, level it 0.9 s, shoot (`shoot`: the
    remains, the sweepers, `gone()`), the next, then put the gun back;
  - no booth: they walk into the fire (`pyre`, `bury`); no fire either: deleted at once.
  `DOOMED` (doom, queue, condemned, pyre) keeps work, an opened session and the office manager off them. Kept: taken
  out of the line (picked up and let go anywhere but the booth), the booth's undo, or the remains clicked
  (`w.spared`). A burnt desk is hidden at once (`w.razed`, left out of `layout`) but `forget` is posted only when
  the last of its bots is gone; rescuing one brings the desk back where it stood. A reload before then shows the
  desk and what is left of its bots again (nothing lost). The fire's tip: "burn NAME, its bots to the booth" /
  ", shoot its bots" / " and its bots". The completed list's ✕✕ condemns too (deletes directly only if the
  bot has no figure). Tested in the demo: five bots through the booth and the desk gone after, five shot in
  hardcore, a ✕✕, and a rescue from the line that brought the desk back.
- Decided myself (his to overrule): a session waiting for input in a directory without a desk is hidden like the
  rest; only the cabinet's list says so. README brought up to date (desks, the booth instead of the door, a door
  from the closet). (Superseded 2026-10-27: every session has a desk, see "Desks follow the sessions".)

## Seats in arrival order; the gun closet in the first person (2026-10-26)

He: "A new bot on a desk should append after the existing ones, now it gets a seat closest to the pc." And: "The gun
closet in hardcore mode: If I click it I can kill bots. Make it such that the full screen is now showing a 3d view,
very simple sprite graphics like wolf3d or doom, of the office, and we can kill the bots as a classic fps".
- **Seats**: the row was sorted main checkout first, then each worktree, so a new bot in the main checkout sat down in
  front of the worktree bots, and one whose process came back went by its old start time. Now `sitters` keeps a row
  per unit (`seatRows`): who sits keeps the chair, a newcomer takes the far end, the rest close up when one leaves;
  on page load in the order they started. The worktree grouping is gone (a worktree's laptop still lights its bots).
- **The view** (web/fps.js, `startFps(room, done)`, started by `firstPerson` in room.js; the 2D crosshair, `aiming`,
  is gone): a raycaster into a 200-row framebuffer (as wide as the window's shape, 440 at most), scaled up in
  big pixels over the whole window, a depth and an id per pixel. It shows the room as it is: after each drawing
  while it is on, `scene3()` builds the scene from what was just drawn (the room keeps thinking at 12 a second;
  fps.js moves the figures smoothly between ticks):
  - the floor canvas (planks, the rug, the mats rolled out, the oil), the wall (the 2D wall band, `panelled` with a
    rail and panels for up close, then what hangs there: `wallPics`, recorded by `onTheWall` and for the closet and
    the switchboard), the other three walls plain; walls 40 dots high (the 26-dot band stretched: doors come out a
    bot's height), the eye at 21, a ceiling of tiles and lights; the floor and ceiling from smaller copies far off
    (`mips`: the planks' seams shimmered in rings), fog toward a dark navy;
  - a desk is blocks (cabinet z 0–11, desktop z 5–9 on legs) faced with crops of its own `drawDesk` picture (the
    name plate on the cabinet's front, the keyboards and mugs on top), its ends and back in its paint; its screen
    and laptops and its chairs are sprites;
  - every other piece is a sprite of what the room just drew (`pics`: recorded in `piece()` and by the chess
    table, the fire, the booth or gun closet, the bench, the cabinet), so its state shows (the coffee, the board);
  - figures from `body()` in their pose (flipped by the way they face as you see them), the lying ones turned, a ?
    over one that waits, the pieces of shot ones as they fly and land, the ball. Not you: you are the eyes.
- **Playing**: you start in front of the gun closet facing the room's middle. WASD or arrows, Shift runs, the mouse
  looks (the pointer locked; up and down by shifting the horizon), a click (or F, Enter) fires: the nearest bot in a
  spread round the crosshair goes to pieces (the room's `shoot`: remains, sweepers, then `claude stop` + `claude rm`),
  a miss puffs where it hits. E (or Space): the pieces under the crosshair, or the nearest in front within 70 dots,
  are put back together (`unshoot`). A shot sends the idle bots within 110 dots running, hands up (`alarm`: `loiter`
  with `panic`); not those at work, opened, or let go of (`DOOMED`). The office manager can't be shot (since 2026-10-27 she can: see "Susan is a bot"). Desks and
  pieces stop you (`solid`); bots don't. Your figure in the room follows you (`fps` act, the gun in hand); the
  automatic hunt for condemned bots (`huntStep`) is off while you are in there (you shoot them yourself) and starts
  over when you leave. Esc (losing the pointer lock) leaves, as does hardcore switched off; you stand where you went.
- **Over it**: the shotgun (a pump gun, your skin and sleeves from your portrait), its kick, the pump, a flash that
  lights the room; the crosshair, red on a bot; the name of who is in your sights at the top (under the crosshair
  it covered their face), "E: put back together" near remains; the keys for the first six seconds. No sounds.
- Tested in a demo started from a room file with desks (`BURROW_DEMO_ROOM`; the demo has one desk since "Desks are
  yours"): look.mjs, `__fps` (set, turn, look, fire, use, under, ms, leave) since headless Chrome has no pointer
  lock (then a click fires and a drag looks); the gun closet clicked in the room, shots, the pieces, E, Esc, walking
  into desks, condemned bots shot in there and the hunt taking over after. About 1 ms a frame. Not tested: the
  pointer lock and the mouse in his own Chrome.

## The reception: office managers in three styles; the 3d goggles (2026-10-26, evening)

He: "I want to generalize the concept of the notes bot. It's the office manager. Make their desk a bit bigger,
like, she's really sitting behind a booth. When you add this, you get a popup with different office manager
styles. The basis is just a claude code session with knowledge of this whole thing, and the power to remove
bots, add desks, ask help, etc. The notes thing Stijn made is actually already a quite elaborate version of
this, and is thus one of the options. They're all female and have simple female names, like Susan".
- **The reception** (`manager` kind, still one at a time; `drawReception`, two layers: a back panel and posts
  under a sign in her colour with her name, then the counter, a slab, over her): 46 x 45 dots, she sits at
  `x + 27, y - 5`, her head and shoulders over the counter. On it a bell, and for a session her screen (runs
  while a turn runs, cursor blinking while she asks, dark while her process is off); Linda's stack of notes.
- **The styles** (server.py `MANAGERS`, room.json `manager: {style, id, since}`; no entry = Linda, so his room
  kept its manager): **Susan** (`ask`, susan-10, teal) a `claude --bg -n Susan --append-system-prompt …` session
  of her own in `OFFICE` (`~/.config/burrow/office`; `BURROW_OFFICE` to move it), whose CLAUDE.md burrow writes
  (`MANUAL`: the office, how it maps onto Claude Code, `/api/office`, rules, her style's brief; rewritten when the
  server starts, for the port); **Joan** (`rounds`, joan-10, lilac) the same with the first prompt `/loop 30m Do
  your round…` (Claude Code's own: CronCreate in her session, a round at once, then every half hour; she logs
  them in `rounds.md` there; a session cron expires after seven days); **Linda** (`notes`, manager-16, pink),
  Stijn's notes as they were. rc and yolo apply to her session as to any bot.
- **Hiring**: the closet's reception, let go in the room, waits there (`room.pend(at, null, "manager")`) while the
  OFFICE MANAGER view shows the styles as cards (her figure, `room.figure`); a pick posts `/api/manager {hire,
  at}`: the booth there (an existing one moves), the one before let go (`claude stop` + `rm`), hers started.
  Burning the booth (`hub.fire`) lets her go; she walks out. ✕ puts the booth back in the closet.
- **Her session is not a bot**: `take_manager` takes her row (and anything else in OFFICE) out of this computer's
  feed (`hub.mrow`); `state.manager` carries her style, look, id and busy/waiting/alive. In the room she is the
  folk "manager" (`staff`): typing at the booth while a turn runs, hand up and the ? while a dialog is open in her
  session ("ask help": AskUserQuestion), dozing while her process is off. A click on her or the booth
  (`{kind: "manager"}`) opens her terminal (`/api/manager {open}` starts her again if she is gone) or, for Linda,
  the notes. The page maps her session id to "manager" (`asFolk`) for the open tile, the rings and the hover; you
  stand at the counter's end while her tile has the focus. Beams to and from her are hers (`her()` in watch).
- **`/api/office`** (GET: machines, desks, every session with its state, desk or none, summary; the settings;
  `removing`, `kept`) and POST `{do: remove | desk | start | say}`. A remove, with the room open and a booth,
  goes into `hub.removing`; the page puts those first in `app.cleanup()`, so she fetches the bot and walks it to
  the booth (hardcore: shoots it) with the same `cleanStep` as the clearing out; one you rescue (`spare()`) is
  posted back as `kept`. With no page open it is `claude stop` + `rm` at once. start puts up the desk too.
  Tested on real sessions in the demo: Susan, asked in her terminal, removed tidy-tess (curl, then the shotgun,
  the sweepers, deleted, ~50 s); Joan's first round read the office and wrote her log; the booth burnt left no
  session behind. The demo hires a real one only with `BURROW_OFFICE` set (its term attaches to her locally).
- **The notes switch is gone**: Linda hired is the notes on (`notes_on()`); the switchboard has four levers.

He: "3d mode: let's generalize this. Add a 3d mode furniture piece. When i click it, i switch to 3d mode. Of
course, I can still click it with my crosshairs enabled, so i get into the doom mode that way."
- **The goggles** (`goggles` kind, `drawGoggles`: a headset on a stand; `furnish` v 12 adds `goggles0`): a click is
  `firstPerson(key, false)`: the same view with nothing in your hands, a click (or E) uses what is under the
  crosshair (`room.use`, `room.label` in fps.js): the gun closet (hardcore) takes a gun or puts it back
  (`fpsArmed`), the goggles take you out, a bot or the office manager leaves 3d and opens. The gun closet in the
  room still starts armed. Hardcore switched off in there puts the gun back (you stay in 3d). The goggles are off
  their stand while you wear them (`fpsWear`), also on your figure.

And, the same evening:
- "The clock should be a wall object, I can drag it onto the floor! On the wall, the drag mechanics should be the
  same as on the floor": a wall piece that can burn (clock, window, a door) no longer comes off the wall while
  carried; it slides along it and settles clear of the others as it moves (the floor's rule, so nothing overlaps
  there either). Only when its free position would be over the fire is it shown there, washed red, to burn.
- "When I'm next to a desk to talk to an agent, I should be standing behind the desk ... I want to be able to click
  the bots, now I'm in front and I click my own avatar": you stand behind the desktop over the bot's shoulder
  (`at.y - 12`), drawn before the desk and the bot, so the pointer on the bot is the bot.
- "When a bot is liquidated, its chair disappears immediately. When it is restored, the chair is put back": a bot
  in pieces, in the booth or stopped gives up its chair at once (`gone` in `layout`), keeping its place in the row
  (`seatRows`): put back together it sits where it sat.

## Several managers, shooting by hand, fear, Doom's bar (2026-10-26, night)

He: "you should allow multiple office managers ofcourse! The canope is a bit over the top, remove that and put
her name on the desk below"; "When i drag a bot for deletion on the booth, it should be the same mechanic as if I
do it in hardcore mode, but then one of the office managers, or me, will go there, grab a gun, and kill the bot";
"Remove the possibility to burn the bots, that's a bit cruel"; "In hardcore mode, add a mechanic where the other
bots are increasingly frightened"; "The 3d mode, make it a little more detailed! Add a bottom bar like in doom";
"3D: also check some inconsistencies, like the fire is floating. Test all the assets, and add 4 or 8 viewpoints
to some of them, especially to the bots"; "When a bot is walking around (idle) and I open their chat, we should
walk towards eachother ... Only when i've sent my message, the bot will go to their desk"; and "Both on the wall
and on the floor, I want there to be overlap! It's the responsability of the user to declutter".
- **Several office managers**: a reception each (room.json `managers: {booth key: {style, id, since}}`; the one
  `manager` entry of the evening's version moves to the first booth; a booth without an entry is Linda). One of
  each style (the popup greys the ones at a booth; the name would be one session's twice). Each session manager
  has her own directory, `OFFICE/<name>` (Claude Code keys sessions by directory), and her manual names her
  colleagues (`write_manuals` on every hire and fire). `state.managers` is a list, `state.removing` the removals
  asked; in the room she is `mgr:<booth key>` (`boothOf`, `managerSeat(booth)`), the page maps a session id to
  that (`asFolk`, `managerOf`). Her name is on a plate on the counter's front; no sign over her.
- **One shooting mechanic** (hardcore): whoever is to go is condemned (hands up where it stands): a robot dropped
  on the gun closet (it stands beside it; the closet goes red, "shoot NAME"), a burnt desk's robots, ✕✕ in the
  list, what an office manager clears out or her session asks to remove. `huntStep` is per shooter (`f.hunt`):
  an office manager if there is one in the room (each claims the nearest target at once, so two never go for the
  same one), else you; gun, walk, aim, shot, the next, the gun back. The gun closet shows a gun gone for each one
  out. In normal mode the booth is as it was; managers escort per manager (`f.clean`, the escort follows `act.by`).
- **No burning robots**: `bury` and `pyre` are gone; a robot let go at the fire warms its hands (`useAt` fire); a
  burnt desk's robots with no booth are deleted and walk out of their door.
- **Fear** (`w.fear`, hardcore only: `FEAR.kill` 0.25 a kill, fading over `FEAR.fade` 240 s): above 0.2 they
  tremble (the figure jitters a dot); above 0.45 a drop of sweat runs down; above 0.35 idle ones flee whoever holds
  a gun (`scare`, reach grows with fear) and a shot is heard further (`alarm`); above 0.5 a break is likely spent
  hiding (`hide` act, `hideSpot`: behind the piece furthest from the gun closet, eyes peeking over its top, which
  `visTop` measures from the piece's picture; never a desk, too low); above 0.6 the ones at work glance over their
  shoulder; fewer come to sweep. `__room.fear(v)` sets it for a look.
- **Meeting halfway** (`meet` act): the focused tile's robot, idle and nothing sent yet, walks to a point halfway to
  you; you stand 14 dots from it, facing it. `room.typed(id, sent)`: your bubble while typing, its dots when you
  pause (`f.reply`); Enter (`sent`) and it goes to its desk, you behind the desk as before. Not a bot you have just
  started (2026-10-27, he: "Not when I just created a bot, then I should walk to its desk, the bot will also walk
  there"): the + marks it (`room.fresh(id)` before its tile opens), it goes to its desk and you behind it, until it
  has had its first message or its tile closes.
- **Overlap**: `settle` is gone. A piece, a desk or a piece out of the closet lands exactly where it is let go,
  over whatever is there (the front one takes the clicks); wall pieces still only slide along the wall.
- **3d**: Doom's status bar (`statusBar` in fps.js, 32 rows under a 168-row view): AMMO (8 shells a gun; put it back in
  the gun closet and take it again for more), HEALTH (five-hour limit left), KILLS (`w.kills`), your face (looks about, its frame lit a
  moment after a kill), ARMOR (week left), the doors as keys (their paint, dark when out of reach), the sessions
  as the ammo table (WORK, WAIT, IDLE, DONE). `room.status()`, `room.face()`. The floor has pools of light under
  the ceiling's lights and darkens along the walls, walls darken where they meet it, every figure and piece casts
  a shadow on it. The fire's pit is on the floor (`drawFire(…, part)`: "pit" onto the floor canvas, "flames" the
  sprite), the ping-pong table a table (boxes faced with its sprite: top, edge, net, legs). Figures have three
  pictures (`views`: front, back from `backOf`, side as the room draws it), picked by the way they walk or look
  against the way to you. All pieces were looked at from the front in 3d (a contact sheet); the rest stand right.

## The goggles go; you stay put while a bot goes (2026-10-27)

He: "Let's remove the 3d furniture actually", "I mean the 3d goggle furniture. 3d view is only used for the
shotgun experience"; and "when i delete a bot, i walk towards its desk, while the office manager is busy grabbing
the gun etc."
- **No goggles**: the kind is gone (`furnish` drops `goggles*` from every room; not in the closet, `KINDS`,
  `CLICK`). The first person is the gun closet's only (`firstPerson(key)`, hardcore): you always hold the gun
  there (`room.armed()` is true), E on the gun closet loads it again (8 shells; "E: reload" under the crosshair),
  hardcore switched off takes you out. fps.js still has its unarmed branches; nothing reaches them.
- **Staying put**: with a bot's tile open you are `with` it, and that act walked you to stand behind its desk
  whatever it was doing; a bot dropped on the gun closet (or into the booth, or in the line for it) kept its chair,
  so you walked to its desk while she fetched the gun. Now, while the bot you are with is condemned, queued, in the
  booth or being walked there, you stand where you are and watch it.

## Round of 2026-10-27: typing is typing, desks named, folders picked, tiles to the edge

He: the bubble over you while killing a bot ("It really should only do that literally when I'm typing"); terminal
colour schemes of ssh sessions ("How do we handle that? Can we override them?"); the switchboard "smaller, and
it's just one big lever"; "I can click on a desk's name, that opens a UI pane about the desk, where I can change
it's name"; the new desk's path "really not user-friendly. Check how other tools do that, like vs code ... Do not
show a hint"; tiles: "when I have two vertical tiles, and I drag one of them down, the hint should be the full
horizontal bottom ... check how other tools do this, like wayland or orca", "dragging to make them wider doesn't
work"; "The floor should not have such a wide horizontal lower bound"; "A bot was standing right behind a desk. I
clicked the desk's monitor ... it acted on the bot".
- **Typing**: xterm's `onData` carries the terminal's own reports too, which Claude Code turns on: focus in/out
  (mode 1004: a press in the room blurs the tile, `\e[O`), mouse reports (SGR, X10), colour and cursor answers.
  Those were counted as typing (the bubble). `REPORT` in index.html filters them; only keys and pastes count.
- **Themes**, looked at: each session draws itself with its own machine's Claude Code `theme` (this Mac `dark`,
  arthroid and odyssey `auto`). `claude --settings '{"theme":"light"}' attach <id>` changes nothing (tested: the
  same colours). `auto` asks the terminal for its background (OSC 11, seen in the attach's output) and xterm.js
  answers with its theme's, so burrow's dark terminal makes the `auto` machines dark. Burrow gives a machine whose
  theme starts with `light` a light terminal, the rest dark. The only real override is the machine's own setting
  (`theme` in its ~/.claude/settings.json, what `/theme` writes), which changes Claude Code there for every terminal;
  not done, his call.
- **The switchboard** is a small steel box (10 x 14) with one big lever, its red knob up; pulled down while the
  settings' panel is open. `SWITCHES` is gone (the panel lists the settings).
- **A desk's panel** (`#vDesk`, `openDesk`; a click on its name plate, `{kind: "desk", unit}`, the plate lit gold
  under the pointer, the tip its whole name): its name (empty = its folder's; room.json `names`, `/api/desk {unit,
  name}`, `hub.rename`; dropped with the desk), its path and machine, its sessions (a click opens one), RENAME,
  TERMINAL. A given name shows up to 14 letters on the plate (a folder's 8), in the notes too (`deskName`).
- **The new desk's folder** (`browse` in index.html, the NEW DESK view): as VS Code's own folder dialog (Remote-SSH's
  "Open Folder"), a path field over a list that is always there: "..", then the folders in that folder (`/api/complete`,
  the machine's `ls -d dir*/`, 300 at most; cached per folder while the form is open); in the starting folder (home)
  first where Claude Code ran lately, the ones with sessions not in the room on top with how many. A click goes in,
  typing narrows, ↑ ↓ and Enter/Tab go in, Tab with one match goes in, Enter with nothing picked or ADD DESK adds the
  folder in the field, a double click adds the one clicked. No prose, no placeholder.
- **Tiles to the edge**, as Sway does at a workspace's edge: within 30 px (or 8%) of the pane's own edge the zone is
  that whole side of the pane (`splitOuter`: the tile beside everything else, along the whole side; one more along it
  if the pane is split that way already); elsewhere a tile's side or middle as before. Works for a bot carried in
  from the room too. **Resizing** was broken: every move laid the tiles out again, which removed the gap being
  dragged (and its pointer capture) after one step; it is followed on the window now (`resizing-row/-col` on the pane
  keeps the cursor and keeps the terminals from taking the pointer).
- **The floor** reaches the room's edges (`L.floor`: 11 dots from the sides, a figure's half width; 2 from the bottom),
  a held figure too.
- **What is in front takes the pointer**: a figure under the pointer is dropped as the one under it when something
  solid is drawn over it there (items carry what of them is solid: a desk its screen, cabinet and desktop; a piece
  its footprint; a reception its panel and counter). A bot behind a desk's screen: the click is the screen's.

## The dog; an empty desk has no desktop; a quieter name plate (2026-10-27)

He: "Let's add a dog house to the furniture. This has a dog going around the floor, resting in it's house,
interacting with robots and my avatar"; "A desk without bots should not yet have that little table for the first
bot"; "the hover on the desk name is way too disruptive, should be more subtle".
- **The dog house** (`doghouse` kind, in the closet, not in new rooms; `drawDoghouse`: plank walls, a red gabled
  roof, a dark arched door, a bowl) and **the dog**, a dachshund (a badger dog, as burrows go): `w.dog`, not a folk;
  `thinkDog` after everyone else, `dogPic(pose, wag)` from letter grids (`DOGS`: stand, walk1, walk2, sit, lie;
  `TAIL` two positions for the wag; outlined in ink as `body()` does). What it does (`g.act`): `home` (in through
  its door, its head out of it, z's when asleep), `wander` (a spot, a sniff), `visit` (beside a bot or you, sitting,
  wagging; an idle standing bot pets it: its arms set after its own think; hearts), `follow` (at your heels),
  `chase` (while `w.ball` is in play), `bark` (at a figure that comes in late: `w.dogNews`, set where figures are
  made), `petted` (a click on it, `{kind: "pet"}`: you get the `pet` act, walk over, reach down). At night (23 to 7,
  this computer's clock), while you are away, and with `w.fear` over 0.4 it stays home (afraid: only its eyes in
  the door). Its house burnt, it trots out through this computer's door. In 3d it is a sprite like the rest.
- **An empty desk** is its cabinet and screen only: `w` is `cab` with no bots, no desktop, legs or keyboard
  (`d.empty` is gone); the first bot brings the desktop with its chair (the desk grows then; he asked for it, over
  the earlier "keeps its one-seat length").
- **The name plate** under the pointer: only its letters turn gold; no fill, no tip.

## The clock is just there, and tells the time (2026-10-27)

He: "the clock is not optional, it's just there. shouldn't be in the furniture catalog. And show the real time."
- Fixed like the fire, the closet and the switchboard (`FIXED` in room.js and server.py: it slides along the wall,
  never burns), out of the closet (`CATALOGUE`) and of the kinds that can be added (`KINDS`); `furnish` puts one back
  in a room that burnt it.
- It was Pixel Agents' 12 x 10 sprite with hands of two and three dots: no time could be read off it. Now it is drawn
  (`drawClock(ctx, x0, y0, awake)`, `CLOCK` its cells): a round face 17 dots across, its rim red above and slate below,
  the hours as dots (the quarters darker), the hour hand short and black, the minute hand long, a red seconds hand
  ticking (not with reduced motion). The awake wedges and the hatching are on its face as before. The hover says the
  time ("00:52"), then the awake state. `__room.clock(awake)` returns a 17 x 17 face.

## Desks follow the sessions (2026-10-27)

He lost a conversation: lucky-lou (started at odyssey's `~` desk) ran EnterWorktree into
`~/docker/.claude/worktrees/immich-ml`; `claude agents --json` reports the current directory, so its unit became
`odyssey:~/docker`, which had no desk, and it walked out of the room mid-conversation while it waited on him. He
then started cozy-cora at the `~` desk (the +) and asked it where the chat went. He: "Maybe it's because we're not
showing bots that don't have desk. That might have been a bad decision, from first principle"; torn, then: "Once,
I saw a bot initializing a new desk and walking between the two. I loved that actually! The fact it gets
cluttered fits the design philosophy: it means it *is* cluttered and you should free up space to maintain a
healthy concentration level and focus". And whether to drop `--all` ("I've been fighting the 'complete' state
all this time"): no, argued and he agreed. Without `--all` a done session drops out once the supervisor stops
its process (an hour unattached, or sooner when memory is low: docs, agent-view), i.e. when you come back to
read it; agent view itself shows Completed; the docs say to poll `--all`. Blocked/working rows stay either way.
- **Every session is in the room** (`state()` carries `hub.bots()`; `on_floor` is gone). `Hub.desks()` = the kept
  desks (room.json `desks`), then one per other unit a session is in, in the order its first one started; each
  has `kept`. A bot whose directory changes gets up and walks to the new desk (tested in the demo by faking a unit).
- **A desk that came with its sessions goes with the last of them**: the page stops drawing it as soon as its
  last bot is deleted from here (`w.stopped`, in `draw()`'s `shown`); the server's list loses it at the next poll;
  two minutes later `fade(mid)` (every poll, `self.absent`) drops its place from the layout, so it comes back at
  a free spot. Its name stays in `names`. Burnt (`/api/forget`), place and name go at once, as before.
- **Kept** desks stay empty: put down from the project cabinet (`/api/dir`), or KEEP in a desk's panel
  (`/api/desk {unit, keep: true}`; the button only shows on a desk that isn't kept). His five desks were kept already.
- The cabinet's list is only where Claude Code ran lately with no desk now (no session counts: every session has
  a desk). The office managers' manual says so; `do: start` no longer puts up a kept desk (the session brings its
  own), `do: desk` does; `/api/office` desks carry `kept`. The notes cover every session again.

## The booth without a countdown; the dog to play with (2026-10-27)

He: "When I delete a bot in normal mode, in the booth. NO progress bar! I've said that before. There's a removal
animation that takes some time, about the same as the progress bar now, and all the time I can click the booth to
cancel." (The "before" was the gravestone round.) And: "I should be able to click on the dog, interact with it."
- **The booth is the clock**: the countdown sign over it is gone. Its five seconds (`BOOTH`, `boothT(f)`: seconds
  in) are drawn: the glass door slides shut from the right (0.4 s, `boothShut`), a light rises from the cabin's
  floor (`boothRise`, 0.6 to 4 s) and the bot goes up in it feet first (`dissolve`: what the light has passed
  turns pale and thins out through a 4x4 ordered dither, `BAYER`), motes rising in the light, the cabin glowing
  more and more (`boothGlow`), the roof sign flickering in the last second; then `claude stop` + `rm` and the old
  fading light. The whole time the booth is a hot spot (gold outline, tip "keep NAME"): a click, or on the bot,
  sets `act.back`, the seconds run back six times as fast (the light goes down, the bot comes back, the door
  opens) and it steps out (`resume`). Carrying it out still undoes at once. `drawBooth(ctx, bo, front, t, flash,
  armed)` takes the seconds in (-1 empty).
- **The dog was unclickable at night**: from 23:00 to 07:00 it sleeps in its house, and nothing there was a hot
  spot (he tried at 1 a.m.). Now the house is in `CLICK` and its footprint is the dog's hot spot too: asleep, a
  click wakes it ("wake the dog"), it walks a few steps out (`petted` with `out`) and sits for you; afraid (fear
  in hardcore) it stays in and you reach in at the door, hearts there ("comfort the dog"). With you it stays up
  at night (`withYou` in thinkDog) and goes back in after.
- **Fetch**: while you pet it (`canThrow`), a click throws a stick (`throwStick`: 40 to 80 dots out, clear floor,
  `w.stick`, a 0.8 s arc, tumbling through four turns, `drawStick`); it runs for it, picks it up, brings it back
  across its mouth (`MOUTH` per pose, pale wood `STICK` so it shows against the dog), drops it at your feet and
  sits wagging 12 s for the next throw ("throw the stick"); then it wanders off and you pick the stick up (your
  `fetch` act). The dog's hot spot is pushed after the pieces' (it stands in front of them), the piece under it
  is not outlined, and its tip goes over your head when you are beside it.
- **Picked up**: a drag on the dog lifts it by the scruff (`dogHeld`, `g.held`, drawn over everything, not into
  a piece move under it); let go, it shakes (`shake`) and goes about its business, into its house if let go
  there, never in the fire (put down in front of it).
- Tested in the demo from a copy of his room (`BURROW_DEMO_ROOM`): the night house click, petting, two throws, the
  carry, the wait, a pick-up and drop (it went home: night); the booth through its five seconds on clear floor
  and an undo at 2.8 s (back out, not deleted). look.mjs takes `clip`/`clipTo` on a shot now; `room.at(id)` is on
  the page's `room`, not `__room` (`__room.dog()`, `__room.stick()` are new).

## No bench; desks walked to; the pouf; the robots' dream (2026-10-27)

He: "just remove the bench"; "a bot spawning another desk, and then going back to its first desk, does not walk
there, it just teleports"; "SUSAN is hard to read, the name on her desk ... the UI to select one ... low contrast
background, make it happier, each has a color"; "the green sofa more rotation invariant: remove the lamp, remove the
backrest"; "I don't want the blinking dot above a working robot's head. I don't want the floating zzzz's above a
sleeping robot. No talking bubbles on a robot playing chess"; and the whiteboard: "it's messy, just a bunch of
arrows. Make it more cozy, a bit funny. It represents robot's dreams ... two happy robots, in love, in the
countryside, running on a sunny hill. Or on the beach. The drawing is done in pixel art."
- **The bench is gone**: the kind, its break, the closet entry, its click and the COMPLETED panel (`fillDone`, `#vDone`,
  its CSS). `furnish` drops `bench*` from every room on load. A desk's panel lists a desk's sessions, completed too;
  the booth deletes.
- **Teleporting to a desk**: `sitAt` snapped a seated bot onto its seat whenever the seat was within 40 dots (meant for
  the chairs closing up along a desk), so a bot whose directory changed to a desk close by jumped there and back.
  Reproduced in the demo (a fake unit and a fake desk 30 dots below, via a `JSON.parse` wrapper on the page: jumps of
  30 dots both ways). Now a seat carries its desk (`L.seat(id).desk`) and the snap is only for the same desk
  (`f.satAt`); another desk is walked to, however close.
- **Her name on the counter** is a desk's plate (`plate()`): dark (her colour at 26%), the letters in her colour
  lightened; it was ink on her colour with a highlight row through the letters. **The picker** (`#vManager`): each card
  in her own colour (`--c` on the button: a pastel card, a bevel in her colour, her figure on a swatch of it, her name
  in it with an ink shadow); the one at a reception already is desaturated, not grey.
- **The pouf** (the kind stays `armchair`, named "pouf"): a round green cushion drawn as a short cylinder from above
  (`pouf()`, Pixel Agents' armchair greens), the same from any side; no lamp, no back. Whoever sits on it is drawn
  over it (`piece(…, dz)`: its depth 2.5 dots behind its front), facing the middle of the room (`poufFace`).
  Then he: "You took the rotation invariance quite literally. In the style of the tool, I actually meant Z_4
  invariance :) Keep them square like they where". So it is square: the old armchair's seat without its back, a
  16 x 12 letter grid (`POUF_ROWS`), its top lit along the front edge, a button in the middle; in 3d (props3d.js)
  a 7-dot box under whoever sits on it, as high as a seated robot's thighs (3.4), its top `poufTop()`.
  Then (2026-10-27): "add little feet to the poofs? They are not easily interpreted as that. And you can bend the
  corners a bit": its corners rounded a dot, two splayed wooden feet (Pixel Agents' wood #885c47, plum #532e3d), so
  the grid is 16 x 14 and the cushion two dots higher: the seat is `y - 4` and the piece's `dz` 4.5 (the sitter over
  it); in 3d the box (z 1-3.4) stands on four legs, centred under the seat.
- **Quieter robots**: no blinking `rc` light over the antenna (the APP button on the tile stays); no z's over a
  sleeping or dozing robot (you still get them, and the dog in its house); no talk bubble on a robot at the chess
  board, and no random chatter there.
- **The robots' dream** (`dreamScene`, `DREAMS`, `w.dream`): the meeting host's random boxes and arrows (`stroke`)
  are gone. A scene is 65 x 27 dream pixels, two by two of the board's ink: the hill (sun, clouds, birds, the grass
  with flowers, a tree) or the beach (sun, gulls, the sea with little waves, sand, a palm, a crab); in both the two
  of them running hand in hand, him teal, her pink with a bow, blushing, motion lines behind, a heart over them and
  two small ones. It comes in layer by layer (sky, ground, things, the two, the hearts), outlines first, left to
  right with a jitter (`paintDream`), at 0.6 pixels a second, 1.5 more for every robot napping (it is their dream),
  8 more while a meeting's host draws at the board (`act.drawing`). A page opens on a dream 30% drawn. Done, it stays;
  a meeting four minutes later, or twenty minutes, wipes it and starts the other. Each layer keeps its own pixels and
  paints over the dream's earlier ones (`mine`), never over your marker; the view's wipe starts the dream again. The
  small board in the room shows each dot's middle pixel now (it showed the first inked one). `__room.dream(scene, k)`
  shows a scene k drawn.

## A bot that is to go lets go of its terminal (2026-10-27)

He: "When i delete a desk, its bot walks to the booth, but the terminal should close right away. The same for when i
delete a bot." The tile closed only when the attach ended, i.e. after `claude stop`, the walk and the booth's five
seconds (or the gun) later. Now `unopen(id)` (room.js, `act {kind: "close", id}`, every tile of that bot) runs the
moment it is to go: `condemn` (a burnt desk's bots, a bot on the gun closet, her hardcore clear-out), a bot let go in
the booth, the office manager taking one to the booth (`escort`), a shot (`shoot`: the gun, 3d). Deferred to a
microtask: `closeTile` draws the room, and `shoot` and `cleanStep` run inside `think()` (a nested draw would think
twice). Kept (undone, rescued, put back together), its terminal stays closed; a click opens it again. Tested in the
demo: a bot dropped in the booth, a desk burnt with its bot's tile open, a shot; the pane empty at once each time.

## The 3d view made for 3d: Doom, not the room's pixel art (2026-10-27)

He: "add more 3d assets, especially the furniture, like the floor manager, coffee machine, printer, the bots should
be 8 angle sprites. The style in 2d is great, but you should not simply port that to 3d. The black outlines do not
work, and it's too low res. Do it more like in doom. Add simple lighting effects, like the fire. The way I hold my
shotgun should be much more similar to doom. The bottom bar as well. Show how much tokens I have left in the bottom
bar. Overall, make it resemble doom more. Also when it starts, use the classic 'floating blood' ... transition".
- **web/props3d.js** (new, before fps.js): everything the view shows, made for it, four texels a dot (`TS`), no ink
  outlines; a texel at alpha `GLOW` (254) shines (the view does not light it: screens, eyes, lamps, embers, flames).
  - **Models** (`render`): boxes in a figure's own frame drawn orthographically from eight sides (k 0 its front,
    counting round to its right), a z-buffer, Lambert light from the front-left, a lit top edge and a dark bottom
    edge, rounded corners on a box (`round`). `robotBoxes(sp)`: legs (walk1/walk2 strides, sit), the torso in its
    colour with a chest light, her skirt and bow (`femme`), arms in every pose the room uses (down, sw1/sw2, up,
    wave, fwd1/fwd2, reach, book, mug, paper, can, roll, gun), the head with the portrait's own face on its front:
    pixel.js got `pixelAvatar.robot(seed)` (its top, antenna, eyes, teeth, picked in robot()'s order). `dogBoxes`:
    the dachshund standing, walking, sitting, lying. `figure(sp, k)` is kept per pose and side; `lying` is the side
    turned a quarter.
  - **Furniture** (`build(p)`, `desk(d)`): boxes faced with painted textures (wood with grain and bevels, steel,
    plastic, `over()` for details and the room's letters), kept by what they show (`once`, `tinted`): the coffee
    corner (counter, brewer, the pot filled to the five-hour limit, a mug), the cooler (its bottle to the week),
    the printer (a page out while it prints), the reception (panel, posts, the counter with her name plate, her
    screen or Linda's stack, the bell), the project cabinet, the booth (glass, its back shining as its five seconds
    run, a light) or in hardcore the gun closet (the guns not taken, a red lamp), the neuralyzer (a light when it
    flashes), the dog house (stepped roof), the chess table (the game as it stands, stools), the meeting table, the
    ping-pong table (net), the whiteboard (the dream's ink on its face), a desk (cabinet and plate, desktop with
    keyboards and mugs, legs, the screen and laptops showing what drawDesk shows, a light while at work, an office
    chair a seat). Round things are pictures: the pouf, a potted plant (two kinds, drawn), the pair table, the
    flames (four frames, bigger while something burns), the fire's pit (on the floor overlay).
  - **The walls**: wainscot panels, a wooden rail, plaster, a cornice (`wallBase`); on the room's wall the doors in
    their paint (boarded while out of reach), the window (the sky by the hour), the clock (its hands at the time),
    the closet, the switchboard (`wall()`, one canvas kept until a minute or a door changes). **Floor**: planks five
    dots wide, staggered, grained (`floorTile`); **ceiling**: tiles and lamp panels that shine (`ceilTile`).
- **scene3** (room.js) now says what is there rather than cropping the room's pictures: the floor overlay at two
  texels a dot (soft shadows under the pieces, the rug, the mats, the oil, the pit; redrawn only when its key
  changes), the wall's items (doors from `L.doors`, the window, clock, closet and switchboard from the layout), a
  spec per desk (`screenOf` gives what a screen shows), the state per piece, the lights (the fire: `flicker`), a
  `fig` per figure (an office manager at her counter `facing` the room), the remains cut from the model's front.
  `pics` no longer has `wallPics` beside it; `sideOf`, `onItsBack`, `backOf`, `panelled` are gone.
- **fps.js**: square pixels, 480 to 640 across (`resize`), the bar 48 high. Light (`light`, `shade`): the room's
  own, a pool under each lamp, darker in the corners, the scene's lights (the fire flickers, `LT`), a shot's flash
  (`flashK`); then Doom's dark by distance (`dimOf`). Mipmaps keep what shines. Models pick their side from the way
  they walk or face against the way to you. The gun (`gun`, `pump`, `flashes`): Doom's view down the barrel, the
  pump on it, the receiver; a shot is the muzzle star and a kick, then the pump (the gun dips and turns, the pump
  comes back and forward), lit by the light where you stand (`ctx.filter` brightness). Doom's messages up left
  (the keys, "NAME IS DELETED.", "PICKED UP SOME SHELLS."). The bar (`statusBar`, 480 wide in the middle of the
  stone): AMMO, HEALTH (five-hour limit), BOTS where Doom has ARMS (W ? I D), your face (red after a kill), ARMOR
  (the week), the doors as keys, and the **tokens left** where Doom lists its ammo: every limit /usage reports
  (`status().bars`, "5H 36/100", "WEEK 77/100", per model when there are such; the session and the week when /usage
  gave no list). Big numbers are Doom's red blocks (`BIG`). **The melt** (`startMelt`, `drawMelt`): Doom's wipe, 160
  columns, each a little after its neighbour, 35 tics a second: going in, the room as it was on the page
  (`room.snapshot()`) melts off the 3d view; going out, the last 3d frame melts off the room (the canvas stays,
  transparent, until it is done). Not with reduced motion.
- Tested in the demo from a copy of his room with hardcore on: every piece looked at from in front, a sheet of a
  robot's eight sides and its poses (`Props3D.figure`), a shot (flash, pump), both melts, ten seconds turning: no
  errors, about 5.5 ms a frame at 480 x 300. `__fps.size()`, `__fps.melting()` are new.

## Doom's grain; robots of steel (2026-10-27)

He: "More like doom grain ofcourse!! They have a weirdly human color now btw." The casing was steel3 (#c2c9d9), lit by
an orange fire it went the colour of skin.
- **Grain** (`grit(i, j, s, k)` in props3d.js): speckle over blotches two and four texels across over stains of eight,
  now and then a dark pit. `pix()` grains every texture it makes (k: wood 0.75, metal and plastic 0.5 in `plainTex`,
  glass and water 0.35 in `tinted`, what shines a third), `over()` grains what is painted on (0.35), the models get it
  per texel (0.55), the gun in fps.js's `crisp`.
- **Robots of steel**: their casing #8f9ab2 (edges #c3cbdb and #3f4660), and a model box of metal is lit from the top
  with a highlight band (`metal`, all but the torso in its colour).
- **Doom's light**: in steps (`STEP` 18 in fps.js's `shade`, its colormaps: bands into the dark), the fire's light
  warm rather than orange (col 1, 0.8, 0.58; k 0.9).

## Susan is a bot; no red text (2026-10-27)

He: "I can't shoot susan at her desk though" (the gun skipped `staff`), and to my plan of a new Susan walking in on her
own: "huh why? she's a subclass of the bot concept, no? she's just a bot like all the others??". So an office manager is
shot like any bot: to pieces, E puts her back together, swept up (or rusted) her session is stopped and deleted
(`gone()` → `managerGone` → `/api/manager {gone: key}`, `Hub.manager_gone`: her id popped, `claude stop` + `rm`). What is
hers alone: her reception is left empty (`vacant` in room.json `managers[key]`, in `managers_view`): no figure, no name on
the counter, its screen off, the tip "empty - a click hires NAME again"; a click hires her again (`manager_open` clears
`vacant`, a fresh session; for Linda the notes come back on, `notes_on` skips a vacant one). Tested in the demo: shot from
in front of her counter, her pieces aged, the reception empty, a click, she is back.
And: "Sometimes a red text flashes at the top. That's never necessary." The room's trouble line (red on the wall: a failed
request's `note()` until the next push, or this computer's poll error) is gone; `note()` writes to the console. The 3d
view's Doom messages ("NAME IS DELETED.", "PICKED UP SOME SHELLS.") are gone too; its key help, the first six seconds, is
grey.

## The README, short; a demo with nothing of his (2026-10-27)

He: "update the readme. It's not up to date. Do new screenshots, do not share any info of mine, use a demo room. Make
it much less verbose. Don't list all the furniture, rather explain the high level concept! ... '.., and much more.'
... the list of floor managers, keep it simple". So the README is the principles, what the room shows, what you do,
the furniture in one sentence, the three office managers a line each, a short table, how to run it. Supersedes the
long one of "README with pictures".
- **The demo was not his-free**: it read his real notes (`digest.json`; a desk's paragraph is keyed by directory, and
  the demo's `~/projects/burrow` is his), its projects were four real folders of his (romuald, hexant, football,
  website), its second machine was named odyssey. Now `load_digest()` is empty in the demo (demo_notes makes them
  up), the projects are made up, the machine is "lab", and the made-up notes no longer ask about the bench. Still
  real in the demo: `~` itself (its paths show as `~/…`), the login name (YOU panel only), `~/.ssh/config` and
  Claude Code's recent directories in the forms.
- **Made-up is not enough**: he, at once: "You put atlas on the readme?? I said don't put antyhing personal there???"
  odyssey has `~/super-atlas`; I had checked the names against this Mac's `~/projects` only. The demo's names are
  now kite, teapot, lantern, pebble, puffin (and burrow), checked against `ls ~ ~/projects ~/.claude/projects` on all
  three machines (ssh, read-only); tidy-tess and lucky-lou, real sessions of his, are sunny-sam and jolly-jo. Check
  every name that goes into a picture the same way.
- **The notes named the home desk after the user** (the folder's last name): `deskName` now says "home", as the
  room's `projectLabel` does.
- **Every terminal was 18 px too tall and 24 px too wide** (seen in the hero: Claude Code's status line half under the
  tile's edge). `* { box-sizing: border-box }` made `.term`'s computed size include its padding, and the fit addon
  takes the parent's computed size and subtracts only the `.xterm` element's own padding (0). `.term` is
  `box-sizing: content-box` now; measured: the screen ends 10 px inside the tile at the bottom, 29 px at the right.
  (The 2026-10-24 note "the fit addon takes it off the parent's size, so nothing is cut" was wrong since then.)
- **The pictures** (1280 x 820 through look.mjs, demo on its own port). The hero is a plain Claude Code session in a
  tile ("Do not use the notes as the first printscreen, that's already very specialized. Show a simple claude code
  session"): `BURROW_DEMO_REAL=demo0005=<id>` puts a real session of this computer behind the demo's teapot bot. The
  session was a throwaway in a made-up `~/projects/teapot` (a tea timer, no git; `.claude/settings.json` with
  `worktree.bgIsolation: none`, or its first line is a red "Cannot create a worktree"), started with the `CLAUDE_*`
  variables of the calling session unset (inherited, they turn its transcript saving off), asked for one small
  change; its header shows version, model, plan and `~/projects/teapot`, no name or email. Stopped and removed after
  (`claude stop`, `claude rm`, its folder and transcript deleted). `room.png` is the top of the room at device pixels
  (`clip`), staged: a `waiting` forced on a bot (the defineProperty trick), a nap (`act {kind: "sleep"}`), you kept
  aside by an interval, `room.talk` fired 0.38 s before each of six shots and the best one kept (a beam is a dash in
  flight); the closet and the limits are the `#view` panel clipped (`clipTo` its rect).

## A calmer room; ping-pong on the keys (2026-10-27)

He: "The pong table should only be controlled with arrow keys, and make them move a bit slower to make it harder. Do
not hide the mouse"; "Text bubbles between bots: don't do it as an idle thing. It draws too much attention"; "the idle
behaviour, make it a bit less busy. The robots [should] longer just keep doing the same thing, sitting somewhere etc."
- **Ping-pong** (pong.js): ↑ ↓ only (the pointer and a finger steered it, and W/S), `PAD` 125 field units a second
  (pair's 170), the pointer visible over the field (`.pong canvas` lost `cursor:none` and `touch-action:none`).
  `__pong.get().my` is your paddle, for tests.
- **No chatter**: the chance bubbles of idle robots are gone (cooler, both sides of a visit, a meeting's host and its
  seats, watching a game). A bubble is a real message between sessions (`talk`: the sender, the receiver when the
  beam lands), or you typing and the robot you talk to answering.
- **Long spells** (`pickBreak`, `rest`): two to three times as long as pair's: the pause between breaks 6-16 s (2-7),
  the pouf 45-90 s, a nap 60-150, the cooler 25-50, ping-pong 40-80, football 40-80, chess alone 45-90 and with a
  partner 90-180, a visit 15-30, the fire 20-40, the window 12-25, standing about 8-20, a meeting 45-75 and at most
  every three minutes (one). Measured over three minutes in the demo after a 30 s warm-up: an idle robot changed what
  it did 2.0 times a minute (3.4 before); bubbles away from the desks in 36 samples of 250 ms (686 before), all from
  the demo's made-up messages.

## Next

- More of Claude Code in the room: the file explorer / diff hanging off a project's screen; maybe a
  session's context filling up, its model, its permission mode (all would need a coherent place first).
- A room that fills up: more than six projects overflow the default slots into any free spot.

`.app`: `macos/bundle.sh` (ad-hoc signed; icon from `burrow --icon-dots`).
