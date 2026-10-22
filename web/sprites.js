// The room's furniture as pixel art on a 16-dot grid, seen from above at a slant like Gather or Pixel
// Agents: a light front edge, a dark outline, a soft shadow on the floor. Where Pixel Agents
// (pixel-agents-hq/pixel-agents, MIT, sprites/) has the piece it comes from there as it is; the rest
// is drawn here in the same manner, a letter a dot. What changes with the state of things (the
// screens, the coffee, the water, the chess position, the clock) is painted in by room.js.
//
// sprites.get(name) is a canvas, or null until its png has loaded.

window.sprites = (() => {
  const PAL = {
    s: "rgba(0,0,0,0.2)",
    h: "#cfa86d", w: "#b38857", d: "#885c47", k: "#532e3d",                       // Pixel Agents' wood
    A: "#ffffff", B: "#e1e3e9", C: "#b5bfc7", D: "#757b7c", E: "#595e60", F: "#3f3740", G: "#394449",
    x: "#140f1e", r: "#eb4c3f", R: "#8c323a", o: "#ededed", g: "#60c677", y: "#f2a531", Y: "#ffe27a",
    v: "#4a76b8", M: "#1b6660", m: "#2fa087",
  };
  function art(rows, pal = {}) {
    const p = { ...PAL, ...pal }, w = Math.max(...rows.map((r) => r.length));
    if (rows.some((r) => r.length !== w)) console.warn("sprite rows of different lengths", rows);
    const c = document.createElement("canvas");
    c.width = w; c.height = rows.length;
    const x = c.getContext("2d");
    rows.forEach((row, j) => [...row].forEach((ch, i) => { if (p[ch]) { x.fillStyle = p[ch]; x.fillRect(i, j, 1, 1); } }));
    return c;
  }
  const rep = (row, n) => Array(n).fill(row);
  // an office chair from the side, facing left (its sitter faces the screen); a free one is pale
  const CHAIR = [
    "..........FF.",
    ".........FEGF",
    ...rep(".........FGGF", 7),
    "..FFFFFFFFGGF",
    ".FEEEEEEEEEGF",
    ".FGGGGGGGGGGF",
    "..FFFFFFFFFF.",
    "......FDF....",
    ".FDDDDDDDDDF.",
    ".FFF.....FFF.",
    "..sssssssss..",
  ];

  // a door, and the same door open; another machine's is painted its own colour (swap())
  const DOOR = {
    door: [
      "kkkkkkkkkkkkkkkk",
      "kddddddddddddddk",
      "kdkkkkkkkkkkkkdk",
      "kdkhhhhhhhhhhkdk",
      "kdkhwwwwwwwwwkdk",
      "kdkhwddddddwwkdk",
      ...rep("kdkhwdwwwwhwwkdk", 3),
      "kdkhwdhhhhhwwkdk",
      "kdkhwwwwwwwwwkdk",
      "kdkhwwwwwwwwYkdk",
      "kdkhwddddddwykdk",
      ...rep("kdkhwdwwwwhwwkdk", 4),
      "kdkhwdhhhhhwwkdk",
      "kdkhwwwwwwwwwkdk",
      "kdkddddddddddkdk",
    ],
    doorOpen: [
      "kkkkkkkkkkkkkkkk",
      "kddddddddddddddk",
      "kdkkkkkkkkkkkkdk",
      ...rep("kdkhdxxxxxxxxkdk", 17),
    ],
  };

  const ART = {
    door: art(DOOR.door),
    doorOpen: art(DOOR.doorOpen),
    // the glass is left open: the sky is painted behind it
    window: art([
      ".FFFFFFFFFFFFFFFFFFF.",
      ".FBBBBBBBBBBBBBBBBBF.",
      ...rep(".FB.......B.......CF.", 5),
      ".FBBBBBBBBBBBBBBBBCF.",
      ...rep(".FB.......B.......CF.", 5),
      ".FBCCCCCCCCCCCCCCCCF.",
      "FAAAAAAAAAAAAAAAAAAAF",
      "FCCCCCCCCCCCCCCCCCCCF",
      ".FFFFFFFFFFFFFFFFFFF.",
      "..sssssssssssssssss..",
    ]),
    // Pixel Agents' clock without its hands: the time is drawn on it
    clock: art([
      "....RRRR....",
      "..RRrrrrRR..",
      ".RrroooorrR.",
      "RroooooooorR",
      "RooooooooooR",
      "GooooooooooG",
      "GooooooooooG",
      ".GooooooooG.",
      "..GGooooGG..",
      "....GGGG....",
    ]),
    // a project's screen; the screen itself (18 x 11 from 2,2) is painted in
    monitor: art([
      ".FFFFFFFFFFFFFFFFFFFF.",
      "FBBBBBBBBBBBBBBBBBBBCF",
      ...rep("FB..................CF", 11),
      "FBCCCCCCCCCCCCCCCCCCCF",
      "FCCCCCCCCCCCCCCCCCCCDF",
      ".FFFFFFFFFFFFFFFFFFFF.",
      ".........FCCF.........",
      ".........FCCF.........",
      ".....FFFFFFFFFFFF.....",
      ".....FBBBBBBBBBBF.....",
      ".....FFFFFFFFFFFF.....",
    ]),
    // a worktree's laptop, standing on its project's cabinet; its screen (8 x 5 from 2,2) is painted in
    laptop: art([
      ".FFFFFFFFFF.",
      "FBBBBBBBBBCF",
      ...rep("FB........CF", 5),
      "FCCCCCCCCCDF",
      "FFFFFFFFFFFF",
      "FBBBBBBBBBBF",
      ".FFFFFFFFFF.",
    ]),
    chair: art(CHAIR),
    chairFree: art(CHAIR, { F: "#757b7c", E: "#e1e3e9", G: "#b5bfc7", D: "#b5bfc7" }),
    counter: art([
      ".dddddddddddddddddd.",
      "dhwwwwwwwwwwwwwwwwhd",
      ...rep("dwwwwwwwwwwwwwwwwwwd", 2),
      "dhhhhhhhhhhhhhhhhhhd",
      "hddddddddddddddddddh",
      "dkkkkkkkkkkkkkkkkkkd",
      "kdddddddddkddddddddk",
      "kddddddddhkhdddddddk",
      ...rep("kdddddddddkddddddddk", 2),
      "kkkkkkkkkkkkkkkkkkkk",
      ".ssssssssssssssssss.",
    ]),
    brewer: art([
      "FFFFFFFFF",
      "FEEEEEEEF",
      "FGGGGGGGF",
      "FGGGGGrGF",
      "FGGGGGGGF",
      ...rep("FGxxxxxGF", 2),
      "FGxBBBxGF",
      "FGxCCCxGF",
      "FGGGGGGGF",
      "FFFFFFFFF",
    ]),
    // the pot; its glass (5 x 7 from 1,3) is painted in
    pot: art([
      "..FFF....",
      ".FDDDF...",
      "FCCCCCF..",
      "FA....FFF",
      "FA....F.F",
      "F.....F.F",
      "F.....FFF",
      ...rep("F.....F..", 3),
      ".FFFFF...",
    ]),
    cooler: art([
      ".FFFFFFFF.",
      "FABBBBBBCF",
      "FBBBBBBBCF",
      "FBBrBvBBCF",
      "FBBDBDBBCF",
      "FBxxxxxBCF",
      "FBxCCCxBCF",
      "FBBBBBBBCF",
      "FCCCCCCCDF",
      ...rep("FBBBBBBBCF", 3),
      "FCCCCCCCDF",
      ".FFFFFFFF.",
      ".ssssssss.",
    ]),
    // the cooler's bottle; its water (6 x 8 from 1,3) is painted in
    bottle: art([
      "..FFFF..",
      "..FvvF..",
      ".F....F.",
      "FA.....F",
      "FA.....F",
      ...rep("F......F", 6),
      ".FFFFFF.",
    ]),
    printer: art([
      "..FFFFFFFFFF..",
      "..FAAAAAAAAF..",
      ".FFFFFFFFFFFF.",
      ...rep("FBBBBBBBBBBBBF", 2),
      "FCCCCCCCCCCCgF",
      "FCxxxxxxxxxxCF",
      "FCCCCCCCCCCCCF",
      "FDDDDDDDDDDDDF",
      ".FFFFFFFFFFFF.",
    ]),
    stand: art([
      ".dddddddddddd.",
      "dhhhhhhhhhhhhd",
      "kddddddddddddk",
      "kdddddhhdddddk",
      "kddddddddddddk",
      "kkkkkkkkkkkkkk",
      ".ssssssssssss.",
    ]),
    lamp: art([
      ".FFFFF.",
      ...rep("FYYYYYF", 2),
      "FyyyyyF",
      ".FFFFF.",
      ...rep("...F...", 15),
      "..FDF..",
      ".FDDDF.",
      ".sssss.",
    ]),
    // the chess table: the board (16 x 8 from 2,2) is painted in, the position on it
    chess: art([
      ".dddddddddddddddddd.",
      "dhwwwwwwwwwwwwwwwwhd",
      ...rep("dw................wd", 8),
      "dhhhhhhhhhhhhhhhhhhd",
      "kddddddddddddddddddk",
      ".kkkkkkkkkkkkkkkkkk.",
      ...rep("........kddk........", 3),
      "......kkddddkk......",
      ".....ssssssssss.....",
    ]),
    // ping-pong, the net seen edge on
    pong: art([
      "...............FF...............",
      "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
      "FAAAAAAAAAAAAAACEAAAAAAAAAAAAAAF",
      ...rep("FAmmmmmmmmmmmmmCEmmmmmmmmmmmmmAF", 4),
      "FAAAAAAAAAAAAAACEAAAAAAAAAAAAAAF",
      ...rep("FAmmmmmmmmmmmmmCEmmmmmmmmmmmmmAF", 4),
      "FAAAAAAAAAAAAAACEAAAAAAAAAAAAAAF",
      "FMMMMMMMMMMMMMMFFMMMMMMMMMMMMMMF",
      "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
      ...rep("..FD........................DF..", 4),
      ".ssssssssssssssssssssssssssssss.",
    ]),
  };

  // Pixel Agents' pieces, as they are
  const PNG = {};
  for (const n of ["PLANT", "PLANT_2", "WHITEBOARD", "CUSHIONED_CHAIR_SIDE", "WOODEN_BENCH", "SMALL_TABLE_FRONT", "COFFEE"]) {
    PNG[n] = null;
    const im = new Image();
    im.onload = () => {
      const c = document.createElement("canvas");
      c.width = im.width; c.height = im.height;
      c.getContext("2d").drawImage(im, 0, 0);
      PNG[n] = c;
    };
    im.src = "/sprites/" + n + ".png";
  }
  // a door in another paint (a palette swap, {h, w, d, k} as the wood)
  const swaps = new Map();
  function swap(name, pal) {
    const key = name + JSON.stringify(pal);
    if (!swaps.has(key)) swaps.set(key, art(DOOR[name], pal));
    return swaps.get(key);
  }
  return { get: (n) => ART[n] || PNG[n] || null, swap, PAL };
})();
