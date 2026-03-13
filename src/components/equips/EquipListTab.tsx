import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatImprovements } from "../../utils/format";
import { STORAGE_KEYS } from "../../constants";
import "../common/ListTable.css";
import "../ships/ShipListTab.css";
import type { EquipListResponse } from "../../types";

export function EquipListTab({ portDataVersion }: { portDataVersion: number }) {
  const [data, setData] = useState<EquipListResponse | null>(null);
  const [typeFilters, setTypeFilters] = useState<Set<number>>(() => {
    const saved = localStorage.getItem(STORAGE_KEYS.EQUIP_TYPE_FILTERS);
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
      localStorage.setItem(STORAGE_KEYS.EQUIP_TYPE_FILTERS, JSON.stringify([...next]));
      return next;
    });
  };

  const clearTypeFilters = () => {
    setTypeFilters(new Set());
    localStorage.removeItem(STORAGE_KEYS.EQUIP_TYPE_FILTERS);
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
