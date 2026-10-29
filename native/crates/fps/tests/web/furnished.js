(() => {
  const W = 256, H = 180, WALL = 26;
  const wood = { hi: "#cfa86d", top: "#b38857", d: "#885c47", k: "#532e3d", plate: "#2e1f28" };
  const items = [
    { kind: "door", x0: 10, x1: 26, y0: 4, y1: 26, paint: wood, ok: true },
    { kind: "window", x0: 60, x1: 84, y0: 6, y1: 18, hour: 14 },
    { kind: "clock", x0: 100, x1: 117, y0: 4, y1: 21, time: new Date(2026, 0, 1, 10, 10), awake: true },
    { kind: "closet", x0: 150, x1: 170, y0: 2, y1: 26 },
    { kind: "settings", x0: 180, x1: 194, y0: 6, y1: 20 },
  ];
  const fl = document.createElement("canvas"); fl.width = 512; fl.height = 360; fl.v = 1;
  const fx = fl.getContext("2d"); fx.imageSmoothingEnabled = false;
  fx.fillStyle = "rgba(0,0,0,0.3)"; fx.beginPath(); fx.arc(300, 200, 20, 0, 7); fx.fill();
  fx.drawImage(Props3D.pit(44, 18), 360, 220, 88, 36);
  const boxes = [], things = [], solid = [], lights = [];
  const add = (o) => { boxes.push(...o.boxes); things.push(...o.things); solid.push(...o.solid); lights.push(...o.lights); };
  add(Props3D.desk({ x0: 30, y: 100, w: 76, cab: 30, label: "", paint: wood, color: "#d97757", T: 1, seats: [{ x: 78, tree: "" }], mugs: [{ x: 78, color: "#4a76b8" }], screen: "busy", laptops: ["on"] }));
  const P = (kind, x, y, o = {}) => ({ kind, key: kind + "0", x, y, T: 0, ...o });
  for (const p of [P("coffee", 130, 50, { level: 0.6 }), P("cooler", 165, 50, { level: 0.3 }), P("booth", 230, 60, { gun: true, out: 0 }),
    P("chess", 180, 115, { board: "RNBQKBNRPPPPPPPP................................pppppppprnbqkbnr", seats: [{ x: 166, y: 112 }, { x: 194, y: 112 }] }),
    P("plant", 210, 90, { variant: 0 }), P("armchair", 60, 140), P("radio", 112, 72, { on: true }), P("pair", 140, 140, { linked: true })]) add(Props3D.build(p));
  const f1 = Props3D.flames(1, false);
  things.push({ key: "fire", x: 192, y: 128, z: 0, c: f1.c, sx: 0, sy: 0, sw: f1.c.width, sh: f1.c.height, ax: f1.ax, ay: f1.ay, res: f1.res, piece: "fire0" });
  lights.push({ x: 192, y: 128, z: 9, r: 140, flicker: true });
  const rb = (seed, color, arms, legs) => ({ kind: "robot", seed, color, femme: false, arms, legs });
  for (const [key, x, y, fig, seated] of [["demo0001", 78, 99, rb("demo0001", "#4a76b8", "fwd1", "sit"), true], ["demo0002", 120, 120, rb("demo0002", "#2fa087", "wave", "stand"), false],
    ["demo0003", 150, 95, rb("demo0003", "#f2a531", "mug", "walk1"), false], ["dog", 100, 150, { kind: "dog", pose: "sit", wag: true }, false]])
    things.push({ key, x, y, z: 0, fig, seated, face: 1, bot: key === "dog" ? null : key, name: key === "dog" ? null : key });
  const sc = { v: 1, W, H, WALL, floor: fl, wall: Props3D.wall(W, 40, WALL, items, "furnished"), items, wkey: "furnished", boxes, things, solid, lights };
  startFps({ scene: () => sc, start: () => ({ x: 128, y: 172, a: -Math.PI / 2 - 0.3 }), move() {}, shoot: () => false, unshoot() {}, alarm() {}, armed: () => true, use: () => null, label: () => null, status: () => ({ kills: 0 }), face: () => null, tiny: () => {}, tinyW: (s) => s.length * 4 - 1, snapshot: () => null }, () => {});
  return "started";
})()
