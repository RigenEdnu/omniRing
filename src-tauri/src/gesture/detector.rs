//! Gesture detection state machine for tap, flick 4-direction, and radial hold.
//!
//! Provides deterministic state tracking:
//! - Idle: Waiting for trigger button down.
//! - PendingHold: Trigger down; tracking displacement and duration for Tap vs Flick vs RadialHold.
//! - Flicking: Fast directional swipe detected (> 40px within 150ms); suppresses radial menu.
//! - RadialActive: Hold timer expired within jitter threshold; radial overlay is active.

use crate::hook::{HookEvent, MouseButton};
use serde::{Deserialize, Serialize};

/// 4-way cardinal directions for gesture flicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    /// Determines the 4-way cardinal direction from displacement (dx, dy).
    /// Uses screen coordinate system where X+ is right and Y+ is down.
    pub fn from_delta(dx: i32, dy: i32) -> Self {
        let angle_rad = (dy as f64).atan2(dx as f64);
        Self::from_degrees(angle_rad.to_degrees())
    }

    /// Maps an angle in degrees [-180.0, 180.0] into 4 cardinal quadrants of 90° each:
    /// - Right: [-45°, 45°) (centered at 0°)
    /// - Down:  [45°, 135°) (centered at 90°)
    /// - Left:  >= 135° or < -135° (centered at 180°/-180°)
    /// - Up:    [-135°, -45°) (centered at -90°)
    pub fn from_degrees(degrees: f64) -> Self {
        if degrees >= -45.0 && degrees < 45.0 {
            Direction::Right
        } else if degrees >= 45.0 && degrees < 135.0 {
            Direction::Down
        } else if degrees >= -135.0 && degrees < -45.0 {
            Direction::Up
        } else {
            Direction::Left
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Right => "right",
            Direction::Down => "down",
            Direction::Left => "left",
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Internal states of the gesture engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GestureState {
    Idle,
    PendingHold,
    Flicking,
    RadialActive,
}

/// Events produced by the gesture engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum GestureEvent {
    /// Simple quick tap without movement (< 150ms, < 10px).
    Tap {
        button: MouseButton,
        x: i32,
        y: i32,
    },
    /// Fast directional swipe gesture (> 40px before hold timeout).
    Flick {
        direction: Direction,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        distance: f64,
    },
    /// Flick gesture released (trigger up after flick).
    FlickComplete {
        end_x: i32,
        end_y: i32,
    },
    /// Trigger button held beyond threshold; signals radial overlay to open.
    RadialOpen {
        center_x: i32,
        center_y: i32,
    },
    /// Cursor movement while radial menu is active (used for slice highlighting).
    RadialMove {
        current_x: i32,
        current_y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    /// Trigger button released while radial menu is active; confirms slice selection.
    RadialRelease {
        current_x: i32,
        current_y: i32,
    },
    /// Gesture cancelled or released in dead zone without completing flick or tap.
    Cancelled,
}

/// Configuration parameters for gesture detection thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureConfig {
    /// Time in milliseconds to hold trigger before opening radial menu (default: 150ms).
    pub hold_timeout_ms: u32,
    /// Minimum displacement in pixels to trigger a directional flick (default: 40.0 px).
    pub flick_distance_threshold: f64,
    /// Maximum displacement in pixels to consider release as a tap (default: 10.0 px).
    pub tap_distance_threshold: f64,
    /// Maximum jitter radius in pixels while waiting for hold timeout (default: 15.0 px).
    pub hold_jitter_radius: f64,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            hold_timeout_ms: 150,
            flick_distance_threshold: 40.0,
            tap_distance_threshold: 10.0,
            hold_jitter_radius: 15.0,
        }
    }
}

/// Gesture detector state machine.
#[derive(Debug, Clone)]
pub struct GestureDetector {
    config: GestureConfig,
    state: GestureState,
    trigger_button: MouseButton,
    start_pos: (i32, i32),
    current_pos: (i32, i32),
    start_timestamp: u32,
    last_timestamp: u32,
}

impl GestureDetector {
    /// Creates a new gesture detector with specified configuration and trigger button.
    pub fn new(config: GestureConfig, trigger_button: MouseButton) -> Self {
        Self {
            config,
            state: GestureState::Idle,
            trigger_button,
            start_pos: (0, 0),
            current_pos: (0, 0),
            start_timestamp: 0,
            last_timestamp: 0,
        }
    }

    /// Returns the current state of the gesture detector.
    pub fn state(&self) -> GestureState {
        self.state
    }

    /// Returns a reference to the active gesture configuration.
    pub fn config(&self) -> &GestureConfig {
        &self.config
    }

    /// Updates the active gesture configuration.
    pub fn set_config(&mut self, config: GestureConfig) {
        self.config = config;
    }

