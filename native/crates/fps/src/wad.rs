//! Doom's data, read from a WAD as Doom reads it (a port of web/doom/wad.js): the lump directory; PLAYPAL
//! (fourteen palettes: the normal one, eight reds for pain, four yellows for a pickup, a green), COLORMAP (34 maps:
//! 32 light levels, then invulnerability and black); patches (columns of posts, with offsets), flats (64x64 raw),
//! wall textures composed from patches (TEXTURE1/2, PNAMES), sprites by frame and rotation (POSSA1, POSSA2A8:
//! rotation 2, mirrored as 8), and maps (THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES, SECTORS).
//! Also a way into the palette for burrow's own pictures (quantize): nearest colour, through a 32x32x32 table.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

/// Freedoom's data, bundled (BSD, web/doom/COPYING.txt): what the first person uses, taken by web/doom/extract.py
pub fn freedoom() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../web/doom/freedoom.wad"))
}
/// the mysterious door's own texture (Freedoom's BIGDOOR7), for a WAD that has none (id's shareware)
pub fn door_wad() -> &'static [u8] {
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../web/doom/door.wad"))
}

/// A picture in the palette: its indices, where it is opaque (1; burrow's pictures: 2 where it shines), its offsets.
#[derive(Clone, Debug, Default)]
pub struct Pic {
    pub w: i32,
    pub h: i32,
    pub lo: i32,
    pub to: i32,
    pub px: Vec<u8>,
    pub op: Vec<u8>,
    /// a texture with holes in it (a grate)
    pub masked: bool,
}

