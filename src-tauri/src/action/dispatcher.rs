//! Action and macro dispatcher using Windows SendInput and Shell APIs.
//! Handles keyboard shortcuts, Unicode text injection, application launching, URL opening, and sequences.

use crate::profile::config::ActionConfig;
use std::fmt;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY,
    VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE,
    VK_F1, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17,
    VK_F18, VK_F19, VK_F2, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_F3,
    VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME, VK_INSERT, VK_LCONTROL,
    VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MEDIA_NEXT_TRACK,
    VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU, VK_NEXT,
    VK_NUMLOCK, VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6,
    VK_OEM_7, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PRIOR,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL,
    VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_TAB, VK_UP, VK_VOLUME_DOWN,
    VK_VOLUME_MUTE, VK_VOLUME_UP,
};

/// Errors encountered during action dispatch and macro execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// The specified key name could not be mapped to a Windows Virtual-Key code.
    UnknownKey(String),
    /// A shortcut action was invoked without any keys.
    EmptyShortcut,
    /// A text injection action was invoked with empty text.
    EmptyText,
    /// Action type is not recognized.
    UnknownActionType(String),
    /// The provided URL is invalid or uses an unsupported scheme.
    InvalidUrl(String),
    /// Command execution or process spawn failed.
    ExecutionFailed(String),
    /// Missing mandatory parameter for the action.
    MissingParameter(String),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::UnknownKey(k) => write!(f, "Unknown virtual key name: '{}'", k),
            ActionError::EmptyShortcut => write!(f, "Shortcut action must contain at least one key"),
            ActionError::EmptyText => write!(f, "TypeText action cannot have empty text"),
            ActionError::UnknownActionType(t) => write!(f, "Unsupported action type: '{}'", t),
            ActionError::InvalidUrl(u) => write!(f, "Invalid URL scheme or empty URL: '{}'", u),
            ActionError::ExecutionFailed(msg) => write!(f, "Action execution failed: {}", msg),
            ActionError::MissingParameter(p) => write!(f, "Missing parameter for action: '{}'", p),
        }
    }
}

impl std::error::Error for ActionError {}