    /// Returns the active trigger button.
    pub fn trigger_button(&self) -> MouseButton {
        self.trigger_button
    }

    /// Reconfigures the trigger button.
    pub fn set_trigger_button(&mut self, button: MouseButton) {
        self.trigger_button = button;
    }

    /// Processes an incoming raw `HookEvent`.
    pub fn handle_hook_event(&mut self, event: &HookEvent) -> Option<GestureEvent> {
        match event {
            HookEvent::MouseDown {
                button,
                x,
                y,
                timestamp,
            } => self.on_mouse_down(*button, *x, *y, *timestamp),
            HookEvent::MouseMove { x, y, timestamp } => self.on_mouse_move(*x, *y, *timestamp),
            HookEvent::MouseUp {
                button,
                x,
                y,
                timestamp,
            } => self.on_mouse_up(*button, *x, *y, *timestamp),
            HookEvent::MouseWheel { .. } => None,
        }
    }

    /// Handles a mouse button down event.
    pub fn on_mouse_down(
        &mut self,
        button: MouseButton,
        x: i32,
        y: i32,
        timestamp: u32,
    ) -> Option<GestureEvent> {
        if button != self.trigger_button {
            return None;
        }

        self.start_pos = (x, y);
        self.current_pos = (x, y);
        self.start_timestamp = timestamp;
        self.last_timestamp = timestamp;
        self.state = GestureState::PendingHold;
        None
    }

    /// Handles a mouse move event.
    pub fn on_mouse_move(&mut self, x: i32, y: i32, timestamp: u32) -> Option<GestureEvent> {
        self.last_timestamp = timestamp;

        match self.state {
            GestureState::Idle => None,
            GestureState::PendingHold => {
                self.current_pos = (x, y);
                let dx = x - self.start_pos.0;
                let dy = y - self.start_pos.1;
                let distance = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();

                // Fast path: if flick distance exceeded before hold timeout, trigger flick immediately
                if distance >= self.config.flick_distance_threshold {
                    self.state = GestureState::Flicking;
                    let direction = Direction::from_delta(dx, dy);
                    return Some(GestureEvent::Flick {
                        direction,
                        start_x: self.start_pos.0,
                        start_y: self.start_pos.1,
                        end_x: x,
                        end_y: y,
                        distance,
                    });
                }

                // Check if hold timer elapsed while moving within jitter radius
                let elapsed = timestamp.wrapping_sub(self.start_timestamp);
                if elapsed >= self.config.hold_timeout_ms {
                    self.state = GestureState::RadialActive;
                    return Some(GestureEvent::RadialOpen {
                        center_x: self.start_pos.0,
                        center_y: self.start_pos.1,
                    });
                }

                None
            }
            GestureState::Flicking => {
                self.current_pos = (x, y);
                None
            }
            GestureState::RadialActive => {
                self.current_pos = (x, y);
                let dx = x - self.start_pos.0;
                let dy = y - self.start_pos.1;
                Some(GestureEvent::RadialMove {
                    current_x: x,
                    current_y: y,
                    delta_x: dx,
                    delta_y: dy,
                })
            }
        }
    }

    /// Handles a mouse button up event.
    pub fn on_mouse_up(
        &mut self,
        button: MouseButton,
        x: i32,
        y: i32,
        timestamp: u32,
    ) -> Option<GestureEvent> {
        if button != self.trigger_button {
            return None;
        }

        self.last_timestamp = timestamp;
        self.current_pos = (x, y);
        let dx = x - self.start_pos.0;
        let dy = y - self.start_pos.1;
        let distance = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
        let elapsed = timestamp.wrapping_sub(self.start_timestamp);

        match self.state {
            GestureState::Idle => None,
            GestureState::PendingHold => {
                self.state = GestureState::Idle;
                if elapsed < self.config.hold_timeout_ms
                    && distance <= self.config.tap_distance_threshold
                {
                    Some(GestureEvent::Tap { button, x, y })
                } else {
                    Some(GestureEvent::Cancelled)
                }
            }
            GestureState::Flicking => {
                self.state = GestureState::Idle;
                Some(GestureEvent::FlickComplete { end_x: x, end_y: y })
            }
            GestureState::RadialActive => {
                self.state = GestureState::Idle;
                Some(GestureEvent::RadialRelease {
                    current_x: x,
                    current_y: y,
                })
            }
        }
    }

    /// Periodically called to check whether the hold timer has elapsed.
    pub fn check_timeout(&mut self, current_timestamp: u32) -> Option<GestureEvent> {
        if self.state == GestureState::PendingHold {
            let elapsed = current_timestamp.wrapping_sub(self.start_timestamp);
            if elapsed >= self.config.hold_timeout_ms {
                self.state = GestureState::RadialActive;
                return Some(GestureEvent::RadialOpen {
                    center_x: self.start_pos.0,
                    center_y: self.start_pos.1,
                });
            }
        }
        None
    }

