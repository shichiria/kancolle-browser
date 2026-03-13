import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { STORAGE_KEYS } from "../../constants";
import type { ImprovementListResponse } from "../../types";
import "./ImprovementTab.css";

const DAY_NAMES = ["日", "月", "火", "水", "木", "金", "土"];

export function ImprovementTab({ portDataVersion }: { portDataVersion: number }) {
  const [data, setData] = useState<ImprovementListResponse | null>(null);
  const [typeFilters, setTypeFilters] = useState<Set<number>>(() => {
    const saved = localStorage.getItem(STORAGE_KEYS.IMPROVEMENT_TYPE_FILTERS);
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
        STORAGE_KEYS.IMPROVEMENT_TYPE_FILTERS,
        JSON.stringify([...next])
      );
      return next;
    });
  };

  const clearFilters = () => {
    setTypeFilters(new Set());
    localStorage.removeItem(STORAGE_KEYS.IMPROVEMENT_TYPE_FILTERS);
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
            {item.consumed_equips.length > 0 && (
              <span className="imp-consumed">
                {item.consumed_equips.map((ce) => (
                  <span
                    key={ce.eq_id}
                    className={`imp-consumed-item${ce.owned === 0 ? " imp-consumed-zero" : ""}`}
                    title={`${ce.name}\n★0-5: ×${ce.counts[0]}  ★6-9: ×${ce.counts[1]}  更新: ×${ce.counts[2]}\n所持(ロックなし): ${ce.owned}`}
                  >
                    {ce.name} ×{ce.counts[0]}/{ce.counts[1]}/{ce.counts[2]}
                    <span className={`imp-owned${ce.owned === 0 ? " imp-owned-zero" : ""}`}>
                      ({ce.owned})
                    </span>
                  </span>
                ))}
              </span>
            )}
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
