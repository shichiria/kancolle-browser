import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EVENTS } from "../../constants";
import "./DebugTab.css";

interface ClickEntry {
  id: string;
  ts: string;
  x: number;
  y: number;
  screen: string;
  screenAfter: string;
  screenChangeClick: boolean;
  targetScreen?: string;
  event: unknown;
  image?: string;
  imagePath?: string;
  sampleKind?: "known" | "unknown";
}

interface ClickScreenshotPayload {
  id: string;
  ts: string;
  x: number;
  y: number;
  image: string;
  imagePath: string;
  sampleKind: "known" | "unknown";
  width: number;
  height: number;
}

interface ApiEntry {
  ts: string;
  endpoint: string;
}

interface ScreenSampleSummary {
  known: number;
  unknown: number;
  directory: string;
}

const MAX_ENTRIES = 30;

const SCREEN_JA: Record<string, string> = {
  GameStart: "ゲーム開始",
  Homeport: "母港",
  SortieMenu: "出撃メニュー",
  SortieSelect: "出撃選択",
  SortieSelectChinjufu: "海域選択-鎮守府海域",
  SortieSelectSouthwestIslands: "海域選択-南西諸島海域",
  SortieSelectNorthern: "海域選択-北方海域",
  SortieSelectSouthwestern: "海域選択-南西海域",
  SortieSelectWestern: "海域選択-西方海域",
  SortieSelectSouthern: "海域選択-南方海域",
  SortieSelectCentral: "海域選択-中部海域",
  SortieSelectEvent: "海域選択-期間限定海域",
  AirBaseSupply: "基地航空隊",
  AirBaseSupply1: "基地航空隊-第一基地航空隊",
  AirBaseSupply2: "基地航空隊-第二基地航空隊",
  AirBaseSupply3: "基地航空隊-第三基地航空隊",
  SortieInProgress: "出撃中",
  ExpeditionSelect: "遠征選択",
  ExpeditionFleetSelect: "遠征-艦隊選択",
  FleetComposition: "編成",
  ShipSelection: "編成-艦船選択",
  ShipChangeConfirm: "編成-変更確認",
  Remodel: "改装",
  RemodelEquipmentSelect: "改装-装備選択",
  RemodelEquipmentFilter: "改装-装備種別選択",
  RemodelEquipmentConfirm: "改装-装備変更確認",
  Resupply: "補給",
  RepairDockSelect: "入渠-ドック選択",
  RepairShipSelect: "入渠-艦船選択",
  Factory: "工廠",
  FactoryDevelop: "工廠-開発",
  QuestList: "任務",
  Encyclopedia: "図鑑表示",
  ItemList: "アイテム一覧",
  ItemListHeld: "アイテム一覧-保有アイテム",
  ItemListPurchased: "アイテム一覧-購入済みアイテム",
  ItemListExpansion: "アイテム一覧-拡張アイテム",
  ItemShop: "アイテム屋",
  ItemShopRegular: "アイテム屋-レギュラーコーナー",
  ItemShopSpecial: "アイテム屋-特選コーナー",
  FurnitureChange: "模様替え",
  FurnitureShopCategory: "家具屋",
  FurnitureShopList: "家具一覧",
  GetScreen: "GET画面",
  Unknown: "???",
  "???": "???",
};

function withJa(screenName: string): string {
  if (screenName === "Unknown" || screenName === "???") return "???";
  const ja = SCREEN_JA[screenName];
  return ja ? `${screenName} (${ja})` : screenName;
}

const FLEET_SCREENS = new Set([
  "FleetComposition",
  "Resupply",
  "Remodel",
  "RemodelEquipmentSelect",
  "RemodelEquipmentFilter",
  "RemodelEquipmentConfirm",
  "ExpeditionFleetSelect",
]);