/// Resolves a key name string (case-insensitive) to a Windows VIRTUAL_KEY code.
pub fn parse_key_to_vk(key: &str) -> Result<u16, ActionError> {
    let clean = key.trim();
    if clean.is_empty() {
        return Err(ActionError::UnknownKey(key.to_string()));
    }

    // Direct hexadecimal code support (e.g., "0x41", "0x11")
    if clean.starts_with("0x") || clean.starts_with("0X") {
        if let Ok(code) = u16::from_str_radix(&clean[2..], 16) {
            return Ok(code);
        }
    }

    // Single character parsing: A-Z, 0-9
    if clean.len() == 1 {
        let ch = clean.chars().next().unwrap();
        match ch {
            'a'..='z' => return Ok(ch.to_ascii_uppercase() as u16),
            'A'..='Z' => return Ok(ch as u16),
            '0'..='9' => return Ok(ch as u16),
            ' ' => return Ok(0x20), // VK_SPACE
            '`' | '~' => return Ok(0xC0), // VK_OEM_3
            '-' | '_' => return Ok(0xBD), // VK_OEM_MINUS
            '=' | '+' => return Ok(0xBB), // VK_OEM_PLUS
            '[' | '{' => return Ok(0xDB), // VK_OEM_4
            ']' | '}' => return Ok(0xDD), // VK_OEM_6
            '\\' | '|' => return Ok(0xDC), // VK_OEM_5
            ';' | ':' => return Ok(0xBA), // VK_OEM_1
            '\'' | '"' => return Ok(0xDE), // VK_OEM_7
            ',' | '<' => return Ok(0xBC), // VK_OEM_COMMA
            '.' | '>' => return Ok(0xBE), // VK_OEM_PERIOD
            '/' | '?' => return Ok(0xBF), // VK_OEM_2
            _ => {}
        }
    }

    let lower = clean.to_lowercase();
    let vk_val: u16 = match lower.as_str() {
        // Modifiers
        "ctrl" | "control" => 0x11,     // VK_CONTROL
        "lctrl" | "leftctrl" => 0xA2,   // VK_LCONTROL
        "rctrl" | "rightctrl" => 0xA3,  // VK_RCONTROL
        "shift" => 0x10,                // VK_SHIFT
        "lshift" | "leftshift" => 0xA0, // VK_LSHIFT
        "rshift" | "rightshift" => 0xA1,// VK_RSHIFT
        "alt" | "menu" => 0x12,         // VK_MENU
        "lalt" | "leftalt" => 0xA4,     // VK_LMENU
        "ralt" | "rightalt" | "altgr" => 0xA5, // VK_RMENU
        "win" | "windows" | "super" | "meta" | "lwin" => 0x5B, // VK_LWIN
        "rwin" => 0x5C,                 // VK_RWIN

        // Navigation & Editing
        "enter" | "return" => 0x0D,     // VK_RETURN
        "esc" | "escape" => 0x1B,       // VK_ESCAPE
        "space" | "spacebar" => 0x20,   // VK_SPACE
        "tab" => 0x09,                  // VK_TAB
        "backspace" | "back" => 0x08,   // VK_BACK
        "delete" | "del" => 0x2E,       // VK_DELETE
        "insert" | "ins" => 0x2D,       // VK_INSERT
        "home" => 0x24,                 // VK_HOME
        "end" => 0x23,                  // VK_END
        "pageup" | "pgup" => 0x21,      // VK_PRIOR
        "pagedown" | "pgdn" => 0x22,    // VK_NEXT
        "up" | "arrowup" => 0x26,       // VK_UP
        "down" | "arrowdown" => 0x28,   // VK_DOWN
        "left" | "arrowleft" => 0x25,   // VK_LEFT
        "right" | "arrowright" => 0x27, // VK_RIGHT

        // Function keys F1-F24
        "f1" => 0x70,
        "f2" => 0x71,
        "f3" => 0x72,
        "f4" => 0x73,
        "f5" => 0x74,
        "f6" => 0x75,
        "f7" => 0x76,
        "f8" => 0x77,
        "f9" => 0x78,
        "f10" => 0x79,
        "f11" => 0x7A,
        "f12" => 0x7B,
        "f13" => 0x7C,
        "f14" => 0x7D,
        "f15" => 0x7E,
        "f16" => 0x7F,
        "f17" => 0x80,
        "f18" => 0x81,
        "f19" => 0x82,
        "f20" => 0x83,
        "f21" => 0x84,
        "f22" => 0x85,
        "f23" => 0x86,
        "f24" => 0x87,

        // Symbols / Punctuation named
        "backquote" | "grave" | "tilde" => 0xC0, // VK_OEM_3
        "minus" | "dash" => 0xBD,                // VK_OEM_MINUS
        "equal" | "plus" => 0xBB,                // VK_OEM_PLUS
        "bracketleft" | "braceleft" => 0xDB,     // VK_OEM_4
        "bracketright" | "braceright" => 0xDD,   // VK_OEM_6
        "backslash" | "pipe" => 0xDC,            // VK_OEM_5
        "semicolon" | "colon" => 0xBA,           // VK_OEM_1
        "quote" | "doublequote" => 0xDE,         // VK_OEM_7
        "comma" => 0xBC,                         // VK_OEM_COMMA
        "period" | "dot" => 0xBE,                // VK_OEM_PERIOD
        "slash" | "question" => 0xBF,            // VK_OEM_2

        // Lock & Special Keys
        "capslock" | "capital" => 0x14,          // VK_CAPITAL
        "numlock" => 0x90,                       // VK_NUMLOCK
        "scrolllock" | "scroll" => 0x91,         // VK_SCROLL
        "printscreen" | "prtscr" | "snapshot" => 0x2C, // VK_SNAPSHOT
        "pause" | "break" => 0x13,               // VK_PAUSE

        // Media & Volume Controls
        "volumemute" | "mute" => 0xAD,           // VK_VOLUME_MUTE
        "volumedown" => 0xAE,                    // VK_VOLUME_DOWN
        "volumeup" => 0xAF,                      // VK_VOLUME_UP
        "medianexttrack" | "nexttrack" => 0xB0,  // VK_MEDIA_NEXT_TRACK
        "mediaprevtrack" | "prevtrack" => 0xB1,  // VK_MEDIA_PREV_TRACK
        "mediastop" | "stop" => 0xB2,            // VK_MEDIA_STOP
        "mediaplaypause" | "playpause" => 0xB3,  // VK_MEDIA_PLAY_PAUSE

        _ => return Err(ActionError::UnknownKey(key.to_string())),
    };

    Ok(vk_val)
}

