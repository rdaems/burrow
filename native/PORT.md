# The GPUI port: what of web/ is where

One line per web module; status: todo, wip, done (tested), n/a.

| web/                         | native/                              | status |
|------------------------------|--------------------------------------|--------|
| (canvas 2D)                  | crates/pix (RGBA buffer, Ctx)        | done   |
| server.py `state()` JSON     | crates/model                         | done   |
| pixel.js (avatars, dithers)  | crates/room/src/avatar.rs            | done   |
| sprites.js + sprites/*.png   | crates/room/src/sprites.rs           | done   |
| room.js                      | crates/room                          | done   |
| paintings.js                 | crates/room/src/paintings.rs, art.rs | done   |
| tiles.js                     | crates/app (tiles)                   | done   |
| index.html (page, panels)    | crates/app                           | done   |
| burrow.css                   | crates/app (theme)                   | done   |
| xterm.js terminals           | crates/term                          | done   |
| server.py + chess.py         | crates/core                          | done   |
| outside.js                   | crates/games/outside                 | done   |
| pong.js                      | crates/games/pong                    | done   |
| chessboard.js                | crates/app (chess view)              | done   |
| sound.js                     | crates/sound                         | done   |
| fps.js, props3d.js, doom/    | crates/fps                           | done   |
| look.mjs (tests)             | cargo test, PNG snapshots            | done   |

In step with web/ up to origin/main 3f3764b (2026-10-29): the easel and its paintings, the pair telephone and its
beams, a carried bot lit, the condemned taken aback, the mutiny, the + in an untrusted folder.
