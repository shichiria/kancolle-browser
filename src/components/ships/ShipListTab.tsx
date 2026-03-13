import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { condColor } from "../../utils/color";
import { STORAGE_KEYS } from "../../constants";
import "../common/ListTable.css";
import "./ShipListTab.css";
import type { ShipListItem, ShipListResponse, ShipSortKey } from "../../types";

export function ShipListTab({ portDataVersion }: { portDataVersion: number }) {
  const [data, setData] = useState<ShipListResponse | null>(null);
  const [stypeFilters, setStypeFilters] = useState<Set<number>>(() => {
    const saved = localStorage.getItem(STORAGE_KEYS.SHIP_STYPE_FILTERS);
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
      localStorage.setItem(STORAGE_KEYS.SHIP_STYPE_FILTERS, JSON.stringify([...next]));
      return next;
    });
  };

  const clearStypeFilters = () => {
    setStypeFilters(new Set());
    localStorage.removeItem(STORAGE_KEYS.SHIP_STYPE_FILTERS);
  };

  const displayShips = useMemo(() => {
    if (!data) return [];
    let ships = data.ships;
    if (stypeFilters.size > 0) {
      ships = ships.filter((s) => stypeFilters.has(s.stype));
    }
    const numericKeys: Record<string, (s: ShipListItem) => number> = {
      lv: (s) => s.lv,
      firepower: (s) => s.firepower,
      torpedo: (s) => s.torpedo,
      aa: (s) => s.aa,
      armor: (s) => s.armor,
      asw: (s) => s.asw,
      evasion: (s) => s.evasion,
      los: (s) => s.los,
      luck: (s) => s.luck,
      cond: (s) => s.cond,
    };
    return [...ships].sort((a, b) => {
      let cmp = 0;
      if (sortKey === "name") {
        cmp = a.name.localeCompare(b.name);
      } else if (sortKey === "stype") {
        cmp = a.stype - b.stype || a.stype_name.localeCompare(b.stype_name);
      } else if (sortKey === "locked") {
        cmp = (a.locked ? 1 : 0) - (b.locked ? 1 : 0);
      } else {
        const getter = numericKeys[sortKey];
        cmp = getter(a) - getter(b);
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