/// Identifies if a virtual key code requires the Win32 KEYEVENTF_EXTENDEDKEY flag.
pub fn is_extended_key_code(vk: u16) -> bool {
    matches!(
        vk,
        0xA3  // VK_RCONTROL
        | 0xA5 // VK_RMENU
        | 0x5B // VK_LWIN
        | 0x5C // VK_RWIN
        | 0x2D // VK_INSERT
        | 0x2E // VK_DELETE
        | 0x24 // VK_HOME
        | 0x23 // VK_END
        | 0x21 // VK_PRIOR (PageUp)
        | 0x22 // VK_NEXT (PageDown)
        | 0x26 // VK_UP
        | 0x28 // VK_DOWN
        | 0x25 // VK_LEFT
        | 0x27 // VK_RIGHT
        | 0x90 // VK_NUMLOCK
        | 0x6F // VK_DIVIDE
        | 0x2C // VK_SNAPSHOT
        | 0xAD // VK_VOLUME_MUTE
        | 0xAE // VK_VOLUME_DOWN
        | 0xAF // VK_VOLUME_UP
        | 0xB0 // VK_MEDIA_NEXT_TRACK
        | 0xB1 // VK_MEDIA_PREV_TRACK
        | 0xB2 // VK_MEDIA_STOP
        | 0xB3 // VK_MEDIA_PLAY_PAUSE
    )
}

/// Structure representing a low-level keyboard event pair (Down/Up) before Win32 dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawKeyEvent {
    pub vk: u16,
    pub scan: u16,
    pub is_up: bool,
    pub is_unicode: bool,
    pub is_extended: bool,
}

/// Builds the ordered sequence of key events for a shortcut combination:
/// 1. Key down for all keys in order.
/// 2. Key up for all keys in reverse order.
pub fn plan_shortcut_events(keys: &[String]) -> Result<Vec<RawKeyEvent>, ActionError> {
    if keys.is_empty() {
        return Err(ActionError::EmptyShortcut);
    }

    let mut vks = Vec::with_capacity(keys.len());
    for k in keys {
        vks.push(parse_key_to_vk(k)?);
    }

    let mut events = Vec::with_capacity(vks.len() * 2);

    // Press down in original order
    for &vk in &vks {
        events.push(RawKeyEvent {
            vk,
            scan: 0,
            is_up: false,
            is_unicode: false,
            is_extended: is_extended_key_code(vk),
        });
    }

    // Release in reverse order
    for &vk in vks.iter().rev() {
        events.push(RawKeyEvent {
            vk,
            scan: 0,
            is_up: true,
            is_unicode: false,
            is_extended: is_extended_key_code(vk),
        });
    }

    Ok(events)
}

/// Builds the ordered sequence of Unicode key events for text injection.
pub fn plan_type_text_events(text: &str) -> Result<Vec<RawKeyEvent>, ActionError> {
    if text.is_empty() {
        return Err(ActionError::EmptyText);
    }

    let utf16_units: Vec<u16> = text.encode_utf16().collect();
    let mut events = Vec::with_capacity(utf16_units.len() * 2);

    for &code_unit in &utf16_units {
        // Unicode Down
        events.push(RawKeyEvent {
            vk: 0,
            scan: code_unit,
            is_up: false,
            is_unicode: true,
            is_extended: false,
        });
        // Unicode Up
        events.push(RawKeyEvent {
            vk: 0,
            scan: code_unit,
            is_up: true,
            is_unicode: true,
            is_extended: false,
        });
    }

    Ok(events)
}

/// Executes a shortcut combination via Windows SendInput.
pub fn execute_shortcut(keys: &[String]) -> Result<(), ActionError> {
    let events = plan_shortcut_events(keys)?;
    dispatch_raw_key_events(&events)
}

/// Injects a Unicode text string into the active foreground window via SendInput.
pub fn execute_type_text(text: &str) -> Result<(), ActionError> {
    let events = plan_type_text_events(text)?;
    dispatch_raw_key_events(&events)
}

