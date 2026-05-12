import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./DebugTab.css";

interface ClickEntry {
  ts: string;
  x: number;
  y: number;
  screen: string;
  event: unknown;
  image?: string;
}

interface ClickScreenshotPayload {
  ts: string;
  x: number;
  y: number;
  image: string;
}

interface ApiEntry {
  ts: string;
  endpoint: string;
}

const MAX_ENTRIES = 30;

const SCREEN_JA: Record<string, string> = {
  Homeport: "母港",
  SortieSelect: "出撃選択",
  ExpeditionSelect: "遠征選択",
  ExpeditionFleetSelect: "遠征-艦隊選択",
  FleetComposition: "編成",
  ShipSelection: "編成-艦船選択",
  ShipChangeConfirm: "編成-変更確認",
  Remodel: "改装",
  Resupply: "補給",
  RepairDockSelect: "入渠-ドック選択",
  RepairShipSelect: "入渠-艦船選択",
  Factory: "工廠",
  FactoryDevelop: "工廠-開発",
  QuestList: "任務",
  GetScreen: "GET画面",
  Unknown: "不明",
};

function withJa(screenName: string): string {
  const ja = SCREEN_JA[screenName];
  return ja ? `${screenName} (${ja})` : screenName;
}

const FLEET_SCREENS = new Set([
  "FleetComposition",
  "Resupply",
  "Remodel",
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
  const pausedRef = useRef(paused);
  pausedRef.current = paused;

  useEffect(() => {
    invoke<string>("get_current_screen").then(setCurrentScreen).catch(() => {});
    invoke<number | null>("get_current_fleet").then(setCurrentFleet).catch(() => {});
    invoke<QuestFilters>("get_quest_filters").then(setQuestFilters).catch(() => {});

    const unlistenScreen = listen<string>("screen-changed", (event) => {
      if (pausedRef.current) return;
      setCurrentScreen(event.payload);
      setScreenChangedAt(new Date().toLocaleTimeString());
    });

    const unlistenFleet = listen<number | null>("fleet-view-changed", (event) => {
      if (pausedRef.current) return;
      setCurrentFleet(event.payload);
    });

    const unlistenQuestFilters = listen<QuestFilters>(
      "quest-filters-changed",
      (event) => {
        if (pausedRef.current) return;
        setQuestFilters(event.payload);
      }
    );

    const unlistenClick = listen<ClickEntry>("click-event", (event) => {
      if (pausedRef.current) return;
      setClicks((prev) => [event.payload, ...prev].slice(0, MAX_ENTRIES));
    });

    const unlistenScreenshot = listen<ClickScreenshotPayload>(
      "click-screenshot",
      (event) => {
        if (pausedRef.current) return;
        const { ts, image } = event.payload;
        setClicks((prev) =>
          prev.map((c) => (c.ts === ts ? { ...c, image } : c))
        );
      }
    );

    const unlistenApi = listen<{ endpoint: string }>("kancolle-api", (event) => {
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
                <th>検知</th>
                <th>クリック地点</th>
              </tr>
            </thead>
            <tbody>
              {clicks.length === 0 ? (
                <tr>
                  <td colSpan={5} className="debug-empty">
                    ゲーム画面をクリックすると表示されます
                  </td>
                </tr>
              ) : (
                clicks.map((c, i) => (
                  <tr key={i}>
                    <td className="debug-ts">{c.ts}</td>
                    <td className="debug-coord">
                      ({c.x}, {c.y})
                    </td>
                    <td className="debug-screen-cell">{withJa(c.screen)}</td>
                    <td className="debug-event">{formatEvent(c.event)}</td>
                    <td className="debug-shot">
                      {c.image ? (
                        <img
                          src={c.image}
                          alt={`crop @${c.x},${c.y}`}
                          className="debug-shot-thumb"
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
