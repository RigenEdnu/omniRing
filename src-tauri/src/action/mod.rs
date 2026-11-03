pub mod dispatcher;

pub use dispatcher::{
    execute_action, execute_open_app, execute_open_url, execute_sequence, execute_shortcut,
    execute_type_text, is_extended_key_code, parse_key_to_vk, plan_shortcut_events,
    plan_type_text_events, ActionError, RawKeyEvent,
};
