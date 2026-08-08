use serde::{Deserialize, Deserializer, Serialize};
use tauri::Manager;

use crate::api::models::GameStateInner;
use crate::game_window::CONTROL_BAR_HEIGHT;
use crate::ui_event::Screen;
use crate::AppState;

const AKASHI_ID: i32 = 182;
const AKASHI_KAI_ID: i32 = 187;
const ASAHI_KAI_ID: i32 = 958;
const NOZAKI_ID: i32 = 996;
const NOZAKI_KAI_ID: i32 = 1002;
const REPAIR_FACILITY_ID: i32 = 86;
const SUPPLY_INTERVAL_MS: i64 = 15 * 60 * 1000;
const REPAIR_INTERVAL_MS: i64 = 20 * 60 * 1000;
const FLEET_COUNT: usize = 4;

// Native 1200x720 game-canvas coordinates. This is the gap immediately to
// the right of the fourth fleet tab on the composition screen. Two cards fit
// side by side when Akashi and Nozaki are used together.
const OVERLAY_X: f64 = 360.0;
const OVERLAY_Y: f64 = 174.0;
const OVERLAY_W: f64 = 220.0;
const OVERLAY_H: f64 = 44.0;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct FleetTimerInfo {
    /// Fleet composition only. Equipment changes and remodeling do not reset
    /// either game timer, so neither is included in this signature.
    signature: String,
    ship_name: String,
    effect: String,
    target_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct MechanismTimer {
    deadline_ms: Option<i64>,
    fleets: Vec<Option<FleetTimerInfo>>,
}

impl Default for MechanismTimer {
    fn default() -> Self {
        Self {
            deadline_ms: None,
            fleets: vec![None; FLEET_COUNT],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SupportTimers {
    nozaki: MechanismTimer,
    repair: MechanismTimer,
}

impl Default for SupportTimers {
    fn default() -> Self {
        Self {
            nozaki: MechanismTimer::default(),
            repair: MechanismTimer::default(),
        }
    }
}

impl SupportTimers {
    fn normalize(&mut self) {
        for mechanism in [&mut self.nozaki, &mut self.repair] {
            mechanism.fleets.resize(FLEET_COUNT, None);
            mechanism.fleets.truncate(FLEET_COUNT);
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyNozakiTimer {
    signature: String,
    deadline_ms: i64,
    ship_name: String,
    recovery: i32,
    target_count: usize,
}

/// The former implementation persisted either one Nozaki timer or a vector of
/// per-fleet timers. Accept both shapes so existing users keep their countdown.
impl<'de> Deserialize<'de> for SupportTimers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Current {
            #[serde(default)]
            nozaki: MechanismTimer,
            #[serde(default)]
            repair: MechanismTimer,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Persisted {
            Multiple(Vec<Option<LegacyNozakiTimer>>),
            Legacy(Option<LegacyNozakiTimer>),
            Current(Current),
        }

        let mut state = match Persisted::deserialize(deserializer)? {
            Persisted::Multiple(mut timers) => {
                timers.resize(FLEET_COUNT, None);
                timers.truncate(FLEET_COUNT);
                let deadline_ms = timers
                    .iter()
                    .filter_map(|timer| timer.as_ref().map(|timer| timer.deadline_ms))
                    .min();
                let fleets = timers
                    .into_iter()
                    .map(|timer| timer.map(legacy_nozaki_info))
                    .collect();
                Self {
                    nozaki: MechanismTimer {
                        deadline_ms,
                        fleets,
                    },
                    repair: MechanismTimer::default(),
                }
            }
            Persisted::Legacy(timer) => {
                let mut fleets = vec![None; FLEET_COUNT];
                let deadline_ms = timer.as_ref().map(|timer| timer.deadline_ms);
                // The legacy single timer was shown only for the third fleet.
                fleets[2] = timer.map(legacy_nozaki_info);
                Self {
                    nozaki: MechanismTimer {
                        deadline_ms,
                        fleets,
                    },
                    repair: MechanismTimer::default(),
                }
            }
            Persisted::Current(current) => Self {
                nozaki: current.nozaki,
                repair: current.repair,
            },
        };
        state.normalize();
        Ok(state)
    }
}

fn legacy_nozaki_info(timer: LegacyNozakiTimer) -> FleetTimerInfo {
    FleetTimerInfo {
        signature: timer.signature,
        ship_name: timer.ship_name,
        effect: format!("給糧 COND+{}", timer.recovery),
        target_count: timer.target_count,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FleetShipSnapshot {
    instance_id: i32,
    master_id: i32,
    repair_facilities: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncReason {
    Observe,
    CompositionChange,
    PortRefresh,
}

fn fleet_signature(fleet_id: usize, ships: &[FleetShipSnapshot]) -> String {
    let ship_ids = ships
        .iter()
        .map(|ship| ship.instance_id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{fleet_id}:{ship_ids}")
}

fn detect_nozaki(fleet_id: usize, ships: &[FleetShipSnapshot]) -> Option<FleetTimerInfo> {
    if ships.len() < 2 {
        return None;
    }

    let provider = ships
        .iter()
        .take(2)
        .find(|ship| matches!(ship.master_id, NOZAKI_ID | NOZAKI_KAI_ID))?;
    let (ship_name, recovery) = match provider.master_id {
        NOZAKI_ID => ("野埼", 2),
        NOZAKI_KAI_ID => ("野埼改", 3),
        _ => unreachable!(),
    };

    Some(FleetTimerInfo {
        signature: fleet_signature(fleet_id, ships),
        ship_name: ship_name.to_string(),
        effect: format!("給糧 COND+{recovery}"),
        target_count: ships.len() - 1,
    })
}

fn is_workship(master_id: i32) -> bool {
    matches!(master_id, AKASHI_ID | AKASHI_KAI_ID | ASAHI_KAI_ID)
}

fn detect_repair(fleet_id: usize, ships: &[FleetShipSnapshot]) -> Option<FleetTimerInfo> {
    let provider = ships.first()?;
    if !is_workship(provider.master_id) {
        return None;
    }

    let (ship_name, base_range) = match provider.master_id {
        AKASHI_ID => ("明石", 2),
        AKASHI_KAI_ID => ("明石改", 2),
        ASAHI_KAI_ID => ("朝日改", 0),
        _ => unreachable!(),
    };
    let mut repair_range = base_range + provider.repair_facilities;

    // When Akashi and Asahi Kai occupy the first two positions, facilities on
    // the second workship also extend the repair range. Two of the same ship do
    // not receive this pairing bonus.
    if let Some(partner) = ships.get(1) {
        let is_pair = is_workship(partner.master_id)
            && (provider.master_id == ASAHI_KAI_ID) != (partner.master_id == ASAHI_KAI_ID);
        if is_pair {
            repair_range += partner.repair_facilities;
        }
    }
    let target_count = repair_range.min(ships.len());
    if target_count == 0 {
        return None;
    }

    Some(FleetTimerInfo {
        signature: fleet_signature(fleet_id, ships),
        ship_name: ship_name.to_string(),
        effect: format!("泊地修理 上位{target_count}隻"),
        target_count,
    })
}

fn signatures_changed(
    previous: &[Option<FleetTimerInfo>],
    next: &[Option<FleetTimerInfo>],
) -> bool {
    (0..FLEET_COUNT).any(|index| {
        previous
            .get(index)
            .and_then(Option::as_ref)
            .map(|info| info.signature.as_str())
            != next
                .get(index)
                .and_then(Option::as_ref)
                .map(|info| info.signature.as_str())
    })
}

fn reconcile_mechanism(
    previous: &MechanismTimer,
    mut fleets: Vec<Option<FleetTimerInfo>>,
    observed_at: i64,
    interval_ms: i64,
    reason: SyncReason,
) -> MechanismTimer {
    fleets.resize(FLEET_COUNT, None);
    fleets.truncate(FLEET_COUNT);
    let has_eligible_fleet = fleets.iter().any(Option::is_some);
    let changed = signatures_changed(&previous.fleets, &fleets);
    let mut deadline_ms = previous.deadline_ms;

    if has_eligible_fleet {
        match reason {
            SyncReason::CompositionChange if changed => {
                deadline_ms = Some(observed_at + interval_ms);
            }
            SyncReason::PortRefresh
                if deadline_ms.map_or(true, |deadline| observed_at >= deadline) =>
            {
                deadline_ms = Some(observed_at + interval_ms);
            }
            SyncReason::Observe if deadline_ms.is_none() => {
                deadline_ms = Some(observed_at + interval_ms);
            }
            _ => {}
        }
    }

    MechanismTimer {
        deadline_ms,
        fleets,
    }
}

fn snapshot_fleets(game_state: &GameStateInner) -> Vec<Vec<FleetShipSnapshot>> {
    (0..FLEET_COUNT)
        .map(|index| {
            game_state
                .profile
                .fleets
                .get(index)
                .map(|ship_ids| {
                    ship_ids
                        .iter()
                        .map(|instance_id| {
                            let ship = game_state.profile.ships.get(instance_id);
                            let repair_facilities = ship
                                .into_iter()
                                .flat_map(|ship| ship.slot.iter())
                                .filter_map(|slot_id| game_state.profile.slotitems.get(slot_id))
                                .filter(|item| item.slotitem_id == REPAIR_FACILITY_ID)
                                .count();
                            FleetShipSnapshot {
                                instance_id: *instance_id,
                                master_id: ship.map(|ship| ship.ship_id).unwrap_or(0),
                                repair_facilities,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Reconcile the shared Nozaki (15-minute) and anchorage-repair (20-minute)
/// countdowns after an observed game-state update.
pub(crate) fn sync(app: &tauri::AppHandle, game_state: &GameStateInner, reason: SyncReason) {
    let fleets = snapshot_fleets(game_state);
    let nozaki = fleets
        .iter()
        .enumerate()
        .map(|(index, ships)| detect_nozaki(index + 1, ships))
        .collect::<Vec<_>>();
    let repair = fleets
        .iter()
        .enumerate()
        .map(|(index, ships)| detect_repair(index + 1, ships))
        .collect::<Vec<_>>();

    let observed_at = chrono::Utc::now().timestamp_millis();
    let app_state = app.state::<AppState>();
    let mut guard = crate::lock_or_recover(&app_state.overlay.support_timers, "support_timers");
    let previous = guard.clone();
    let next = SupportTimers {
        nozaki: reconcile_mechanism(
            &previous.nozaki,
            nozaki,
            observed_at,
            SUPPLY_INTERVAL_MS,
            reason,
        ),
        repair: reconcile_mechanism(
            &previous.repair,
            repair,
            observed_at,
            REPAIR_INTERVAL_MS,
            reason,
        ),
    };
    *guard = next.clone();
    drop(guard);

    if previous != next {
        if let Err(error) =
            crate::settings::persist_json(app, crate::settings::SUPPORT_TIMERS, &next)
        {
            log::warn!("[SupportTimer] failed to persist timers: {}", error);
        }
    }
    refresh_overlay(app);
}

fn should_show(screen: Screen, fleet: Option<u32>, timer_count: usize) -> bool {
    screen == Screen::FleetComposition
        && fleet.is_some_and(|fleet| (1..=FLEET_COUNT as u32).contains(&fleet))
        && timer_count > 0
}

#[derive(Serialize)]
struct OverlayTimer {
    kind: &'static str,
    deadline_ms: i64,
    ship_name: String,
    effect: String,
    target_count: usize,
}

fn overlay_timers(state: &SupportTimers, fleet_index: usize) -> Vec<OverlayTimer> {
    let mut timers = Vec::with_capacity(2);
    if let (Some(deadline_ms), Some(info)) = (
        state.nozaki.deadline_ms,
        state.nozaki.fleets.get(fleet_index).cloned().flatten(),
    ) {
        timers.push(OverlayTimer {
            kind: "supply",
            deadline_ms,
            ship_name: info.ship_name,
            effect: info.effect,
            target_count: info.target_count,
        });
    }
    if let (Some(deadline_ms), Some(info)) = (
        state.repair.deadline_ms,
        state.repair.fleets.get(fleet_index).cloned().flatten(),
    ) {
        timers.push(OverlayTimer {
            kind: "repair",
            deadline_ms,
            ship_name: info.ship_name,
            effect: info.effect,
            target_count: info.target_count,
        });
    }
    timers
}

/// Apply current screen/fleet visibility to the click-through game overlay.
pub(crate) fn refresh_overlay(app: &tauri::AppHandle) {
    let app_state = app.state::<AppState>();
    let screen = *crate::lock_or_recover(&app_state.navigation.current_screen, "current_screen");
    let fleet = *crate::lock_or_recover(&app_state.navigation.current_fleet, "current_fleet");
    let state = crate::lock_or_recover(&app_state.overlay.support_timers, "support_timers").clone();
    let timers = fleet
        .and_then(|fleet| fleet.checked_sub(1))
        .map(|index| overlay_timers(&state, index as usize))
        .unwrap_or_default();

    if should_show(screen, fleet, timers.len()) {
        if let Err(error) = show_overlay(app, &timers) {
            log::warn!("[SupportTimer] failed to show overlay: {}", error);
        }
    } else {
        hide_overlay(app);
    }
}

fn position_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let game_window = app.get_window("game").ok_or("Game window not found")?;
    let timer_window = app
        .get_window("nozaki-timer")
        .ok_or("Support timer window not found")?;
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

fn show_overlay(app: &tauri::AppHandle, timers: &[OverlayTimer]) -> Result<(), String> {
    position_overlay(app)?;
    let webview = app
        .get_webview("nozaki-timer-content")
        .ok_or("Support timer webview not found")?;
    let data = serde_json::to_string(timers).map_err(|e| e.to_string())?;
    webview
        .eval(format!("window.showSupportTimers({data})"))
        .map_err(|e| e.to_string())?;
    app.get_window("nozaki-timer")
        .ok_or("Support timer window not found")?
        .show()
        .map_err(|e| e.to_string())?;
    app.state::<AppState>()
        .overlay
        .support_timer_visible
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

fn hide_overlay(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("nozaki-timer") {
        let _ = window.hide();
    }
    app.state::<AppState>()
        .overlay
        .support_timer_visible
        .store(false, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn reposition_overlay(app: &tauri::AppHandle) {
    if !app
        .state::<AppState>()
        .overlay
        .support_timer_visible
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return;
    }
    if let Err(error) = position_overlay(app) {
        log::warn!("[SupportTimer] failed to reposition overlay: {}", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ship(instance_id: i32, master_id: i32) -> FleetShipSnapshot {
        FleetShipSnapshot {
            instance_id,
            master_id,
            repair_facilities: 0,
        }
    }

    fn ship_with_facilities(
        instance_id: i32,
        master_id: i32,
        repair_facilities: usize,
    ) -> FleetShipSnapshot {
        FleetShipSnapshot {
            instance_id,
            master_id,
            repair_facilities,
        }
    }

    #[test]
    fn nozaki_works_as_flagship_or_second_ship() {
        assert_eq!(
            detect_nozaki(1, &[ship(10, NOZAKI_ID), ship(20, 1), ship(30, 2)])
                .map(|value| value.effect),
            Some("給糧 COND+2".to_string())
        );
        assert_eq!(
            detect_nozaki(2, &[ship(20, 1), ship(10, NOZAKI_KAI_ID), ship(30, 2)])
                .map(|value| value.effect),
            Some("給糧 COND+3".to_string())
        );
        assert!(detect_nozaki(3, &[ship(20, 1), ship(30, 2), ship(10, NOZAKI_ID)]).is_none());
        assert!(detect_nozaki(4, &[ship(10, NOZAKI_ID)]).is_none());
    }

    #[test]
    fn akashi_repair_range_is_two_plus_repair_facilities() {
        let info = detect_repair(
            1,
            &[
                ship_with_facilities(10, AKASHI_KAI_ID, 2),
                ship(20, 1),
                ship(30, 2),
                ship(40, 3),
                ship(50, 4),
            ],
        )
        .unwrap();
        assert_eq!(info.target_count, 4);
        assert_eq!(info.effect, "泊地修理 上位4隻");
        assert!(detect_repair(2, &[ship(10, NOZAKI_ID), ship(20, 1)]).is_none());
    }

    #[test]
    fn akashi_and_asahi_pair_adds_second_ship_facilities() {
        let info = detect_repair(
            1,
            &[
                ship_with_facilities(10, AKASHI_KAI_ID, 1),
                ship_with_facilities(20, ASAHI_KAI_ID, 2),
                ship(30, 2),
                ship(40, 3),
                ship(50, 4),
            ],
        )
        .unwrap();
        assert_eq!(info.target_count, 5);
    }

    #[test]
    fn common_timer_resets_for_manual_composition_change() {
        let first = vec![
            detect_nozaki(1, &[ship(10, NOZAKI_ID), ship(20, 1)]),
            detect_nozaki(2, &[ship(30, NOZAKI_ID), ship(40, 1)]),
            None,
            None,
        ];
        let started = reconcile_mechanism(
            &MechanismTimer::default(),
            first.clone(),
            1_000,
            SUPPLY_INTERVAL_MS,
            SyncReason::CompositionChange,
        );
        assert_eq!(started.deadline_ms, Some(1_000 + SUPPLY_INTERVAL_MS));

        let changed = vec![
            detect_nozaki(1, &[ship(10, NOZAKI_ID), ship(21, 1)]),
            first[1].clone(),
            None,
            None,
        ];
        let reset = reconcile_mechanism(
            &started,
            changed,
            2_000,
            SUPPLY_INTERVAL_MS,
            SyncReason::CompositionChange,
        );
        assert_eq!(reset.deadline_ms, Some(2_000 + SUPPLY_INTERVAL_MS));
        assert_ne!(reset.deadline_ms, started.deadline_ms);
    }

    #[test]
    fn remodel_and_equipment_observations_do_not_reset_timer() {
        let base = vec![
            detect_nozaki(1, &[ship(10, NOZAKI_ID), ship(20, 1)]),
            None,
            None,
            None,
        ];
        let started = reconcile_mechanism(
            &MechanismTimer::default(),
            base,
            1_000,
            SUPPLY_INTERVAL_MS,
            SyncReason::Observe,
        );
        let remodeled = vec![
            detect_nozaki(1, &[ship(10, NOZAKI_KAI_ID), ship(20, 1)]),
            None,
            None,
            None,
        ];
        let unchanged = reconcile_mechanism(
            &started,
            remodeled,
            2_000,
            SUPPLY_INTERVAL_MS,
            SyncReason::Observe,
        );
        assert_eq!(unchanged.deadline_ms, started.deadline_ms);
        assert_eq!(unchanged.fleets[0].as_ref().unwrap().effect, "給糧 COND+3");
    }

    #[test]
    fn due_port_refresh_starts_the_next_period() {
        let fleets = vec![
            detect_repair(1, &[ship(10, AKASHI_ID), ship(20, 1)]),
            None,
            None,
            None,
        ];
        let started = reconcile_mechanism(
            &MechanismTimer::default(),
            fleets.clone(),
            1_000,
            REPAIR_INTERVAL_MS,
            SyncReason::Observe,
        );
        let early = reconcile_mechanism(
            &started,
            fleets.clone(),
            2_000,
            REPAIR_INTERVAL_MS,
            SyncReason::PortRefresh,
        );
        assert_eq!(early.deadline_ms, started.deadline_ms);

        let refreshed_at = started.deadline_ms.unwrap() + 5_000;
        let next = reconcile_mechanism(
            &started,
            fleets,
            refreshed_at,
            REPAIR_INTERVAL_MS,
            SyncReason::PortRefresh,
        );
        assert_eq!(next.deadline_ms, Some(refreshed_at + REPAIR_INTERVAL_MS));
    }

    #[test]
    fn overlay_supports_both_timers_on_each_fleet() {
        assert!(should_show(Screen::FleetComposition, Some(1), 2));
        assert!(should_show(Screen::FleetComposition, Some(4), 1));
        assert!(!should_show(Screen::FleetComposition, Some(5), 1));
        assert!(!should_show(Screen::ShipSelection, Some(3), 1));
        assert!(!should_show(Screen::FleetComposition, Some(3), 0));
    }

    #[test]
    fn legacy_timer_is_migrated_to_third_fleet() {
        let json = r#"{"signature":"3:10,20:996","deadline_ms":901000,"ship_name":"野埼","recovery":2,"target_count":1}"#;
        let restored: SupportTimers = serde_json::from_str(json).unwrap();
        assert_eq!(restored.nozaki.deadline_ms, Some(901000));
        assert_eq!(
            restored.nozaki.fleets[2].as_ref().unwrap().effect,
            "給糧 COND+2"
        );
        assert!(restored.nozaki.fleets[0].is_none());
    }
}
