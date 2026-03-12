import { useState, useEffect, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./SortieQuestChecker.css";
import type {
  SortieQuestDef, ActiveQuestDetail, SortieQuestCheckResult,
  QuestProgressSummary, DropdownQuest,
} from "../../types";
import { QuestProgressDisplay } from "./QuestProgressDisplay";

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

export function SortieQuestChecker({
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
