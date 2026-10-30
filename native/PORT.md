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
| matrix.html, .js, .css       | crates/app/src/matrix, core matrix.rs | done  |
| sound.js                     | crates/sound                         | done   |
| fps.js, props3d.js, doom/    | crates/fps                           | done   |
| the files' tile (index.html) | crates/app/src/files.rs, files_tile.rs; core files.rs | done |
| editor.js (CodeMirror 6)     | crates/app/src/editor.rs, syntax.rs (smaller: no search, folding, brackets) | done |
| marked + DOMPurify           | crates/app/src/markdown.rs (common cases; nothing runs) | done |
| look.mjs (tests)             | cargo test, PNG snapshots            | done   |

In step with web/ up to origin/main af5ab39 (2026-10-30): since 3f3764b, furniture shot in the first person, the
easel's canvas, the desk out of the closet, the filing cabinet and the files tile (a native editor and markdown, not
CodeMirror and marked), the sausages, Halloween, the mutiny counting her kills, the matrix and the operator's console,
a machine out of reach (snow, frozen bots), the fire taking what the pointer holds, the office port's Origin check.
Not ported, web only: the wry window (src/), the closet's kinds read from the page and the layout merge (races between
page and server that the app, in process, doesn't have).