    /// Cancels any active gesture and resets to Idle.
    pub fn cancel(&mut self) -> Option<GestureEvent> {
        if self.state != GestureState::Idle {
            let was_radial = self.state == GestureState::RadialActive;
            self.state = GestureState::Idle;
            if was_radial {
                Some(GestureEvent::Cancelled)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Resets all internal state variables to defaults.
    pub fn reset(&mut self) {
        self.state = GestureState::Idle;
        self.start_pos = (0, 0);
        self.current_pos = (0, 0);
        self.start_timestamp = 0;
        self.last_timestamp = 0;
    }
}

/// Replays a synthetic mouse click to the system when a tap is detected,
/// ensuring suppressed trigger clicks are forwarded cleanly to the underlying window.
pub fn replay_tap_click(button: MouseButton) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
            MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN,
            MOUSEEVENTF_XUP,
        };

        match button {
            MouseButton::X1 => {
                mouse_event(MOUSEEVENTF_XDOWN, 0, 0, 1, 0);
                mouse_event(MOUSEEVENTF_XUP, 0, 0, 1, 0);
            }
            MouseButton::X2 => {
                mouse_event(MOUSEEVENTF_XDOWN, 0, 0, 2, 0);
                mouse_event(MOUSEEVENTF_XUP, 0, 0, 2, 0);
            }
            MouseButton::Middle => {
                mouse_event(MOUSEEVENTF_MIDDLEDOWN, 0, 0, 0, 0);
                mouse_event(MOUSEEVENTF_MIDDLEUP, 0, 0, 0, 0);
            }
            MouseButton::Right => {
                mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
                mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
            }
            MouseButton::Left => {
                mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
                mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
            }
            MouseButton::Other(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_cardinal() {
        // Up: dy < 0, dx = 0 (-90 deg)
        assert_eq!(Direction::from_delta(0, -50), Direction::Up);
        // Right: dx > 0, dy = 0 (0 deg)
        assert_eq!(Direction::from_delta(50, 0), Direction::Right);
        // Down: dy > 0, dx = 0 (90 deg)
        assert_eq!(Direction::from_delta(0, 50), Direction::Down);
        // Left: dx < 0, dy = 0 (180 deg)
        assert_eq!(Direction::from_delta(-50, 0), Direction::Left);
    }

    #[test]
    fn test_direction_boundary_angles() {
        assert_eq!(Direction::from_degrees(-45.0), Direction::Right);
        assert_eq!(Direction::from_degrees(0.0), Direction::Right);
        assert_eq!(Direction::from_degrees(44.9), Direction::Right);

        assert_eq!(Direction::from_degrees(45.0), Direction::Down);
        assert_eq!(Direction::from_degrees(90.0), Direction::Down);
        assert_eq!(Direction::from_degrees(134.9), Direction::Down);

        assert_eq!(Direction::from_degrees(135.0), Direction::Left);
        assert_eq!(Direction::from_degrees(180.0), Direction::Left);
        assert_eq!(Direction::from_degrees(-180.0), Direction::Left);
        assert_eq!(Direction::from_degrees(-135.1), Direction::Left);

        assert_eq!(Direction::from_degrees(-135.0), Direction::Up);
        assert_eq!(Direction::from_degrees(-90.0), Direction::Up);
        assert_eq!(Direction::from_degrees(-45.1), Direction::Up);
    }

    #[test]
    fn test_direction_as_str_and_display() {
        assert_eq!(Direction::Up.as_str(), "up");
        assert_eq!(Direction::Right.as_str(), "right");
        assert_eq!(Direction::Down.as_str(), "down");
        assert_eq!(Direction::Left.as_str(), "left");

        assert_eq!(format!("{}", Direction::Up), "up");
        assert_eq!(format!("{}", Direction::Down), "down");
    }

    #[test]
    fn test_tap_detection_success() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        // Mouse Down at t=1000
        let ev1 = detector.on_mouse_down(MouseButton::X1, 100, 100, 1000);
        assert_eq!(ev1, None);
        assert_eq!(detector.state(), GestureState::PendingHold);

        // Small jitter movement at t=1050 (< 10px)
        let ev2 = detector.on_mouse_move(103, 102, 1050);
        assert_eq!(ev2, None);
        assert_eq!(detector.state(), GestureState::PendingHold);

        // Mouse Up at t=1100 (elapsed = 100ms < 150ms)
        let ev3 = detector.on_mouse_up(MouseButton::X1, 103, 102, 1100);
        assert_eq!(
            ev3,
            Some(GestureEvent::Tap {
                button: MouseButton::X1,
                x: 103,
                y: 102,
            })
        );
        assert_eq!(detector.state(), GestureState::Idle);
    }

    #[test]
    fn test_flick_up_detection() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        detector.on_mouse_down(MouseButton::X1, 200, 200, 1000);
        assert_eq!(detector.state(), GestureState::PendingHold);

        // Move up by 50px (dx=0, dy=-50, distance=50 >= 40) at t=1040 (< 150ms)
        let ev = detector.on_mouse_move(200, 150, 1040);
        assert_eq!(
            ev,
            Some(GestureEvent::Flick {
                direction: Direction::Up,
                start_x: 200,
                start_y: 200,
                end_x: 200,
                end_y: 150,
                distance: 50.0,
            })
        );
        assert_eq!(detector.state(), GestureState::Flicking);

        // Release flick
        let ev_up = detector.on_mouse_up(MouseButton::X1, 200, 150, 1080);
        assert_eq!(
            ev_up,
            Some(GestureEvent::FlickComplete {
                end_x: 200,
                end_y: 150,
            })
        );
        assert_eq!(detector.state(), GestureState::Idle);
    }

    #[test]
    fn test_flick_all_directions() {
        let dirs = [
            (200, 140, Direction::Up),    // dy = -60
            (260, 200, Direction::Right), // dx = 60
            (200, 260, Direction::Down),  // dy = 60
            (140, 200, Direction::Left),  // dx = -60
        ];

        for (end_x, end_y, expected_dir) in dirs {
            let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);
            detector.on_mouse_down(MouseButton::X1, 200, 200, 1000);

            let ev = detector.on_mouse_move(end_x, end_y, 1030);
            match ev {
                Some(GestureEvent::Flick { direction, .. }) => {
                    assert_eq!(direction, expected_dir);
                }
                other => panic!("Expected Flick, got {:?}", other),
            }
            assert_eq!(detector.state(), GestureState::Flicking);
        }
    }

