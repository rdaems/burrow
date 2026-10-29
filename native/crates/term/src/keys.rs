//! keys as xterm sends them: xterm.js's own table (src/common/input/Keyboard.ts, what the web version
//! sent) with Option as Meta, plus burrow's few: shift+Enter, ⌘← ⌘→ ⌘⌫

use gpui::Keystroke;

#[derive(Debug, PartialEq, Eq)]
pub enum KeyAction {
    Send(Vec<u8>),
    Copy,
    Paste,
    SelectAll,
    PageUp,
    PageDown,
}

/// what a key does in the terminal; None: not the terminal's (plain text goes through the input
/// handler, so dead keys and IME work; other ⌘ keys are the app's)
pub fn encode(ks: &Keystroke, app_cursor: bool) -> Option<KeyAction> {
    let m = &ks.modifiers;
    let (shift, alt, ctrl) = (m.shift, m.alt, m.control);
    let send = |s: &str| Some(KeyAction::Send(s.as_bytes().to_vec()));
    let key = ks.key.as_str();
    // ⌘← ⌘→ ⌘⌫ as Ghostty sends them: Ctrl+A, Ctrl+E, Ctrl+U, which Claude Code's prompt and the shells know
    if m.platform {
        if alt || ctrl || shift {
            return None;
        }
        return match key {
            "c" => Some(KeyAction::Copy),
            "v" => Some(KeyAction::Paste),
            "a" => Some(KeyAction::SelectAll),
            "left" => send("\x01"),
            "right" => send("\x05"),
            "backspace" => send("\x15"),
            _ => None,
        };
    }
    let mods = 1 + shift as u8 + 2 * alt as u8 + 4 * ctrl as u8;
    // a cursor key: ESC [ 1 ; mods X with modifiers, else ESC O X in application cursor mode
    let cursor = |x: char| {
        let s = if mods > 1 {
            format!("\x1b[1;{mods}{x}")
        } else if app_cursor {
            format!("\x1bO{x}")
        } else {
            format!("\x1b[{x}")
        };
        Some(KeyAction::Send(s.into_bytes()))
    };
    let tilde = |n: u8| {
        let s = if mods > 1 { format!("\x1b[{n};{mods}~") } else { format!("\x1b[{n}~") };
        Some(KeyAction::Send(s.into_bytes()))
    };
    let ss3 = |x: char| {
        let s = if mods > 1 { format!("\x1b[1;{mods}{x}") } else { format!("\x1bO{x}") };
        Some(KeyAction::Send(s.into_bytes()))
    };
    match key {
        "up" => cursor('A'),
        "down" => cursor('B'),
        "right" => cursor('C'),
        "left" => cursor('D'),
        "home" => cursor('H'),
        "end" => cursor('F'),
        "insert" if !shift && !ctrl => send("\x1b[2~"),
        "insert" => None,
        "delete" => tilde(3),
        "pageup" if shift => Some(KeyAction::PageUp),
        "pagedown" if shift => Some(KeyAction::PageDown),
        "pageup" if ctrl => tilde(5),
        "pagedown" if ctrl => tilde(6),
        "pageup" => send("\x1b[5~"),
        "pagedown" => send("\x1b[6~"),
        "f1" => ss3('P'),
        "f2" => ss3('Q'),
        "f3" => ss3('R'),
        "f4" => ss3('S'),
        "f5" => tilde(15),
        "f6" => tilde(17),
        "f7" => tilde(18),
        "f8" => tilde(19),
        "f9" => tilde(20),
        "f10" => tilde(21),
        "f11" => tilde(23),
        "f12" => tilde(24),
        "backspace" => match (ctrl, alt) {
            (true, true) => send("\x1b\x08"),
            (true, false) => send("\x08"),
            (false, true) => send("\x1b\x7f"),
            (false, false) => send("\x7f"),
        },
        "tab" if shift => send("\x1b[Z"),
        "tab" => send("\t"),
        // shift+Enter is a new line, as Claude Code reads it: ESC CR (meta+enter)
        "enter" if shift || alt => send("\x1b\r"),
        "enter" => send("\r"),
        "escape" if alt => send("\x1b\x1b"),
        "escape" => send("\x1b"),
        "space" => match (ctrl, alt) {
            (true, true) => send("\x1b\0"),
            (true, false) => send("\0"),
            (false, true) => send("\x1b "),
            (false, false) => None,
        },
        _ => {
            let mut chars = key.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else { return None };
            if ctrl {
                let b = control(c)?;
                return Some(KeyAction::Send(if alt { vec![0x1b, b] } else { vec![b] }));
            }
            if alt {
                // Option as Meta: ESC and the key itself (not the character Option makes of it). gpui
                // keeps shift as a modifier for a-z only, other keys come shifted already ("!")
                let c = if shift { c.to_ascii_uppercase() } else { c };
                let mut s = String::from('\x1b');
                s.push(c);
                return Some(KeyAction::Send(s.into_bytes()));
            }
            None
        }
    }
}

