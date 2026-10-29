//! not a test: renders a few frames into $FPS_SHOTS to look at (cargo test --test look -- --ignored)
use burrow_fps::*;
use std::sync::Arc;

#[test]
#[ignore]
fn look() {
    let dir = std::env::var("FPS_SHOTS").unwrap();
    let w = Wad::open_bytes(wad::freedoom()).unwrap();
    let dw = Wad::open_bytes(wad::door_wad()).ok();
    let sc = Arc::new(Scene::empty(256.0, 180.0, 26.0));
    let mut f = Fps::new(w, dw, sc, Start { x: 128.0, y: 150.0, a: -std::f64::consts::PI / 2.0 }, Options { seed: 7, ..Default::default() }).unwrap();
    let mut im = pix::Image::new(640, 400);
    f.tick(100.0);
    f.draw(&mut im);
    im.save_png(format!("{dir}/office.png")).unwrap();
    f.set(76.0, 160.0, Some(std::f64::consts::PI / 2.0));
    f.draw(&mut im);
    im.save_png(format!("{dir}/door.png")).unwrap();
    let st = f.level().start.clone();
    let p = f.level().player.clone();
    let cam = LCam { x: st.x, y: st.y, z: p.z + 41.0, a: st.angle.to_radians() };
    f.snap(cam).save_png(format!("{dir}/level.png")).unwrap();
    f.into_level();
    f.tick(30.0);
    f.draw(&mut im);
    im.save_png(format!("{dir}/inlevel.png")).unwrap();
    f.turn(std::f64::consts::PI);
    f.draw(&mut im);
    im.save_png(format!("{dir}/back.png")).unwrap();
}
