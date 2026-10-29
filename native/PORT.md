# The GPUI port: what of web/ is where

One line per web module; status: todo, wip, done (tested), n/a.

| web/                         | native/                              | status |
|------------------------------|--------------------------------------|--------|
| (canvas 2D)                  | crates/pix (RGBA buffer, Ctx)        | done   |
| server.py `state()` JSON     | crates/model                         | done   |
| pixel.js (avatars, dithers)  | crates/room/src/avatar.rs            | todo   |
| sprites.js + sprites/*.png   | crates/room/src/sprites.rs           | todo   |
| room.js                      | crates/room                          | todo   |
| tiles.js                     | crates/app (tiles)                   | todo   |
| index.html (page, panels)    | crates/app                           | todo   |
| burrow.css                   | crates/app (theme)                   | todo   |
| xterm.js terminals           | crates/term (TermView, Screen)       | done   |
| server.py + chess.py         | crates/core                          | todo   |
| outside.js                   | crates/games/outside                 | todo   |
| pong.js                      | crates/games/pong                    | todo   |
| chessboard.js                | crates/app (chess view)              | todo   |
| sound.js                     | crates/sound                         | todo   |
| fps.js, props3d.js, doom/    | crates/fps                           | todo   |
| look.mjs (tests)             | cargo test, PNG snapshots            | todo   |
