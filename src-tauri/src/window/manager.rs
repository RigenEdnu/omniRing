//! Win32 window management for non-activating overlay (WS_EX_NOACTIVATE) and screen boundary clamping.

use serde::{Deserialize, Serialize};
use windows::core::Result as WinResult;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE,
    HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_HIDE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

/// Represents an axis-aligned screen rectangle in virtual screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    #[inline]
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    #[inline]
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// Geometry metadata for a physical or virtual display monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorGeometry {
    pub monitor_rect: ScreenRect,
    pub work_rect: ScreenRect,
    pub is_primary: bool,
}

/// Full radial overlay placement calculation containing screen and window-relative coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadialPlacement {
    /// Original raw cursor X coordinate in virtual screen coordinates.
    pub cursor_x: i32,
    /// Original raw cursor Y coordinate in virtual screen coordinates.
    pub cursor_y: i32,
    /// Clamped center X coordinate of the radial ring in virtual screen coordinates.
    pub center_x: i32,
    /// Clamped center Y coordinate of the radial ring in virtual screen coordinates.
    pub center_y: i32,
    /// Top-left X coordinate of the overlay window in virtual screen coordinates.
    pub window_x: i32,
    /// Top-left Y coordinate of the overlay window in virtual screen coordinates.
    pub window_y: i32,
    /// Width of the overlay window in pixels.
    pub window_width: i32,
    /// Height of the overlay window in pixels.
    pub window_height: i32,
    /// Radial ring center X relative to the top-left of the overlay window.
    pub relative_center_x: i32,
    /// Radial ring center Y relative to the top-left of the overlay window.
    pub relative_center_y: i32,
    /// Outer radius used for clamping calculation.
    pub radius: i32,
}

/// Pure calculation function for radial placement and screen boundary clamping.
/// Ensures the radial ring with `radius` stays 100% within `work_rect`,
/// and the overlay window is positioned to contain the ring cleanly.
pub fn calculate_radial_placement(
    cursor_x: i32,
    cursor_y: i32,
    radius: i32,
    window_width: i32,
    window_height: i32,
    work_rect: ScreenRect,
) -> RadialPlacement {
    // 1. Clamp the radial ring center so that `center - radius >= work.left`
    // and `center + radius <= work.right`.
    let min_center_x = work_rect.left + radius;
    let max_center_x = (work_rect.right - radius).max(min_center_x);
    let clamped_center_x = cursor_x.clamp(min_center_x, max_center_x);

    let min_center_y = work_rect.top + radius;
    let max_center_y = (work_rect.bottom - radius).max(min_center_y);
    let clamped_center_y = cursor_y.clamp(min_center_y, max_center_y);

    // 2. Center the overlay window on the clamped radial center,
    // then clamp the window rectangle to the work area.
    let half_w = window_width / 2;
    let half_h = window_height / 2;

    let ideal_win_x = clamped_center_x - half_w;
    let ideal_win_y = clamped_center_y - half_h;

    let min_win_x = work_rect.left;
    let max_win_x = (work_rect.right - window_width).max(min_win_x);
    let clamped_win_x = ideal_win_x.clamp(min_win_x, max_win_x);

    let min_win_y = work_rect.top;
    let max_win_y = (work_rect.bottom - window_height).max(min_win_y);
    let clamped_win_y = ideal_win_y.clamp(min_win_y, max_win_y);

    // 3. Compute relative center coordinates within the overlay window (canvas space).
    let relative_center_x = clamped_center_x - clamped_win_x;
    let relative_center_y = clamped_center_y - clamped_win_y;

    RadialPlacement {
        cursor_x,
        cursor_y,
        center_x: clamped_center_x,
        center_y: clamped_center_y,
        window_x: clamped_win_x,
        window_y: clamped_win_y,
        window_width,
        window_height,
        relative_center_x,
        relative_center_y,
        radius,
    }
}

/// Retrieves the current physical cursor position via Win32 `GetCursorPos`.
pub fn get_current_cursor_pos() -> WinResult<POINT> {
    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut pt)?;
    }
    Ok(pt)
}

