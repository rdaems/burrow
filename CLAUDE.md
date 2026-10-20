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
- `claude --bg` may print "Starting background service…" before the `backgrounded · <id> · …` line.
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

## Next

- (open) He hasn't asked for more yet. Ideas only if he wants them: the "has a question" state
  (see above), more rooms in one window.

`.app`: `macos/bundle.sh` (ad-hoc signed; icon from `burrow --icon-dots`).
