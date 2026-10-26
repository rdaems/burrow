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
  (`#vClock`, `fillClock`, `AWAKE`), the clock's light (`SP.Y` dot) while the inhibitor runs, the tip says
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

## Next

- More of Claude Code in the room: the file explorer / diff hanging off a project's screen; maybe a
  session's context filling up, its model, its permission mode (all would need a coherent place first).
- A room that fills up: more than six projects overflow the default slots into any free spot.

`.app`: `macos/bundle.sh` (ad-hoc signed; icon from `burrow --icon-dots`).
