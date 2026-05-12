import { useState, useMemo } from "react";
import "./QuestTab.css";
import type { SortieQuestDef, ActiveQuestDetail, QuestProgressSummary, SortieQuestCondition } from "../../types";

interface QuestTabProps {
  sortieQuests: SortieQuestDef[];
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
}

type ViewMode = "category" | "area";

const RESET_LABELS: Record<string, string> = {
  daily: "デイリー",
  weekly: "ウィークリー",
  monthly: "マンスリー",
  quarterly: "クォータリー",
  yearly: "イヤーリー",
  once: "単発",
  other: "その他"
};

const RESET_ORDER = ["daily", "weekly", "monthly", "quarterly", "yearly", "once", "other"];

function formatCondition(cond: SortieQuestCondition): string {
  switch (cond.type) {
    case "ShipCount": return `艦隊の艦数 ${cond.value}隻以上`;
    case "ShipTypeCount": return `${cond.ship_type} ${cond.value}隻以上`;
    case "FlagshipType": return `旗艦: ${cond.ship_type}`;
    case "ContainsShipName": return `${cond.names.join("・")} ${cond.count}隻以上`;
    case "ContainsShipNameAny": return `${cond.names.join("/")}から ${cond.count}隻以上`;
    case "OnlyShipTypes": return `${cond.desc}のみで編成`;
    case "MaxShipTypeCount": return `${cond.ship_type} ${cond.value}隻以下`;
    case "OrConditions": return `条件: ${cond.desc}`;
    default: return "";
  }
}

function getCombinedRequirements(quests: SortieQuestDef[]): string[] {
  if (quests.length === 0) return [];
  const reqs = new Set<string>();
  const shipCountMap = new Map<string, { value: number, desc: (v: number) => string }>();

  // Use a map to handle Min/Max overlaps easily
  for (const q of quests) {
    if (!q.conditions) continue;
    for (const c of q.conditions) {
      if (c.type === "ShipCount") {
        const k = "ShipCount";
        const val = shipCountMap.get(k)?.value || 0;
        if (c.value > val) shipCountMap.set(k, { value: c.value, desc: (v) => `艦隊の艦数 ${v}隻以上` });
      } else if (c.type === "ShipTypeCount") {
        const k = `Type_${c.ship_type}`;
        const val = shipCountMap.get(k)?.value || 0;
        if (c.value > val) shipCountMap.set(k, { value: c.value, desc: (v) => `${c.ship_type} ${v}隻以上` });
      } else if (c.type === "MaxShipTypeCount") {
        const k = `MaxType_${c.ship_type}`;
        const val = shipCountMap.get(k)?.value ?? 999;
        if (c.value < val) shipCountMap.set(k, { value: c.value, desc: (v) => `${c.ship_type} ${v}隻以下` });
      } else {
        const str = formatCondition(c);
        if (str) reqs.add(str);
      }
    }
  }

  const results: string[] = [];
  for (const item of shipCountMap.values()) {
    results.push(item.desc(item.value));
  }
  for (const str of reqs) {
    results.push(str);
  }
  
  if (results.length === 0 && quests.some(q => !q.no_conditions)) {
    return ["※条件不明または指定なし"];
  }
  return results.length > 0 ? results : ["自由編成"];
}

