import { useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SortieQuestDef, QuestProgressSummary } from "../../types";

export function QuestProgressDisplay({
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
