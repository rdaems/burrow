//! The app's icon: one of the room's own robots, standing on a rug, on a dark rounded square (Apple's grid:
//! 824 of 1024 px, rounded corners). `burrow --icon out.png` writes it for macos/bundle.sh.

use burrow_room::figure::{Arms, Legs, body};
use pix::{Color, Image};

pub fn icon() -> Image {
    const N: i32 = 1024;
    const K: i32 = 26; // pixels a robot pixel
    let (ink, floor, rug) = (Color::rgb(0x140f1e), Color::rgb(0x130f1b), Color::rgb(0x3a221c));
    let mut im = Image::new(N as u32, N as u32);
    let (m, r) = (100, 180);
    for y in m..N - m {
        for x in m..N - m {
            let cx = x.clamp(m + r, N - m - r - 1);
            let cy = y.clamp(m + r, N - m - r - 1);
            if (x - cx).pow(2) + (y - cy).pow(2) <= r * r {
                im.set(x, y, if y < 420 { ink.0 } else { floor.0 });
            }
        }
    }
    for y in 560..820 {
        for x in 200..824 {
            im.set(x, y, rug.0);
        }
    }
    let robot = body("burrow", Color::rgb(0xd97757), Arms::Down, Legs::Stand, false, false);
    let (mut x0, mut x1, mut y1) = (i32::MAX, 0, 0);
    for y in 0..robot.h as i32 {
        for x in 0..robot.w as i32 {
            if robot.get(x, y)[3] > 0 {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    let ox = (N - (x1 - x0 + 1) * K) / 2 - x0 * K;
    let oy = 760 - (y1 + 1) * K;
    for y in 0..robot.h as i32 {
        for x in 0..robot.w as i32 {
            let p = robot.get(x, y);
            if p[3] == 0 {
                continue;
            }
            for yy in 0..K {
                for xx in 0..K {
                    im.set(ox + x * K + xx, oy + y * K + yy, p);
                }
            }
        }
    }
    im
}
