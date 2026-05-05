import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { FleetPanel } from "../homeport/FleetPanel";
import "./KantaiView.css";
import type {
  PortData, ExpeditionDef, MapRecommendationDef,
  SortieQuestDef, ActiveQuestDetail, QuestProgressSummary,
} from "../../types";

const STORAGE_KEY = "kc-kantai-fleet-id";
const ZOOM_STORAGE_KEY = "kc-kantai-ui-zoom";
const MIN_ZOOM = 50;
const MAX_ZOOM = 200;

interface KantaiViewProps {
  portData: PortData | null;
  now: number;
  expeditions: ExpeditionDef[];
  sortieQuests: SortieQuestDef[];
  mapRecommendations: MapRecommendationDef[];
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
  portDataVersion: number;
  weaponIconSheet: string | null;
}

export function KantaiView({
  portData,
  now,
  expeditions,
  sortieQuests,
  mapRecommendations,
  activeQuests,
  questProgress,
  portDataVersion,
  weaponIconSheet,
}: KantaiViewProps) {
  const [selectedFleetId, setSelectedFleetId] = useState<number>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    const parsed = saved ? Number(saved) : 1;
    return parsed >= 1 && parsed <= 4 ? parsed : 1;
  });

  const [uiZoom, setUiZoom] = useState<number>(() => {
    const saved = localStorage.getItem(ZOOM_STORAGE_KEY);
    const parsed = saved ? Number(saved) : 100;
    return parsed >= MIN_ZOOM && parsed <= MAX_ZOOM ? parsed : 100;
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(selectedFleetId));
  }, [selectedFleetId]);

  useEffect(() => {
    localStorage.setItem(ZOOM_STORAGE_KEY, String(uiZoom));
  }, [uiZoom]);

  // Auto-switch when the user clicks a fleet tab in the game window
  // (emitted from mouse_hook::consume_clicks for FleetSelect / SupplyFleetSelect).
  // Payload is null when the user navigates away from a fleet-bearing screen;
  // we keep the last selection in that case rather than resetting.
  useEffect(() => {
    const unlisten = listen<number | null>("fleet-view-changed", (event) => {
      const fleet = event.payload;
      if (typeof fleet === "number" && fleet >= 1 && fleet <= 4) {
        setSelectedFleetId(fleet);
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  if (!portData) {
    return (
      <div className="kantai-empty">
        ゲーム画面で母港画面を開くとAPIデータが読み込まれます。
      </div>
    );
  }

  const fleets = portData.fleets ?? [];
  const selectedFleet =
    fleets.find((f) => f.id === selectedFleetId) ?? fleets[0];
  const selectedIndex = selectedFleet ? fleets.indexOf(selectedFleet) : 0;

  return (
    <div className="kantai-view">
      <div className="kantai-zoom-wrapper" style={{ zoom: uiZoom / 100 }}>
        <div className="kantai-tabs">
          {[1, 2, 3, 4].map((id) => {
            const fleet = fleets.find((f) => f.id === id);
            const disabled = !fleet;
            return (
              <button
                key={id}
                className={`kantai-tab ${selectedFleetId === id ? "active" : ""}`}
                onClick={() => setSelectedFleetId(id)}
                disabled={disabled}
              >
                第{id}艦隊
              </button>
            );
          })}
        </div>
        {selectedFleet && (
          <div className="kantai-body">
            <FleetPanel
              fleet={selectedFleet}
              now={now}
              fleetIndex={selectedIndex}
              expeditions={expeditions}
              portDataVersion={portDataVersion}
              sortieQuests={sortieQuests}
              mapRecommendations={mapRecommendations}
              activeQuests={activeQuests}
              questProgress={questProgress}
              weaponIconSheet={weaponIconSheet}
            />
          </div>
        )}
      </div>
      <div className="kantai-zoom-bar">
        <span className="kantai-zoom-label">UI</span>
        <input
          type="range"
          min={MIN_ZOOM}
          max={MAX_ZOOM}
          step={5}
          value={uiZoom}
          onChange={(e) => setUiZoom(Number(e.target.value))}
          className="kantai-zoom-slider"
        />
        <span className="kantai-zoom-value">{uiZoom}%</span>
        <button
          className="kantai-zoom-reset"
          onClick={() => setUiZoom(100)}
          title="リセット"
        >
          ↺
        </button>
      </div>
    </div>
  );
}
