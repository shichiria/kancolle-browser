import { useEffect, useRef, useState, useCallback, useMemo } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./App.css";
import edgesData from "./data/edges.json";

/** Lookup node label from KC3Kai edges data. Returns destination node label for given edge ID. */
function getNodeLabel(mapDisplay: string, edgeId: number): string | null {
  const key = `World ${mapDisplay}`;
  const edges = edgesData as Record<string, Record<string, string[]>>;
  const mapEdges = edges[key];
  if (!mapEdges) return null;
  const edge = mapEdges[String(edgeId)];
  if (!edge || edge.length < 2) return null;
  return edge[1]; // [source, destination] - we want destination
}

interface SpecialEquip {
  name: string;
  icon_type: number;
}

interface ShipData {
  name: string;
  lv: number;
  hp: number;
  maxhp: number;
  cond: number;
  fuel: number;
  bull: number;
  damecon_name?: string | null;
  special_equips: SpecialEquip[];
  can_opening_asw?: boolean;
  soku: number;
}

interface FleetExpedition {
  mission_name: string;
  return_time: number; // unix timestamp in ms, 0 = not on expedition
}

interface FleetData {
  id: number;
  name: string;
  expedition?: FleetExpedition | null;
  ships: ShipData[];
  // Legacy fields
  ship_ids?: number[];
  mission?: unknown[];
}

interface NdockData {
  id: number;
  state: number;
  ship_name?: string;
  ship_id?: number;
  complete_time: number;
}

interface PortData {
  admiral_name: string;
  admiral_level: number;
  admiral_rank?: number;
  ship_count: number;
  ship_capacity?: number;
  // Resources
  fuel: number;
  ammo: number;
  steel: number;
  bauxite: number;
  instant_repair?: number;
  instant_build?: number;
  dev_material?: number;
  improvement_material?: number;
  // Fleets
  fleets: FleetData[];
  // Repair docks
  ndock: NdockData[];
}

interface SenkaSummary {
  total: number;
  exp_senka: number;
  eo_bonus: number;
  quest_bonus: number;
  monthly_exp_gain: number;
  tracking_active: boolean;
  next_checkpoint: string;
  checkpoint_crossed: boolean;
  eo_cutoff_active: boolean;
  quest_cutoff_active: boolean;
  confirmed_senka: number | null;
  confirmed_cutoff: string | null;
  is_confirmed_base: boolean;
}

interface ApiLogEntry {
  time: string;
  endpoint: string;
}

interface ExpeditionDef {
  id: number;
  display_id: string;
  name: string;
  great_success_type: "Regular" | "Drum" | "Level";
  duration_minutes: number;
}

interface SortieQuestDef {
  id: number;
  quest_id: string;
  name: string;
  area: string;
  rank: string;
  boss_only: boolean;
  count: number;
  reset: string;
  no_conditions: boolean;
  sub_goals?: { name: string; count: number; boss_only: boolean; rank: string }[];
}

interface ActiveQuestDetail {
  id: number;
  title: string;
  category: number;
}

interface MapRecommendedResult {
  area: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}

interface MapRecommendationDef {
  area: string;
  name: string;
}

interface MapRouteCheckResult {
  desc: string;
  satisfied: boolean;
  conditions: ConditionResult[];
}

interface MapRecommendationCheckResult {
  area: string;
  name: string;
  routes: MapRouteCheckResult[];
}

interface SortieQuestCheckResult {
  quest_id: string;
  quest_name: string;
  area: string;
  rank: string;
  boss_only: boolean;
  count: number;
  no_conditions: boolean;
  note: string | null;
  satisfied: boolean;
  conditions: ConditionResult[];
  recommended: MapRecommendedResult[];
}

interface QuestProgressSummary {
  quest_id: number;
  quest_id_str: string;
  area_progress: { area: string; cleared: boolean; count: number; count_max: number }[];
  count: number;
  count_max: number;
  completed: boolean;
}

interface ConditionResult {
  condition: string;
  satisfied: boolean;
  current_value: string;
  required_value: string;
}

interface ExpeditionCheckResult {
  expedition_id: number;
  expedition_name: string;
  display_id: string;
  result: "Failure" | "Success" | "GreatSuccess";
  conditions: ConditionResult[];
}

// ── Battle Log types ──

interface HpState {
  before: number;
  after: number;
  max: number;
}

interface SlotItemSnapshot {
  id: number;
  rf?: number;
  mas?: number;
}

interface EnemyShip {
  ship_id: number;
  level: number;
  name?: string;
  slots?: number[];
}

interface AirBattleResult {
  air_superiority?: number;
  friendly_plane_count?: [number, number];
  enemy_plane_count?: [number, number];
}

interface BattleDetail {
  rank: string;
  enemy_name: string;
  enemy_ships: EnemyShip[];
  formation: [number, number, number];
  air_battle?: AirBattleResult;
  friendly_hp: HpState[];
  enemy_hp: HpState[];
  drop_ship?: string;
  drop_ship_id?: number;
  mvp?: number;
  base_exp?: number;
  ship_exp: number[];
  night_battle: boolean;
}

interface BattleNode {
  cell_no: number;
  event_kind: number;
  event_id?: number;
  battle?: BattleDetail;
  // Legacy fields (from old saved records, migrated on load)
  rank?: string;
  enemy_name?: string;
  drop_ship?: string;
  drop_ship_id?: number;
  mvp?: number;
  base_exp?: number;
}

interface SortieShip {
  name: string;
  ship_id: number;
  lv: number;
  stype: number;
  slots?: SlotItemSnapshot[];
  slot_ex?: SlotItemSnapshot;
}

interface SortieRecord {
  id: string;
  fleet_id: number;
  map_display: string;
  ships: SortieShip[];
  nodes: BattleNode[];
  start_time: string;
  end_time?: string;
}

interface BattleLogsResponse {
  records: SortieRecord[];
  total: number;
}

type TabId = "homeport" | "battle" | "improvement" | "ships" | "equips" | "options";

const RANK_NAMES: Record<number, string> = {
  1: "元帥",
  2: "大将",
  3: "中将",
  4: "少将",
  5: "大佐",
  6: "中佐",
  7: "新米中佐",
  8: "少佐",
  9: "中堅少佐",
  10: "新米少佐",
};

function getRankName(rank?: number): string {
  if (rank == null) return "";
  return RANK_NAMES[rank] ?? `Rank ${rank}`;
}

