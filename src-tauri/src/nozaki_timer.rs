use serde::{Deserialize, Deserializer, Serialize};
use tauri::Manager;

use crate::api::models::GameStateInner;
use crate::game_window::CONTROL_BAR_HEIGHT;
use crate::ui_event::Screen;
use crate::AppState;

const NOZAKI_ID: i32 = 996;
const NOZAKI_KAI_ID: i32 = 1002;
const SUPPLY_INTERVAL_MS: i64 = 15 * 60 * 1000;
const FLEET_COUNT: usize = 4;

// Native 1200x720 game-canvas coordinates. This is the gap immediately to
// the right of the fourth fleet tab on the composition screen.
const OVERLAY_X: f64 = 360.0;
const OVERLAY_Y: f64 = 174.0;
const OVERLAY_W: f64 = 108.0;
const OVERLAY_H: f64 = 44.0;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct NozakiSupplyTimer {
    signature: String,
    deadline_ms: i64,
    ship_name: String,
    recovery: i32,
    target_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct NozakiSupplyTimers(Vec<Option<NozakiSupplyTimer>>);

impl Default for NozakiSupplyTimers {
    fn default() -> Self {
        Self(vec![None; FLEET_COUNT])
    }
}

impl<'de> Deserialize<'de> for NozakiSupplyTimers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Persisted {
            Multiple(Vec<Option<NozakiSupplyTimer>>),
            Legacy(Option<NozakiSupplyTimer>),
        }

        let mut timers = match Persisted::deserialize(deserializer)? {
            Persisted::Multiple(timers) => timers,
            Persisted::Legacy(timer) => {
                let mut timers = vec![None; FLEET_COUNT];
                timers[2] = timer;
                timers
            }
        };
        timers.resize(FLEET_COUNT, None);
        timers.truncate(FLEET_COUNT);
        Ok(Self(timers))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Eligibility {
    signature: String,
    ship_name: &'static str,
    recovery: i32,
    target_count: usize,
}

fn detect_eligibility(fleet_id: usize, ships: &[(i32, i32)]) -> Option<Eligibility> {
    if ships.len() < 2 {
        return None;
    }

    let provider = ships.first()?;
    let (ship_name, recovery) = match provider.1 {
        NOZAKI_ID => ("野埼", 2),
        NOZAKI_KAI_ID => ("野埼改", 3),
        _ => return None,
    };
    let ship_ids = ships
        .iter()
        .map(|(instance_id, _)| instance_id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    Some(Eligibility {
        signature: format!("{fleet_id}:{ship_ids}:{}", provider.1),
        ship_name,
        recovery,
        target_count: ships.len() - 1,
    })
}

fn reconcile_timer(
    previous: Option<NozakiSupplyTimer>,
    eligibility: Option<Eligibility>,
    observed_at: i64,
    is_port_refresh: bool,
) -> Option<NozakiSupplyTimer> {
    let eligibility = eligibility?;

    if let Some(mut timer) = previous {
        if timer.signature == eligibility.signature {
            timer.ship_name = eligibility.ship_name.to_string();
            timer.recovery = eligibility.recovery;
            timer.target_count = eligibility.target_count;
            if is_port_refresh && observed_at >= timer.deadline_ms {
                timer.deadline_ms = observed_at + SUPPLY_INTERVAL_MS;
            }
            return Some(timer);
        }
    }

    Some(NozakiSupplyTimer {
        signature: eligibility.signature,
        deadline_ms: observed_at + SUPPLY_INTERVAL_MS,
        ship_name: eligibility.ship_name.to_string(),
        recovery: eligibility.recovery,
        target_count: eligibility.target_count,
    })
}

/// Reconcile every fleet's supply timer after a fleet or port update.
pub(crate) fn sync(app: &tauri::AppHandle, game_state: &GameStateInner, is_port_refresh: bool) {
    let eligibility = (0..FLEET_COUNT)
        .map(|index| {
            let fleet_id = index + 1;
            let ships = game_state.profile.fleets.get(index).map(|ship_ids| {
                ship_ids
                    .iter()
                    .map(|instance_id| {
                        let master_id = game_state
                            .profile
                            .ships
                            .get(instance_id)
                            .map(|ship| ship.ship_id)
                            .unwrap_or(0);
                        (*instance_id, master_id)
                    })
                    .collect::<Vec<_>>()
            });
            ships
                .as_deref()
                .and_then(|ships| detect_eligibility(fleet_id, ships))
        })
        .collect::<Vec<_>>();
    let observed_at = chrono::Utc::now().timestamp_millis();
    let app_state = app.state::<AppState>();
    let mut guard = crate::lock_or_recover(
        &app_state.overlay.nozaki_supply_timer,
        "nozaki_supply_timer",
    );
    let previous = guard.clone();
    let next = NozakiSupplyTimers(
        eligibility
            .into_iter()
            .enumerate()
            .map(|(index, eligibility)| {
                reconcile_timer(
                    previous.0.get(index).cloned().flatten(),
                    eligibility,
                    observed_at,
                    is_port_refresh,
                )
            })
            .collect(),
    );
    *guard = next.clone();
    drop(guard);

    if previous != next {
        if let Err(error) =
            crate::settings::persist_json(app, crate::settings::NOZAKI_SUPPLY_TIMER, &next)
        {
            log::warn!("[NozakiTimer] failed to persist timer: {}", error);
        }
    }
    refresh_overlay(app);
}

fn should_show(screen: Screen, fleet: Option<u32>, has_timer: bool) -> bool {
    screen == Screen::FleetComposition
        && fleet.is_some_and(|fleet| (1..=FLEET_COUNT as u32).contains(&fleet))
        && has_timer
}

/// Apply current screen/fleet visibility to the click-through game overlay.
pub(crate) fn refresh_overlay(app: &tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let screen = *crate::lock_or_recover(&app_state.navigation.current_screen, "current_screen");
    let fleet = *crate::lock_or_recover(&app_state.navigation.current_fleet, "current_fleet");
    let timers = crate::lock_or_recover(
        &app_state.overlay.nozaki_supply_timer,
        "nozaki_supply_timer",
    )
    .clone();
    let timer = fleet
        .and_then(|fleet| fleet.checked_sub(1))
        .and_then(|index| timers.0.get(index as usize))
        .cloned()
        .flatten();

    if should_show(screen, fleet, timer.is_some()) {
        if let Some(timer) = timer {
            if let Err(error) = show_overlay(app, &timer) {
                log::warn!("[NozakiTimer] failed to show overlay: {}", error);
            }
        }
    } else {
        hide_overlay(app);
    }
}

fn position_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let game_window = app.get_window("game").ok_or("Game window not found")?;
    let timer_window = app
        .get_window("nozaki-timer")
        .ok_or("Nozaki timer window not found")?;
    let inner_pos = game_window.inner_position().map_err(|e| e.to_string())?;
    let scale = game_window.scale_factor().unwrap_or(1.0);
    let zoom = *crate::lock_or_recover(&app.state::<AppState>().overlay.game_zoom, "game_zoom");

    let dx = (OVERLAY_X * zoom * scale).round() as i32;
    let dy = ((CONTROL_BAR_HEIGHT + OVERLAY_Y) * zoom * scale).round() as i32;
    #[cfg(target_os = "macos")]
    let (dx, dy) = (dx + (6.0 * scale) as i32, dy + (30.0 * scale) as i32);
    let width = (OVERLAY_W * zoom * scale).round().max(1.0) as u32;
    let height = (OVERLAY_H * zoom * scale).round().max(1.0) as u32;

    timer_window
        .set_position(tauri::PhysicalPosition::new(
            inner_pos.x + dx,
            inner_pos.y + dy,
        ))
        .map_err(|e| e.to_string())?;
    timer_window
        .set_size(tauri::PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    if let Some(webview) = app.get_webview("nozaki-timer-content") {
        webview
            .set_size(tauri::PhysicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        webview.set_zoom(zoom).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn show_overlay(app: &tauri::AppHandle, timer: &NozakiSupplyTimer) -> Result<(), String> {
    position_overlay(app)?;
    let webview = app
        .get_webview("nozaki-timer-content")
        .ok_or("Nozaki timer webview not found")?;
    let data = serde_json::to_string(timer).map_err(|e| e.to_string())?;
    webview
        .eval(format!("window.showNozakiTimer({data})"))
        .map_err(|e| e.to_string())?;
    app.get_window("nozaki-timer")
        .ok_or("Nozaki timer window not found")?
        .show()
        .map_err(|e| e.to_string())?;
    app.state::<AppState>()
        .overlay
        .nozaki_timer_visible
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

fn hide_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("nozaki-timer") {
        let _ = window.hide();
    }
    app.state::<AppState>()
        .overlay
        .nozaki_timer_visible
        .store(false, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn reposition_overlay(app: &tauri::AppHandle) {
    if !app
        .state::<AppState>()
        .overlay
        .nozaki_timer_visible
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return;
    }
    if let Err(error) = position_overlay(app) {
        log::warn!("[NozakiTimer] failed to reposition overlay: {}", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_only_flagship() {
        assert_eq!(
            detect_eligibility(1, &[(10, NOZAKI_ID), (20, 1), (30, 2)]).map(|value| value.recovery),
            Some(2)
        );
        assert_eq!(
            detect_eligibility(2, &[(10, NOZAKI_KAI_ID), (20, 1)]).map(|value| value.recovery),
            Some(3)
        );
        assert!(detect_eligibility(3, &[(20, 1), (10, NOZAKI_ID)]).is_none());
        assert!(detect_eligibility(4, &[(10, NOZAKI_ID)]).is_none());
    }

    #[test]
    fn due_port_refresh_starts_the_next_period() {
        let eligibility = detect_eligibility(1, &[(10, NOZAKI_ID), (20, 1)]).unwrap();
        let started = reconcile_timer(None, Some(eligibility.clone()), 1_000, false).unwrap();
        let unchanged = reconcile_timer(
            Some(started.clone()),
            Some(eligibility.clone()),
            2_000,
            true,
        )
        .unwrap();
        assert_eq!(unchanged.deadline_ms, started.deadline_ms);

        let refreshed_at = started.deadline_ms + 5_000;
        let next = reconcile_timer(Some(started), Some(eligibility), refreshed_at, true).unwrap();
        assert_eq!(next.deadline_ms, refreshed_at + SUPPLY_INTERVAL_MS);
    }

    #[test]
    fn overlay_supports_each_fleet_on_composition_screen() {
        assert!(should_show(Screen::FleetComposition, Some(1), true));
        assert!(should_show(Screen::FleetComposition, Some(2), true));
        assert!(should_show(Screen::FleetComposition, Some(3), true));
        assert!(should_show(Screen::FleetComposition, Some(4), true));
        assert!(!should_show(Screen::FleetComposition, Some(5), true));
        assert!(!should_show(Screen::ShipSelection, Some(3), true));
        assert!(!should_show(Screen::FleetComposition, Some(3), false));
    }

    #[test]
    fn legacy_timer_is_migrated_to_third_fleet() {
        let timer = reconcile_timer(
            None,
            detect_eligibility(3, &[(10, NOZAKI_ID), (20, 1)]),
            1_000,
            false,
        )
        .unwrap();
        let json = serde_json::to_string(&timer).unwrap();
        let restored: NozakiSupplyTimers = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.0[2], Some(timer));
        assert!(restored.0[0].is_none());
    }
}
