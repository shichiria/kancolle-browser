import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EVENTS, STORAGE_KEYS } from "../../constants";
import eventEquipmentUsage from "../../data/eventEquipmentUsage.json";
import type { ImprovementListResponse } from "../../types";
import "./ImprovementTab.css";

const DAY_NAMES = ["日", "月", "火", "水", "木", "金", "土"];
const EVENT_EQUIPMENT_USAGE = new Map(
  eventEquipmentUsage.items.map((item) => [item.id, item.usage]),
);

export function eventEquipmentUseCount(eqId: number): number {
  return EVENT_EQUIPMENT_USAGE.get(eqId) ?? 0;
}

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

  useEffect(() => {
    const unlisten = listen(EVENTS.IMPROVEMENT_UPDATED, () => {
      invoke<ImprovementListResponse>("get_improvement_list")
        .then(setData)
        .catch(console.error);
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

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
      // 2. Equipment used by the current event formations first
      const aEventUsage = eventEquipmentUseCount(a.eq_id);
      const bEventUsage = eventEquipmentUseCount(b.eq_id);
      if ((aEventUsage > 0) !== (bEventUsage > 0))
        return aEventUsage > 0 ? -1 : 1;
      // 3. Previously improved first
      if (a.previously_improved !== b.previously_improved)
        return a.previously_improved ? -1 : 1;
      // 4. Sort value (primary stat) descending
      if (a.sort_value !== b.sort_value) return b.sort_value - a.sort_value;
      // 5. Name
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
  const eventEquipmentCount = displayItems.filter(
    (item) => eventEquipmentUseCount(item.eq_id) > 0,
  ).length;

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
        <span
          className="improvement-event-count"
          title="2026夏イベントの推奨編成で使われている改修可能装備"
        >
          ◆ イベント使用 {eventEquipmentCount}件
        </span>
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
        <span
          className="imp-cost-legend"
          title="消費装備の必要数を、改修元の★段階別に表示します"
        >
          素材：★0～5 / ★6～9 / 更新（所持はロック込み）
        </span>
      </div>

      {/* Equipment list */}
      <div className="improvement-list">
        {displayItems.map((item) => {
          const eventUsage = eventEquipmentUseCount(item.eq_id);
          const ownedLevelText = item.owned_levels.length > 0
            ? item.owned_levels
              .map(([level, count]) => `${level > 0 ? `★${level}` : "★0"}×${count}`)
              .join("、")
            : "未所持";
          return (
            <div
              key={item.eq_id}
              className={`imp-row ${item.available_today ? "imp-available" : "imp-unavailable"
                } ${item.matches_secretary ? "imp-match" : ""} ${eventUsage > 0 ? "imp-event-used" : ""}`}
            >
              <span className="imp-name" title={item.name}>
                {item.name}
              </span>
              <span className="imp-type">{item.type_name}</span>
              <span
                className={`imp-base-owned${item.owned_count === 0 ? " imp-base-owned-zero" : ""}`}
                title={`改修元装備の総所持数（ロック込み）\n${ownedLevelText}`}
              >
                元装備 {item.owned_count}
              </span>
              <span className="imp-readiness-slot">
                {item.can_improve_now ? (
                  <span
                    className="imp-ready-badge imp-ready-now"
                    title="現在の曜日・2番艦で、改修元の★段階と必要装備が揃っています"
                  >
                    改修可
                  </span>
                ) : item.equipment_ready ? (
                  <span
                    className="imp-ready-badge imp-equipment-ready"
                    title="改修元の★段階と必要装備が揃っています。曜日と担当艦を確認してください"
                  >
                    装備OK
                  </span>
                ) : null}
              </span>
              <span className="imp-event-badge-slot">
                {eventUsage > 0 && (
                  <span
                    className="imp-event-badge"
                    title={`2026夏イベント推奨編成で${eventUsage}枠使用`}
                  >
                    イベント
                  </span>
                )}
              </span>
              {item.consumed_equips.length > 0 && (
                <span className="imp-consumed">
                  {item.consumed_equips.map((ce) => (
                    <span
                      key={ce.eq_id}
                      className={`imp-consumed-item${ce.owned === 0 ? " imp-consumed-zero" : ""}`}
                      title={`${ce.name}\n必要数 ★0-5: ×${ce.counts[0]}  ★6-9: ×${ce.counts[1]}  更新: ×${ce.counts[2]}\n総所持数(ロック込み): ${ce.owned}`}
                    >
                      {ce.name} 必要×{ce.counts[0]}/{ce.counts[1]}/{ce.counts[2]}
                      <span className={`imp-owned${ce.owned === 0 ? " imp-owned-zero" : ""}`}>
                        (所持{ce.owned})
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
                  ? item.today_helpers.map((helper) => (
                    <span
                      key={helper.name}
                      className={`imp-helper${helper.level === null ? " imp-helper-missing" : ""}`}
                      title={helper.level === null ? `${helper.name} (未所持)` : `${helper.name} Lv${helper.level}`}
                    >
                      {helper.name}
                      {helper.level !== null && (
                        <span className="imp-helper-lv">Lv{helper.level}</span>
                      )}
                    </span>
                  ))
                  : "-"}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