/// Opens a web URL using the system's default browser.
pub fn execute_open_url(url: &str) -> Result<(), ActionError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(ActionError::InvalidUrl("Empty URL provided".to_string()));
    }

    // Security validation: verify safe URL scheme
    let lower = trimmed.to_lowercase();
    if !lower.starts_with("http://")
        && !lower.starts_with("https://")
        && !lower.starts_with("mailto:")
        && !lower.starts_with("ftp://")
    {
        return Err(ActionError::InvalidUrl(format!(
            "Only http, https, mailto, and ftp schemes are permitted: '{}'",
            trimmed
        )));
    }

    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", trimmed])
            .spawn()
            .map_err(|e| ActionError::ExecutionFailed(format!("Failed to open URL: {}", e)))?;
    }

    #[cfg(not(windows))]
    {
        let _ = trimmed;
    }

    Ok(())
}

/// Launches an application executable or command line.
pub fn execute_open_app(command: &str) -> Result<(), ActionError> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(ActionError::MissingParameter("Command cannot be empty".to_string()));
    }

    #[cfg(windows)]
    {
        // Execute via cmd /c start "" to handle both direct executables and system associations
        std::process::Command::new("cmd.exe")
            .args(["/c", "start", "", trimmed])
            .spawn()
            .map_err(|e| ActionError::ExecutionFailed(format!("Failed to launch app: {}", e)))?;
    }

    #[cfg(not(windows))]
    {
        let _ = trimmed;
    }

    Ok(())
}

/// Executes a sequence of actions with an optional delay between steps.
pub fn execute_sequence(actions: &[ActionConfig], default_delay_ms: Option<u64>) -> Result<(), ActionError> {
    let delay = default_delay_ms.unwrap_or(50);

    for (i, act) in actions.iter().enumerate() {
        execute_action(act)?;
        if i + 1 < actions.len() && delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }
    }

    Ok(())
}

/// Central dispatcher for any ActionConfig.
pub fn execute_action(action: &ActionConfig) -> Result<(), ActionError> {
    match action.action_type.as_str() {
        "Shortcut" => {
            let keys = action.keys.as_ref().ok_or_else(|| {
                ActionError::MissingParameter("Shortcut action requires 'keys' array".to_string())
            })?;
            execute_shortcut(keys)
        }
        "TypeText" => {
            let text = action.text.as_ref().ok_or_else(|| {
                ActionError::MissingParameter("TypeText action requires 'text' string".to_string())
            })?;
            execute_type_text(text)
        }
        "OpenUrl" => {
            let url = action.url.as_ref().ok_or_else(|| {
                ActionError::MissingParameter("OpenUrl action requires 'url' string".to_string())
            })?;
            execute_open_url(url)
        }
        "OpenApp" => {
            let cmd = action.command.as_ref().ok_or_else(|| {
                ActionError::MissingParameter("OpenApp action requires 'command' string".to_string())
            })?;
            execute_open_app(cmd)
        }
        "Sequence" => {
            let actions = action.actions.as_ref().ok_or_else(|| {
                ActionError::MissingParameter("Sequence action requires 'actions' array".to_string())
            })?;
            execute_sequence(actions, action.delay_ms)
        }
        "Submenu" => {
            // Submenu switching is handled at the UI/navigation state layer.
            Ok(())
        }
        other => Err(ActionError::UnknownActionType(other.to_string())),
    }
}

/// Converts planned RawKeyEvents into Win32 INPUT structs and calls SendInput.
#[cfg(windows)]
fn dispatch_raw_key_events(events: &[RawKeyEvent]) -> Result<(), ActionError> {
    if events.is_empty() {
        return Ok(());
    }

    let mut inputs: Vec<INPUT> = Vec::with_capacity(events.len());

    for ev in events {
        let mut flags = 0u32;
        if ev.is_up {
            flags |= KEYEVENTF_KEYUP.0;
        }
        if ev.is_unicode {
            flags |= KEYEVENTF_UNICODE.0;
        }
        if ev.is_extended {
            flags |= KEYEVENTF_EXTENDEDKEY.0;
        }

        let ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(ev.vk),
            wScan: ev.scan,
            dwFlags: KEYBD_EVENT_FLAGS(flags),
            time: 0,
            dwExtraInfo: 0,
        };

        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki },
        });
    }

    let sent = unsafe {
        SendInput(
            &inputs,
            std::mem::size_of::<INPUT>() as i32,
        )
    };

    if sent != inputs.len() as u32 {
        return Err(ActionError::ExecutionFailed(format!(
            "SendInput dispatched {} of {} events",
            sent,
            inputs.len()
        )));
    }

    Ok(())
}

