# The GPUI port: what of web/ is where

One line per web module; status: todo, wip, done (tested), n/a.

| web/                         | native/                              | status |
|------------------------------|--------------------------------------|--------|
| (canvas 2D)                  | crates/pix (RGBA buffer, Ctx)        | done   |
| server.py `state()` JSON     | crates/model                         | done   |
| pixel.js (avatars, dithers)  | crates/room/src/avatar.rs            | done   |
| sprites.js + sprites/*.png   | crates/room/src/sprites.rs           | done   |
| room.js                      | crates/room                          | wip    |
| tiles.js                     | crates/app (tiles)                   | done   |
| index.html (page, panels)    | crates/app                           | todo   |
| burrow.css                   | crates/app (theme)                   | todo   |
| xterm.js terminals           | crates/app (alacritty_terminal)      | todo   |
| server.py + chess.py         | crates/core                          | done   |
| outside.js                   | crates/games/outside                 | done   |
| pong.js                      | crates/games/pong                    | done   |
| chessboard.js                | crates/app (chess view)              | todo   |
| sound.js                     | crates/sound                         | done   |
| fps.js, props3d.js, doom/    | crates/fps                           | todo   |
| look.mjs (tests)             | cargo test, PNG snapshots            | todo   |
