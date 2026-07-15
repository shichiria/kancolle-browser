import { useEffect, useRef, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { STORAGE_KEYS } from "./constants";
import type {
  FleetData, PortData, ApiLogEntry,
  SenkaSummary, DriveStatus,
  ExpeditionDef,
  MapRecommendationDef,
  SortieQuestDef, ActiveQuestDetail, QuestProgressSummary,
  SortieRecord, BattleLogsResponse,
  TabId,
} from "./types";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { HomeportTab } from "./components/homeport";
import { BattleTab } from "./components/battle";
import { ShipListTab } from "./components/ships";
import { EquipListTab } from "./components/equips";
import { ImprovementTab } from "./components/improvement";
import { SettingsTab } from "./components/settings";
import { KantaiView } from "./components/kantai";
import { QuestTab } from "./components/quests";
import { DebugTab } from "./components/debug";

// View mode is decided once at startup from the Tauri window label.
// Each window loads the same React bundle; the label distinguishes them.
//   label="management" → full SPA (toolbar + tabs)
//   label="kantai"     → fleet-only view
const VIEW_MODE: "management" | "kantai" | "quests" = (() => {
  try {
    const label = getCurrentWindow().label;
    if (label === "kantai") return "kantai";
    if (label === "quests") return "quests";
    return "management";
  } catch {
    return "management";
  }
})();
console.log(`[App] VIEW_MODE=${VIEW_MODE} label=${getCurrentWindow().label}`);


function App() {
  const [proxyPort, setProxyPort] = useState<number>(0);
  const [portData, setPortData] = useState<PortData | null>(null);
  const [senkaData, setSenkaData] = useState<SenkaSummary | null>(null);
  const [senkaCheckpoint, setSenkaCheckpoint] = useState(false);
  const [apiLog, setApiLog] = useState<ApiLogEntry[]>([]);
  const [now, setNow] = useState(Date.now());
  const [expeditions, setExpeditions] = useState<ExpeditionDef[]>([]);
  const [sortieQuests, setSortieQuests] = useState<SortieQuestDef[]>([]);
  const [mapRecommendations, setMapRecommendations] = useState<MapRecommendationDef[]>([]);
  const [activeQuests, setActiveQuests] = useState<ActiveQuestDetail[]>([]);
  const [questProgress, setQuestProgress] = useState<Map<number, QuestProgressSummary>>(new Map());
  const [portDataVersion, setPortDataVersion] = useState(0);
  const [battleLogs, setBattleLogs] = useState<SortieRecord[]>([]);
  const [battleLogsTotal, setBattleLogsTotal] = useState(0);
  const [battleDateFrom, setBattleDateFrom] = useState(() => {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-01`;
  });
  const [battleDateTo, setBattleDateTo] = useState(() => {
    const now = new Date();
    const lastDay = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(lastDay).padStart(2, "0")}`;
  });
  const [activeTab, setActiveTab] = useState<TabId>("homeport");
  const [uiZoom, setUiZoom] = useState<number>(() => {
    const saved = localStorage.getItem(STORAGE_KEYS.UI_ZOOM);
    return saved ? Number(saved) : 135;
  });
  // Google Drive sync state
  const [driveStatus, setDriveStatus] = useState<DriveStatus>({ authenticated: false, syncing: false });
  const [driveLoading, setDriveLoading] = useState(false);

  const [showApiLog, setShowApiLog] = useState<boolean>(() => {
    return localStorage.getItem(STORAGE_KEYS.SHOW_API_LOG) === "true";
  });
  const [rawApiEnabled, setRawApiEnabled] = useState(true);

  // Weapon icon sprite sheet for damecon indicator
  const [weaponIconSheet, setWeaponIconSheet] = useState<string | null>(null);
  const weaponIconLoadedRef = useRef(false);

  // Tick every second for countdown timers
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, []);

  // Expedition completion notification (1 minute before return)
  const prevNotifyKeyRef = useRef("");
  useEffect(() => {
    if (!portData) return;

    const ready: { fleet_id: number; mission_name: string }[] = [];
    for (const fleet of portData.fleets) {
      if (!fleet.expedition || fleet.expedition.return_time <= 0) continue;
      if (fleet.expedition.return_time - now <= 60000) {
        ready.push({
          fleet_id: fleet.id,
          mission_name: fleet.expedition.mission_name,
        });
      }
    }

    const key = ready.map((f) => f.fleet_id).sort().join(",");
    if (key === prevNotifyKeyRef.current) return;
    prevNotifyKeyRef.current = key;

    if (ready.length > 0) {
      invoke("show_expedition_notification", { notifications: ready }).catch(
        console.error,
      );
    } else {
      invoke("hide_expedition_notification").catch(console.error);
    }
  }, [portData, now]);

  const refreshBattleLogs = useCallback(async () => {
    try {
      let params: Record<string, unknown>;
      if (battleDateFrom && battleDateTo) {
        const from = battleDateFrom.replace(/-/g, "");
        const to = battleDateTo.replace(/-/g, "");
        params = { dateFrom: from, dateTo: to };
      } else {
        params = { limit: 200, offset: 0 };
      }
      const data = await invoke<BattleLogsResponse>("get_battle_logs", params);
      setBattleLogs(data.records);
      setBattleLogsTotal(data.total);
    } catch (e) {
      console.error("Failed to load battle logs:", e);
    }
  }, [battleDateFrom, battleDateTo]);

  // Keep a ref to the latest refreshBattleLogs so event listeners never go stale
  const refreshBattleLogsRef = useRef(refreshBattleLogs);
  useEffect(() => {
    refreshBattleLogsRef.current = refreshBattleLogs;
  }, [refreshBattleLogs]);

  // Re-fetch when view mode or date selection changes
  useEffect(() => {
    refreshBattleLogs();
  }, [refreshBattleLogs]);

  useEffect(() => {
    const unlistenProxy = listen<number>("proxy-ready", (event) => {
      setProxyPort(event.payload);
    });

    const unlistenPort = listen<PortData>("port-data", (event) => {
      setPortData(event.payload);
      setPortDataVersion((v) => v + 1);
      // Load weapon icon sprite sheet once for damecon display
      if (!weaponIconLoadedRef.current) {
        weaponIconLoadedRef.current = true;
        invoke<string>("get_cached_resource", {
          path: "kcs2/img/common/common_icon_weapon.png",
        }).then((dataUri) => {
          if (dataUri) setWeaponIconSheet(dataUri);
        }).catch(() => { weaponIconLoadedRef.current = false; });
      }
    });

    const unlistenSortie = listen<SortieRecord>("sortie-complete", (event) => {
      // Upsert: replace in-progress record or add new
      setBattleLogs((prev) => {
        const idx = prev.findIndex((r) => r.id === event.payload.id);
        if (idx >= 0) {
          const updated = [...prev];
          updated[idx] = event.payload;
          return updated;
        }
        // Only increment total when a genuinely new record is added
        setBattleLogsTotal((prev) => prev + 1);
        return [event.payload, ...prev].slice(0, 200);
      });
    });

    const unlistenSortieUpdate = listen<SortieRecord>("sortie-update", (event) => {
      // Upsert: update existing in-progress record or insert at top
      setBattleLogs((prev) => {
        const idx = prev.findIndex((r) => r.id === event.payload.id);
        if (idx >= 0) {
          const updated = [...prev];
          updated[idx] = event.payload;
          return updated;
        }
        return [event.payload, ...prev].slice(0, 200);
      });
    });

    const unlistenFleet = listen<FleetData[]>("fleet-updated", (event) => {
      setPortData((prev) => {
        if (!prev) return prev;
        return { ...prev, fleets: event.payload };
      });
      setPortDataVersion((v) => v + 1);
    });

    const unlistenQuest = listen<ActiveQuestDetail[]>("quest-list-updated", (event) => {
      setActiveQuests(event.payload);
      // Refresh quest progress when active quests change
      invoke<QuestProgressSummary[]>("get_quest_progress").then((progress) => {
        const map = new Map<number, QuestProgressSummary>();
        for (const p of progress) map.set(p.quest_id, p);
        setQuestProgress(map);
      }).catch(console.error);
    });

    const unlistenQuestProgress = listen<QuestProgressSummary[]>("quest-progress-updated", (event) => {
      const map = new Map<number, QuestProgressSummary>();
      for (const p of event.payload) map.set(p.quest_id, p);
      setQuestProgress(map);
    });

    const unlistenSenka = listen<SenkaSummary>("senka-updated", (event) => {
      setSenkaData(event.payload);
      if (event.payload.checkpoint_crossed) {
        setSenkaCheckpoint(true);
        setTimeout(() => setSenkaCheckpoint(false), 10000);
      }
    });

    const unlistenDriveStatus = listen<DriveStatus>("drive-sync-status", (event) => {
      setDriveStatus(event.payload);
    });

    const unlistenDriveData = listen("drive-data-updated", () => {
      // Reload all data that may have been updated from remote sync
      invoke<QuestProgressSummary[]>("get_quest_progress").then((progress) => {
        const map = new Map<number, QuestProgressSummary>();
        for (const p of progress) map.set(p.quest_id, p);
        setQuestProgress(map);
      }).catch(console.error);
      refreshBattleLogsRef.current();
      // Trigger improvement tab and fleet panels to re-fetch from backend
      setPortDataVersion((v) => v + 1);
    });

    const unlistenApi = listen<{ endpoint: string }>("kancolle-api", (event) => {
      const d = new Date();
      const time = `${d.getHours().toString().padStart(2, "0")}:${d
        .getMinutes()
        .toString()
        .padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}`;
      setApiLog((prev) => [...prev.slice(-200), { time, endpoint: event.payload.endpoint }]);
    });

    invoke<number>("get_proxy_port").then((port) => {
      if (port > 0) {
        setProxyPort(port);
      }
    });

    invoke<ExpeditionDef[]>("get_expeditions").then(setExpeditions).catch(console.error);
    invoke<SortieQuestDef[]>("get_sortie_quests").then(setSortieQuests).catch(console.error);
    invoke<MapRecommendationDef[]>("get_map_recommendations").then(setMapRecommendations).catch(console.error);
    invoke<QuestProgressSummary[]>("get_quest_progress").then((progress) => {
      const map = new Map<number, QuestProgressSummary>();
      for (const p of progress) map.set(p.quest_id, p);
      setQuestProgress(map);
    }).catch(console.error);

    // Load existing battle logs
    refreshBattleLogs();

    // Load Google Drive sync status
    invoke<DriveStatus>("get_drive_status").then(setDriveStatus).catch(console.error);

    // Backend enables complete API diagnostics at the start of every launch.
    // Read its actual state instead of allowing stale localStorage to disable it.
    invoke<boolean>("get_raw_api_enabled")
      .then((enabled) => {
        setRawApiEnabled(enabled);
        localStorage.setItem(STORAGE_KEYS.RAW_API_ENABLED, String(enabled));
      })
      .catch(console.error);

    return () => {
      unlistenProxy.then((f) => f());
      unlistenPort.then((f) => f());
      unlistenFleet.then((f) => f());
      unlistenSortie.then((f) => f());
      unlistenSortieUpdate.then((f) => f());
      unlistenQuest.then((f) => f());
      unlistenQuestProgress.then((f) => f());
      unlistenSenka.then((f) => f());
      unlistenDriveStatus.then((f) => f());
      unlistenDriveData.then((f) => f());
      unlistenApi.then((f) => f());
    };
  }, []);

  if (VIEW_MODE === "quests") {
    return (
      <QuestTab
        sortieQuests={sortieQuests}
        activeQuests={activeQuests}
        questProgress={questProgress}
      />
    );
  }

  if (VIEW_MODE === "kantai") {
    return (
      <KantaiView
        portData={portData}
        now={now}
        expeditions={expeditions}
        sortieQuests={sortieQuests}
        mapRecommendations={mapRecommendations}
        activeQuests={activeQuests}
        questProgress={questProgress}
        portDataVersion={portDataVersion}
        weaponIconSheet={weaponIconSheet}
      />
    );
  }

  return (
    <div className="app" style={{ zoom: uiZoom / 100 }}>
      {/* Toolbar */}
      <div className="toolbar">
        <span className="toolbar-title">KanColle Browser</span>
        <span className={`status ${proxyPort > 0 ? "connected" : ""}`}>
          {proxyPort > 0 ? `Proxy: ${proxyPort}` : "Proxy starting..."}
        </span>
      </div>

      {/* Tab bar */}
      <div className="tab-bar">
        <button
          className={`tab-btn ${activeTab === "homeport" ? "active" : ""}`}
          onClick={() => setActiveTab("homeport")}
        >
          母港
        </button>
        <button
          className={`tab-btn ${activeTab === "battle" ? "active" : ""}`}
          onClick={() => {
            setActiveTab("battle");
            refreshBattleLogs();
          }}
        >
          戦闘
          {battleLogs.length > 0 && (
            <span className="tab-badge">{battleLogs.length}</span>
          )}
        </button>
        <button
          className={`tab-btn ${activeTab === "improvement" ? "active" : ""}`}
          onClick={() => setActiveTab("improvement")}
        >
          改修
        </button>
        <button
          className={`tab-btn ${activeTab === "ships" ? "active" : ""}`}
          onClick={() => setActiveTab("ships")}
        >
          艦娘
        </button>
        <button
          className={`tab-btn ${activeTab === "equips" ? "active" : ""}`}
          onClick={() => setActiveTab("equips")}
        >
          装備
        </button>
        <button
          className={`tab-btn ${activeTab === "debug" ? "active" : ""}`}
          onClick={() => setActiveTab("debug")}
          style={{ marginLeft: "auto" }}
        >
          🐛 Debug
        </button>
        <button
          className={`tab-btn ${activeTab === "options" ? "active" : ""}`}
          onClick={() => setActiveTab("options")}
        >
          設定
        </button>
      </div>

      {/* Main content */}
      <div className="main-content">
        {/* ── Home Port Tab ── */}
        {activeTab === "homeport" && (
          <HomeportTab
            portData={portData} senkaData={senkaData} senkaCheckpoint={senkaCheckpoint}
            now={now} expeditions={expeditions} sortieQuests={sortieQuests}
            mapRecommendations={mapRecommendations} activeQuests={activeQuests}
            questProgress={questProgress} portDataVersion={portDataVersion}
            weaponIconSheet={weaponIconSheet}
            showApiLog={showApiLog} apiLog={apiLog}
          />
        )}

        {/* ── Battle Tab ── */}
        {activeTab === "battle" && (
          <BattleTab
            battleLogs={battleLogs}
            onRefresh={refreshBattleLogs}
            totalRecords={battleLogsTotal}
            dateFrom={battleDateFrom}
            dateTo={battleDateTo}
            onDateChange={(from, to) => { setBattleDateFrom(from); setBattleDateTo(to); }}
          />
        )}
        {activeTab === "improvement" && (
          <ImprovementTab portDataVersion={portDataVersion} />
        )}
        {activeTab === "ships" && (
          <ShipListTab portDataVersion={portDataVersion} />
        )}
        {activeTab === "equips" && (
          <EquipListTab portDataVersion={portDataVersion} />
        )}
        {activeTab === "debug" && <DebugTab />}
        {activeTab === "options" && (
          <SettingsTab
            uiZoom={uiZoom} driveStatus={driveStatus}
            driveLoading={driveLoading} showApiLog={showApiLog}
            rawApiEnabled={rawApiEnabled}
            onZoomChange={setUiZoom}
            onDriveStatusChange={setDriveStatus}
            onDriveLoadingChange={setDriveLoading}
            onShowApiLogChange={setShowApiLog}
            onRawApiChange={setRawApiEnabled}
            onClearBattleLogs={() => { setBattleLogs([]); setBattleLogsTotal(0); }}
          />
        )}
      </div>
    </div>
  );
}

export default App;