export function QuestTab({ sortieQuests, activeQuests, questProgress }: QuestTabProps) {
  const [viewMode, setViewMode] = useState<ViewMode>("area");
  const [showAllOnce, setShowAllOnce] = useState(false);
  const [pinnedIds, setPinnedIds] = useState<Set<string>>(() => {
    const saved = localStorage.getItem("pinned_quests");
    return saved ? new Set(JSON.parse(saved)) : new Set();
  });

  const togglePin = (questId: string) => {
    const next = new Set(pinnedIds);
    if (next.has(questId)) next.delete(questId);
    else next.add(questId);
    setPinnedIds(next);
    localStorage.setItem("pinned_quests", JSON.stringify(Array.from(next)));
  };

  const activeApiIds = useMemo(() => new Set(activeQuests.map(q => q.id)), [activeQuests]);

  // Quests filtered and sorted: pinned first, then by reset order
  const processedQuests = useMemo(() => {
    let list = [...sortieQuests];
    
    // Filter out inactive 'once' quests unless 'showAllOnce' is true
    if (!showAllOnce) {
      list = list.filter(q => {
        if (pinnedIds.has(q.quest_id)) return true;
        if (activeApiIds.has(q.id)) return true;
        if (q.reset && q.reset !== "once") return true;
        return false;
      });
    }

    return list.sort((a, b) => {
      const pinA = pinnedIds.has(a.quest_id) ? 1 : 0;
      const pinB = pinnedIds.has(b.quest_id) ? 1 : 0;
      if (pinA !== pinB) return pinB - pinA;

      const orderA = RESET_ORDER.indexOf(a.reset || "other");
      const orderB = RESET_ORDER.indexOf(b.reset || "other");
      if (orderA !== orderB) return orderA - orderB;

      return a.quest_id.localeCompare(b.quest_id);
    });
  }, [sortieQuests, pinnedIds, activeApiIds, showAllOnce]);

  const groupedByReset = useMemo(() => {
    const groups: Record<string, SortieQuestDef[]> = {};
    for (const q of processedQuests) {
      const r = q.reset || "other";
      if (!groups[r]) groups[r] = [];
      groups[r].push(q);
    }
    return groups;
  }, [processedQuests]);

  const areaGroups = useMemo(() => {
    const groups: Record<string, SortieQuestDef[]> = {};
    for (const q of processedQuests) {
      if (!q.area || q.area === "任意") {
        if (!groups["その他"]) groups["その他"] = [];
        groups["その他"].push(q);
        continue;
      }
      const areas = q.area.split("/");
      for (const a of areas) {
        if (!groups[a]) groups[a] = [];
        groups[a].push(q);
      }
    }
    // Sort areas (e.g., 1-1, 1-2, 2-1...)
    const sortedAreas = Object.keys(groups).sort((a, b) => {
      if (a === "その他") return 1;
      if (b === "その他") return -1;
      return a.localeCompare(b, undefined, { numeric: true });
    });

    return sortedAreas.map(area => ({ area, quests: groups[area] }));
  }, [processedQuests]);

  const renderQuestCard = (q: SortieQuestDef) => {
    const isActive = activeApiIds.has(q.id);
    const progress = questProgress.get(q.id);
    const isPinned = pinnedIds.has(q.quest_id);

    return (
      <div key={q.quest_id} className={`quest-card ${isActive ? "active" : ""} ${isPinned ? "pinned" : ""}`}>
        <div className="quest-header">
          <input
            type="checkbox"
            checked={isPinned}
            onChange={() => togglePin(q.quest_id)}
            title="ピン留め"
          />
          <span className="quest-id">{q.quest_id}</span>
          <span className="quest-name">{q.name}</span>
          {isPinned && <span className="pinned-badge">選択中</span>}
          {isActive && <span className="active-badge">遂行中</span>}
        </div>
        <div className="quest-body">
          <div className="quest-meta">
            <span className="quest-area">{q.area}</span>
            <span className="quest-reset">{RESET_LABELS[q.reset || "other"]}</span>
          </div>
          {progress && !q.sub_goals && (
            <div className="quest-progress">
              <div className="progress-bar">
                <div
                  className="progress-fill"
                  style={{ width: `${(progress.count / progress.count_max) * 100}%` }}
                />
              </div>
              <span className="progress-text">{progress.count} / {progress.count_max}</span>
            </div>
          )}
          {q.sub_goals && (
            <div className="quest-subgoals">
              {q.sub_goals.map((sg, idx) => {
                const sgProgress = progress?.area_progress.find(ap => ap.area === sg.name);
                const current = sgProgress?.count ?? 0;
                return (
                  <div key={idx} className="subgoal-item">
                    <span className="subgoal-name">{sg.name}</span>
                    <span className="subgoal-count">{current} / {sg.count}</span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="quest-tab">
      <div className="quest-controls">
        <div className="view-selector">
          <button
            className={viewMode === "category" ? "active" : ""}
            onClick={() => setViewMode("category")}
          >
            カテゴリ別
          </button>
          <button
            className={viewMode === "area" ? "active" : ""}
            onClick={() => setViewMode("area")}
          >
            海域別 (同時達成)
          </button>
        </div>
        <div className="filter-options">
          <label>
            <input
              type="checkbox"
              checked={showAllOnce}
              onChange={(e) => setShowAllOnce(e.target.checked)}
            />
            単発任務をすべて表示
          </label>
        </div>
      </div>

      <div className="quest-list">
        {viewMode === "category" ? (
          RESET_ORDER.map(reset => {
            const quests = groupedByReset[reset];
            if (!quests || quests.length === 0) return null;
            return (
              <section key={reset} className="quest-section">
                <h3>{RESET_LABELS[reset]}</h3>
                <div className="quest-grid">
                  {quests.map(renderQuestCard)}
                </div>
              </section>
            );
          })
        ) : (
          areaGroups.map(({ area, quests }) => {
            const pinnedQuests = quests.filter(q => pinnedIds.has(q.quest_id));
            const requirements = pinnedQuests.length > 0 ? getCombinedRequirements(pinnedQuests) : null;
            
            return (
              <section key={area} className="quest-section">
                <h3>{area}</h3>
                {requirements && (
                  <div className="combined-requirements">
                    <h4>選択中の任務の編成条件</h4>
                    <ul>
                      {requirements.map((req, idx) => (
                        <li key={idx}>{req}</li>
                      ))}
                    </ul>
                  </div>
                )}
                <div className="quest-grid">
                  {quests.map(renderQuestCard)}
                </div>
              </section>
            );
          })
        )}
      </div>
    </div>
  );
}