function withFleet(screenName: string, fleet: number | null): string {
  const base = withJa(screenName);
  if (fleet && FLEET_SCREENS.has(screenName)) {
    const label = fleet === 5 ? "他" : `第${fleet}艦隊`;
    return `${base} - ${label}`;
  }
  return base;
}

function withQuestFilters(
  screenName: string,
  period: string | null,
  category: string | null
): string {
  const base = withJa(screenName);
  if (screenName !== "QuestList") return base;
  const parts = [period, category].filter((s): s is string => !!s);
  return parts.length > 0 ? `${base} - ${parts.join(" × ")}` : base;
}

function formatEvent(event: unknown): string {
  if (typeof event === "string") return event;
  if (event && typeof event === "object") {
    const entries = Object.entries(event as Record<string, unknown>);
    if (entries.length === 1) {
      const [name, payload] = entries[0];
      if (payload && typeof payload === "object") {
        const inner = Object.entries(payload as Record<string, unknown>)
          .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
          .join(" ");
        return `${name}{${inner}}`;
      }
      return name;
    }
  }
  return JSON.stringify(event);
}

interface QuestFilters {
  period: string | null;
  category: string | null;
}

export function DebugTab() {
  const [currentScreen, setCurrentScreen] = useState<string>("(loading)");
  const [currentFleet, setCurrentFleet] = useState<number | null>(null);
  const [questFilters, setQuestFilters] = useState<QuestFilters>({
    period: null,
    category: null,
  });
  const [screenChangedAt, setScreenChangedAt] = useState<string>("-");
  const [clicks, setClicks] = useState<ClickEntry[]>([]);
  const [apis, setApis] = useState<ApiEntry[]>([]);
  const [paused, setPaused] = useState(false);
  const [samples, setSamples] = useState<ScreenSampleSummary>({
    known: 0,
    unknown: 0,
    directory: "",
  });
  const pausedRef = useRef(paused);
  pausedRef.current = paused;

  useEffect(() => {
    invoke<string>("get_current_screen").then(setCurrentScreen).catch(() => {});
    invoke<number | null>("get_current_fleet").then(setCurrentFleet).catch(() => {});
    invoke<QuestFilters>("get_quest_filters").then(setQuestFilters).catch(() => {});
    invoke<ScreenSampleSummary>("get_screen_sample_summary")
      .then(setSamples)
      .catch(() => {});

    const unlistenScreen = listen<string>(EVENTS.SCREEN_CHANGED, (event) => {
      if (pausedRef.current) return;
      setCurrentScreen(event.payload);
      setScreenChangedAt(new Date().toLocaleTimeString());
    });

    const unlistenFleet = listen<number | null>(EVENTS.FLEET_VIEW_CHANGED, (event) => {
      if (pausedRef.current) return;
      setCurrentFleet(event.payload);
    });

    const unlistenQuestFilters = listen<QuestFilters>(
      EVENTS.QUEST_FILTERS_CHANGED,
      (event) => {
        if (pausedRef.current) return;
        setQuestFilters(event.payload);
      }
    );

    const unlistenClick = listen<ClickEntry>(EVENTS.CLICK_EVENT, (event) => {
      if (pausedRef.current) return;
      setClicks((prev) => [event.payload, ...prev].slice(0, MAX_ENTRIES));
    });

    const unlistenScreenshot = listen<ClickScreenshotPayload>(
      EVENTS.CLICK_SCREENSHOT,
      (event) => {
        if (pausedRef.current) return;
        const { id, ts, image, imagePath, sampleKind } = event.payload;
        setClicks((prev) =>
          prev.map((c) =>
            c.id === id || (!c.id && c.ts === ts)
              ? { ...c, image, imagePath, sampleKind }
              : c
          )
        );
        setSamples((prev) => ({
          ...prev,
          [sampleKind]: prev[sampleKind] + 1,
        }));
      }
    );

    const unlistenApi = listen<{ endpoint: string }>(EVENTS.KANCOLLE_API, (event) => {
      if (pausedRef.current) return;
      const ts = new Date().toLocaleTimeString();
      setApis((prev) => [{ ts, endpoint: event.payload.endpoint }, ...prev].slice(0, MAX_ENTRIES));
    });

    return () => {
      unlistenScreen.then((f) => f());
      unlistenFleet.then((f) => f());
      unlistenQuestFilters.then((f) => f());
      unlistenClick.then((f) => f());
      unlistenScreenshot.then((f) => f());
      unlistenApi.then((f) => f());
    };
  }, []);

  return (
    <div className="debug-tab">
      <div className="debug-header">
        <div className="debug-screen-card">
          <div className="debug-label">現在認識中の画面</div>
          <div className="debug-screen-value">
            {currentScreen === "QuestList"
              ? withQuestFilters(currentScreen, questFilters.period, questFilters.category)
              : withFleet(currentScreen, currentFleet)}
          </div>
          <div className="debug-screen-meta">最終更新: {screenChangedAt}</div>
          <div className="debug-screen-meta">
            全画面サンプル: 既知 {samples.known}件 / 未知 {samples.unknown}件
            {samples.directory && <span title={samples.directory}>（保存先を確認）</span>}
          </div>
        </div>
        <div className="debug-controls">
          <button
            className={`debug-btn ${paused ? "paused" : ""}`}
            onClick={() => setPaused((p) => !p)}
          >
            {paused ? "▶ 再開" : "⏸ 一時停止"}
          </button>
          <button
            className="debug-btn"
            onClick={() => {
              setClicks([]);
              setApis([]);
            }}
          >
            🗑 クリア
          </button>
        </div>
      </div>

      <div className="debug-columns">
        <div className="debug-panel">
          <div className="debug-panel-title">直近クリック ({clicks.length}件)</div>
          <table className="debug-table">
            <thead>
              <tr>
                <th>時刻</th>
                <th>座標</th>
                <th>画面</th>
                <th>種類</th>
                <th>検知</th>
                <th>ゲーム全画面</th>
              </tr>
            </thead>
            <tbody>
              {clicks.length === 0 ? (
                <tr>
                  <td colSpan={6} className="debug-empty">
                    ゲーム画面をクリックすると表示されます
                  </td>
                </tr>
              ) : (
                clicks.map((c, i) => (
                  <tr key={c.id || i} className={c.screen === "???" ? "debug-unknown-row" : ""}>
                    <td className="debug-ts">{c.ts}</td>
                    <td className="debug-coord">
                      ({c.x}, {c.y})
                    </td>
                    <td className="debug-screen-cell">{withJa(c.screen)}</td>
                    <td className="debug-click-kind">
                      {c.screenChangeClick ? (
                        <span className="debug-transition">
                          画面切替 → {c.targetScreen || withJa(c.screenAfter)}
                        </span>
                      ) : (
                        <span className="debug-operation">画面内操作</span>
                      )}
                    </td>
                    <td className="debug-event">{formatEvent(c.event)}</td>
                    <td className="debug-shot">
                      {c.image ? (
                        <img
                          src={c.image}
                          alt={`full game screen @${c.x},${c.y}`}
                          className="debug-shot-thumb"
                          title={c.imagePath || "1200×720 全ゲーム画面"}
                        />
                      ) : (
                        <span className="debug-shot-pending">…</span>
                      )}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        <div className="debug-panel">
          <div className="debug-panel-title">直近 API ({apis.length}件)</div>
          <table className="debug-table">
            <thead>
              <tr>
                <th>時刻</th>
                <th>endpoint</th>
              </tr>
            </thead>
            <tbody>
              {apis.length === 0 ? (
                <tr>
                  <td colSpan={2} className="debug-empty">
                    API 通信を待機中...
                  </td>
                </tr>
              ) : (
                apis.map((a, i) => (
                  <tr key={i}>
                    <td className="debug-ts">{a.ts}</td>
                    <td className="debug-endpoint">{a.endpoint}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