#[cfg(not(windows))]
fn dispatch_raw_key_events(_events: &[RawKeyEvent]) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modifiers() {
        assert_eq!(parse_key_to_vk("Ctrl").unwrap(), 0x11);
        assert_eq!(parse_key_to_vk("control").unwrap(), 0x11);
        assert_eq!(parse_key_to_vk("Shift").unwrap(), 0x10);
        assert_eq!(parse_key_to_vk("Alt").unwrap(), 0x12);
        assert_eq!(parse_key_to_vk("Win").unwrap(), 0x5B);
        assert_eq!(parse_key_to_vk("Meta").unwrap(), 0x5B);
        assert_eq!(parse_key_to_vk("Super").unwrap(), 0x5B);
    }

    #[test]
    fn test_parse_alphanumeric() {
        assert_eq!(parse_key_to_vk("a").unwrap(), 0x41);
        assert_eq!(parse_key_to_vk("Z").unwrap(), 0x5A);
        assert_eq!(parse_key_to_vk("0").unwrap(), 0x30);
        assert_eq!(parse_key_to_vk("9").unwrap(), 0x39);
    }

    #[test]
    fn test_parse_function_keys() {
        assert_eq!(parse_key_to_vk("F1").unwrap(), 0x70);
        assert_eq!(parse_key_to_vk("f12").unwrap(), 0x7B);
        assert_eq!(parse_key_to_vk("F24").unwrap(), 0x87);
    }

    #[test]
    fn test_parse_navigation_and_editing() {
        assert_eq!(parse_key_to_vk("Enter").unwrap(), 0x0D);
        assert_eq!(parse_key_to_vk("Return").unwrap(), 0x0D);
        assert_eq!(parse_key_to_vk("Esc").unwrap(), 0x1B);
        assert_eq!(parse_key_to_vk("Escape").unwrap(), 0x1B);
        assert_eq!(parse_key_to_vk("Space").unwrap(), 0x20);
        assert_eq!(parse_key_to_vk("Tab").unwrap(), 0x09);
        assert_eq!(parse_key_to_vk("Backspace").unwrap(), 0x08);
        assert_eq!(parse_key_to_vk("Delete").unwrap(), 0x2E);
        assert_eq!(parse_key_to_vk("Up").unwrap(), 0x26);
        assert_eq!(parse_key_to_vk("Down").unwrap(), 0x28);
        assert_eq!(parse_key_to_vk("Left").unwrap(), 0x25);
        assert_eq!(parse_key_to_vk("Right").unwrap(), 0x27);
    }

    #[test]
    fn test_parse_symbols() {
        assert_eq!(parse_key_to_vk("`").unwrap(), 0xC0);
        assert_eq!(parse_key_to_vk("Backquote").unwrap(), 0xC0);
        assert_eq!(parse_key_to_vk("-").unwrap(), 0xBD);
        assert_eq!(parse_key_to_vk("=").unwrap(), 0xBB);
        assert_eq!(parse_key_to_vk("[").unwrap(), 0xDB);
        assert_eq!(parse_key_to_vk("]").unwrap(), 0xDD);
        assert_eq!(parse_key_to_vk(";").unwrap(), 0xBA);
        assert_eq!(parse_key_to_vk("/").unwrap(), 0xBF);
    }

    #[test]
    fn test_parse_hexadecimal() {
        assert_eq!(parse_key_to_vk("0x41").unwrap(), 0x41);
        assert_eq!(parse_key_to_vk("0x11").unwrap(), 0x11);
    }

    #[test]
    fn test_parse_unknown_key() {
        assert!(matches!(
            parse_key_to_vk("NonExistentKey123"),
            Err(ActionError::UnknownKey(_))
        ));
        assert!(matches!(parse_key_to_vk(""), Err(ActionError::UnknownKey(_))));
    }

    #[test]
    fn test_plan_shortcut_events_order() {
        let keys = vec!["Control".to_string(), "Shift".to_string(), "P".to_string()];
        let events = plan_shortcut_events(&keys).expect("planning shortcut failed");

        // 3 down + 3 up = 6 events
        assert_eq!(events.len(), 6);

        // Downs in order: Ctrl (0x11), Shift (0x10), P (0x50)
        assert_eq!(events[0].vk, 0x11);
        assert!(!events[0].is_up);
        assert_eq!(events[1].vk, 0x10);
        assert!(!events[1].is_up);
        assert_eq!(events[2].vk, 0x50);
        assert!(!events[2].is_up);

        // Ups in reverse order: P (0x50), Shift (0x10), Ctrl (0x11)
        assert_eq!(events[3].vk, 0x50);
        assert!(events[3].is_up);
        assert_eq!(events[4].vk, 0x10);
        assert!(events[4].is_up);
        assert_eq!(events[5].vk, 0x11);
        assert!(events[5].is_up);
    }

    #[test]
    fn test_plan_type_text_events() {
        let text = "Hello!";
        let events = plan_type_text_events(text).expect("planning text failed");

        // 6 characters * 2 (down and up) = 12 events
        assert_eq!(events.len(), 12);

        // First char 'H' (ASCII 72 / 0x48)
        assert_eq!(events[0].vk, 0);
        assert_eq!(events[0].scan, 'H' as u16);
        assert!(!events[0].is_up);
        assert!(events[0].is_unicode);

        assert_eq!(events[1].vk, 0);
        assert_eq!(events[1].scan, 'H' as u16);
        assert!(events[1].is_up);
        assert!(events[1].is_unicode);
    }

    #[test]
    fn test_plan_empty_inputs_error() {
        assert_eq!(
            plan_shortcut_events(&[]),
            Err(ActionError::EmptyShortcut)
        );
        assert_eq!(
            plan_type_text_events(""),
            Err(ActionError::EmptyText)
        );
    }

    #[test]
    fn test_url_validation() {
        assert!(execute_open_url("").is_err());
        assert!(execute_open_url("javascript:alert(1)").is_err());
        assert!(execute_open_url("powershell -c evil").is_err());
        // Valid schemes
        assert!(execute_open_url("https://github.com").is_ok());
        assert!(execute_open_url("http://localhost:3000").is_ok());
        assert!(execute_open_url("mailto:test@example.com").is_ok());
    }

    #[test]
    fn test_execute_action_routing() {
        // Shortcut
        let shortcut_act = ActionConfig {
            action_type: "Shortcut".to_string(),
            keys: Some(vec!["Control".to_string(), "C".to_string()]),
            url: None,
            command: None,
            text: None,
            target_ring_id: None,
            delay_ms: None,
            actions: None,
        };
        assert!(execute_action(&shortcut_act).is_ok());

        // TypeText
        let text_act = ActionConfig {
            action_type: "TypeText".to_string(),
            keys: None,
            url: None,
            command: None,
            text: Some("Hello World".to_string()),
            target_ring_id: None,
            delay_ms: None,
            actions: None,
        };
        assert!(execute_action(&text_act).is_ok());

        // Submenu
        let submenu_act = ActionConfig {
            action_type: "Submenu".to_string(),
            keys: None,
            url: None,
            command: None,
            text: None,
            target_ring_id: Some("git_tools".to_string()),
            delay_ms: None,
            actions: None,
        };
        assert!(execute_action(&submenu_act).is_ok());

        // Sequence
        let seq_act = ActionConfig {
            action_type: "Sequence".to_string(),
            keys: None,
            url: None,
            command: None,
            text: None,
            target_ring_id: None,
            delay_ms: Some(0),
            actions: Some(vec![shortcut_act, text_act]),
        };
        assert!(execute_action(&seq_act).is_ok());

        // Unknown
        let unknown_act = ActionConfig {
            action_type: "MagicSpell".to_string(),
            keys: None,
            url: None,
            command: None,
            text: None,
            target_ring_id: None,
            delay_ms: None,
            actions: None,
        };
        assert!(matches!(
            execute_action(&unknown_act),
            Err(ActionError::UnknownActionType(_))
        ));
    }
}