    #[test]
    fn test_radial_open_via_timeout() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        detector.on_mouse_down(MouseButton::X1, 300, 300, 1000);

        // Timeout check before 150ms -> None
        let ev1 = detector.check_timeout(1100);
        assert_eq!(ev1, None);
        assert_eq!(detector.state(), GestureState::PendingHold);

        // Timeout check at t=1160 (elapsed 160ms >= 150ms)
        let ev2 = detector.check_timeout(1160);
        assert_eq!(
            ev2,
            Some(GestureEvent::RadialOpen {
                center_x: 300,
                center_y: 300,
            })
        );
        assert_eq!(detector.state(), GestureState::RadialActive);
    }

    #[test]
    fn test_radial_move_and_release() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        detector.on_mouse_down(MouseButton::X1, 300, 300, 1000);
        detector.check_timeout(1200);
        assert_eq!(detector.state(), GestureState::RadialActive);

        // Move in RadialActive
        let move_ev = detector.on_mouse_move(350, 320, 1220);
        assert_eq!(
            move_ev,
            Some(GestureEvent::RadialMove {
                current_x: 350,
                current_y: 320,
                delta_x: 50,
                delta_y: 20,
            })
        );

        // Release in RadialActive
        let release_ev = detector.on_mouse_up(MouseButton::X1, 350, 320, 1250);
        assert_eq!(
            release_ev,
            Some(GestureEvent::RadialRelease {
                current_x: 350,
                current_y: 320,
            })
        );
        assert_eq!(detector.state(), GestureState::Idle);
    }

    #[test]
    fn test_incomplete_flick_cancelled_on_early_release() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        detector.on_mouse_down(MouseButton::X1, 100, 100, 1000);
        // Move 25px (dx=25, dy=0): > tap (10px) but < flick (40px)
        detector.on_mouse_move(125, 100, 1050);

        // Released before hold timeout (elapsed = 70ms < 150ms)
        let ev = detector.on_mouse_up(MouseButton::X1, 125, 100, 1070);
        assert_eq!(ev, Some(GestureEvent::Cancelled));
        assert_eq!(detector.state(), GestureState::Idle);
    }

    #[test]
    fn test_cancel_during_radial_active() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        detector.on_mouse_down(MouseButton::X1, 300, 300, 1000);
        detector.check_timeout(1200);
        assert_eq!(detector.state(), GestureState::RadialActive);

        let cancel_ev = detector.cancel();
        assert_eq!(cancel_ev, Some(GestureEvent::Cancelled));
        assert_eq!(detector.state(), GestureState::Idle);
    }

    #[test]
    fn test_non_trigger_button_ignored() {
        let mut detector = GestureDetector::new(GestureConfig::default(), MouseButton::X1);

        // Left button down
        let ev = detector.on_mouse_down(MouseButton::Left, 100, 100, 1000);
        assert_eq!(ev, None);
        assert_eq!(detector.state(), GestureState::Idle);
    }
}

