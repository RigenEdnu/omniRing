pub mod manager;

pub use manager::{
    apply_overlay_styles, calculate_clamped_position, calculate_radial_placement,
    get_current_cursor_pos, get_monitor_geometry_from_point, get_radial_placement, hide_overlay,
    show_overlay_at_cursor, show_overlay_clamped, MonitorGeometry, RadialPlacement, ScreenRect,
};

