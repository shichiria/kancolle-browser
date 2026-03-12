import { useState } from "react";
import { toDateStr } from "../../utils/format";
import { DateRangePicker } from "../common";
import "./BattleTab.css";
import { RANK_COLORS } from "./constants";
import { BattleDetailView } from "./BattleDetailView";
import type { SortieRecord } from "../../types";

/** Battle tab - full page battle log viewer */
export function BattleTab({
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
