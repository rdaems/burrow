# burrow, native

The room as a macOS app on [GPUI](https://gpui.rs) (Zed's UI framework): the same room, the same Claude Code
underneath, no browser and no Python server. It reads and writes the same `~/.config/burrow/` as the web app, so
don't run both against it at once (`BURROW_CONFIG_DIR` gives one its own).

```sh
cargo run -p burrow-app --release        # the app
BURROW_DEMO=1 cargo run -p burrow-app    # made-up bots: nothing real is touched
macos/bundle.sh                          # target/release/Burrow.app
cargo test                               # everything, headless
```

## How it is put together

The room is pixel art, so it is drawn the way the web page drew it, into a buffer, and GPUI only shows that buffer
at whole device pixels. Everything with logic in it is plain Rust without GPUI, and is tested without a window.

| crate          | what it is                                                                    | from web/            |
|----------------|-------------------------------------------------------------------------------|----------------------|
| `pix`          | an RGBA buffer and the part of the browser's 2D canvas the room drew with      | (canvas)             |
| `burrow-model` | the state, field for field the JSON server.py pushed                           | `state()`            |
| `burrow-room`  | the room: layout, what everyone does, the painters, the pointer; draws itself  | room.js, pixel.js, sprites.js |
| `burrow-core`  | the backend in process: polls, room.json, ptys, the notes, office managers    | server.py, chess.py  |
| `burrow-term`  | a terminal as a GPUI view, on alacritty_terminal                              | xterm.js             |
| `burrow-games` | the window's outside with its duel, ping-pong                                  | outside.js, pong.js  |
| `burrow-sound` | the room's sounds, synthesized as sound.js made them                          | sound.js             |
| `burrow-fps`   | the gun closet's first person, a small Doom engine                             | fps.js, props3d.js, doom/ |
| `burrow-app`   | the window: the room, tiles, the view's panels, the notes                      | index.html, tiles.js, burrow.css |

`PORT.md` says what of web/ is where and how far.

## Tests

- Golden pictures from the real web room: `tools/golden.mjs URL cases.json outdir` loads the page in headless
  Chrome (from a `BURROW_DEMO=1` server, never a real one), with room.js's insides exposed and its dice seeded,
  and saves what a JS expression draws. `crates/room/tests/` compares avatars, figures, lettering, every painter
  and the layout with them, pixel for pixel. The games and sounds have their own (`crates/games/tests/web`).
- The room left to itself: `crates/room/tests/sim.rs` runs the demo for ten minutes and checks it stays sane,
  and writes `target/room-demo.png` to look at.
- The backend runs against a fake `Runner`: no test calls claude or ssh.
- `tools/sprites.mjs` regenerates `crates/room/src/sprites_gen.rs` from web/sprites.js.

## Notes

- GPUI's shaders are compiled at run time (`runtime_shaders`): no Metal toolchain needed to build.
- The fonts are the page's, as TTF (`assets/fonts`).
- The office managers' sessions curl `/api/office`: the app serves only that, on `BURROW_PORT` or a free port, and
  only by its own name (Host localhost or 127.0.0.1 at its port) to what sends no Origin (a foreign one gets 403).
