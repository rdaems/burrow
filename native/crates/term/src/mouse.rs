//! mouse reports, for a program that asked for them (1000 clicks, 1002 drags, 1003 all motion; 1006
//! SGR, 1005 UTF-8, else the classic CSI M)

use alacritty_terminal::term::TermMode;
use gpui::Modifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    /// motion with no button down
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Press,
    Release,
    Motion,
}

/// the report for a mouse event at a cell (0-based, of the viewport), if the program wants it
pub fn report(button: Button, kind: Kind, col: usize, row: usize, mods: &Modifiers, mode: TermMode) -> Option<Vec<u8>> {
    if !mode.intersects(TermMode::MOUSE_MODE) {
        return None;
    }
    if kind == Kind::Motion {
        let held = button != Button::None;
        if !(mode.contains(TermMode::MOUSE_MOTION) || held && mode.contains(TermMode::MOUSE_DRAG)) {
            return None;
        }
    }
    let base: u32 = match button {
        Button::Left => 0,
        Button::Middle => 1,
        Button::Right => 2,
        Button::None => 3,
        Button::WheelUp => 64,
        Button::WheelDown => 65,
    };
    let mut code = base
        + if kind == Kind::Motion { 32 } else { 0 }
        + if mods.shift { 4 } else { 0 }
        + if mods.alt { 8 } else { 0 }
        + if mods.control { 16 } else { 0 };
    let (x, y) = (col as u32 + 1, row as u32 + 1);
    if mode.contains(TermMode::SGR_MOUSE) {
        let end = if kind == Kind::Release { 'm' } else { 'M' };
        return Some(format!("\x1b[<{code};{x};{y}{end}").into_bytes());
    }
    if kind == Kind::Release {
        code = (code & !3) | 3; // the old encodings don't say which button went up
    }
    let mut s = b"\x1b[M".to_vec();
    s.push((32 + code) as u8);
    if mode.contains(TermMode::UTF8_MOUSE) {
        for v in [x, y] {
            let c = char::from_u32(32 + v.min(2015)).unwrap_or(' ');
            let mut b = [0; 4];
            s.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
        }
    } else {
        s.push((32 + x.min(223)) as u8);
        s.push((32 + y.min(223)) as u8);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO: Modifiers = Modifiers { control: false, alt: false, shift: false, platform: false, function: false };

    #[test]
    fn encodings() {
        let click = TermMode::MOUSE_REPORT_CLICK;
        let sgr = click | TermMode::SGR_MOUSE;
        assert_eq!(report(Button::Left, Kind::Press, 0, 0, &NO, TermMode::empty()), None);
        assert_eq!(report(Button::Left, Kind::Press, 4, 2, &NO, sgr).unwrap(), b"\x1b[<0;5;3M");
        assert_eq!(report(Button::Left, Kind::Release, 4, 2, &NO, sgr).unwrap(), b"\x1b[<0;5;3m");
        assert_eq!(report(Button::Right, Kind::Press, 0, 0, &NO, sgr).unwrap(), b"\x1b[<2;1;1M");
        assert_eq!(report(Button::WheelUp, Kind::Press, 9, 9, &NO, sgr).unwrap(), b"\x1b[<64;10;10M");
        assert_eq!(report(Button::WheelDown, Kind::Press, 9, 9, &NO, sgr).unwrap(), b"\x1b[<65;10;10M");
        let ctrl = Modifiers { control: true, ..NO };
        assert_eq!(report(Button::Left, Kind::Press, 0, 0, &ctrl, sgr).unwrap(), b"\x1b[<16;1;1M");
        // the classic one: 32 + code, 32 + x, 32 + y; a release is button 3
        assert_eq!(report(Button::Left, Kind::Press, 4, 2, &NO, click).unwrap(), b"\x1b[M\x20\x25\x23");
        assert_eq!(report(Button::Left, Kind::Release, 4, 2, &NO, click).unwrap(), b"\x1b[M\x23\x25\x23");
        assert_eq!(report(Button::Left, Kind::Press, 300, 0, &NO, click).unwrap()[4], 255);
        let utf8 = click | TermMode::UTF8_MOUSE;
        assert_eq!(report(Button::Left, Kind::Press, 300, 0, &NO, utf8).unwrap(), "\x1b[M\x20\u{14d}\x21".as_bytes());
    }

    #[test]
    fn motion_only_when_asked() {
        let click = TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE;
        let drag = TermMode::MOUSE_DRAG | TermMode::SGR_MOUSE;
        let all = TermMode::MOUSE_MOTION | TermMode::SGR_MOUSE;
        assert_eq!(report(Button::Left, Kind::Motion, 1, 1, &NO, click), None);
        assert_eq!(report(Button::Left, Kind::Motion, 1, 1, &NO, drag).unwrap(), b"\x1b[<32;2;2M");
        assert_eq!(report(Button::None, Kind::Motion, 1, 1, &NO, drag), None);
        assert_eq!(report(Button::None, Kind::Motion, 1, 1, &NO, all).unwrap(), b"\x1b[<35;2;2M");
    }
}
