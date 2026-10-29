use burrow_fps::{Level, Wad, wad};

#[test]
fn directory() {
    let w = Wad::open_bytes(wad::freedoom()).unwrap();
    assert_eq!(w.dir.len(), 1040);
    assert_eq!(w.dir[0].name, "PLAYPAL");
    assert_eq!(w.rgb.len(), 14);
    assert_eq!(w.cmap.len(), 34 * 256);
    let d = Wad::open_bytes(wad::door_wad()).unwrap();
    assert_eq!(d.dir.len(), 7);
    let t = d.texture("BIGDOOR7").unwrap();
    assert_eq!((t.w, t.h), (128, 128));
    assert!(w.patch("STBAR").is_some_and(|p| p.w == 320 && p.h == 32));
    assert!(w.flat("FLAT5_2").is_some());
    assert!(w.texture("STARTAN3").is_some());
    let s = w.sprite_frame("SHTG", 0, 1).unwrap();
    assert!(s.pic.is_some());
}

#[test]
fn e1m1() {
    let w = Wad::open_bytes(wad::freedoom()).unwrap();
    let m = w.map("E1M1").unwrap();
    assert_eq!(m.things.len(), 292);
    assert_eq!(m.lines.len(), 1175);
    assert_eq!(m.sides.len(), 1829);
    assert_eq!(m.vertexes.len(), 1196);
    assert_eq!(m.segs.len(), 2057);
    assert_eq!(m.ssectors.len(), 682);
    assert_eq!(m.nodes.len(), 681);
    assert_eq!(m.sectors.len(), 182);
    let lv = Level::new(&w, "E1M1", 1).unwrap();
    assert_eq!(lv.start.ty, 1);
    let spot = lv.door_spot(64.0, 72.0).expect("a place for the door");
    assert!((spot.nx.hypot(spot.ny) - 1.0).abs() < 1e-9);
}