// Ctrl and a key: the C0 control it stands for
fn control(c: char) -> Option<u8> {
    Some(match c {
        'a'..='z' => c as u8 - b'a' + 1,
        'A'..='Z' => c as u8 - b'A' + 1,
        '@' | '2' => 0,
        '[' | '3' => 0x1b,
        '\\' | '4' => 0x1c,
        ']' | '5' => 0x1d,
        '^' | '6' => 0x1e,
        '_' | '7' | '/' => 0x1f,
        '8' => 0x7f,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ks(s: &str) -> Keystroke {
        Keystroke::parse(s).unwrap()
    }
    fn sent(s: &str, app: bool) -> Option<Vec<u8>> {
        match encode(&ks(s), app) {
            Some(KeyAction::Send(b)) => Some(b),
            _ => None,
        }
    }

    #[test]
    fn the_table() {
        let table: &[(&str, bool, &str)] = &[
            ("up", false, "\x1b[A"),
            ("up", true, "\x1bOA"),
            ("left", true, "\x1bOD"),
            ("shift-up", true, "\x1b[1;2A"),
            ("alt-left", false, "\x1b[1;3D"),
            ("ctrl-right", false, "\x1b[1;5C"),
            ("ctrl-shift-down", false, "\x1b[1;6B"),
            ("home", false, "\x1b[H"),
            ("end", true, "\x1bOF"),
            ("delete", false, "\x1b[3~"),
            ("shift-delete", false, "\x1b[3;2~"),
            ("insert", false, "\x1b[2~"),
            ("pageup", false, "\x1b[5~"),
            ("ctrl-pagedown", false, "\x1b[6;5~"),
            ("f1", false, "\x1bOP"),
            ("shift-f1", false, "\x1b[1;2P"),
            ("f5", false, "\x1b[15~"),
            ("ctrl-f12", false, "\x1b[24;5~"),
            ("backspace", false, "\x7f"),
            ("ctrl-backspace", false, "\x08"),
            ("alt-backspace", false, "\x1b\x7f"),
            ("tab", false, "\t"),
            ("shift-tab", false, "\x1b[Z"),
            ("enter", false, "\r"),
            ("shift-enter", false, "\x1b\r"),
            ("alt-enter", false, "\x1b\r"),
            ("escape", false, "\x1b"),
            ("alt-escape", false, "\x1b\x1b"),
            ("ctrl-space", false, "\0"),
            ("alt-space", false, "\x1b "),
            ("ctrl-c", false, "\x03"),
            ("ctrl-a", false, "\x01"),
            ("ctrl-[", false, "\x1b"),
            ("ctrl-\\", false, "\x1c"),
            ("ctrl-]", false, "\x1d"),
            ("ctrl-_", false, "\x1f"),
            ("ctrl-alt-x", false, "\x1b\x18"),
            ("alt-b", false, "\x1bb"),
            ("alt-shift-b", false, "\x1bB"),
            ("alt-.", false, "\x1b."),
            ("alt-!", false, "\x1b!"),
            ("cmd-left", false, "\x01"),
            ("cmd-right", false, "\x05"),
            ("cmd-backspace", false, "\x15"),
        ];
        for (k, app, want) in table {
            assert_eq!(sent(k, *app), Some(want.as_bytes().to_vec()), "{k}");
        }
    }

    #[test]
    fn not_ours() {
        for k in ["a", "shift-a", "space", "1", "é", "cmd-w", "cmd-shift-c", "cmd-up", "shift-insert"] {
            assert_eq!(encode(&ks(k), false), None, "{k}");
        }
        assert_eq!(encode(&ks("cmd-c"), false), Some(KeyAction::Copy));
        assert_eq!(encode(&ks("cmd-v"), false), Some(KeyAction::Paste));
        assert_eq!(encode(&ks("cmd-a"), false), Some(KeyAction::SelectAll));
        assert_eq!(encode(&ks("shift-pageup"), false), Some(KeyAction::PageUp));
    }
}
