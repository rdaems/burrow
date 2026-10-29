# burrow — notes for Claude sessions

Rembert's own replacement for Orca (too bloated). Read README.md first. This file is how burrow is **now** and
why; `docs/history.md` is the story of every round from 2026-10-20 to 2026-10-28 (the old CLAUDE.md, word for
word): grep it for why something is as it is, but don't take it as current.

**Keeping this file**: change the section a thing belongs to, in place. No dated round sections: the commit
message is the log. When something goes, take it out here. Keep the file under about 25k characters (it was
190k on 2026-10-28, over Claude Code's limit, and every session loaded all of it).

## What he wants

- Persistent Claude Code sessions: closing the app never stops them, reopening reconnects. Machines over ssh,
  all in **one room**.
- "It's ultimately a frontend for the functionalities of Claude Code! Any action you take is maximally embedded
  in this room representation, and any state of the Claude Code system is also embedded in this representation,
  in a maximally coherent and consistent way." A wrapper on `claude agents`, mapping it closely; burrow keeps only
  what Claude Code has no notion of (desks, the layout, furniture, your face).
- **State is shown only by where a bot is and what it does.** No notifications, badges, counts, red error text.
  No progress bars or countdown signs: a delay is the object's own animation, and a click on it cancels.
- A click on a bot opens its session as a real terminal (the actual `claude` TUI through `claude attach`):
  nothing of Claude Code re-implemented.
- The look: pair's room (rdaems/pair-prompting, pair.daems.me) for the UI and the figures, Pixel Agents'
  furniture, 16-bit throughout. Keep it simple; he notices clutter ("we've overdone it a bit").
- He decides what he hasn't decided yet; say what you'd do and why, and hold a position you can argue.
- He can't be your eyes: look at the result yourself (see Testing) before calling anything fixed. Twice a
  "fixed" drag was not fixed in his room; it cost his trust.

## The code

- `web/` is the app. `python3 web/server.py` (stdlib only: polls, ptys, a hand-written websocket, SSE) serves
  http://localhost:7777 from the main checkout; it reads the files per request. He works *in* burrow (the app) since
  2026-10-29, on main as it is: a change to the page shows when he reloads (⌘R), a change to server.py when he restarts
  the server (burrow ▸ Restart Server, ⌘⇧R), which closes his attached terminals and ends his shells (the sessions go
  on). So never restart or kill his server yourself: say a restart is needed. Test on the demo before it lands. It answers only its own page
  and programs that send no Origin (`trusted`: Host localhost or 127.0.0.1 at its port; a foreign Origin gets 403):
  a websocket is not held to the same-origin rule, so any page in his browser could otherwise open a shell here.
  - `index.html` the page (tiles, terminals, the view's panels, the act handler), `room.js` the room (layout,
    think, draw, pointer), `tiles.js` the tiling, `sprites.js` letter-grid sprites, `paintings.js` the easel's
    paintings, `pixel.js` and the fonts pair's (avatars, dithers), `burrow.css`, `sound.js`, `outside.js` (the
    window), `fps.js` + `props3d.js` + `doom/` (the first person), `chessboard.js`/`chess.py`/`pong.js` (pair's
    games), `look.mjs` (tests).
  - xterm.js 5.5.0 and its addons (fit, clipboard, web-links) from jsdelivr.
- `src/main.rs`: burrow's window (wry 0.57, tao, muda; his choice over porting, the egui app is in git's history):
  the system's webview on the same page, no title bar to be seen (transparent, in the page's night, no title; like
  Spotify's), a dock icon, the keys a browser keeps (no menu takes ⌘W: the page
  closes the focused tile), links in the browser, OSC 52 copies via IPC (`clip:`). No server on the port: it starts
  one, detached, from `$SHELL -lc` (log `~/.cache/burrow/server.log`) in the checkout it was built in, `BURROW_WEB`,
  or the bundle's copy; Restart Server in its menu. `macos/bundle.sh` (in the main checkout only: the app keeps the
  path it was built in) builds and installs /Applications/burrow.app, nothing left in target/. Test with `BURROW_PORT`,
  `BURROW_SHOT=<png>`, `BURROW_TEST_JS` (a window opens for a few seconds).