/// Retrieves monitor geometry (display rect and work area) for the monitor containing `point`.
pub fn get_monitor_geometry_from_point(point: POINT) -> Option<MonitorGeometry> {
    unsafe {
        let hmonitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };

        if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
            Some(MonitorGeometry {
                monitor_rect: ScreenRect::new(
                    mi.rcMonitor.left,
                    mi.rcMonitor.top,
                    mi.rcMonitor.right,
                    mi.rcMonitor.bottom,
                ),
                work_rect: ScreenRect::new(
                    mi.rcWork.left,
                    mi.rcWork.top,
                    mi.rcWork.right,
                    mi.rcWork.bottom,
                ),
                is_primary: (mi.dwFlags & 1) != 0, // MONITORINFOF_PRIMARY = 1
            })
        } else {
            None
        }
    }
}

/// Computes the radial placement for a given cursor and overlay window dimensions,
/// resolving the monitor geometry automatically.
pub fn get_radial_placement(
    cursor: POINT,
    radius: i32,
    window_width: i32,
    window_height: i32,
) -> RadialPlacement {
    let work_rect = if let Some(geom) = get_monitor_geometry_from_point(cursor) {
        geom.work_rect
    } else {
        // Fallback default work area if monitor query fails
        ScreenRect::new(0, 0, 1920, 1080)
    };

    calculate_radial_placement(
        cursor.x,
        cursor.y,
        radius,
        window_width,
        window_height,
        work_rect,
    )
}

/// Configures a Win32 window handle as a non-activating, topmost tool window.
/// This prevents stealing focus from active foreground applications (`WS_EX_NOACTIVATE`).
pub fn apply_overlay_styles(hwnd: HWND) -> WinResult<()> {
    unsafe {
        let current_ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let target_ex_style = current_ex_style
            | (WS_EX_NOACTIVATE.0 as isize)
            | (WS_EX_TOPMOST.0 as isize)
            | (WS_EX_TOOLWINDOW.0 as isize);

        if current_ex_style != target_ex_style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex_style);
            // Refresh window frame styles without activating
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )?;
        }
        Ok(())
    }
}

/// Legacy wrapper for backward compatibility: clamps center coordinates so the bounding box fits within target monitor.
pub fn calculate_clamped_position(
    cursor: POINT,
    window_width: i32,
    window_height: i32,
) -> (i32, i32) {
    let placement = get_radial_placement(cursor, window_width / 4, window_width, window_height);
    (placement.window_x, placement.window_y)
}

/// Repositions the overlay window to center on the cursor with monitor boundary clamping and shows it without activating.
pub fn show_overlay_clamped(
    hwnd: HWND,
    cursor: POINT,
    radius: i32,
    window_width: i32,
    window_height: i32,
) -> WinResult<RadialPlacement> {
    let placement = get_radial_placement(cursor, radius, window_width, window_height);

    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            placement.window_x,
            placement.window_y,
            placement.window_width,
            placement.window_height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
    }

    Ok(placement)
}

/// Legacy wrapper for `show_overlay_clamped`.
pub fn show_overlay_at_cursor(
    hwnd: HWND,
    cursor: POINT,
    window_width: i32,
    window_height: i32,
) -> WinResult<()> {
    let _ = show_overlay_clamped(hwnd, cursor, window_width / 4, window_width, window_height)?;
    Ok(())
}

