//! Shared show/hide/toggle behavior for auxiliary application windows.

use log::info;
use tauri::Manager;

pub const AUXILIARY_WINDOWS: &[&str] = &[
    "management",
    "kantai",
    "quests",
    "improvement",
    "ships",
    "event",
];

fn window(app: &tauri::AppHandle, label: &str) -> Result<tauri::Window, String> {
    app.get_window(label)
        .ok_or_else(|| format!("Window `{label}` not found"))
}

fn show(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    let win = window(app, label)?;
    win.show().map_err(|error| error.to_string())?;
    win.set_focus().map_err(|error| error.to_string())?;
    info!("{} window shown", label);
    Ok(())
}

fn hide(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    window(app, label)?
        .hide()
        .map_err(|error| error.to_string())?;
    info!("{} window hidden", label);
    Ok(())
}

fn toggle(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    if window(app, label)?
        .is_visible()
        .map_err(|error| error.to_string())?
    {
        hide(app, label)
    } else {
        show(app, label)
    }
}

macro_rules! window_commands {
    ($show:ident, $hide:ident, $toggle:ident, $label:literal) => {
        #[tauri::command]
        pub(crate) fn $show(app: tauri::AppHandle) -> Result<(), String> {
            crate::action_log::record("Command", stringify!($show), None);
            show(&app, $label)
        }

        #[tauri::command]
        pub(crate) fn $hide(app: tauri::AppHandle) -> Result<(), String> {
            crate::action_log::record("Command", stringify!($hide), None);
            hide(&app, $label)
        }

        #[tauri::command]
        pub(crate) fn $toggle(app: tauri::AppHandle) -> Result<(), String> {
            crate::action_log::record("Command", stringify!($toggle), None);
            toggle(&app, $label)
        }
    };
}

window_commands!(
    show_management_window,
    hide_management_window,
    toggle_management_window,
    "management"
);
window_commands!(
    show_kantai_window,
    hide_kantai_window,
    toggle_kantai_window,
    "kantai"
);
window_commands!(
    show_quests_window,
    hide_quests_window,
    toggle_quests_window,
    "quests"
);
window_commands!(
    show_improvement_window,
    hide_improvement_window,
    toggle_improvement_window,
    "improvement"
);
window_commands!(
    show_ships_window,
    hide_ships_window,
    toggle_ships_window,
    "ships"
);
#[tauri::command]
pub(crate) fn show_event_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    crate::action_log::record("Command", "show_event_window", None);
    show(&app, "event")?;
    app.emit(crate::events::EVENT_WINDOW_OPENED, ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn hide_event_window(app: tauri::AppHandle) -> Result<(), String> {
    crate::action_log::record("Command", "hide_event_window", None);
    hide(&app, "event")
}

#[tauri::command]
pub(crate) fn toggle_event_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    crate::action_log::record("Command", "toggle_event_window", None);
    if window(&app, "event")?
        .is_visible()
        .map_err(|error| error.to_string())?
    {
        hide(&app, "event")
    } else {
        show(&app, "event")?;
        app.emit(crate::events::EVENT_WINDOW_OPENED, ())
            .map_err(|error| error.to_string())
    }
}

pub fn intercept_close_as_hide(app: &tauri::AppHandle) {
    for &label in AUXILIARY_WINDOWS {
        let Some(win) = app.get_window(label) else {
            continue;
        };
        let handle = app.clone();
        win.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(win) = handle.get_window(label) {
                    if let Err(error) = win.hide() {
                        log::warn!("Failed to hide {} window on close: {}", label, error);
                    }
                }
            }
        });
    }
}