/** Format remaining milliseconds as HH:MM:SS or MM:SS */
function formatRemaining(targetMs: number, now: number): string {
  const diff = targetMs - now;
  if (diff <= 0) return "完了";
  const totalSec = Math.floor(diff / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

/** HP bar color based on remaining percentage */
function hpColor(hp: number, maxhp: number): string {
  if (maxhp <= 0) return "#4caf50";
  const ratio = hp / maxhp;
  if (ratio > 0.75) return "#4caf50";
  if (ratio > 0.5) return "#ffeb3b";
  if (ratio > 0.25) return "#ff9800";
  return "#f44336";
}

/** Condition text color */
function condColor(cond: number): string {
  if (cond >= 50) return "#ffb74d"; // orange sparkle
  if (cond >= 40) return "#e0e0e0"; // normal
  if (cond >= 30) return "#ffeb3b"; // yellow warning
  return "#f44336"; // red fatigue
}

function condBgClass(cond: number): string {
  if (cond >= 50) return "cond-sparkle";
  if (cond >= 40) return "";
  if (cond >= 30) return "cond-tired";
  return "cond-red";
}

function HpBar({ hp, maxhp }: { hp: number; maxhp: number }) {
  const pct = maxhp > 0 ? (hp / maxhp) * 100 : 100;
  return (
    <div className="hp-bar-container">
      <div
        className="hp-bar-fill"
        style={{ width: `${pct}%`, backgroundColor: hpColor(hp, maxhp) }}
      />
      <span className="hp-bar-text">
        {hp}/{maxhp}
      </span>
    </div>
  );
}

// @ts-ignore: SupplyBar will be used when master data is available
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function SupplyBar({ rate, type }: { rate: number; type: "fuel" | "ammo" }) {
  const color = type === "fuel" ? "#4caf50" : "#795548";
  const dimColor = rate < 100 ? 0.5 : 1;
  return (
    <div className="supply-bar-container">
      <div
        className="supply-bar-fill"
        style={{
          width: `${rate}%`,
          backgroundColor: color,
          opacity: dimColor,
        }}
      />
    </div>
  );
}

function ExpeditionChecker({
  fleetIndex,
  expeditions,
  portDataVersion,
  currentExpedition,
  now,
}: {
  fleetIndex: number;
  expeditions: ExpeditionDef[];
  portDataVersion: number;
  currentExpedition?: FleetExpedition | null;
  now: number;
}) {
  const storageKey = `expedition-fleet-${fleetIndex}`;
  const [selectedId, setSelectedId] = useState<number | null>(() => {
    const saved = localStorage.getItem(storageKey);
    return saved ? Number(saved) : null;
  });
  const [checkResult, setCheckResult] = useState<ExpeditionCheckResult | null>(null);
  const [checking, setChecking] = useState(false);

  // Auto-check on mount and when port data updates
  useEffect(() => {
    if (selectedId != null && expeditions.length > 0) {
      doCheck(selectedId);
    }
  }, [expeditions.length, portDataVersion]); // eslint-disable-line react-hooks/exhaustive-deps

  const doCheck = async (expId: number) => {
    setSelectedId(expId);
    localStorage.setItem(storageKey, String(expId));
    setChecking(true);
    try {
      const result = await invoke<ExpeditionCheckResult>("check_expedition_cmd", {
        fleetIndex,
        expeditionId: expId,
      });
      setCheckResult(result);
    } catch (e) {
      console.error("Expedition check failed:", e);
      setCheckResult(null);
    } finally {
      setChecking(false);
    }
  };

  const resultLabel = (r: string) => {
    if (r === "GreatSuccess") return { text: "大成功", cls: "result-great" };
    if (r === "Success") return { text: "成功", cls: "result-success" };
    return { text: "失敗", cls: "result-failure" };
  };

  return (
    <div className="expedition-checker">
      <select
        className="expedition-select"
        value={selectedId ?? ""}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (v > 0) doCheck(v);
        }}
      >
        <option value="">遠征を選択...</option>
        {expeditions.map((exp) => (
          <option key={exp.id} value={exp.id}>
            {exp.display_id} {exp.name} ({formatDuration(exp.duration_minutes)})
          </option>
        ))}
      </select>
      {currentExpedition && currentExpedition.return_time > 0 && (
        <div className="expedition-timer">
          <span className="expedition-timer-remaining">
            残り {formatRemaining(currentExpedition.return_time, now)}
          </span>
          <span className="expedition-timer-eta">
            帰還 {new Date(currentExpedition.return_time).toLocaleTimeString("ja-JP", { hour: "2-digit", minute: "2-digit", second: "2-digit" })}
          </span>
        </div>
      )}
      {checking && <span className="checking">確認中...</span>}
      {checkResult && !checking && (
        <div className="expedition-result">
          <span className={`expedition-result-label ${resultLabel(checkResult.result).cls}`}>
            {resultLabel(checkResult.result).text}
          </span>
          <div className="expedition-conditions">
            {checkResult.conditions.map((c, i) => (
              <div key={i} className={`exp-cond ${c.satisfied ? "cond-ok" : "cond-ng"}`}>
                <span className="exp-cond-name">{c.condition}</span>
                <span className="exp-cond-value">{c.current_value}</span>
                <span className="exp-cond-sep">/</span>
                <span className="exp-cond-req">{c.required_value}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function MapRecommendationChecker({
  mapRecommendations,
  portDataVersion,
}: {
  mapRecommendations: MapRecommendationDef[];
  portDataVersion: number;
}) {
  const storageKey = "map-rec-area";
  const [selectedArea, setSelectedArea] = useState<string | null>(() => {
    return localStorage.getItem(storageKey);
  });
  const [checkResult, setCheckResult] = useState<MapRecommendationCheckResult | null>(null);
  const [checking, setChecking] = useState(false);

  // Group maps by world number
  const grouped = useMemo(() => {
    const map = new Map<number, MapRecommendationDef[]>();
    for (const m of mapRecommendations) {
      const world = parseInt(m.area.split("-")[0], 10);
      if (!map.has(world)) map.set(world, []);
      map.get(world)!.push(m);
    }
    return map;
  }, [mapRecommendations]);

  const doCheck = useCallback(async (area: string) => {
    setSelectedArea(area);
    localStorage.setItem(storageKey, area);
    setChecking(true);
    try {
      const result = await invoke<MapRecommendationCheckResult>("check_map_recommendation_cmd", {
        fleetIndex: 0,
        area,
      });
      setCheckResult(result);
    } catch (e) {
      console.error("Map recommendation check failed:", e);
      setCheckResult(null);
    } finally {
      setChecking(false);
    }
  }, []);

  // Auto-check on mount and when port data updates
  useEffect(() => {
    if (selectedArea != null && mapRecommendations.length > 0) {
      doCheck(selectedArea);
    }
  }, [mapRecommendations.length, portDataVersion]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="map-rec-checker">
      <select
        className="map-rec-select"
        value={selectedArea ?? ""}
        onChange={(e) => {
          const v = e.target.value;
          if (v) {
            doCheck(v);
          } else {
            setSelectedArea(null);
            setCheckResult(null);
            localStorage.removeItem(storageKey);
          }
        }}
      >
        <option value="">海域を選択...</option>
        {Array.from(grouped.entries())
          .sort(([a], [b]) => a - b)
          .map(([world, maps]) => (
            <optgroup key={world} label={`第${world}海域`}>
              {maps.map((m) => (
                <option key={m.area} value={m.area}>
                  {m.area} {m.name}
                </option>
              ))}
            </optgroup>
          ))}
      </select>
      {checking && <span className="checking">確認中...</span>}
      {checkResult && !checking && (
        <div className="map-rec-result">
          {checkResult.routes.map((route, ri) => (
            <div key={ri} className="map-rec-route">
              <div className="map-rec-route-header">
                <span className="map-rec-route-desc">{route.desc}</span>
                <span className={`map-rec-route-status ${route.satisfied ? "rec-ok" : "rec-ng"}`}>
                  {route.satisfied ? "OK" : "NG"}
                </span>
              </div>
              {route.conditions.map((c, ci) => (
                <div key={ci} className={`exp-cond ${c.satisfied ? "cond-ok" : "cond-ng"}`}>
                  <span className="exp-cond-name">{c.condition}</span>
                  <span className="exp-cond-value">{c.current_value}</span>
                  <span className="exp-cond-sep">/</span>
                  <span className="exp-cond-req">{c.required_value}</span>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const CATEGORY_LABELS: Record<number, string> = {
  1: "編成",
  2: "出撃",
  3: "演習",
  8: "出撃",
  9: "出撃",
  10: "出撃",
};

// Categories shown in the quest dropdown (fleet-related)
const QUEST_CATEGORIES = new Set([1, 2, 3, 8, 9, 10]);

interface DropdownQuest {
  key: string;       // quest_id (from JSON) or api_no as string
  label: string;     // display name
  category: number;
  hasData: boolean;   // true if conditions data exists in sortie_quests.json
}

function SortieQuestChecker({
  fleetIndex,
  sortieQuests,
  portDataVersion,
  activeQuests,
  questProgress,
}: {
  fleetIndex: number;
  sortieQuests: SortieQuestDef[];
  portDataVersion: number;
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
}) {
  const storageKey = `sortie-quest-fleet-${fleetIndex}`;
  const [selectedId, setSelectedId] = useState<string | null>(() => {
    return localStorage.getItem(storageKey);
  });
  const [checkResult, setCheckResult] = useState<SortieQuestCheckResult | null>(null);
  const [checking, setChecking] = useState(false);

  // Build quest lookup from sortie_quests.json by api_no (id field)
  const questById = useMemo(() => {
    const map = new Map<number, SortieQuestDef>();
    for (const q of sortieQuests) map.set(q.id, q);
    return map;
  }, [sortieQuests]);

  // Build dropdown items from active quests (API) merged with JSON data
  const dropdownQuests = useMemo(() => {
    const items: DropdownQuest[] = [];
    for (const aq of activeQuests) {
      if (!QUEST_CATEGORIES.has(aq.category)) continue;
      const def = questById.get(aq.id);
      items.push({
        key: def ? def.quest_id : `api_${aq.id}`,
        label: def ? `${def.quest_id} ${def.name}` : aq.title,
        category: aq.category,
        hasData: !!def,
      });
    }
    return items;
  }, [activeQuests, questById]);

  // Track quest started via dedicated backend event (not fragile diff detection)
  const pendingStartedRef = useRef<number | null>(null);
  const doCheckVersionRef = useRef(0);

  useEffect(() => {
    const unlisten = listen<number>("quest-started", (event) => {
      pendingStartedRef.current = event.payload;
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  // Clear selection when any quest is stopped in game
  useEffect(() => {
    const unlisten = listen<number>("quest-stopped", () => {
      setSelectedId(null);
      setCheckResult(null);
      localStorage.removeItem(storageKey);
    });
    return () => { unlisten.then((f) => f()); };
  }, [storageKey]);

  // Auto-select just-started quest, or clear selection when quest removed
  useEffect(() => {
    // Check if a quest was just started and is now in the dropdown
    if (pendingStartedRef.current != null) {
      const startedApiNo = pendingStartedRef.current;
      const match = dropdownQuests.find((q) => {
        // Match by api_no: check JSON def id or api_ prefix key
        const def = questById.get(startedApiNo);
        if (def && q.key === def.quest_id) return true;
        if (q.key === `api_${startedApiNo}`) return true;
        return false;
      });
      if (match && (match.category >= 2 && match.category <= 3 || match.category >= 8)) {
        pendingStartedRef.current = null;
        doCheck(match.key);
        return;
      }
      // If not found yet, keep pending for next update
    }

    // Clear selection if selected quest no longer active
    if (selectedId != null) {
      const stillActive = dropdownQuests.some((q) => q.key === selectedId);
      if (!stillActive) {
        setSelectedId(null);
        setCheckResult(null);
        localStorage.removeItem(storageKey);
      }
    }
  }, [activeQuests, dropdownQuests]); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-check on mount and when port data / fleet composition updates
  useEffect(() => {
    if (selectedId != null && dropdownQuests.length > 0) {
      doCheck(selectedId);
    }
  }, [sortieQuests.length, portDataVersion]); // eslint-disable-line react-hooks/exhaustive-deps

  const doCheck = async (questId: string) => {
    const version = ++doCheckVersionRef.current;
    setSelectedId(questId);
    localStorage.setItem(storageKey, questId);
    // For quests without JSON data (exercise, composition, etc.), show basic info
    if (questId.startsWith("api_")) {
      const dq = dropdownQuests.find((q) => q.key === questId);
      if (doCheckVersionRef.current === version) {
        setCheckResult({
          quest_id: questId,
          quest_name: dq?.label ?? "",
          area: "",
          rank: "",
          boss_only: false,
          count: 0,
          satisfied: false,
          no_conditions: false,
          note: null,
          conditions: [],
          recommended: [],
        });
      }
      return;
    }
    setChecking(true);
    try {
      const result = await invoke<SortieQuestCheckResult>("check_sortie_quest_cmd", {
        fleetIndex,
        questId,
      });
      if (doCheckVersionRef.current === version) {
        setCheckResult(result);
      }
    } catch (e) {
      console.error("Sortie quest check failed:", e);
      if (doCheckVersionRef.current === version) {
        setCheckResult(null);
      }
    } finally {
      if (doCheckVersionRef.current === version) {
        setChecking(false);
      }
    }
  };

  // Group dropdown quests by category
  const grouped = dropdownQuests.reduce<Record<number, DropdownQuest[]>>((acc, q) => {
    // Merge subcategories 8/9/10 into 2 (sortie)
    const cat = q.category >= 8 ? 2 : q.category;
    if (!acc[cat]) acc[cat] = [];
    acc[cat].push(q);
    return acc;
  }, {});

  const categoryOrder = [2, 3, 1]; // 出撃, 演習, 編成

  // Resolve current quest's API ID and area progress for inline display
  const currentApiNo = useMemo(() => {
    if (!selectedId) return null;
    for (const [id, def] of questById) {
      if (def.quest_id === selectedId) return id;
    }
    if (selectedId.startsWith("api_")) {
      const n = parseInt(selectedId.slice(4), 10);
      return isNaN(n) ? null : n;
    }
    return null;
  }, [selectedId, questById]);

  const currentProgress = currentApiNo != null ? questProgress.get(currentApiNo) : undefined;
  const areaProgressMap = useMemo(() => {
    const map = new Map<string, { count: number; count_max: number; cleared: boolean }>();
    if (currentProgress) {
      for (const ap of currentProgress.area_progress) map.set(ap.area, ap);
    }
    return map;
  }, [currentProgress]);

  const setAreaCount = async (area: string, value: number) => {
    if (currentApiNo == null) return;
    await invoke("update_quest_progress", { questId: currentApiNo, area, count: value });
  };

  // Which areas are covered by recommended section
  const recommendedAreas = useMemo(() => {
    if (!checkResult) return new Set<string>();
    return new Set(checkResult.recommended.map((r) => r.area));
  }, [checkResult]);

  return (
    <div className="sortie-quest-checker">
      <select
        className="sortie-quest-select"
        value={selectedId ?? ""}
        onChange={(e) => {
          const v = e.target.value;
          if (v) {
            doCheck(v);
          } else {
            setSelectedId(null);
            setCheckResult(null);
            localStorage.removeItem(storageKey);
          }
        }}
      >
        <option value="">{dropdownQuests.length > 0 ? "任務を選択..." : "任務画面を開いて下さい"}</option>
        {categoryOrder.map((cat) => {
          const quests = grouped[cat];
          if (!quests || quests.length === 0) return null;
          return (
            <optgroup key={cat} label={CATEGORY_LABELS[cat] ?? `cat${cat}`}>
              {quests.map((q) => (
                <option key={q.key} value={q.key}>
                  {q.label}
                </option>
              ))}
            </optgroup>
          );
        })}
      </select>
      {checking && <span className="checking">確認中...</span>}
      {checkResult && !checking && (
        <div className="sortie-quest-result">
          <div className="sortie-quest-info">
            {checkResult.area ? (
              <>
                <span className="sortie-quest-area">{checkResult.area}</span>
                <span className="sortie-quest-rank">
                  {checkResult.boss_only ? "ボス" : ""}{checkResult.rank}
                  {checkResult.count > 1 && ` x${checkResult.count}`}
                </span>
                <span className={`sortie-quest-status ${checkResult.conditions.length === 0 ? (checkResult.no_conditions ? "quest-ok" : "quest-unknown") : checkResult.satisfied ? "quest-ok" : "quest-ng"}`}>
                  {checkResult.conditions.length === 0 ? (checkResult.no_conditions ? "条件なし" : "条件不明") : checkResult.satisfied ? "OK" : "NG"}
                </span>
              </>
            ) : (
              <span className="sortie-quest-status quest-unknown">データなし</span>
            )}
          </div>
          {checkResult.note && (
            <div className="sortie-quest-note">{checkResult.note}</div>
          )}
          {checkResult.conditions.length > 0 && (
            <div className="sortie-quest-conditions">
              {checkResult.conditions.map((c, i) => (
                <div key={i} className={`exp-cond ${c.satisfied ? "cond-ok" : "cond-ng"}`}>
                  <span className="exp-cond-name">{c.condition}</span>
                  <span className="exp-cond-value">{c.current_value}</span>
                  <span className="exp-cond-sep">/</span>
                  <span className="exp-cond-req">{c.required_value}</span>
                </div>
              ))}
            </div>
          )}
          {(() => {
            // Merge recommended areas and progress-only areas into one list
            const recAreas = checkResult.recommended;
            // Exclude sub_goals entries from uncovered areas (shown in QuestProgressDisplay)
            const curDef = currentApiNo != null ? questById.get(currentApiNo) : undefined;
            const isSubGoals = curDef?.sub_goals && curDef.sub_goals.length > 0;
            const uncoveredAreas = currentProgress && !isSubGoals
              ? currentProgress.area_progress.filter((ap) => !recommendedAreas.has(ap.area))
              : [];
            if (recAreas.length === 0 && uncoveredAreas.length === 0) return null;
            return (
              <div className="sortie-quest-recommended">
                {recAreas.map((rec, ri) => {
                  const ap = areaProgressMap.get(rec.area);
                  const areaCleared = ap?.cleared ?? false;
                  return (
                    <div key={ri} className="sortie-rec-map">
                      <div className="sortie-rec-map-header">
                        <span className="sortie-rec-map-area">{rec.area}</span>
                        {areaCleared ? (
                          <span className="sortie-rec-map-status rec-done">達成済</span>
                        ) : (
                          <span className={`sortie-rec-map-status ${rec.satisfied ? "rec-ok" : "rec-ng"}`}>
                            {rec.satisfied ? "OK" : "NG"}
                          </span>
                        )}
                        {ap && (
                          <span className={`qp-inline-progress ${ap.cleared ? "cleared" : ""}`}>
                            <select
                              className="qp-area-select"
                              value={ap.count}
                              onClick={(e) => e.stopPropagation()}
                              onChange={(e) => setAreaCount(rec.area, parseInt(e.target.value, 10))}
                            >
                              {Array.from({ length: ap.count_max + 1 }, (_, i) => (
                                <option key={i} value={i}>{i}</option>
                              ))}
                            </select>
                            /{ap.count_max}
                          </span>
                        )}
                      </div>
                      {!areaCleared && rec.conditions.map((c, ci) => (
                        <div key={ci} className={`exp-cond ${c.satisfied ? "cond-ok" : "cond-ng"}`}>
                          <span className="exp-cond-name">{c.condition}</span>
                          <span className="exp-cond-value">{c.current_value}</span>
                          <span className="exp-cond-sep">/</span>
                          <span className="exp-cond-req">{c.required_value}</span>
                        </div>
                      ))}
                    </div>
                  );
                })}
                {uncoveredAreas.map((ap) => (
                  <div key={ap.area} className="sortie-rec-map">
                    <div className="sortie-rec-map-header">
                      <span className="sortie-rec-map-area">{ap.area}</span>
                      {ap.cleared ? (
                        <span className="sortie-rec-map-status rec-done">達成済</span>
                      ) : (
                        <span className="sortie-rec-map-status rec-ng">未達</span>
                      )}
                      <span className={`qp-inline-progress ${ap.cleared ? "cleared" : ""}`}>
                        <select
                          className="qp-area-select"
                          value={ap.count}
                          onClick={(e) => e.stopPropagation()}
                          onChange={(e) => setAreaCount(ap.area, parseInt(e.target.value, 10))}
                        >
                          {Array.from({ length: ap.count_max + 1 }, (_, i) => (
                            <option key={i} value={i}>{i}</option>
                          ))}
                        </select>
                        /{ap.count_max}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            );
          })()}
          <QuestProgressDisplay
            questId={selectedId}
            questById={questById}
            questProgress={questProgress}
            skipAreas={recommendedAreas}
          />
        </div>
      )}
    </div>
  );
}

function QuestProgressDisplay({
  questId,
  questById,
  questProgress,
  skipAreas,
}: {
  questId: string | null;
  questById: Map<number, SortieQuestDef>;
  questProgress: Map<number, QuestProgressSummary>;
  skipAreas?: Set<string>;
}) {
  const apiNo = useMemo(() => {
    if (!questId) return null;
    for (const [id, def] of questById) {
      if (def.quest_id === questId) return id;
    }
    if (questId.startsWith("api_")) {
      const n = parseInt(questId.slice(4), 10);
      return isNaN(n) ? null : n;
    }
    return null;
  }, [questId, questById]);

  const questDef = useMemo(() => {
    if (apiNo == null) return null;
    return questById.get(apiNo) ?? null;
  }, [apiNo, questById]);

  const progress = apiNo != null ? questProgress.get(apiNo) : undefined;
  if (!progress) return null;

  const hasSubGoals = questDef?.sub_goals && questDef.sub_goals.length > 0;

  // Sub-goals quests: show each sub-goal as a simple progress row
  if (hasSubGoals) {
    const setSubGoalCount = async (name: string, value: number) => {
      if (apiNo == null) return;
      await invoke("update_quest_progress", { questId: apiNo, area: name, count: value });
    };

    return (
      <div className="quest-progress quest-progress-sub-goals">
        <span className="quest-progress-label">進捗</span>
        {progress.completed && <span className="quest-progress-badge">達成</span>}
        {progress.area_progress.map((ap) => (
          <div key={ap.area} className="sub-goal-row">
            <span className={`sub-goal-name ${ap.cleared ? "sub-goal-cleared" : ""}`}>{ap.area}</span>
            <select
              className="qp-count-select"
              value={ap.count}
              onChange={(e) => setSubGoalCount(ap.area, parseInt(e.target.value, 10))}
            >
              {Array.from({ length: ap.count_max + 1 }, (_, i) => (
                <option key={i} value={i}>{i}</option>
              ))}
            </select>
            <span className="qp-count-max">/{ap.count_max}</span>
          </div>
        ))}
      </div>
    );
  }

  // Area quests with all areas covered by recommended → nothing to show here
  const hasUncoveredAreas = progress.area_progress.some(
    (ap) => !skipAreas || !skipAreas.has(ap.area)
  );

  // Counter quests (任意/演習) only
  if (progress.area_progress.length > 0 && !hasUncoveredAreas) return null;
  if (progress.area_progress.length > 0) return null; // All area quests handled inline

  const setCount = async (value: number) => {
    if (apiNo == null) return;
    await invoke("update_quest_progress", { questId: apiNo, count: value });
  };

  return (
    <div className="quest-progress">
      <span className="quest-progress-label">進捗</span>
      {progress.completed && <span className="quest-progress-badge">達成</span>}
      <select
        className="qp-count-select"
        value={progress.count}
        onChange={(e) => setCount(parseInt(e.target.value, 10))}
      >
        {Array.from({ length: progress.count_max + 1 }, (_, i) => (
          <option key={i} value={i}>{i}</option>
        ))}
      </select>
      <span className="qp-count-max">/{progress.count_max}</span>
    </div>
  );
}

function formatDuration(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h > 0 && m > 0) return `${h}h${m.toString().padStart(2, "0")}m`;
  if (h > 0) return `${h}h`;
  return `${m}m`;
}

function FleetPanel({
  fleet,
  now,
  fleetIndex,
  expeditions,
  portDataVersion,
  sortieQuests,
  mapRecommendations,
  activeQuests,
  questProgress,
  weaponIconSheet,
}: {
  fleet: FleetData;
  now: number;
  fleetIndex: number;
  expeditions: ExpeditionDef[];
  portDataVersion: number;
  sortieQuests: SortieQuestDef[];
  mapRecommendations: MapRecommendationDef[];
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
  weaponIconSheet: string | null;
}) {
  const expedition = fleet.expedition;
  const isOnExpedition =
    expedition != null && expedition.return_time > 0;
  const ships = fleet.ships ?? [];
  // Legacy: count ship_ids if ships array is empty
  const shipCount =
    ships.length > 0
      ? ships.length
      : (fleet.ship_ids?.filter((id) => id > 0).length ?? 0);

  return (
    <div className="fleet-panel">
      <div className="fleet-header">
        <span className="fleet-name">
          <span className="fleet-id">#{fleet.id}</span> {fleet.name}
        </span>
        {ships.length > 0 && (() => {
          const minSoku = Math.min(...ships.map(s => s.soku));
          const tag = minSoku >= 20 ? { label: "最速", cls: "speed-fastest" }
            : minSoku >= 15 ? { label: "高速+", cls: "speed-fast-plus" }
            : minSoku >= 10 ? { label: "高速", cls: "speed-fast" }
            : { label: "低速混合", cls: "speed-slow" };
          return <span className={`fleet-speed-tag ${tag.cls}`}>{tag.label}</span>;
        })()}
        {isOnExpedition && expedition && (
          <span className="fleet-expedition">
            {expedition.mission_name} [{formatRemaining(expedition.return_time, now)}]
          </span>
        )}
      </div>
      {ships.length > 0 ? (
        <div className="fleet-ships">
          {ships.map((ship, i) => (
            <div key={i} className="ship-row">
              <span className="ship-name" title={ship.name}>
                {ship.name}
              </span>
              <span className="ship-lv">Lv{ship.lv}</span>
              <HpBar hp={ship.hp} maxhp={ship.maxhp} />
              <span
                className={`ship-cond ${condBgClass(ship.cond)}`}
                style={{ color: condColor(ship.cond) }}
              >
                {ship.cond}
              </span>
              {ship.damecon_name && (
                <span
                  className={weaponIconSheet ? "damecon-icon" : "mark-noimage"}
                  title={ship.damecon_name}
                  style={weaponIconSheet ? { backgroundImage: `url(${weaponIconSheet})` } : undefined}
                />
              )}
              {ship.special_equips.length > 0 && (
                ship.special_equips.map((eq, j) => (
                  <span
                    key={`seq-${j}`}
                    className={weaponIconSheet ? `special-equip-icon special-equip-icon-${eq.icon_type}` : "mark-noimage mark-noimage-sm"}
                    title={eq.name}
                    style={weaponIconSheet ? { backgroundImage: `url(${weaponIconSheet})` } : undefined}
                  />
                ))
              )}
              {ship.can_opening_asw && (
                <span
                  className={weaponIconSheet ? "opening-asw-icon" : "mark-noimage"}
                  title="先制対潜"
                  style={weaponIconSheet ? { backgroundImage: `url(${weaponIconSheet})` } : undefined}
                />
              )}
            </div>
          ))}
        </div>
      ) : shipCount > 0 ? (
        <div className="fleet-no-detail">{shipCount}隻 (詳細なし)</div>
      ) : null}
      {fleetIndex === 0 && (
        <>
          <MapRecommendationChecker
            mapRecommendations={mapRecommendations}
            portDataVersion={portDataVersion}
          />
          <SortieQuestChecker
            fleetIndex={fleetIndex}
            sortieQuests={sortieQuests}
            portDataVersion={portDataVersion}
            activeQuests={activeQuests}
            questProgress={questProgress}
          />
        </>
      )}
      {fleetIndex > 0 && (
        <ExpeditionChecker
          fleetIndex={fleetIndex}
          expeditions={expeditions}
          portDataVersion={portDataVersion}
          currentExpedition={fleet.expedition}
          now={now}
        />
      )}
    </div>
  );
}

const FORMATION_NAMES: Record<number, string> = {
  1: "単縦陣",
  2: "複縦陣",
  3: "輪形陣",
  4: "梯形陣",
  5: "単横陣",
  6: "警戒陣",
  11: "第一警戒航行序列",
  12: "第二警戒航行序列",
  13: "第三警戒航行序列",
  14: "第四警戒航行序列",
};

const ENGAGEMENT_NAMES: Record<number, string> = {
  1: "同航戦",
  2: "反航戦",
  3: "T字有利",
  4: "T字不利",
};

const RANK_COLORS: Record<string, string> = {
  S: "#ffd700",
  A: "#ff6b6b",
  B: "#ff9800",
  C: "#888",
  D: "#666",
  E: "#555",
};

// EVENT_LABELS keyed by api_color_no (fallback)
const EVENT_LABELS: Record<number, string> = {
  0: "始点",
  2: "資源",
  3: "渦潮",
  4: "戦闘",
  5: "ボス",
  6: "気のせい",
  7: "航空戦",
  8: "空襲",
  9: "揚陸",
  10: "泊地",
};

// EVENT_ID_LABELS keyed by api_event_id (takes priority over color_no)
const EVENT_ID_LABELS: Record<number, string> = {
  6: "航路選択",
};

const AIR_SUPERIORITY_LABELS: Record<number, { text: string; color: string }> = {
  0: { text: "航空劣勢", color: "#f44336" },
  1: { text: "航空優勢", color: "#4caf50" },
  2: { text: "制空権確保", color: "#2196f3" },
  3: { text: "航空均衡", color: "#ff9800" },
  4: { text: "制空権喪失", color: "#d32f2f" },
};

// ── Battle Tab Components ──

/** HP bar used in battle detail view (wider, with before->after display) */
function BattleHpBar({
  before,
  after,
  max,
  shipName,
}: {
  before: number;
  after: number;
  max: number;
  shipName?: string;
}) {
  const afterClamped = Math.max(0, after);
  const pctBefore = max > 0 ? (before / max) * 100 : 100;
  const pctAfter = max > 0 ? (afterClamped / max) * 100 : 100;
  const damage = before - afterClamped;
  const isSunk = afterClamped <= 0;

  return (
    <div className="battle-hp-row">
      {shipName && (
        <span className={`battle-hp-name ${isSunk ? "sunk" : ""}`}>{shipName}</span>
      )}
      <div className="battle-hp-bar-wrap">
        <div className="battle-hp-bar-bg">
          {/* Ghost bar showing pre-battle HP */}
          <div
            className="battle-hp-bar-ghost"
            style={{ width: `${pctBefore}%` }}
          />
          {/* Actual after-battle HP */}
          <div
            className="battle-hp-bar-fill"
            style={{
              width: `${pctAfter}%`,
              backgroundColor: hpColor(afterClamped, max),
            }}
          />
        </div>
        <span className="battle-hp-text">
          {afterClamped}/{max}
          {damage > 0 && <span className="battle-hp-dmg"> (-{damage})</span>}
        </span>
      </div>
    </div>
  );
}

/** Battle detail view for a single sortie record */
/** Build a predeck JSON for kc-web aircalc */
function buildPredeckUrl(record: SortieRecord): string {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const predeck: any = { version: 4, hqlv: 120 };

  // Fleet 1 (the sortie fleet)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const fleet: any = {};
  record.ships.forEach((ship, idx) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const s: any = { id: ship.ship_id, lv: ship.lv, luck: -1, items: {} };
    if (ship.slots) {
      ship.slots.forEach((slot, si) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const item: any = { id: slot.id, rf: slot.rf ?? 0 };
        if (slot.mas != null) item.mas = slot.mas;
        s.items[`i${si + 1}`] = item;
      });
    }
    if (ship.slot_ex) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const exItem: any = { id: ship.slot_ex.id, rf: ship.slot_ex.rf ?? 0 };
      if (ship.slot_ex.mas != null) exItem.mas = ship.slot_ex.mas;
      s.items.ix = exItem;
    }
    fleet[`s${idx + 1}`] = s;
  });
  predeck.f1 = fleet;

  // Sortie data (enemy compositions per cell)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const cells: any[] = [];
  for (const node of record.nodes) {
    const b = node.battle;
    if (!b) continue;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const enemyShips: any[] = b.enemy_ships.map((e) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const items: any[] = (e.slots ?? []).map((eid) => ({ id: eid }));
      return { id: e.ship_id, items };
    });
    cells.push({
      c: node.cell_no,
      pf: b.formation[0],
      ef: b.formation[1],
      f1: { s: enemyShips },
    });
  }

  // Parse map_display "X-Y" -> world X, map Y
  const [worldStr, mapStr] = record.map_display.split("-");
  const world = parseInt(worldStr, 10) || 0;
  const mapId = parseInt(mapStr, 10) || 0;

  if (cells.length > 0) {
    predeck.s = { a: world, i: mapId, c: cells };
  }

  const json = JSON.stringify(predeck);
  return `https://noro6.github.io/kc-web/?predeck=${encodeURIComponent(json)}`;
}

/** Map route colors by event_kind */
const CELL_COLORS: Record<number, string> = {
  0: "#4caf50",  // start - green
  2: "#ffc107",  // resource - yellow
  3: "#9c27b0",  // maelstrom - purple
  4: "#f44336",  // battle - red
  5: "#d32f2f",  // boss - dark red
  6: "#78909c",  // nothing - grey
  7: "#29b6f6",  // aerial - light blue
  8: "#ff7043",  // air raid - orange
  9: "#8d6e63",  // landing - brown
  10: "#26a69a", // anchorage - teal
};

interface MapSpot {
  no: number;
  x: number;
  y: number;
  line?: { x: number; y: number; img?: string };
}

interface MapInfo {
  bg: string[];
  spots: MapSpot[];
}

interface AtlasFrame {
  frame: { x: number; y: number; w: number; h: number };
}

interface MapSprites {
  bg?: string;         // terrain background
  point?: string;      // cell markers overlay (red dots)
  routes: { uri: string; x: number; y: number; w: number; h: number; spotNo: number; isVisited?: boolean }[]; // route connection sprites
}

/** Display the map with the sortie route overlaid */
function MapRouteView({ mapDisplay, nodes, onCellClick, mapMaxWidth }: { mapDisplay: string; nodes: BattleNode[]; onCellClick?: (cellNo: number) => void; mapMaxWidth?: number }) {
  const [mapInfo, setMapInfo] = useState<MapInfo | null>(null);
  const [sprites, setSprites] = useState<MapSprites>({ routes: [] });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const parts = mapDisplay.split("-");
    const area = (parseInt(parts[0], 10) || 0).toString().padStart(3, "0");
    const map = (parseInt(parts[1], 10) || 0).toString().padStart(2, "0");
    const infoPath = `kcs2/resources/map/${area}/${map}_info.json`;
    const atlasPath = `kcs2/resources/map/${area}/${map}_image.json`;

    let cancelled = false;
    (async () => {
      try {
        // Load info and atlas in parallel
        const [infoStr, atlasStr] = await Promise.all([
          invoke<string>("get_cached_resource", { path: infoPath }),
          invoke<string>("get_cached_resource", { path: atlasPath }),
        ]);
        if (cancelled) return;
        if (!infoStr) return;

        const info = JSON.parse(infoStr) as MapInfo;
        setMapInfo(info);

        // Parse atlas for frame dimensions
        let atlas: { frames: Record<string, AtlasFrame> } | null = null;
        if (atlasStr) {
          try { atlas = JSON.parse(atlasStr); } catch { /* ignore */ }
        }

        // Load all sprites in parallel
        const prefix = `map${area}${map}_`;
        const spritePromises: Promise<void>[] = [];
        const result: MapSprites = { routes: [] };

        // Background terrain
        if (info.bg?.[0]) {
          spritePromises.push(
            invoke<string>("get_map_sprite", { mapDisplay, frameName: info.bg[0] })
              .then((uri) => { if (uri) result.bg = uri; })
              .catch(() => { })
          );
        }

        // Point overlay (cell markers)
        if (info.bg?.[1]) {
          spritePromises.push(
            invoke<string>("get_map_sprite", { mapDisplay, frameName: info.bg[1] })
              .then((uri) => { if (uri) result.point = uri; })
              .catch(() => { })
          );
        }

        // Route sprites - each route_N connects to spots[N] using line offset
        const spotsWithLine = info.spots.filter((s) => s.line);

        // Build a lookup: spot no -> MapSpot
        const spotMap = new Map<number, MapSpot>();
        for (const spot of info.spots) {
          spotMap.set(spot.no, spot);
        }

        // Resolve visited cells to coordinates (in order)
        const visitedCells = nodes
          .map((node) => {
            const spot = spotMap.get(node.cell_no);
            if (!spot) return null;
            const label = getNodeLabel(mapDisplay, node.cell_no);
            return { ...spot, event_kind: node.event_kind, event_id: node.event_id, cell_no: node.cell_no, label };
          })
          .filter((c) => c != null);

        for (let i = 0; i < spotsWithLine.length; i++) {
          const spot = spotsWithLine[i];
          // route_N uses 1-based index within spotsWithLine (N = i+1),
          // NOT a sequential counter that skips img spots.
          // If spot has line.img (e.g. "arrow1"), use that as the sprite name.
          const routeN = i + 1;
          let frameName: string;
          if (spot.line?.img) {
            frameName = spot.line.img;
          } else {
            frameName = `route_${routeN}`;
          }
          const fullFrameName = `${prefix}${frameName}`;
          const frame = atlas?.frames[fullFrameName]?.frame;

          if (frame && spot.line) {
            const rx = spot.x + spot.line.x;
            const ry = spot.y + spot.line.y;
            const rw = frame.w;
            const rh = frame.h;
            // spot.no = edge ID = api_no (cell_no in battle log).
            // If the player visited this cell_no, the route sprite should be cyan.
            const tintCyan = visitedCells.some((c) => c.cell_no === spot.no);

            spritePromises.push(
              invoke<string>("get_map_sprite", {
                mapDisplay,
                frameName,
                tintCyan: false,
                routeIdx: i + 1,
                spotNo: spot.no
              })
                .then((uri) => {
                  if (!uri) return;
                  result.routes.push({ uri, x: rx, y: ry, w: rw, h: rh, spotNo: spot.no, isVisited: tintCyan });
                })
                .catch(() => { })
            );
          }
        }

        await Promise.all(spritePromises);
        if (!cancelled) setSprites(result);
      } catch {
        // Cache miss
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [mapDisplay, nodes]);

  if (loading) return null;
  if (!mapInfo) {
    return (
      <div className="map-route-no-data">
        マップデータ未取得（該当マップに出撃するとキャッシュされます）
      </div>
    );
  }

  // Keep existing code for rendering below
  const hasBg = !!sprites.bg;

  // Resolve visited cells to coordinates (in order)
  // Need to duplicate this outside useEffect for rendering pins
  const spotMap = new Map<number, MapSpot>();
  for (const spot of mapInfo.spots) {
    spotMap.set(spot.no, spot);
  }

  const renderedVisitedCells = nodes
    .map((node) => {
      const spot = spotMap.get(node.cell_no);
      if (!spot) return null;
      const label = getNodeLabel(mapDisplay, node.cell_no);
      return { ...spot, event_kind: node.event_kind, event_id: node.event_id, cell_no: node.cell_no, label, hasBattle: node.battle != null };
    })
    .filter((c) => c != null);

  if (renderedVisitedCells.length === 0 && !hasBg) {
    return (
      <div className="map-route-no-data">
        ルート座標データなし
      </div>
    );
  }

  // viewBox
  let vbX: number, vbY: number, vbW: number, vbH: number;
  if (hasBg) {
    vbX = 0; vbY = 0; vbW = 1200; vbH = 720;
  } else {
    const pad = 60;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const s of mapInfo.spots) {
      if (s.x < minX) minX = s.x;
      if (s.y < minY) minY = s.y;
      if (s.x > maxX) maxX = s.x;
      if (s.y > maxY) maxY = s.y;
    }
    vbX = minX - pad; vbY = minY - pad;
    vbW = maxX - minX + pad * 2; vbH = maxY - minY + pad * 2;
  }

  // Start spot
  const startSpot = mapInfo.spots.find((s) => !s.line);

  // Fallback connection lines (when no sprites)
  const connectionLines = !hasBg ? mapInfo.spots
    .filter((s) => s.line)
    .map((s) => ({
      x1: s.x + s.line!.x, y1: s.y + s.line!.y,
      x2: s.x, y2: s.y,
      spotNo: s.no
    })) : [];

  // Map canvas size: 1200x720. All coordinates are in this space.
  // Use a relative container and percentage-based positioning.
  return (
    <div className="map-route-container">
      <div className="map-route-wrapper" style={{ position: "relative", width: "100%", maxWidth: mapMaxWidth != null ? `${mapMaxWidth}px` : "600px", margin: "0 auto" }}>
        {/* Layer 1: Background terrain (HTML img) */}
        {sprites.bg && (
          <img src={sprites.bg} alt="" style={{ width: "100%", display: "block" }} draggable={false} />
        )}
        {/* Invisible sizer if no bg */}
        {!sprites.bg && (
          <div style={{ width: "100%", paddingBottom: `${(vbH / vbW) * 100}%` }} />
        )}

        {/* Layer 2: Cell markers & overlay terrain (HTML img) */}
        {sprites.point && (
          <img src={sprites.point} alt="" draggable={false}
            style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", pointerEvents: "none", zIndex: 1 }} />
        )}

        {/* Layer 3: Route connection sprites (HTML img with CSS filter for visited) */}
        {/* drawn over points so original white routes in sprites.point are overwritten by cyan ones */}
        {sprites.routes.map((r, i) => (
          <img
            key={`route-${r.spotNo}-${i}`}
            src={r.uri}
            alt=""
            draggable={false}
            style={{
              position: "absolute",
              left: `${(r.x / 1200) * 100}%`,
              top: `${(r.y / 720) * 100}%`,
              width: `${(r.w / 1200) * 100}%`,
              height: `${(r.h / 720) * 100}%`,
              pointerEvents: "none",
              zIndex: 2,
              filter: r.isVisited ? "none" : "grayscale(100%) opacity(40%)"
            }}
          />
        ))}

        {/* Layer 4-5: SVG overlay for vector elements (start marker, cell labels, fallback lines) */}
        <svg style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", zIndex: 3 }}
          viewBox={`${vbX} ${vbY} ${vbW} ${vbH}`} preserveAspectRatio="xMidYMid meet">
          {/* Fallback connection lines (no sprites) */}
          {connectionLines.map((seg, i) => (
            <line key={`conn-${i}`} x1={seg.x1} y1={seg.y1} x2={seg.x2} y2={seg.y2}
              stroke="rgba(64,192,216,0.4)" strokeWidth="2" strokeDasharray="6 4" />
          ))}
          {/* Start position marker */}
          {startSpot && (
            <g>
              <circle cx={startSpot.x} cy={startSpot.y} r={14} fill="none" stroke="rgba(76,175,80,0.8)" strokeWidth="3" strokeDasharray="4 3" />
              <text x={startSpot.x} y={startSpot.y + 1} textAnchor="middle" dominantBaseline="central"
                fill="#4caf50" fontSize="11" fontWeight="bold" style={{ pointerEvents: "none" }}>
                出撃
              </text>
            </g>
          )}
          {/* Visited cell labels */}
          {renderedVisitedCells.map((cell, i) => {
            if (!cell) return null;
            const isBoss = cell.event_id === 5 || cell.event_kind === 5;
            const isNonBattle = !cell.hasBattle;
            const color = isNonBattle && (cell.event_kind === 4 || cell.event_kind === 5) ? "#b0bec5" : (CELL_COLORS[cell.event_kind] ?? "#78909c");
            const r = isBoss ? 18 : 14;
            const label = cell.label ?? String(cell.cell_no);
            return (
              <g key={`cell-${i}`} style={{ cursor: (onCellClick && !isNonBattle) ? "pointer" : "default" }}
                onClick={() => { if (!isNonBattle) onCellClick?.(cell.cell_no); }}>
                <circle cx={cell.x} cy={cell.y} r={r} fill={color} stroke="#fff" strokeWidth="1.5" />
                <text x={cell.x} y={cell.y} textAnchor="middle" dominantBaseline="central"
                  fill="#fff" fontSize={label.length > 2 ? "9" : isBoss ? "14" : "11"} fontWeight="bold"
                  style={{ pointerEvents: "none" }}>
                  {label}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}

function BattleDetailView({
  record,
  onBack,
}: {
  record: SortieRecord;
  onBack: () => void;
}) {
  const [highlightCellNo, setHighlightCellNo] = useState<number | null>(null);
  const nodeRefs = useRef<Map<number, HTMLDivElement | null>>(new Map());
  const splitContainerRef = useRef<HTMLDivElement | null>(null);
  const splitTopRef = useRef<HTMLDivElement | null>(null);
  const [mapRatio, setMapRatio] = useState(0.4);
  const [mapMaxWidth, setMapMaxWidth] = useState(600);
  const draggingRef = useRef(false);

  useEffect(() => {
    const el = splitTopRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      const h = entries[0].contentRect.height;
      // Map aspect ratio 1200:720 = 5:3. Subtract padding (6px*2=12px).
      setMapMaxWidth(Math.min(600, Math.max(100, (h - 12) * (1200 / 720))));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const openAircalc = async () => {
    const url = buildPredeckUrl(record);
    await openUrl(url);
  };

  const handleCellClick = useCallback((cellNo: number) => {
    setHighlightCellNo(cellNo);
    const el = nodeRefs.current.get(cellNo);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
    setTimeout(() => setHighlightCellNo(null), 1500);
  }, []);

  const handleSplitterMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    draggingRef.current = true;
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";

    const onMouseMove = (ev: MouseEvent) => {
      if (!draggingRef.current || !splitContainerRef.current) return;
      const rect = splitContainerRef.current.getBoundingClientRect();
      const y = ev.clientY - rect.top;
      const ratio = Math.min(0.8, Math.max(0.15, y / rect.height));
      setMapRatio(ratio);
    };

    const onMouseUp = () => {
      draggingRef.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }, []);

  return (
    <div className="battle-detail">
      <div className="battle-detail-header">
        <button className="battle-back-btn" onClick={onBack}>
          &larr; 戻る
        </button>
        <span className="battle-detail-map">{record.map_display}</span>
        <span className="battle-detail-fleet">第{record.fleet_id}艦隊</span>
        <span className="battle-detail-time">{record.start_time}</span>
        <button className="aircalc-btn" onClick={openAircalc} title="制空権計算機で開く">
          制空計算
        </button>
      </div>

      {/* Fleet composition */}
      <div className="battle-detail-fleet-comp">
        <div className="battle-section-title">出撃艦隊</div>
        <div className="battle-fleet-ships">
          {record.ships.map((ship, i) => (
            <span key={i} className="battle-fleet-ship">
              {ship.name} <span className="battle-ship-lv">Lv{ship.lv}</span>
            </span>
          ))}
        </div>
      </div>

      {/* Resizable split: Map + Nodes */}
      <div className="battle-split-container" ref={splitContainerRef}>
        <div className="battle-split-top" ref={splitTopRef} style={{ flex: `0 0 ${mapRatio * 100}%` }}>
          <MapRouteView mapDisplay={record.map_display} nodes={record.nodes} onCellClick={handleCellClick} mapMaxWidth={mapMaxWidth} />
        </div>
        <div className="battle-splitter" onMouseDown={handleSplitterMouseDown}>
          <div className="battle-splitter-handle" />
        </div>
        <div className="battle-split-bottom">
          {record.nodes.filter((n) => n.battle != null).map((node, i) => (
            <div
              key={node.cell_no}
              ref={(el) => { nodeRefs.current.set(node.cell_no, el); }}
              className={highlightCellNo === node.cell_no ? "node-highlight" : ""}
            >
              <BattleNodeDetail node={node} ships={record.ships} nodeIndex={i} mapDisplay={record.map_display} />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/** Detailed view of a single battle node */
function BattleNodeDetail({
  node,
  ships,
  mapDisplay,
}: {
  node: BattleNode;
  ships: SortieShip[];
  nodeIndex: number;
  mapDisplay: string;
}) {
  const b = node.battle;
  const isBattle = b != null;
  const eventLabel = (node.event_id != null && EVENT_ID_LABELS[node.event_id]) || EVENT_LABELS[node.event_kind] || `Event ${node.event_kind}`;
  const cellLabel = node.cell_no > 0 ? getNodeLabel(mapDisplay, node.cell_no) : null;

  return (
    <div className={`battle-node-detail ${isBattle ? "has-battle" : "no-battle"}`}>
      {/* Node header */}
      <div className="battle-node-header">
        <span className="battle-node-cell">
          {node.cell_no > 0 ? (cellLabel ? `${cellLabel}` : `${node.cell_no}マス`) : "出撃"}
        </span>
        <span className={`battle-node-event event-${node.event_kind}`}>
          {eventLabel}
        </span>
        {b?.rank && (
          <span
            className="battle-node-rank"
            style={{ color: RANK_COLORS[b.rank] ?? "#888" }}
          >
            {b.rank}
          </span>
        )}
        {b?.enemy_name && (
          <span className="battle-node-enemy">{b.enemy_name}</span>
        )}
        {b?.mvp != null && b.mvp > 0 && (
          <span className="battle-node-mvp">
            MVP: {ships[b.mvp - 1]?.name ?? `#${b.mvp}`}
          </span>
        )}
        {b?.base_exp != null && (
          <span className="battle-node-exp">+{b.base_exp} exp</span>
        )}
        {b?.night_battle && (
          <span className="battle-node-night">夜戦</span>
        )}
        {b?.drop_ship && (
          <span className="battle-node-drop">
            drop: {b.drop_ship}
          </span>
        )}
      </div>

      {/* Battle details */}
      {b && (
        <div className="battle-node-body">
          {/* Formation info */}
          {b.formation && (
            <div className="battle-formation-row">
              <span className="formation-label">陣形:</span>
              <span className="formation-friendly">
                {FORMATION_NAMES[b.formation[0]] ?? `F${b.formation[0]}`}
              </span>
              <span className="formation-vs">vs</span>
              <span className="formation-enemy">
                {FORMATION_NAMES[b.formation[1]] ?? `F${b.formation[1]}`}
              </span>
              <span className="formation-sep">|</span>
              <span className="formation-engagement">
                {ENGAGEMENT_NAMES[b.formation[2]] ?? `E${b.formation[2]}`}
              </span>
            </div>
          )}

          {/* Air battle result */}
          {b.air_battle && (
            <div className="battle-air-row">
              {b.air_battle.air_superiority != null && (
                <span
                  className="air-superiority"
                  style={{ color: AIR_SUPERIORITY_LABELS[b.air_battle.air_superiority]?.color ?? "#888" }}
                >
                  {AIR_SUPERIORITY_LABELS[b.air_battle.air_superiority]?.text ?? `制空${b.air_battle.air_superiority}`}
                </span>
              )}
              {b.air_battle.friendly_plane_count && (
                <span className="air-planes friendly">
                  味方 {b.air_battle.friendly_plane_count[0] - b.air_battle.friendly_plane_count[1]}/{b.air_battle.friendly_plane_count[0]}
                </span>
              )}
              {b.air_battle.enemy_plane_count && (
                <span className="air-planes enemy">
                  敵 {b.air_battle.enemy_plane_count[0] - b.air_battle.enemy_plane_count[1]}/{b.air_battle.enemy_plane_count[0]}
                </span>
              )}
            </div>
          )}

          {/* Fleet HP side-by-side: friendly left, enemy right */}
          <div className="battle-fleets-row">
            {/* Friendly fleet HP (left) */}
            {b.friendly_hp.length > 0 && (
              <div className="battle-hp-section battle-hp-friendly">
                <div className="battle-hp-label">味方艦隊</div>
                <div className="battle-hp-list">
                  {b.friendly_hp.map((hp, idx) => (
                    <BattleHpBar
                      key={idx}
                      before={hp.before}
                      after={hp.after}
                      max={hp.max}
                      shipName={ships[idx]?.name}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* Enemy fleet HP (right) */}
            {b.enemy_hp.length > 0 && (
              <div className="battle-hp-section battle-hp-enemy">
                <div className="battle-hp-label">{b.enemy_name || "敵艦隊"}</div>
                <div className="battle-hp-list">
                  {b.enemy_hp.map((hp, idx) => {
                    const enemy = b.enemy_ships[idx];
                    const enemyName = enemy?.name ?? (enemy ? `ID:${enemy.ship_id}` : undefined);
                    return (
                      <BattleHpBar
                        key={idx}
                        before={hp.before}
                        after={hp.after}
                        max={hp.max}
                        shipName={enemyName}
                      />
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/** Helper: format YYYY-MM-DD to compact display */
function fmtDate(d: string) {
  const [y, m, dd] = d.split("-");
  return `${y}/${m}/${dd}`;
}

/** Helper: get days in month */
function daysInMonth(year: number, month: number) {
  return new Date(year, month + 1, 0).getDate();
}

/** Helper: YYYY-MM-DD string from Date parts */
function toDateStr(y: number, m: number, d: number) {
  return `${y}-${String(m + 1).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

/** Date range picker with calendar - select FROM and TO by clicking */
function DateRangePicker({
  dateFrom,
  dateTo,
  onChange,
}: {
  dateFrom: string;
  dateTo: string;
  onChange: (from: string, to: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [viewYear, setViewMonth_year] = useState(() => {
    const d = dateFrom ? new Date(dateFrom) : new Date();
    return d.getFullYear();
  });
  const [viewMonth, setViewMonth_month] = useState(() => {
    const d = dateFrom ? new Date(dateFrom) : new Date();
    return d.getMonth();
  });
  // Selection state: null = picking start, string = start picked, picking end
  const [pickStart, setPickStart] = useState<string | null>(null);
  const [hoverDate, setHoverDate] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setPickStart(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const prevMonth = () => {
    if (viewMonth === 0) { setViewMonth_year(viewYear - 1); setViewMonth_month(11); }
    else setViewMonth_month(viewMonth - 1);
  };
  const nextMonth = () => {
    if (viewMonth === 11) { setViewMonth_year(viewYear + 1); setViewMonth_month(0); }
    else setViewMonth_month(viewMonth + 1);
  };

  const handleDayClick = (dateStr: string) => {
    if (pickStart === null) {
      // First click: set start
      setPickStart(dateStr);
    } else {
      // Second click: set range (auto-swap if needed)
      const [a, b] = pickStart <= dateStr ? [pickStart, dateStr] : [dateStr, pickStart];
      onChange(a, b);
      setPickStart(null);
      setOpen(false);
    }
  };

  const today = new Date();
  const todayStr = toDateStr(today.getFullYear(), today.getMonth(), today.getDate());

  // Build calendar grid
  const firstDow = new Date(viewYear, viewMonth, 1).getDay(); // 0=Sun
  const totalDays = daysInMonth(viewYear, viewMonth);
  const weeks: (number | null)[][] = [];
  let week: (number | null)[] = Array(firstDow).fill(null);
  for (let d = 1; d <= totalDays; d++) {
    week.push(d);
    if (week.length === 7) { weeks.push(week); week = []; }
  }
  if (week.length > 0) {
    while (week.length < 7) week.push(null);
    weeks.push(week);
  }

  // Determine effective range for highlighting
  const effFrom = pickStart ?? dateFrom;
  const effTo = pickStart ? (hoverDate ?? pickStart) : dateTo;
  const rangeStart = effFrom <= effTo ? effFrom : effTo;
  const rangeEnd = effFrom <= effTo ? effTo : effFrom;

  return (
    <div className="drp-container" ref={containerRef}>
      <button className="drp-trigger" onClick={() => { setOpen(!open); setPickStart(null); }}>
        {dateFrom && dateTo ? `${fmtDate(dateFrom)} 〜 ${fmtDate(dateTo)}` : "全期間"}
      </button>
      {open && (
        <div className="drp-dropdown">
          <div className="drp-calendar">
            <div className="drp-nav">
              <button onClick={prevMonth}>&lt;</button>
              <span>{viewYear}年{viewMonth + 1}月</span>
              <button onClick={nextMonth}>&gt;</button>
            </div>
            <div className="drp-grid">
              {["日", "月", "火", "水", "木", "金", "土"].map((w) => (
                <div key={w} className="drp-dow">{w}</div>
              ))}
              {weeks.flat().map((day, i) => {
                if (day === null) return <div key={`e${i}`} className="drp-cell drp-empty" />;
                const ds = toDateStr(viewYear, viewMonth, day);
                const isInRange = ds >= rangeStart && ds <= rangeEnd;
                const isStart = ds === rangeStart;
                const isEnd = ds === rangeEnd;
                const isToday = ds === todayStr;
                return (
                  <div
                    key={ds}
                    className={[
                      "drp-cell",
                      isInRange ? "drp-in-range" : "",
                      isStart ? "drp-start" : "",
                      isEnd ? "drp-end" : "",
                      isToday ? "drp-today" : "",
                    ].join(" ")}
                    onClick={() => handleDayClick(ds)}
                    onMouseEnter={() => setHoverDate(ds)}
                    onMouseLeave={() => setHoverDate(null)}
                  >
                    {day}
                  </div>
                );
              })}
            </div>
          </div>
          {pickStart && (
            <div className="drp-hint">終了日を選択してください</div>
          )}
        </div>
      )}
    </div>
  );
}

/** Battle tab - full page battle log viewer */
function BattleTab({
  battleLogs,
  onRefresh,
  totalRecords,
  dateFrom,
  dateTo,
  onDateChange,
}: {
  battleLogs: SortieRecord[];
  onRefresh: () => void;
  totalRecords: number;
  dateFrom: string;
  dateTo: string;
  onDateChange: (from: string, to: string) => void;
}) {
  const [selectedRecord, setSelectedRecord] = useState<SortieRecord | null>(null);
  const [mapFilter, setMapFilter] = useState("");

  // Get unique maps for filter
  const uniqueMaps = Array.from(new Set(battleLogs.map((r) => r.map_display))).sort();

  const filteredLogs = mapFilter
    ? battleLogs.filter((r) => r.map_display === mapFilter)
    : battleLogs;

  if (selectedRecord) {
    return (
      <BattleDetailView
        record={selectedRecord}
        onBack={() => setSelectedRecord(null)}
      />
    );
  }

  return (
    <div className="battle-tab">
      {/* Date range + filter bar */}
      <div className="battle-filter-bar">
        <DateRangePicker dateFrom={dateFrom} dateTo={dateTo} onChange={onDateChange} />
        <button className="battle-preset-btn" onClick={() => {
          const now = new Date();
          const t = toDateStr(now.getFullYear(), now.getMonth(), now.getDate());
          onDateChange(t, t);
        }}>今日</button>
        <button className="battle-preset-btn" onClick={() => {
          const now = new Date();
          const from = toDateStr(now.getFullYear(), now.getMonth(), 1);
          const lastDay = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
          const to = toDateStr(now.getFullYear(), now.getMonth(), lastDay);
          onDateChange(from, to);
        }}>今月</button>
        <button className="battle-preset-btn" onClick={() => {
          onDateChange("", "");
        }}>全て</button>
        <select
          className="battle-filter-select"
          value={mapFilter}
          onChange={(e) => setMapFilter(e.target.value)}
        >
          <option value="">全マップ</option>
          {uniqueMaps.map((map) => (
            <option key={map} value={map}>{map}</option>
          ))}
        </select>
        <span className="battle-record-count">
          {filteredLogs.length}件 / {totalRecords}件
        </span>
        <button className="battle-refresh-btn" onClick={onRefresh}>
          更新
        </button>
      </div>

      {/* Record list */}
      <div className="battle-record-list">
        {filteredLogs.length === 0 ? (
          <div className="no-data">出撃記録なし</div>
        ) : (
          filteredLogs.map((record) => (
            <div
              key={record.id}
              className="battle-record-row"
              onClick={() => setSelectedRecord(record)}
            >
              <span className="br-map">{record.map_display}</span>
              {!record.end_time && (
                <span className="br-in-progress">出撃中</span>
              )}
              <span className="br-fleet">第{record.fleet_id}艦隊</span>
              <span className="br-ships">
                {record.ships.map((s) => s.name).join(", ")}
              </span>
              <span className="br-ranks">
                {record.nodes
                  .filter((n) => n.battle?.rank)
                  .map((n, i) => (
                    <span
                      key={i}
                      className="br-rank-badge"
                      style={{ color: RANK_COLORS[n.battle!.rank] ?? "#888" }}
                    >
                      {n.battle!.rank}
                    </span>
                  ))}
              </span>
              <span className="br-drops">
                {record.nodes
                  .filter((n) => n.battle?.drop_ship)
                  .map((n) => n.battle!.drop_ship)
                  .join(", ")}
              </span>
              <span className="br-time">{record.start_time}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ── Ship List Tab ──

interface ShipListItem {
  id: number;
  ship_id: number;
  name: string;
  stype: number;
  stype_name: string;
  lv: number;
  hp: number;
  maxhp: number;
  cond: number;
  firepower: number;
  torpedo: number;
  aa: number;
  armor: number;
  asw: number;
  evasion: number;
  los: number;
  luck: number;
  locked: boolean;
}

interface ShipListResponse {
  ships: ShipListItem[];
  stypes: [number, string][];
}

type ShipSortKey = "lv" | "name" | "stype" | "firepower" | "torpedo" | "aa" | "armor" | "asw" | "evasion" | "los" | "luck" | "cond" | "locked";

function ShipListTab({ portDataVersion }: { portDataVersion: number }) {
  const [data, setData] = useState<ShipListResponse | null>(null);
  const [stypeFilters, setStypeFilters] = useState<Set<number>>(() => {
    const saved = localStorage.getItem("ship-stype-filters");
    return saved ? new Set(JSON.parse(saved) as number[]) : new Set();
  });
  const [sortKey, setSortKey] = useState<ShipSortKey>("lv");
  const [sortAsc, setSortAsc] = useState(false);

  useEffect(() => {
    invoke<ShipListResponse>("get_ship_list")
      .then(setData)
      .catch(console.error);
  }, [portDataVersion]);

  // Build stype list from actual ship data (only types that exist)
  const stypes = useMemo(() => {
    if (!data) return [];
    const typeMap = new Map<number, string>();
    for (const ship of data.ships) {
      if (!typeMap.has(ship.stype)) {
        typeMap.set(ship.stype, ship.stype_name);
      }
    }
    return Array.from(typeMap.entries()).sort((a, b) => a[0] - b[0]);
  }, [data]);

  const handleSort = (key: ShipSortKey) => {
    if (sortKey === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(key === "name" || key === "stype");
    }
  };

  const sortIndicator = (key: ShipSortKey) =>
    sortKey === key ? (sortAsc ? " ▲" : " ▼") : "";

  const toggleStype = (stypeId: number) => {
    setStypeFilters((prev) => {
      const next = new Set(prev);
      if (next.has(stypeId)) next.delete(stypeId);
      else next.add(stypeId);
      localStorage.setItem("ship-stype-filters", JSON.stringify([...next]));
      return next;
    });
  };

  const clearStypeFilters = () => {
    setStypeFilters(new Set());
    localStorage.removeItem("ship-stype-filters");
  };

  const displayShips = useMemo(() => {
    if (!data) return [];
    let ships = data.ships;
    if (stypeFilters.size > 0) {
      ships = ships.filter((s) => stypeFilters.has(s.stype));
    }
    return [...ships].sort((a, b) => {
      let cmp = 0;
      if (sortKey === "name") {
        cmp = a.name.localeCompare(b.name);
      } else if (sortKey === "stype") {
        cmp = a.stype - b.stype || a.stype_name.localeCompare(b.stype_name);
      } else if (sortKey === "locked") {
        cmp = (a.locked ? 1 : 0) - (b.locked ? 1 : 0);
      } else {
        cmp = (a[sortKey] as number) - (b[sortKey] as number);
      }
      return sortAsc ? cmp : -cmp;
    });
  }, [data, stypeFilters, sortKey, sortAsc]);

  if (!data) {
    return (
      <div className="ship-list-tab">
        <div className="no-data">データ読込中...</div>
      </div>
    );
  }

  if (data.ships.length === 0) {
    return (
      <div className="ship-list-tab">
        <div className="no-data">母港データ未読込</div>
      </div>
    );
  }

  return (
    <div className="ship-list-tab">
      <div className="list-header">
        <span className="list-count">
          {stypeFilters.size > 0
            ? `${displayShips.length}/${data.ships.length}隻`
            : `${data.ships.length}隻`}
        </span>
      </div>
      <div className="list-filters">
        {stypes.map(([id, name]) => (
          <button
            key={id}
            className={`list-filter-btn ${stypeFilters.size === 0 || stypeFilters.has(id) ? "active" : ""}`}
            onClick={() => toggleStype(id)}
          >
            {name}
          </button>
        ))}
        {stypeFilters.size > 0 && (
          <button className="list-filter-clear" onClick={clearStypeFilters}>
            全表示
          </button>
        )}
      </div>
      <div className="list-table-wrap">
        <table className="list-table">
          <thead>
            <tr>
              <th className="col-name sortable" onClick={() => handleSort("name")}>名前{sortIndicator("name")}</th>
              <th className="col-stype sortable" onClick={() => handleSort("stype")}>艦種{sortIndicator("stype")}</th>
              <th className="col-num sortable" onClick={() => handleSort("lv")}>Lv{sortIndicator("lv")}</th>
              <th className="col-num sortable" onClick={() => handleSort("firepower")}>火力{sortIndicator("firepower")}</th>
              <th className="col-num sortable" onClick={() => handleSort("torpedo")}>雷装{sortIndicator("torpedo")}</th>
              <th className="col-num sortable" onClick={() => handleSort("aa")}>対空{sortIndicator("aa")}</th>
              <th className="col-num sortable" onClick={() => handleSort("armor")}>装甲{sortIndicator("armor")}</th>
              <th className="col-num sortable" onClick={() => handleSort("asw")}>対潜{sortIndicator("asw")}</th>
              <th className="col-num sortable" onClick={() => handleSort("evasion")}>回避{sortIndicator("evasion")}</th>
              <th className="col-num sortable" onClick={() => handleSort("los")}>索敵{sortIndicator("los")}</th>
              <th className="col-num sortable" onClick={() => handleSort("luck")}>運{sortIndicator("luck")}</th>
              <th className="col-num sortable" onClick={() => handleSort("cond")}>cond{sortIndicator("cond")}</th>
              <th className="col-lock sortable" onClick={() => handleSort("locked")}>鍵{sortIndicator("locked")}</th>
            </tr>
          </thead>
          <tbody>
            {displayShips.map((ship) => (
              <tr key={ship.id} className={ship.locked ? "" : "unlocked"}>
                <td className="col-name">{ship.name}</td>
                <td className="col-stype">{ship.stype_name}</td>
                <td className="col-num">{ship.lv}</td>
                <td className="col-num">{ship.firepower}</td>
                <td className="col-num">{ship.torpedo}</td>
                <td className="col-num">{ship.aa}</td>
                <td className="col-num">{ship.armor}</td>
                <td className="col-num">{ship.asw}</td>
                <td className="col-num">{ship.evasion}</td>
                <td className="col-num">{ship.los}</td>
                <td className="col-num">{ship.luck}</td>
                <td className="col-num" style={{ color: condColor(ship.cond) }}>{ship.cond}</td>
                <td className="col-lock">{ship.locked ? "🔒" : ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ── Equipment List Tab ──

interface EquipListItem {
  master_id: number;
  name: string;
  type_id: number;
  type_name: string;
  icon_type: number;
  total_count: number;
  locked_count: number;
  improvements: [number, number][]; // [level, count]
}

interface EquipListResponse {
  items: EquipListItem[];
  equip_types: [number, string][];
}

function formatImprovements(improvements: [number, number][]): string {
  return improvements
    .filter(([level]) => level > 0)
    .map(([level, count]) => {
      const label = level >= 10 ? "★max" : `★${level}`;
      return `${label}×${count}`;
    })
    .join(" ");
}

function EquipListTab({ portDataVersion }: { portDataVersion: number }) {
  const [data, setData] = useState<EquipListResponse | null>(null);
  const [typeFilters, setTypeFilters] = useState<Set<number>>(() => {
    const saved = localStorage.getItem("equip-type-filters");
    return saved ? new Set(JSON.parse(saved) as number[]) : new Set();
  });

  useEffect(() => {
    invoke<EquipListResponse>("get_equipment_list")
      .then(setData)
      .catch(console.error);
  }, [portDataVersion]);

  // Build type list from actual equipment data (only types that exist)
  const types = useMemo(() => {
    if (!data) return [];
    const typeMap = new Map<number, string>();
    for (const item of data.items) {
      if (!typeMap.has(item.type_id)) {
        typeMap.set(item.type_id, item.type_name);
      }
    }
    return Array.from(typeMap.entries()).sort((a, b) => a[0] - b[0]);
  }, [data]);

  const toggleType = (typeId: number) => {
    setTypeFilters((prev) => {
      const next = new Set(prev);
      if (next.has(typeId)) next.delete(typeId);
      else next.add(typeId);
      localStorage.setItem("equip-type-filters", JSON.stringify([...next]));
      return next;
    });
  };

  const clearTypeFilters = () => {
    setTypeFilters(new Set());
    localStorage.removeItem("equip-type-filters");
  };

  const displayItems = useMemo(() => {
    if (!data) return [];
    let items = data.items;
    if (typeFilters.size > 0) {
      items = items.filter((i) => typeFilters.has(i.type_id));
    }
    return items;
  }, [data, typeFilters]);

  if (!data) {
    return (
      <div className="equip-list-tab">
        <div className="no-data">データ読込中...</div>
      </div>
    );
  }

  if (data.items.length === 0) {
    return (
      <div className="equip-list-tab">
        <div className="no-data">装備データ未読込</div>
      </div>
    );
  }

  return (
    <div className="equip-list-tab">
      <div className="list-header">
        <span className="list-count">
          {typeFilters.size > 0
            ? `${displayItems.length}/${data.items.length}種`
            : `${data.items.length}種`}
        </span>
      </div>
      <div className="list-filters">
        {types.map(([id, name]) => (
          <button
            key={id}
            className={`list-filter-btn ${typeFilters.size === 0 || typeFilters.has(id) ? "active" : ""}`}
            onClick={() => toggleType(id)}
          >
            {name}
          </button>
        ))}
        {typeFilters.size > 0 && (
          <button className="list-filter-clear" onClick={clearTypeFilters}>
            全表示
          </button>
        )}
      </div>
      <div className="list-table-wrap">
        <table className="list-table">
          <thead>
            <tr>
              <th className="col-name">装備名</th>
              <th className="col-stype">装備種</th>
              <th className="col-num">個数</th>
              <th className="col-num">ロック</th>
              <th className="col-improvements">改修内訳</th>
            </tr>
          </thead>
          <tbody>
            {displayItems.map((item) => (
              <tr key={item.master_id}>
                <td className="col-name">{item.name}</td>
                <td className="col-stype">{item.type_name}</td>
                <td className="col-num">{item.total_count}</td>
                <td className="col-num">{item.locked_count}</td>
                <td className="col-improvements">{formatImprovements(item.improvements)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ── Improvement Tab Types & Component ──

interface ImprovementItem {
  eq_id: number;
  name: string;
  eq_type: number;
  type_name: string;
  sort_value: number;
  available_today: boolean;
  today_helpers: string[];
  matches_secretary: boolean;
  previously_improved: boolean;
}

interface ImprovementListResponse {
  items: ImprovementItem[];
  day_of_week: number;
  secretary_ship: string;
}

const DAY_NAMES = ["日", "月", "火", "水", "木", "金", "土"];

function ImprovementTab({ portDataVersion }: { portDataVersion: number }) {
  const [data, setData] = useState<ImprovementListResponse | null>(null);
  const [typeFilters, setTypeFilters] = useState<Set<number>>(() => {
    const saved = localStorage.getItem("improvement-type-filters");
    return saved ? new Set(JSON.parse(saved) as number[]) : new Set();
  });

  useEffect(() => {
    invoke<ImprovementListResponse>("get_improvement_list")
      .then(setData)
      .catch(console.error);
  }, [portDataVersion]);

  const types = useMemo(() => {
    if (!data) return [];
    const typeMap = new Map<number, string>();
    for (const item of data.items) {
      if (!typeMap.has(item.eq_type)) {
        typeMap.set(item.eq_type, item.type_name);
      }
    }
    return Array.from(typeMap.entries()).sort((a, b) => a[0] - b[0]);
  }, [data]);

  const displayItems = useMemo(() => {
    if (!data) return [];
    let items = data.items;

    if (typeFilters.size > 0) {
      items = items.filter((item) => typeFilters.has(item.eq_type));
    }

    return [...items].sort((a, b) => {
      // 1. Available today first
      if (a.available_today !== b.available_today)
        return a.available_today ? -1 : 1;
      // 2. Previously improved first
      if (a.previously_improved !== b.previously_improved)
        return a.previously_improved ? -1 : 1;
      // 3. Sort value (primary stat) descending
      if (a.sort_value !== b.sort_value) return b.sort_value - a.sort_value;
      // 4. Name
      return a.name.localeCompare(b.name);
    });
  }, [data, typeFilters]);

  const toggleType = (typeId: number) => {
    setTypeFilters((prev) => {
      const next = new Set(prev);
      if (next.has(typeId)) next.delete(typeId);
      else next.add(typeId);
      localStorage.setItem(
        "improvement-type-filters",
        JSON.stringify([...next])
      );
      return next;
    });
  };

  const clearFilters = () => {
    setTypeFilters(new Set());
    localStorage.removeItem("improvement-type-filters");
  };

  if (!data || data.items.length === 0) {
    return (
      <div className="improvement-tab">
        <div className="no-data">
          {data ? "マスターデータ未読込" : "データ読込中..."}
        </div>
      </div>
    );
  }

  const todayCount = displayItems.filter((i) => i.available_today).length;

  return (
    <div className="improvement-tab">
      {/* Header: day + secretary */}
      <div className="improvement-header">
        <span className="improvement-day">
          {DAY_NAMES[data.day_of_week]}曜日
        </span>
        {data.secretary_ship && (
          <span className="improvement-secretary">
            2番艦: {data.secretary_ship}
          </span>
        )}
        <span className="improvement-count">
          {typeFilters.size > 0
            ? `${todayCount}/${displayItems.length}件`
            : `本日 ${todayCount}/${data.items.length}件`}
        </span>
      </div>

      {/* Type filter toggles */}
      <div className="improvement-filters">
        {types.map(([typeId, typeName]) => (
          <button
            key={typeId}
            className={`imp-filter-btn ${typeFilters.size === 0 || typeFilters.has(typeId) ? "active" : ""
              }`}
            onClick={() => toggleType(typeId)}
          >
            {typeName}
          </button>
        ))}
        {typeFilters.size > 0 && (
          <button className="imp-filter-clear" onClick={clearFilters}>
            全表示
          </button>
        )}
      </div>

      {/* Equipment list */}
      <div className="improvement-list">
        {displayItems.map((item) => (
          <div
            key={item.eq_id}
            className={`imp-row ${item.available_today ? "imp-available" : "imp-unavailable"
              } ${item.matches_secretary ? "imp-match" : ""}`}
          >
            <span className="imp-name" title={item.name}>
              {item.name}
            </span>
            <span className="imp-type">{item.type_name}</span>
            {item.previously_improved && (
              <span className="imp-history" title="改修済み">
                ★
              </span>
            )}
            <span className="imp-helpers">
              {item.today_helpers.length > 0
                ? item.today_helpers.join(", ")
                : "-"}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function App() {
  const [proxyPort, setProxyPort] = useState<number>(0);
  const [portData, setPortData] = useState<PortData | null>(null);
  const [senkaData, setSenkaData] = useState<SenkaSummary | null>(null);
  const [senkaCheckpoint, setSenkaCheckpoint] = useState(false);
  const [apiLog, setApiLog] = useState<ApiLogEntry[]>([]);
  const [gameOpen, setGameOpen] = useState(false);
  const [caInstalled, setCaInstalled] = useState<boolean | null>(null);
  const [caInstalling, setCaInstalling] = useState(false);
  const [logCollapsed, setLogCollapsed] = useState(false);
  const [now, setNow] = useState(Date.now());
  const logRef = useRef<HTMLDivElement>(null);
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
    const saved = localStorage.getItem("ui-zoom");
    return saved ? Number(saved) : 135;
  });
  // Google Drive sync state
  const [driveStatus, setDriveStatus] = useState<{
    authenticated: boolean;
    email?: string;
    syncing: boolean;
    last_sync?: string;
    error?: string;
  }>({ authenticated: false, syncing: false });
  const [driveLoading, setDriveLoading] = useState(false);

  const [showApiLog, setShowApiLog] = useState<boolean>(() => {
    return localStorage.getItem("show-api-log") === "true";
  });
  const [rawApiEnabled, setRawApiEnabled] = useState<boolean>(() => {
    return localStorage.getItem("raw-api-enabled") === "true";
  });

  // Weapon icon sprite sheet for damecon indicator
  const [weaponIconSheet, setWeaponIconSheet] = useState<string | null>(null);

  // Tick every second for countdown timers
  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, []);

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

  // Re-fetch when view mode or date selection changes
  useEffect(() => {
    refreshBattleLogs();
  }, [refreshBattleLogs]);

  useEffect(() => {
    const unlistenProxy = listen<number>("proxy-ready", (event) => {
      setProxyPort(event.payload);
      checkCa();
    });

    let weaponIconLoaded = false;
    const unlistenPort = listen<PortData>("port-data", (event) => {
      setPortData(event.payload);
      setPortDataVersion((v) => v + 1);
      // Load weapon icon sprite sheet once for damecon display
      if (!weaponIconLoaded) {
        weaponIconLoaded = true;
        invoke<string>("get_cached_resource", {
          path: "kcs2/img/common/common_icon_weapon.png",
        }).then((dataUri) => {
          if (dataUri) setWeaponIconSheet(dataUri);
        }).catch(() => { weaponIconLoaded = false; });
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
        return [event.payload, ...prev].slice(0, 200);
      });
      setBattleLogsTotal((prev) => prev + 1);
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

    const unlistenDriveStatus = listen<typeof driveStatus>("drive-sync-status", (event) => {
      setDriveStatus(event.payload);
    });

    const unlistenDriveData = listen("drive-data-updated", () => {
      // Reload all data that may have been updated from remote sync
      invoke<QuestProgressSummary[]>("get_quest_progress").then((progress) => {
        const map = new Map<number, QuestProgressSummary>();
        for (const p of progress) map.set(p.quest_id, p);
        setQuestProgress(map);
      }).catch(console.error);
      refreshBattleLogs();
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
    invoke<typeof driveStatus>("get_drive_status").then(setDriveStatus).catch(console.error);

    // Restore raw API enabled state from localStorage to backend
    const savedRawApi = localStorage.getItem("raw-api-enabled") === "true";
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

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [apiLog]);

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
          <>
            {portData ? (
              <>
                {/* Top bar: Admiral + Resources */}
                <div className="top-bar">
                  {/* Admiral section */}
                  <div className="admiral-section">
                    <span className="admiral-name">{portData.admiral_name}</span>
                    <span className="admiral-detail">
                      Lv.{portData.admiral_level}
                    </span>
                    {portData.admiral_rank != null && (
                      <span className="admiral-detail">
                        {getRankName(portData.admiral_rank)}
                      </span>
                    )}
                    <span className="admiral-detail">
                      艦:{portData.ship_count}
                      {portData.ship_capacity != null && `/${portData.ship_capacity}`}
                    </span>
                    {senkaData && senkaData.tracking_active && (
                      senkaData.is_confirmed_base ? (
                        <span className="admiral-detail senka-display" title={
                          `確認済み: ${senkaData.confirmed_senka ?? 0} (${senkaData.confirmed_cutoff ? new Date(senkaData.confirmed_cutoff).toLocaleTimeString('ja-JP', {hour: '2-digit', minute: '2-digit'}) + 'まで反映' : '?'})` +
                          `\n追加経験値: +${senkaData.exp_senka.toFixed(1)} (exp +${senkaData.monthly_exp_gain.toLocaleString()})` +
                          (senkaData.eo_bonus > 0 ? `\n追加EO: +${senkaData.eo_bonus}` : '') +
                          (senkaData.quest_bonus > 0 ? `\n追加任務: +${senkaData.quest_bonus}` : '')
                        }>
                          戦果:{senkaData.total.toFixed(1)}
                          <span className="senka-breakdown">
                            ({senkaData.confirmed_senka}+{senkaData.exp_senka.toFixed(1)}
                            {senkaData.eo_bonus > 0 && `+EO${senkaData.eo_bonus}`}
                            {senkaData.quest_bonus > 0 && `+任${senkaData.quest_bonus}`})
                          </span>
                        </span>
                      ) : (
                        <span className="admiral-detail senka-unconfirmed">
                          戦果:ランキング画面で確認してください
                        </span>
                      )
                    )}
                  </div>
                  {senkaCheckpoint && senkaData?.is_confirmed_base && (
                    <div className="senka-checkpoint-notice">
                      ランキング更新を通過しました - ランキング画面で戦果を再確認してください
                    </div>
                  )}

                  {/* Resources section */}
                  <div className="resources-section">
                    <div className="resource-row">
                      <div className="res-item">
                        <span className="res-label fuel-color">燃</span>
                        <span className="res-value">{(portData.fuel ?? 0).toLocaleString()}</span>
                      </div>
                      <div className="res-item">
                        <span className="res-label ammo-color">弾</span>
                        <span className="res-value">{(portData.ammo ?? 0).toLocaleString()}</span>
                      </div>
                      <div className="res-item">
                        <span className="res-label steel-color">鋼</span>
                        <span className="res-value">{(portData.steel ?? 0).toLocaleString()}</span>
                      </div>
                      <div className="res-item">
                        <span className="res-label bauxite-color">ボ</span>
                        <span className="res-value">{(portData.bauxite ?? 0).toLocaleString()}</span>
                      </div>
                    </div>
                    <div className="resource-row">
                      <div className="res-item">
                        <span className="res-label repair-color">修</span>
                        <span className="res-value">{(portData.instant_repair ?? 0).toLocaleString()}</span>
                      </div>
                      <div className="res-item">
                        <span className="res-label build-color">建</span>
                        <span className="res-value">{(portData.instant_build ?? 0).toLocaleString()}</span>
                      </div>
                      <div className="res-item">
                        <span className="res-label dev-color">開</span>
                        <span className="res-value">{(portData.dev_material ?? 0).toLocaleString()}</span>
                      </div>
                      <div className="res-item">
                        <span className="res-label improve-color">改</span>
                        <span className="res-value">{(portData.improvement_material ?? 0).toLocaleString()}</span>
                      </div>
                    </div>
                  </div>

                  {/* Repair docks inline */}
                  <div className="ndock-section">
                    <span className="ndock-label">入渠</span>
                    {(portData.ndock ?? []).map((dock) => (
                      <div key={dock.id} className="ndock-item">
                        <span className="ndock-id">#{dock.id}</span>
                        {dock.state === 0 ? (
                          <span className="ndock-empty">-</span>
                        ) : dock.state === -1 ? (
                          <span className="ndock-locked">封鎖</span>
                        ) : (
                          <>
                            <span className="ndock-ship">
                              {dock.ship_name ?? `Ship#${dock.ship_id ?? "?"}`}
                            </span>
                            <span className="ndock-time">
                              {dock.complete_time > 0
                                ? formatRemaining(dock.complete_time, now)
                                : ""}
                            </span>
                          </>
                        )}
                      </div>
                    ))}
                  </div>
                </div>

                {/* Fleet panels */}
                <div className="fleets-area">
                  {(portData.fleets ?? []).map((fleet, i) => (
                    <FleetPanel key={fleet.id} fleet={fleet} now={now} fleetIndex={i} expeditions={expeditions} portDataVersion={portDataVersion} sortieQuests={sortieQuests} mapRecommendations={mapRecommendations} activeQuests={activeQuests} questProgress={questProgress} weaponIconSheet={weaponIconSheet} />
                  ))}
                </div>
              </>
            ) : (
              <div className="no-data-panel">
                {caInstalled === false
                  ? 'CA証明書をインストールしてください。「Install CA Cert」を押すとmacOSのパスワード入力を求められます。'
                  : gameOpen
                    ? "ゲームウィンドウを開きました。APIデータ待機中..."
                    : '「Open Game」でゲームを起動してください。'}
              </div>
            )}

            {/* API Log - collapsible, hideable via settings */}
            {showApiLog && <div className={`api-log-panel ${logCollapsed ? "collapsed" : ""}`}>
              <div
                className="api-log-header"
                onClick={() => setLogCollapsed(!logCollapsed)}
              >
                <span>
                  {logCollapsed ? "▸" : "▾"} API Log ({apiLog.length})
                </span>
              </div>
              {!logCollapsed && (
                <div className="api-log" ref={logRef}>
                  {apiLog.length === 0 ? (
                    <div className="no-data">API通信なし</div>
                  ) : (
                    apiLog.map((entry, i) => (
                      <div key={i} className="api-log-entry">
                        <span className="time">{entry.time}</span>
                        <span className="endpoint">{entry.endpoint}</span>
                      </div>
                    ))
                  )}
                </div>
              )}
            </div>}
          </>
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
          <div className="options-tab">
            <div className="options-section">
              <div className="options-section-title">表示</div>
              <div className="options-row">
                <label className="options-label">UIサイズ</label>
                <input
                  type="range"
                  min={50}
                  max={200}
                  step={5}
                  value={uiZoom}
                  onChange={(e) => {
                    const v = Number(e.target.value);
                    setUiZoom(v);
                    localStorage.setItem("ui-zoom", String(v));
                  }}
                  className="options-slider"
                />
                <span className="options-value">{uiZoom}%</span>
                <button
                  className="options-reset-btn"
                  onClick={() => {
                    setUiZoom(135);
                    localStorage.setItem("ui-zoom", "135");
                  }}
                >
                  リセット
                </button>
              </div>
            </div>

            <div className="options-section">
              <div className="options-section-title">Google Drive 同期</div>
              {!driveStatus.authenticated ? (
                <div className="drive-sync-content">
                  <p className="drive-sync-desc">
                    Google Driveと同期して、複数端末間でデータを共有できます。
                  </p>
                  {driveStatus.error && (
                    <p className="drive-sync-error">{driveStatus.error}</p>
                  )}
                  <button
                    className="drive-sync-btn"
                    disabled={driveLoading}
                    onClick={async () => {
                      setDriveLoading(true);
                      try {
                        await invoke("drive_login");
                        const status = await invoke<typeof driveStatus>("get_drive_status");
                        setDriveStatus(status);
                      } catch (e) {
                        console.error("Drive login failed:", e);
                        setDriveStatus((prev) => ({
                          ...prev,
                          error: String(e),
                        }));
                      } finally {
                        setDriveLoading(false);
                      }
                    }}
                  >
                    {driveLoading ? "認証中" : "Googleでログイン"}
                  </button>
                </div>
              ) : (
                <div className="drive-sync-content">
                  <div className="drive-sync-row">
                    <span className="drive-sync-email">{driveStatus.email || "認証済み"}</span>
                    <span className={`drive-sync-status-value ${driveStatus.syncing ? "syncing" : driveStatus.error ? "error" : ""}`}>
                      {driveStatus.syncing ? "同期中" : driveStatus.error ? `エラー: ${driveStatus.error}` : "変更待機中"}
                    </span>
                    <button
                      className="drive-sync-btn drive-sync-btn-sm"
                      disabled={driveLoading || driveStatus.syncing}
                      onClick={async () => {
                        setDriveLoading(true);
                        try {
                          await invoke("drive_force_sync");
                        } catch (e) {
                          console.error("Force sync failed:", e);
                        } finally {
                          setDriveLoading(false);
                        }
                      }}
                    >
                      手動同期
                    </button>
                    <button
                      className="drive-sync-btn drive-sync-btn-sm"
                      onClick={async () => {
                        setDriveLoading(true);
                        try {
                          await invoke("drive_logout");
                          setDriveStatus({ authenticated: false, syncing: false });
                        } catch (e) {
                          console.error("Drive logout failed:", e);
                        } finally {
                          setDriveLoading(false);
                        }
                      }}
                      disabled={driveLoading}
                    >
                      ログアウト
                    </button>
                  </div>
                  {driveStatus.last_sync && (
                    <div className="drive-sync-status-row">
                      <span className="drive-sync-status-label">最終同期:</span>
                      <span className="drive-sync-status-value">{driveStatus.last_sync}</span>
                    </div>
                  )}
                </div>
              )}
            </div>

            <div className="options-section">
              <div className="options-section-title">開発者オプション</div>
              <div className="options-row">
                <label className="options-label">APIログ表示</label>
                <label className="options-toggle">
                  <input
                    type="checkbox"
                    checked={showApiLog}
                    onChange={(e) => {
                      setShowApiLog(e.target.checked);
                      localStorage.setItem("show-api-log", String(e.target.checked));
                    }}
                  />
                  <span className="options-toggle-text">母港にAPIログを表示</span>
                </label>
              </div>
              <div className="options-row">
                <label className="options-label">全ログ保存</label>
                <label className="options-toggle">
                  <input
                    type="checkbox"
                    checked={rawApiEnabled}
                    onChange={async (e) => {
                      const enabled = e.target.checked;
                      setRawApiEnabled(enabled);
                      localStorage.setItem("raw-api-enabled", String(enabled));
                      await invoke("set_raw_api_enabled", { enabled });
                    }}
                  />
                  <span className="options-toggle-text">全APIレスポンスをディスクに保存</span>
                </label>
              </div>
            </div>

            <div className="options-section">
              <div className="options-section-title">データクリア</div>
              <div className="options-clear-list">
                <div className="options-clear-row">
                  <span className="options-clear-label">改修履歴</span>
                  <span className="options-clear-desc">改修した装備の記録</span>
                  <button
                    className="options-clear-btn"
                    onClick={async () => {
                      if (!confirm("改修履歴をクリアしますか？")) return;
                      await invoke("clear_improved_history");
                    }}
                  >
                    クリア
                  </button>
                </div>
                <div className="options-clear-row">
                  <span className="options-clear-label">戦闘ログ</span>
                  <span className="options-clear-desc">出撃・戦闘の記録</span>
                  <button
                    className="options-clear-btn"
                    onClick={async () => {
                      if (!confirm("戦闘ログをクリアしますか？")) return;
                      await invoke("clear_battle_logs");
                      setBattleLogs([]);
                      setBattleLogsTotal(0);
                    }}
                  >
                    クリア
                  </button>
                </div>
                <div className="options-clear-row">
                  <span className="options-clear-label">生APIダンプ</span>
                  <span className="options-clear-desc">傍受したAPIの生データ</span>
                  <button
                    className="options-clear-btn"
                    onClick={async () => {
                      if (!confirm("生APIダンプをクリアしますか？")) return;
                      await invoke("clear_raw_api");
                    }}
                  >
                    クリア
                  </button>
                </div>
                <div className="options-clear-row">
                  <span className="options-clear-label">任務進捗</span>
                  <span className="options-clear-desc">任務の進捗データ</span>
                  <button
                    className="options-clear-btn"
                    onClick={async () => {
                      if (!confirm("任務進捗をクリアしますか？")) return;
                      await invoke("clear_quest_progress");
                    }}
                  >
                    クリア
                  </button>
                </div>
                <div className="options-clear-row">
                  <span className="options-clear-label">ブラウザキャッシュ</span>
                  <span className="options-clear-desc">WebViewのHTTP/GPUキャッシュ</span>
                  <button
                    className="options-clear-btn"
                    onClick={async () => {
                      if (!confirm("ブラウザキャッシュを削除しますか？（ゲーム画面を閉じてから実行）")) return;
                      try {
                        const msg = await invoke<string>("clear_browser_cache");
                        alert(msg);
                      } catch (e) {
                        alert(`エラー: ${e}`);
                      }
                    }}
                  >
                    クリア
                  </button>
                </div>
                <div className="options-clear-row">
                  <span className="options-clear-label">保存リソース</span>
                  <span className="options-clear-desc">プロキシ経由で保存したマップ画像等</span>
                  <button
                    className="options-clear-btn"
                    onClick={async () => {
                      if (!confirm("保存リソースを削除しますか？次回出撃時に再取得されます。")) return;
                      try {
                        const msg = await invoke<string>("clear_resource_cache");
                        alert(msg);
                      } catch (e) {
                        alert(`エラー: ${e}`);
                      }
                    }}
                  >
                    クリア
                  </button>
                </div>
                <div className="options-clear-row">
                  <span className="options-clear-label">Cookie</span>
                  <span className="options-clear-desc">DMM保存Cookie（再ログイン必要）</span>
                  <button
                    className="options-clear-btn options-clear-btn-danger"
                    onClick={async () => {
                      if (!confirm("保存済みCookieをクリアしますか？次回起動時に再ログインが必要です。")) return;
                      await invoke("clear_cookies");
                    }}
                  >
                    クリア
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