#[derive(Clone, Debug)]
pub struct Lump {
    pub name: String,
    pub pos: usize,
    pub size: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SpriteEntry {
    lump: usize,
    pub flip: bool,
}
/// a sprite's picture for a frame from one side
#[derive(Clone, Debug)]
pub struct SpriteFrame {
    pub pic: Option<Arc<Pic>>,
    pub flip: bool,
}

struct TexDef {
    w: i32,
    h: i32,
    ps: Vec<(i32, i32, String)>,
}

pub struct Wad {
    data: Vec<u8>,
    pub dir: Vec<Lump>,
    by: HashMap<String, usize>,
    /// per palette: 256 colours, straight RGBA
    pub rgb: Vec<[[u8; 4]; 256]>,
    /// 34 maps of 256: a light level's index for each colour
    pub cmap: Vec<u8>,
    cube: OnceCell<Vec<u8>>,
    patches: RefCell<HashMap<String, Option<Arc<Pic>>>>,
    flats: RefCell<HashMap<String, Option<Arc<[u8]>>>>,
    defs: OnceCell<HashMap<String, TexDef>>,
    textures: RefCell<HashMap<String, Option<Arc<Pic>>>>,
    sprites: OnceCell<HashMap<String, Vec<[Option<SpriteEntry>; 9]>>>,
}

fn text(d: &[u8], o: usize, n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        match d.get(o + i) {
            Some(&c) if c != 0 => s.push((c as char).to_ascii_uppercase()),
            _ => break,
        }
    }
    s
}
fn i16at(d: &[u8], o: usize) -> i32 {
    d.get(o..o + 2).map_or(0, |b| i16::from_le_bytes([b[0], b[1]]) as i32)
}
fn u16at(d: &[u8], o: usize) -> usize {
    d.get(o..o + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize)
}
fn i32at(d: &[u8], o: usize) -> i32 {
    d.get(o..o + 4).map_or(0, |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

impl Wad {
    pub fn open(data: Vec<u8>) -> Result<Wad, String> {
        if data.len() < 12 || (data[1..4] != *b"WAD") {
            return Err("not a WAD".into());
        }
        let n = i32at(&data, 4).max(0) as usize;
        let off = i32at(&data, 8).max(0) as usize;
        if off + 16 * n > data.len() {
            return Err("a WAD cut short".into());
        }
        let mut dir = Vec::with_capacity(n);
        for i in 0..n {
            let o = off + 16 * i;
            let pos = i32at(&data, o).max(0) as usize;
            let size = i32at(&data, o + 4).max(0) as usize;
            dir.push(Lump { name: text(&data, o + 8, 8), pos: pos.min(data.len()), size: size.min(data.len().saturating_sub(pos)) });
        }
        let mut by = HashMap::new();
        for (i, l) in dir.iter().enumerate() {
            by.entry(l.name.clone()).or_insert(i);
        }
        let mut w = Wad {
            data,
            dir,
            by,
            rgb: Vec::new(),
            cmap: Vec::new(),
            cube: OnceCell::new(),
            patches: RefCell::new(HashMap::new()),
            flats: RefCell::new(HashMap::new()),
            defs: OnceCell::new(),
            textures: RefCell::new(HashMap::new()),
            sprites: OnceCell::new(),
        };
        let pp = w.lump("PLAYPAL").ok_or("no PLAYPAL")?.to_vec();
        for p in 0..pp.len() / 768 {
            let mut a = [[0u8; 4]; 256];
            for (i, c) in a.iter_mut().enumerate() {
                *c = [pp[p * 768 + i * 3], pp[p * 768 + i * 3 + 1], pp[p * 768 + i * 3 + 2], 255];
            }
            w.rgb.push(a);
        }
        w.cmap = w.lump("COLORMAP").ok_or("no COLORMAP")?.to_vec();
        if w.cmap.len() < 34 * 256 {
            w.cmap.resize(34 * 256, 0);
        }
        Ok(w)
    }
    pub fn open_bytes(data: &[u8]) -> Result<Wad, String> {
        Wad::open(data.to_vec())
    }
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Wad, String> {
        Wad::open(std::fs::read(path).map_err(|e| e.to_string())?)
    }

    fn bytes(&self, l: &Lump) -> &[u8] {
        &self.data[l.pos..l.pos + l.size]
    }
    pub fn lump(&self, name: &str) -> Option<&[u8]> {
        let i = *self.by.get(&name.to_ascii_uppercase())?;
        Some(self.bytes(&self.dir[i]))
    }
    pub fn has(&self, name: &str) -> bool {
        self.by.contains_key(&name.to_ascii_uppercase())
    }
    /// the normal palette's colour of an index
    pub fn rgb_of(&self, i: u8) -> [u8; 3] {
        let c = self.rgb[0][i as usize];
        [c[0], c[1], c[2]]
    }

    // ---- colour
    /// r>>3, g>>3, b>>3 → the nearest of the palette's colours
    pub fn quantizer(&self) -> &[u8] {
        self.cube.get_or_init(|| {
            let mut cube = vec![0u8; 32768];
            let pal = &self.rgb[0];
            for r in 0..32i32 {
                for g in 0..32i32 {
                    for b in 0..32i32 {
                        let (rr, gg, bb) = (r * 8 + 4, g * 8 + 4, b * 8 + 4);
                        let (mut best, mut bd) = (0usize, i32::MAX);
                        for (i, c) in pal.iter().enumerate() {
                            let (dr, dg, db) = (rr - c[0] as i32, gg - c[1] as i32, bb - c[2] as i32);
                            let dd = 3 * dr * dr + 4 * dg * dg + 2 * db * db;
                            if dd < bd {
                                bd = dd;
                                best = i;
                            }
                        }
                        cube[((r << 10) | (g << 5) | b) as usize] = best as u8;
                    }
                }
            }
            cube
        })
    }
    pub fn nearest(&self, r: u8, g: u8, b: u8) -> u8 {
        self.quantizer()[((r as usize >> 3) << 10) | ((g as usize >> 3) << 5) | (b as usize >> 3)]
    }

    // ---- pictures
    pub fn patch(&self, name: &str) -> Option<Arc<Pic>> {
        let name = name.to_ascii_uppercase();
        if let Some(p) = self.patches.borrow().get(&name) {
            return p.clone();
        }
        let out = self.lump(&name).filter(|p| p.len() > 8).map(|p| Arc::new(decode_patch(p)));
        self.patches.borrow_mut().insert(name, out.clone());
        out
    }
    pub fn flat(&self, name: &str) -> Option<Arc<[u8]>> {
        let name = name.to_ascii_uppercase();
        if let Some(f) = self.flats.borrow().get(&name) {
            return f.clone();
        }
        let f: Option<Arc<[u8]>> = self.lump(&name).filter(|f| f.len() >= 4096).map(|f| Arc::from(&f[..4096]));
        self.flats.borrow_mut().insert(name, f.clone());
        f
    }
    // wall textures: TEXTURE1 (and TEXTURE2, Doom 1's) over PNAMES
    fn defs(&self) -> &HashMap<String, TexDef> {
        self.defs.get_or_init(|| {
            let mut defs = HashMap::new();
            let Some(pn) = self.lump("PNAMES") else { return defs };
            let names: Vec<String> = (0..i32at(pn, 0).max(0) as usize).map(|i| text(pn, 4 + 8 * i, 8)).collect();
            for tl in ["TEXTURE1", "TEXTURE2"] {
                let Some(t) = self.lump(tl) else { continue };
                for i in 0..i32at(t, 0).max(0) as usize {
                    let o = i32at(t, 4 + 4 * i).max(0) as usize;
                    let name = text(t, o, 8);
                    if defs.contains_key(&name) {
                        continue;
                    }
                    let (w, h, np) = (i16at(t, o + 12), i16at(t, o + 14), i16at(t, o + 20));
                    let ps = (0..np.max(0) as usize)
                        .map(|k| {
                            let pi = i16at(t, o + 26 + 10 * k);
                            (i16at(t, o + 22 + 10 * k), i16at(t, o + 24 + 10 * k), names.get(pi.max(0) as usize).cloned().unwrap_or_default())
                        })
                        .collect();
                    defs.insert(name, TexDef { w, h, ps });
                }
            }
            defs
        })
    }
    pub fn texture_names(&self) -> Vec<String> {
        self.defs().keys().cloned().collect()
    }
    pub fn texture(&self, name: &str) -> Option<Arc<Pic>> {
        let name = name.to_ascii_uppercase();
        if let Some(t) = self.textures.borrow().get(&name) {
            return t.clone();
        }
        let out = self.defs().get(&name).map(|def| {
            let n = (def.w.max(0) * def.h.max(0)) as usize;
            let mut out = Pic { w: def.w, h: def.h, px: vec![0; n], op: vec![0; n], ..Default::default() };
            for (px0, py0, pname) in &def.ps {
                let Some(pt) = self.patch(pname) else { continue };
                for y in 0..pt.h {
                    for x in 0..pt.w {
                        let (tx, ty, i) = (px0 + x, py0 + y, (y * pt.w + x) as usize);
                        if pt.op[i] == 0 || tx < 0 || ty < 0 || tx >= def.w || ty >= def.h {
                            continue;
                        }
                        let k = (ty * def.w + tx) as usize;
                        out.px[k] = pt.px[i];
                        out.op[k] = 1;
                    }
                }
            }
            out.masked = out.op.contains(&0);
            Arc::new(out)
        });
        self.textures.borrow_mut().insert(name, out.clone());
        out
    }
    // sprites: frames by letter, each with eight rotations (0 = the same from every side), some mirrored
    fn sprites(&self) -> &HashMap<String, Vec<[Option<SpriteEntry>; 9]>> {
        self.sprites.get_or_init(|| {
            let mut t: HashMap<String, Vec<[Option<SpriteEntry>; 9]>> = HashMap::new();
            let mut add = |name: &str, frame: u8, rot: u8, lump: usize, flip: bool| {
                let s = t.entry(name.to_string()).or_default();
                let f = frame.wrapping_sub(65) as usize;
                if f >= 64 {
                    return;
                }
                if s.len() <= f {
                    s.resize(f + 1, [None; 9]);
                }
                if rot == b'0' {
                    for r in 1..=8 {
                        s[f][r].get_or_insert(SpriteEntry { lump, flip: false });
                    }
                } else if (b'1'..=b'8').contains(&rot) {
                    s[f][(rot - b'0') as usize] = Some(SpriteEntry { lump, flip });
                }
            };
            let mut on = false;
            for (i, l) in self.dir.iter().enumerate() {
                if l.name == "S_START" || l.name == "SS_START" {
                    on = true;
                    continue;
                }
                if l.name == "S_END" || l.name == "SS_END" {
                    on = false;
                    continue;
                }
                let b = l.name.as_bytes();
                if !on || l.size == 0 || b.len() < 6 {
                    continue;
                }
                add(&l.name[..4], b[4], b[5], i, false);
                if b.len() >= 8 {
                    add(&l.name[..4], b[6], b[7], i, true);
                }
            }
            t
        })
    }
    /// a sprite's picture for a frame (0 = A), seen from rotation 1..8 (1 = from in front)
    pub fn sprite_frame(&self, name: &str, frame: usize, rot: usize) -> Option<SpriteFrame> {
        let f = self.sprites().get(name)?.get(frame)?;
        let e = f.get(rot).copied().flatten().or(f[1])?;
        Some(SpriteFrame { pic: self.patch(&self.dir[e.lump].name), flip: e.flip })
    }

    // ---- maps
    pub fn map(&self, name: &str) -> Option<Map> {
        let name = name.to_ascii_uppercase();
        let i = *self.by.get(&name)?;
        let mut l: HashMap<&str, &[u8]> = HashMap::new();
        for k in 1..=10 {
            if i + k >= self.dir.len() {
                break;
            }
            let d = &self.dir[i + k];
            l.insert(d.name.as_str(), self.bytes(d));
        }
        let get = |n: &str| l.get(n).copied().unwrap_or(&[]);
        let rows = |u: &[u8], size: usize| (0..u.len() / size).map(move |k| k * size);
        let vx = get("VERTEXES");
        let vertexes: Vec<V> = rows(vx, 4).map(|o| V { x: i16at(vx, o) as f64, y: i16at(vx, o + 2) as f64 }).collect();
        let sc = get("SECTORS");
        let sectors: Vec<Sector> = rows(sc, 26)
            .map(|o| Sector {
                floor: i16at(sc, o) as f64,
                ceil: i16at(sc, o + 2) as f64,
                ftex: text(sc, o + 4, 8),
                ctex: text(sc, o + 12, 8),
                light: i16at(sc, o + 20),
                special: i16at(sc, o + 22),
                tag: i16at(sc, o + 24),
                ..Default::default()
            })
            .collect();
        let sd = get("SIDEDEFS");
        let sides: Vec<Side> = rows(sd, 30)
            .map(|o| Side {
                xo: i16at(sd, o) as f64,
                yo: i16at(sd, o + 2) as f64,
                upper: text(sd, o + 4, 8),
                lower: text(sd, o + 12, 8),
                mid: text(sd, o + 20, 8),
                sector: (i16at(sd, o + 28).max(0) as usize).min(sectors.len().saturating_sub(1)),
            })
            .collect();
        let ld = get("LINEDEFS");
        let vert = |k: usize| vertexes.get(k).copied().unwrap_or_default();
        let side = |s: i32| (s >= 0 && (s as usize) < sides.len()).then_some(s as usize);
        let lines: Vec<Line> = rows(ld, 14)
            .map(|o| {
                let (v1, v2) = (vert(u16at(ld, o)), vert(u16at(ld, o + 2)));
                Line {
                    v1,
                    v2,
                    flags: i16at(ld, o + 4),
                    special: i16at(ld, o + 6),
                    tag: i16at(ld, o + 8),
                    front: side(i16at(ld, o + 10)).unwrap_or(0),
                    back: side(i16at(ld, o + 12)),
                    x0: v1.x.min(v2.x),
                    x1: v1.x.max(v2.x),
                    y0: v1.y.min(v2.y),
                    y1: v1.y.max(v2.y),
                    dx: v2.x - v1.x,
                    dy: v2.y - v1.y,
                }
            })
            .collect();
        let sg = get("SEGS");
        let segs: Vec<Seg> = rows(sg, 12)
            .map(|o| {
                let li = (i16at(sg, o + 6).max(0) as usize).min(lines.len().saturating_sub(1));
                let dirn = i16at(sg, o + 8) != 0;
                let line = &lines[li];
                let (front, back) = if dirn { (line.back.unwrap_or(line.front), Some(line.front)) } else { (line.front, line.back) };
                Seg { v1: vert(u16at(sg, o)), v2: vert(u16at(sg, o + 2)), line: li, side: dirn, offset: i16at(sg, o + 10) as f64, front, back }
            })
            .collect();
        let ss = get("SSECTORS");
        let ssectors: Vec<SubSector> = rows(ss, 4)
            .map(|o| {
                let (n, first) = (i16at(ss, o).max(0) as usize, i16at(ss, o + 2).max(0) as usize);
                let sector = segs.get(first).map_or(0, |s| sides[s.front].sector);
                SubSector { n, first, sector }
            })
            .collect();
        let nd = get("NODES");
        let nodes: Vec<Node> = rows(nd, 28)
            .map(|o| {
                let g = |k: usize| i16at(nd, o + k) as f64;
                Node {
                    x: g(0),
                    y: g(2),
                    dx: g(4),
                    dy: g(6),
                    bbox: [[g(8), g(10), g(12), g(14)], [g(16), g(18), g(20), g(22)]],
                    kids: [u16at(nd, o + 24) as u16, u16at(nd, o + 26) as u16],
                }
            })
            .collect();
        let th = get("THINGS");
        let things: Vec<MapThing> = rows(th, 10)
            .map(|o| MapThing { x: i16at(th, o) as f64, y: i16at(th, o + 2) as f64, angle: i16at(th, o + 4) as f64, ty: i16at(th, o + 6), flags: i16at(th, o + 8) })
            .collect();
        Some(Map { name, vertexes, sectors, sides, lines, segs, ssectors, nodes, things })
    }
}

fn decode_patch(p: &[u8]) -> Pic {
    let (w, h) = (i16at(p, 0), i16at(p, 2));
    let n = (w.max(0) * h.max(0)) as usize;
    let mut out = Pic { w, h, lo: i16at(p, 4), to: i16at(p, 6), px: vec![0; n], op: vec![0; n], masked: false };
    for x in 0..w.max(0) {
        let mut q = i32at(p, 8 + 4 * x as usize).max(0) as usize;
        let mut top = -1i32;
        while q < p.len() && p[q] != 255 {
            let t = p[q] as i32;
            let len = p.get(q + 1).copied().unwrap_or(0) as usize;
            top = if t <= top { top + t } else { t }; // tall patches: a post's top may be relative
            for k in 0..len {
                let y = top + k as i32;
                if y >= 0 && y < h {
                    out.px[(y * w + x) as usize] = p.get(q + 3 + k).copied().unwrap_or(0);
                    out.op[(y * w + x) as usize] = 1;
                }
            }
            q += len + 4;
        }
    }
    out
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, Default)]
pub struct Sector {
    pub floor: f64,
    pub ceil: f64,
    pub ftex: String,
    pub ctex: String,
    pub light: i32,
    pub special: i32,
    pub tag: i32,
    // what the game keeps (level.rs)
    pub lines: Vec<usize>,
    pub light0: i32,
    pub floor0: f64,
    pub ceil0: f64,
    pub phase: i32,
    pub blink: i32,
    /// where its sounds come from (Doom's soundorg)
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, Default)]
pub struct Side {
    pub xo: f64,
    pub yo: f64,
    pub upper: String,
    pub lower: String,
    pub mid: String,
    pub sector: usize,
}
#[derive(Clone, Debug, Default)]
pub struct Line {
    pub v1: V,
    pub v2: V,
    pub flags: i32,
    pub special: i32,
    pub tag: i32,
    pub front: usize,
    pub back: Option<usize>,
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
    pub dx: f64,
    pub dy: f64,
}
#[derive(Clone, Debug, Default)]
pub struct Seg {
    pub v1: V,
    pub v2: V,
    pub line: usize,
    pub side: bool,
    pub offset: f64,
    pub front: usize,
    pub back: Option<usize>,
}
#[derive(Clone, Debug, Default)]
pub struct SubSector {
    pub n: usize,
    pub first: usize,
    pub sector: usize,
}
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    /// per side: [top, bottom, left, right]
    pub bbox: [[f64; 4]; 2],
    pub kids: [u16; 2],
}
#[derive(Clone, Debug, Default)]
pub struct MapThing {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub ty: i32,
    pub flags: i32,
}
#[derive(Clone, Debug, Default)]
pub struct Map {
    pub name: String,
    pub vertexes: Vec<V>,
    pub sectors: Vec<Sector>,
    pub sides: Vec<Side>,
    pub lines: Vec<Line>,
    pub segs: Vec<Seg>,
    pub ssectors: Vec<SubSector>,
    pub nodes: Vec<Node>,
    pub things: Vec<MapThing>,
}