/// Hides the overlay window cleanly.
pub fn hide_overlay(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_rect_properties() {
        let rect = ScreenRect::new(0, 0, 1920, 1080);
        assert_eq!(rect.width(), 1920);
        assert_eq!(rect.height(), 1080);
        assert!(rect.contains_point(0, 0));
        assert!(rect.contains_point(1919, 1079));
        assert!(!rect.contains_point(1920, 1080));
        assert!(!rect.contains_point(-1, 0));
    }

    #[test]
    fn test_clamping_center_of_screen() {
        // Standard 1080p primary monitor with taskbar at bottom
        let work = ScreenRect::new(0, 0, 1920, 1040);
        let placement = calculate_radial_placement(960, 520, 150, 600, 600, work);

        assert_eq!(placement.cursor_x, 960);
        assert_eq!(placement.cursor_y, 520);
        assert_eq!(placement.center_x, 960);
        assert_eq!(placement.center_y, 520);
        assert_eq!(placement.window_x, 660);
        assert_eq!(placement.window_y, 220);
        assert_eq!(placement.relative_center_x, 300);
        assert_eq!(placement.relative_center_y, 300);
    }

    #[test]
    fn test_clamping_top_left_corner() {
        // Cursor at extreme top-left (0, 0)
        let work = ScreenRect::new(0, 0, 1920, 1080);
        let placement = calculate_radial_placement(0, 0, 150, 600, 600, work);

        // Center must be clamped to at least radius=150
        assert_eq!(placement.center_x, 150);
        assert_eq!(placement.center_y, 150);
        // Window top-left is clamped to 0,0
        assert_eq!(placement.window_x, 0);
        assert_eq!(placement.window_y, 0);
        // Relative center inside canvas is 150, 150
        assert_eq!(placement.relative_center_x, 150);
        assert_eq!(placement.relative_center_y, 150);

        // Verify boundary guarantee: ring outer edge never exceeds screen edge
        assert!(placement.center_x - placement.radius >= work.left);
        assert!(placement.center_y - placement.radius >= work.top);
    }

    #[test]
    fn test_clamping_bottom_right_corner() {
        // Cursor at extreme bottom-right of work area (1920, 1040)
        let work = ScreenRect::new(0, 0, 1920, 1040);
        let placement = calculate_radial_placement(1920, 1040, 150, 600, 600, work);

        // Center must be clamped so outer radius touches work right/bottom
        assert_eq!(placement.center_x, 1920 - 150); // 1770
        assert_eq!(placement.center_y, 1040 - 150); // 890

        // Window top-left is clamped to max_win_x (1920 - 600 = 1320), max_win_y (1040 - 600 = 440)
        assert_eq!(placement.window_x, 1320);
        assert_eq!(placement.window_y, 440);

        // Relative center inside canvas is 1770 - 1320 = 450, 890 - 440 = 450
        assert_eq!(placement.relative_center_x, 450);
        assert_eq!(placement.relative_center_y, 450);

        // Verify boundary guarantee: ring outer edge never exceeds screen edge
        assert!(placement.center_x + placement.radius <= work.right);
        assert!(placement.center_y + placement.radius <= work.bottom);
    }

    #[test]
    fn test_clamping_secondary_left_monitor_negative_coordinates() {
        // Secondary monitor placed to the left: (-1920, 0) to (0, 1080)
        let work = ScreenRect::new(-1920, 0, 0, 1080);
        let placement = calculate_radial_placement(-1915, 5, 150, 600, 600, work);

        assert_eq!(placement.center_x, -1920 + 150); // -1770
        assert_eq!(placement.center_y, 150);
        assert_eq!(placement.window_x, -1920);
        assert_eq!(placement.window_y, 0);
        assert_eq!(placement.relative_center_x, 150);
        assert_eq!(placement.relative_center_y, 150);

        assert!(placement.center_x - placement.radius >= work.left);
        assert!(placement.center_x + placement.radius <= work.right);
    }

    #[test]
    fn test_clamping_secondary_right_monitor() {
        // Secondary monitor placed to the right: (1920, 0) to (3840, 1080)
        let work = ScreenRect::new(1920, 0, 3840, 1080);
        let placement = calculate_radial_placement(3800, 1000, 150, 600, 600, work);

        assert_eq!(placement.center_x, 3840 - 150); // 3690
        assert_eq!(placement.center_y, 1080 - 150); // 930
        assert_eq!(placement.window_x, 3840 - 600); // 3240
        assert_eq!(placement.window_y, 1080 - 600); // 480
        assert_eq!(placement.relative_center_x, 450);
        assert_eq!(placement.relative_center_y, 450);

        assert!(placement.center_x - placement.radius >= work.left);
        assert!(placement.center_x + placement.radius <= work.right);
    }

    #[test]
    fn test_small_work_area_edge_case() {
        // Narrow display where work width is smaller than 2 * radius
        let work = ScreenRect::new(100, 100, 300, 300); // 200x200
        let placement = calculate_radial_placement(200, 200, 150, 300, 300, work);

        // Clamping should not panic or overflow
        assert!(placement.window_x >= work.left);
        assert!(placement.window_y >= work.top);
    }
}
