import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { STORAGE_KEYS } from "../../constants";
import type { MapRecommendationDef, MapRecommendationCheckResult } from "../../types";
import "./MapRecommendationChecker.css";

export function MapRecommendationChecker({
  mapRecommendations,
  portDataVersion,
}: {
  mapRecommendations: MapRecommendationDef[];
  portDataVersion: number;
}) {
  const storageKey = STORAGE_KEYS.MAP_REC_AREA;
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

  // Keep a ref so the effect below always calls the latest doCheck
  const doCheckRef = useRef(doCheck);
  doCheckRef.current = doCheck;

  // Auto-check on mount and when port data updates
  useEffect(() => {
    if (selectedArea != null && mapRecommendations.length > 0) {
      doCheckRef.current(selectedArea);
    }
  }, [mapRecommendations.length, portDataVersion, selectedArea]);

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
