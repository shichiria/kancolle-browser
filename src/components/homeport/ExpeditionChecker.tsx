import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatDuration, formatRemaining } from "../../utils/format";
import "./ExpeditionChecker.css";
import type { ExpeditionDef, FleetExpedition, ExpeditionCheckResult } from "../../types";

export function ExpeditionChecker({
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