- burrow's own state, `~/.config/burrow/`: `room.json` (layout, furniture, desks kept, names, width, you,
  settings, paint, pair, managers, awake; `v` is its migration number, `furnish` in server.py, now 13),
  `digest.json` (the notes), `office/<name>/` (the office managers' sessions). Uploads in `~/.cache/burrow/drop`.

## Claude Code, as burrow uses it (all verified on 2.1.30x–2.1.312)

- **The room is `claude agents --json --all`**, polled every 3 s focused, 15 s unfocused (it costs ~0.5 s CPU on
  the machine; odyssey's CPU is precious: don't poll faster). Keep `--all`: without it a finished session drops
  out once its process is stopped, i.e. when you come back to read it; agent view shows them, the docs say to poll
  `--all`. `kind: "interactive"` rows (a `claude` in someone's terminal, the Claude app's own chats under a
  `claude remote-control` service) are hidden, as agent view hides them: nothing can attach to them.
- **Tempo, not state.** The job record `~/.claude/jobs/<id>/state.json` has `tempo`: active (a turn runs), blocked
  (waits for you; `needs` says for what), idle. `state` is Claude Code's guess from how the last message ends and
  says `working` for a session that has only a forgotten background command. So: busy = tempo active with a live
  process; **needs input** = `status: "waiting"` in the list (a dialog open; `waitingFor`: "permission prompt",
  "dialog open", "input needed", "sandbox request", "goal proposal"); a last message that only reads as a
  question is idle; no tempo (old record): busy→active, waiting→blocked. The JOBS grep takes `tempo`,
  `updatedAt`, `detail`, `bridgeSessionId`, `resumeSessionId`, `worktreePath` from the records in the same round.
- A row without a process: `claude agents` falls back to the record's first conversation and starting folder;
  burrow takes `resumeSessionId` and `worktreePath` from the record instead (a /clear or a worktree since).
- The supervisor stops a finished session's process after about an hour unattached; attaching restarts it from
  its conversation. So completed and running-but-idle look the same in the room: idle. Every session `--all`
  lists is a figure; the booth (`claude stop` + `claude rm`, the transcript stays for `--resume`) is how one goes.
- A bot just started isn't in the feed until the next poll: keep a session while its attach is alive.
- `claude stop <id>` (not the name). `claude --bg` may print "Starting background service…" first; an unnamed one
  prints `backgrounded · <id> (idle — …)`: take the first word after `·`. `claude attach` in a brand-new,
  untrusted directory fails until `claude` ran there once.
- Started from the room: `claude --bg -n <pair-style name>`, plus `--remote-control` (setting rc, on: also in the
  Claude app; the record's `bridgeSessionId` cse_… gives `https://claude.ai/code/session_<id without cse_>`, the
  tile's APP button) and `--dangerously-skip-permissions` (setting yolo, off).
- `exit` in an attached session does not end `claude attach`: it drops into agent view in the same pty (no title;
  recognised by "describe a task for a new session"). The tile closes there; the session goes on.
- An attached session is fullscreen and captures the mouse: Claude Code copies a drag itself (pbcopy here, OSC 52
  from another machine: the clipboard addon, write-only, a read gets ""). Option+drag selects in xterm
  (`macOptionClickForcesSelection`). Links (OSC 8 and plain URLs) open on a click (`linkHandler`, web-links).
  ⌘← ⌘→ ⌘⌫ send Ctrl+A/E/U as Ghostty does, Shift+Enter ESC CR. Only keys and pastes count as typing in the room
  (`REPORT` filters focus, mouse and colour reports, which Claude Code asks for).
- A dropped file is **pasted** (`term.paste`, bracketed): Claude Code makes a pasted image path `[Image #n]`, a
  typed one stays text. Uploaded first to the machine (scp is SFTP mode: the remote path is literal, don't quote).
- No command messages a background session: `deliver()` attaches in a pty of its own, waits for the screen to
  settle, pastes, Enter, lets go (SIGHUP). `/clear` (the neuralyzer) the same way.
- Messages between sessions: a `type:"user"` line in the **receiver's** transcript with
  `"origin":{"kind":"peer",…,"verifiedPeerPid":<sender's pid>,"name":<sender>}`; messages between sessions in
  different permission modes are held and ignored. The poll greps the last 4 MB of transcripts touched in the last
  minute (a receiver writes ~280 kB right after).
- Limits: `claude -p /usage --no-session-persistence` every 5 min (~2 s), run with `exec` and its pid left out of
  the room (it shows in `claude agents` for half a second). One account: one coffee pot and bottle.
- Terminal colours: a machine whose Claude `theme` (its `~/.claude/settings.json`) starts with `light` gets a light
  terminal, the rest dark; `auto` asks the terminal (OSC 11) and xterm answers dark. `--settings` on attach does
  not change it; only the machine's own setting would (his call, not done).
- Over plain ssh `claude` is not on the PATH: burrow prefixes `PATH`. `screencapture` triggers macOS's screen
  recording prompt for the terminal app: never.

## Machines and doors

- Machines = this one (id `""`) + `sshConfigs` in `~/.claude/settings.json` (Desktop's SSH connections;
  `sshPort`, `sshIdentityFile` used), reread every round; `claude agents` lists one machine, so each is polled over
  ssh (Remote Control's list has no CLI). burrow writes that file in place:
  `json.dumps(indent=2) + "\n"` round-trips his settings byte for byte.
- A door a machine, a wall piece anywhere along the wall; bots come and go through their machine's door. Out of
  reach = boarded up, why on hover. A door from the closet hung on the wall adds a machine (the ssh host, completed
  from `~/.ssh/config`); a door into the fire, or "remove" (asked twice) in the doors' panel, takes it out of
  sshConfigs (the machine itself is left alone). This computer's door only slides and never burns.
- A click on a door: the doors' panel, a line a machine with its paint. A machine's paint (a palette swap,
  `PAINT`, room.json `paint`, else by order; this machine wood) is on its door and its desks: no name prefixes.

## The room

- A fixed width in dots (room.json `width`, the divider between room and pane), as tall as the window, two CSS
  pixels a dot; the wall reaches the top (no header). Pair's 3x5 font, a letter pixel a dot, everywhere in the room:
  leave the lettering alone (two other tries were reverted). Text stays out of the world: names cut short, the rest
  on hover.
- Pixel Agents' manner (pixel-agents-hq/pixel-agents, MIT; its sprites in `web/sprites/` with its licence): seen
  from above at a slant, wood lit along the front and outlined plum #532e3d, metal #3f3740, a shadow row,
  planks #734728, a navy wall #28374b; the rest drawn as letter grids in that palette. Robots, you and the
  lettering are pair's.
- Gold (#f6c784, `GOLD`; the focused tile's band is the same pastel gold) outlines only what a click does
  something on (`CLICK`). Red = danger: what is held over the fire flashes red. Terracotta is your clothes.
- What a robot shows:
  - **working**: seated at its desk, typing; the screen's bars scroll.
  - **needs input**: seated, hand up, a bubble with a `?` (pair's glyph, steady), the screen's cursor blinking.
  - **idle** (done, process off, or only a question in words): breaks, long ones (`pickBreak`, 6–16 s between):
    the pouf, a nap on a mat it rolls out itself, coffee, the cooler, the printer, watering plants, the window,
    chess, ping-pong, football, the fire, a visit, watching a game, a stretch, coffee for a busy colleague,
    painting at the easel, a meeting by it. No idle chatter: a bubble is a real message or you typing.
  - **a message to another session**: a beam from the sender's antenna into the receiver's.
- Elsewhere in the room: the coffee pot = the five-hour limit, the cooler's bottle = the week (a click: a bar per
  `/usage` line); the clock tells the real time and keeps the Mac awake (Amphetamine: off, while local bots work,
  1/3/8 h; `caffeinate -i -s -w <server pid>`, one at a time under `caff_lock`), shown as yellow wedges on its face,
  whole while bots work, hatched while waiting for work. The window opens onto the outside by the hour.
- Sound only while a radio stands in the room and is on, and only for real events (a hand going up, a beam, a door,
  the booth, a shot, the printer when clicked, a game you play or watch, the fire taking something): no ambient
  noise, no keys, no crackle. Synthesised in the browser; Doom's own in the first person.
- Halloween while a pumpkin (`pumpkin` kind, from the closet) stands in the room and is lit (settings `halloween`; the
  pumpkin is the switch, like the radio): everyone in costume (`dressOf`: a bot's by its seed, a witch's hat, a vampire's
  cape or a skeleton's suit; you the hat, the office managers the cape; `body()`'s `dress`, the hat makes the picture
  `HAT` dots taller), cobwebs in the wall's top corners with a spider going up and down. Not in the 3d models yet.

## Desks

- A desk is a **unit**: `<dir>` here, `<machine id>:<dir>` elsewhere, a linked worktree folded into its repo (asked
  once per new cwd: `git rev-parse --show-toplevel` ≠ the parent of `--git-common-dir`; `/.claude/worktrees/` as a
  fallback), shown as a laptop on the repo's cabinet, cabled to the big screen.
- Every session has a desk. A desk nobody keeps goes with its last session (the page drops it at once, the server
  forgets its place two minutes later). Kept desks (room.json `desks`: put down from the project cabinet, or KEEP in
  the desk's panel) stay empty. A bot whose directory changes gets up and walks to the new desk (never jumps: the
  seat snap is only within one piece). A full room is a full room: the booth clears it.
- A chair for every session, in arrival order (`seatRows`: who sits keeps the chair, a newcomer takes the far
  end). An empty desk is its cabinet and screen only; the first bot brings the desktop.
- On the desk: the screen (a click: your shell there, one per screen, kept by the server with 200 kB of replay,
  killed with the desk, not kept over a server restart; `exit` ends it), `+` beside it (a new bot; in a folder Claude
  Code there doesn't trust yet `claude --bg` refuses, so that terminal opens with `claude` in it, its trust prompt
  showing, and the bot is asked for again every 3 s, taking the terminal's place once it starts: `trustFirst`), the name plate
  (8 letters, a given name 14; a click: the desk's panel, rename, its sessions, TERMINAL, KEEP).
- The project cabinet adds a desk: a folder browser as VS Code's Open Folder (a path field over the folder list,
  where Claude Code ran lately first). No prose, no placeholder.

## You

- Pair's human figure in terracotta; the YOU panel (a click on you) sets the face (seed in pair's form
  `<start>-<i>`) and the name (empty = the login). Both reach every linked pair room.
- A session's tile focused: an idle bot you open meets you halfway; once you send (Enter) it goes to its desk and
  you stand behind the desk at its shoulder. A bot you have just started: both of you go to its desk. You stay put
  while a bot you are with is being removed. Your bubble only while you are literally typing.
- A click on bare floor walks you there (and closes the tiles); drag yourself onto a piece to use it; asleep on a
  mat once the window has been unfocused a minute.

## Furniture

- room.json `furniture` (keys kind + number), placed exactly where it is let go: **overlap is allowed**, decluttering
  is his job; wall pieces slide along the wall. What is drawn in front takes the pointer.
- The closet (on the wall) holds the catalogue; drag a piece out into the room. A new kind goes in `CATALOGUE`
  (index.html) **and** `KINDS` (server.py), `NAMES` and `CLICK` in room.js, `furnish` for older rooms.
- Fixed (`FIXED` in room.js and server.py): the fire, the closet, the switchboard, the clock, this computer's door.
  Everything else burns in the fire (a desk with bots sends its bots to the booth).
- What a click does: printer = a report on fanfold paper rising out of a wide printer in the pane, funny about the
  room's statistics; pouf = you sit; plant = you water it; window = the duel; easel = the canvas, big; chess and
  ping-pong = play or watch; coffee or cooler = the limits; clock = keep awake; switchboard (one lever) = settings
  (rc, yolo, hardcore: a slider and a name a line, a help box under them for the line under the pointer, empty
  once the pointer leaves the panel; hardcore stays vague); pair table = its
  pair room; dog house = the dog; radio = sound; filing cabinet = FILES on the bands; pumpkin = Halloween; door = the doors' panel; project cabinet = a new desk; reception =
  the office manager. Dropped on: the neuralyzer gives an idle bot `/clear`; the booth deletes.
- Whoever is at a piece rides along when it is dragged: a spot at a piece carries its key (`on`), `ride()` moves
  them each frame. One rule for every piece: reuse it, don't add a fix per piece.
- The easel (kind `board`, the whiteboard it replaced) holds the robots' painting: famous paintings with every
  person in them a robot (`paintings.js`: Mona Lisa, The Starry Night, The Scream, Girl with a Pearl Earring, The
  Great Wave, American Gothic; public domain), dots in layers in a painter's order. Every person (or animal) in one
  has the room's robot head (`head()`: a steel box, a dark screen with glowing eyes, ear bolts, an antenna) and steel
  hands and neck; an oval face in metal read as a human (he: "I don't see robots"). It grows only while a robot
  paints at it (`paint`, an idle break now and then, 8%, palette and brush, 30 dots a second); done, it hangs ten
  minutes, then the next painter starts another. The room shows it at a third of its size; a click shows it framed,
  CANVAS in the band and under it only "By abc-abc, def-def and xyz-xyz.": its title is never said (not on hover,
  not in the report). Nothing of yours goes on it.
- The dog (a dachshund, `w.dog`, with the dog house): wanders, visits, follows you, chases the ball, sleeps at
  night (23–7); a click pets it, then a click throws a stick; it can be picked up.
- The pair table is one pair room (`BURROW_PAIR`, default pair.daems.me): new or linked by your page or an invite; a
  bot dropped on it gets the room's bring link typed into its session; a click shows the chat, and while it is open
  you stand beside the table holding its telephone's handset to your ear, the cord coiled back to the base (`phone`
  act and arms, `app.pair()`); it goes back on the cradle when the chat closes. A bot posting in a table's room
  beams to its telephone, one reading it gets a beam back (`pairtalk`): pair's agents curl `<pair>/a/<token>/…`,
  found by the same transcript sweep as messages between sessions (`PAIR_GREP`); the table by the room pair named at
  the bot's join. pair is behind Cloudflare: send a User-Agent. pair names its agents itself (a `?name=` would need
  a change in pair).

## Removing a bot

- **The booth**: a bot let go in it stands inside; five seconds of the booth's own animation (the door shuts, a
  light rises, the bot dissolves feet first); a click on the booth or the bot runs it back, carrying it out undoes;
  then `claude stop` + `claude rm`. Never a countdown sign.
- **Hardcore** (a setting): the booth is a gun closet. A bot dropped on it, a burnt desk's bots, what an office
  manager is asked to remove: condemned. An office manager, or you, takes a gun and shoots. While she fetches the
  gun it is only taken aback (a start, then it stands watching her, and starts at every shot); it panics once the gun
  comes its way (she walks over to it; you in the first person), and freezes when aimed at. The remains (its body in parts, cogs, oil) are the undo: a click puts it back
  together. Idle bots sweep the pieces; the last one gone, the session is deleted (rust after 30 s). Fear grows with
  kills (trembling, hiding, fleeing whoever holds a gun).
- **The mutiny** (hardcore, `mutinyTick`/`mutinyStep`): three of *your* kills within about 45 s (`w.blood`, not an
  office manager's) and the idle bots gather away from you, talk in turn (dots), stroll to the gun closet one by one,
  each takes a gun and its eyes go red (`f.red`: `body(…, red)`, props3d `sp.red`), then close in on you, slowly,
  one shot at a time across them: 3–5 wide (dust beside you), then you are hit. You lie in a pool, the room goes red,
  YOU DIED and RESPAWN drawn in the room (only that takes a click); RESPAWN brings you in by the door, they carry the
  guns back, fear is gone. One that gets work or is opened drops out; none left standing, it is over. In the first
  person their shots are Doom's damage (`room.incoming`, fps.js `takeShot`), you can shoot back, and dead the view
  shows YOU DIED and a click respawns both. All of it is the room's own: no session is touched (except a mutineer you
  shoot, as any bot).
- The fire takes furniture and desks, not bots (a bot at the fire warms its hands). A bot that is to go loses its
  tile at once.
- In the first person a shot that finds no bot hits the furniture in the line of fire, at any height (Doom aims up and
  down for you; `inLine()` reads the id buffer's middle columns, boxes carry their piece or desk: `whose()` in
  scene3). Three blasts within 20 s and it goes as into the fire (`room.hit`, `breakPiece`): Doom's barrel explosion,
  a scorch on the room's floor fading over two minutes. Not what the fire won't take (the fixed ones, a desk with
  bots), nor the gun closet.

## Office managers

- The reception (`manager` kind), several, one of each style (`MANAGERS` in server.py): **Susan**, her own
  `claude --bg` session in `office/Susan`, whose CLAUDE.md burrow writes (the office, `/api/office`: GET the room,
  POST remove | desk | start | say); **Joan**, the same with `/loop 30m` rounds logged in `rounds.md`; **Linda**,
  the notes (Stijn's): no session, `claude -p` (`BURROW_DIGEST_MODEL`, default claude-sonnet-5-5) writes notes on
  each session when it leaves busy, starts waiting or goes over.
- Her session is not in the room as a bot but she is one: typing at her counter, hand up when her session asks,
  shot like any bot (then the reception stands empty until a click hires her again). A click opens her terminal,
  or Linda's notes tile: Needs you, Working, Waiting, Idle; each open decision with Explain and Send (`deliver`).
  The notes are a tile at the screen's resolution (system sans), not a 16-bit panel: "the 16 bit room is perfect for
  an overview, notes should help the user really understand in depth what is happening".
- A removal she is asked for goes through the room: she walks the bot to the booth, or shoots it in hardcore;
  one you rescue is kept. Nothing is cleared out on its own.

## Tiles, terminals, the view

- `tiles.js`: a split tree. A click on a bot replaces the focused tile, shift-click adds one beside it, a click on
  the floor closes them all; drag a tile by its band onto a side (splits) or the middle (swaps); within 30 px of the
  pane's edge the zone is that whole side (Sway); gaps drag to resize; a bot carried out of the room onto the pane
  opens there. Tiles are 16-bit windows (ink ring, steel bevel, dithered band in the pixel font, the pastel gold
  when focused or hovered), 6 px gaps. Kinds: bot, shell, notes.
- `.term` is `box-sizing: content-box` (the fit addon takes the parent's computed size).
- **Files** (`openFiles`, kind `file`): one tile beside a session's or a shell's terminal, the tree on its left and
  the file picked on its right (he: "the sidebar with file explorer and editor are just one thing"; the tree in the
  terminal's own tile was tried and dropped). FILES on the terminal's band opens it (again: focus, or close if focused),
  while a filing cabinet (`files` kind) stands in the room and is on (settings `files`; the cabinet is the switch, like
  the radio). One a terminal; a README at its top opens first. The tree (`makeTree`): rooted where the terminal runs (the
  bot's `cwd`, the shell's folder), git's changes coloured (gold changed, green new, a dot on a folder holding some),
  what changed in the last two minutes bold with a dot; `/api/tree` (`TREE`, python3 on the machine: the open folders and
  `git status` in one round) every 15 s and when the bot's turn ends. A file dragged onto a terminal goes in as `@path`
  (Claude Code reads it with the message), its path in a shell (`pathFor`); @ on the band does the same. The editor:
  CodeMirror 6 in `web/editor.js`, bundled by `web/editor/build.sh` (versions in its package.json, licences in
  LICENSES.txt; the page takes the parts, its setup is in index.html), loaded when the first file opens. ⌘S saves through
  `/api/file` (`FILE`): only if the file has not changed since it was read (mtime in ns), through a temporary file renamed
  over it, mode and symlinks kept. Markdown opens rendered (marked + DOMPurify: the page can open shells, nothing in a
  file may run; pictures through `/raw`, links to other files open here), SOURCE / PREVIEW on the band. Every 4 s it
  looks at the file: changed while nothing is unsaved, it comes in; with unsaved edits a red badge and THEIRS / MINE.
  Closing on unsaved edits warns once. While it has the focus you stand at the desk's screen.
- **The view** (`#view`): one 16-bit panel in the pane (pair's door.css: ink ring, bevels, the dithered fill, the
  pixel font, steel buttons), centred, as wide as its kind needs (640 px at most); opening it closes the tiles; the
  floor or Esc closes it. Every popup is one of its panels: keep them the same, simple, streamlined. Fields and
  `<code>` in the terminal's font (the pixel font has no `~`); completions in a steel list (`completes()`).
- Build a panel when it opens, and on a state push only if what it shows changed: rebuilding the closet on every
  push threw away the item being dragged.

## Games

- Chess (pair's board without its list of moves, square corners; `chess.py`, the server plays two plies) against an
  idle bot drawn by lot; bots play each other for real on their breaks; a click on their table watches. He wants it
  kept simple: no animated or 3d pieces (a Battle Chess-like board was offered and declined, 2026-10-29). Ping-pong (`pong.js`, ↑ ↓ only, the pointer stays
  visible), the same. Football between idle bots.
- The window: an artillery duel on the hills (`outside.js`, the genre of VGABomb and Gorillas, not a copy), you
  against an idle bot that knows the physics; two bots can duel and you watch.
- The first person (hardcore, the gun closet: you walk there and take a gun first): `fps.js`, a Doom engine of our
  own with Doom's numbers (640 x 400, PLAYPAL/COLORMAP light, BSP, 35 tics, the status bar with your face, the
  melt), the office in 3d (`props3d.js` models, eight sides), the mysterious skull door in its south wall (only in
  3d) straight into E1M1 as a portal, both ways, no wipe; `doom/level.js` the game. Data: the Freedoom extract in
  `web/doom/` (BSD, `extract.py`); id's WAD in `~/.config/burrow/` is served instead and must never go in the repo.

## Testing (without bothering him)

- The demo: `BURROW_DEMO=1 BURROW_PORT=<port> python3 server.py` from `web/` (made-up bots and machine "lab";
  nothing saved). `BURROW_DEMO_ROOM=<a copy of room.json>` starts it from his room, `BURROW_DEMO_REAL=demo0005=<id>`
  puts a real local session behind a demo bot, `BURROW_OFFICE=<dir>` lets it hire a real office manager,
  `BURROW_WAD=<path>` any WAD. Pick a free port: not 7781, not 7783 (his Chrome keeps a tab there that posts its
  own layout); check `lsof -nP -iTCP:<port>`.
- `node web/look.mjs URL W H steps.json`: headless Chrome over CDP. Steps: `wait`, `js`, `click`/`clickTo`,
  `move`/`moveTo`, `drag`/`dragTo` (`hold`, `alt`), `release`/`releaseTo`, `key`, `press`, `cdp` + `params` (e.g.
  `Input.dispatchDragEvent` with `files` for a file drop), `shot` (`clip`/`clipTo`). Chrome's own `--screenshot`
  doesn't run requestAnimationFrame. An "exception … localhost:<other port>" means it drove another session's
  Chrome: rerun.
- Hooks: `room.at(id)` (on the page's `room`), `__room` (`world`, `layout`, `game`, `football`, `clock`, `art`,
  `fear`, `dog`, `stick`, `shoot`, `unshoot`, …), `__fps` (`set`, `turn`, `fire`, `use`, `under`, `level`, `cam`,
  `snap`, `hom`), `__pong.get()`, `__outside` (`get`, `aim`, `fire`, `solve`, `wind`). A bot waiting:
  `Object.defineProperty(f, "waiting", {get: () => true, set() {}})` on a folk (think resets it every frame).
- Drags: slowly (seconds, many steps) against a room that pushes often (his room through `BURROW_DEMO_ROOM`); quick
  drags in the quiet demo passed while his failed.
- A real session for a test: a throwaway Haiku in a made-up folder, the calling session's `CLAUDE_*` variables
  unset (inherited, they turn transcript saving and trust saving off); the trust prompt's cursor starts on "No,
  exit"; `.claude/settings.json` with `"worktree": {"bgIsolation": "none"}` in a scratch folder. Afterwards
  `claude stop`, `claude rm`, the folder and its transcript deleted.
- Nothing of his in public (README, pictures, the demo): check every name against all three machines (this Mac,
  arthroid, odyssey). The README's pictures come from the demo at 1280 x 820.
- On looks, a second model (Fable) has given good critiques: ask when stuck, weigh it.

## Open

- More of Claude Code in the room: a file explorer / diff off a project's screen; a session's context filling up,
  its model, its permission mode (each needs a coherent place first).
- A machine's Claude theme as an override; pair's names and colour for burrow's bots (changes in pair).
