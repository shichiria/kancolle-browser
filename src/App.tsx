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
import { HomeportTab } from "./components/homeport";
import { BattleTab } from "./components/battle";
import { ShipListTab } from "./components/ships";
import { EquipListTab } from "./components/equips";
import { ImprovementTab } from "./components/improvement";
import { SettingsTab } from "./components/settings";


function App() {
  const [proxyPort, setProxyPort] = useState<number>(0);
  const [portData, setPortData] = useState<PortData | null>(null);
  const [senkaData, setSenkaData] = useState<SenkaSummary | null>(null);
  const [senkaCheckpoint, setSenkaCheckpoint] = useState(false);
  const [apiLog, setApiLog] = useState<ApiLogEntry[]>([]);
  const [gameOpen, setGameOpen] = useState(false);
  const [caInstalled, setCaInstalled] = useState<boolean | null>(null);
  const [caInstalling, setCaInstalling] = useState(false);
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
  const [rawApiEnabled, setRawApiEnabled] = useState<boolean>(() => {
    return localStorage.getItem(STORAGE_KEYS.RAW_API_ENABLED) === "true";
  });

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
    if (!portData || !gameOpen) return;

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
  }, [portData, now, gameOpen]);

  // Check CA status
  const checkCa = useCallback(async () => {
    try {
      const installed = await invoke<boolean>("is_ca_installed");
      setCaInstalled(installed);
    } catch {
      setCaInstalled(false);
    }
  }, []);

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
      checkCa();
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
        checkCa();
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

    // Restore raw API enabled state from localStorage to backend
    const savedRawApi = localStorage.getItem(STORAGE_KEYS.RAW_API_ENABLED) === "true";
    if (savedRawApi) {
      invoke("set_raw_api_enabled", { enabled: true }).catch(console.error);
    }

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
  }, [checkCa]);

  const installCa = async () => {
    setCaInstalling(true);
    try {
      await invoke("install_ca_cert");
      setCaInstalled(true);
    } catch (e) {
      console.error("CA install failed:", e);
      alert(`CA証明書のインストールに失敗しました: ${e}`);
    } finally {
      setCaInstalling(false);
    }
  };

  const openGame = async () => {
    try {
      await invoke("open_game_window");
      setGameOpen(true);
    } catch (e) {
      console.error("Failed to open game window:", e);
      alert(`ゲームウィンドウを開けませんでした: ${e}`);
    }
  };

  const closeGame = async () => {
    try {
      await invoke("close_game_window");
      setGameOpen(false);
    } catch (e) {
      console.error("Failed to close game window:", e);
    }
  };

  return (
    <div className="app" style={{ zoom: uiZoom / 100 }}>
      {/* Toolbar */}
      <div className="toolbar">
        <span className="toolbar-title">KanColle Browser</span>

        {proxyPort > 0 && caInstalled === false && (
          <button
            className="ca-btn"
            onClick={installCa}
            disabled={caInstalling}
          >
            {caInstalling ? "Installing..." : "Install CA Cert"}
          </button>
        )}

        {!gameOpen ? (
          <button onClick={openGame} disabled={proxyPort === 0 || caInstalled !== true}>
            Open Game
          </button>
        ) : (
          <button onClick={closeGame}>Close Game</button>
        )}

        <span className={`status ${proxyPort > 0 ? "connected" : ""}`}>
          {proxyPort > 0 ? `Proxy: ${proxyPort}` : "Proxy starting..."}
        </span>

        {proxyPort > 0 && caInstalled !== null && (
          <span className={`status ${caInstalled ? "connected" : "ca-warning"}`}>
            {caInstalled ? "CA: OK" : "CA: Not Installed"}
          </span>
        )}
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
          className={`tab-btn ${activeTab === "options" ? "active" : ""}`}
          onClick={() => setActiveTab("options")}
          style={{ marginLeft: "auto" }}
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
            weaponIconSheet={weaponIconSheet} caInstalled={caInstalled}
            gameOpen={gameOpen} showApiLog={showApiLog} apiLog={apiLog}
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
