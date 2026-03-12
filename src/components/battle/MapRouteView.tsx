import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getNodeLabel, CELL_COLORS } from "../../utils/map";
import type { BattleNode, MapSpot, MapInfo, AtlasFrame, MapSprites } from "../../types";

/** Display the map with the sortie route overlaid */
export function MapRouteView({ mapDisplay, nodes, onCellClick, mapMaxWidth }: { mapDisplay: string; nodes: BattleNode[]; onCellClick?: (cellNo: number) => void; mapMaxWidth?: number }) {
  const [mapInfo, setMapInfo] = useState<MapInfo | null>(null);
  const [sprites, setSprites] = useState<MapSprites>({ routes: [] });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const parts = mapDisplay.split("-");
    const area = (parseInt(parts[0], 10) || 0).toString().padStart(3, "0");
    const map = (parseInt(parts[1], 10) || 0).toString().padStart(2, "0");
    const infoPath = `kcs2/resources/map/${area}/${map}_info.json`;
    const atlasPath = `kcs2/resources/map/${area}/${map}_image.json`;

    let cancelled = false;
    (async () => {
      try {
        // Load info and atlas in parallel
        const [infoStr, atlasStr] = await Promise.all([
          invoke<string>("get_cached_resource", { path: infoPath }),
          invoke<string>("get_cached_resource", { path: atlasPath }),
        ]);
        if (cancelled) return;
        if (!infoStr) return;

        const info = JSON.parse(infoStr) as MapInfo;
        setMapInfo(info);

        // Parse atlas for frame dimensions
        let atlas: { frames: Record<string, AtlasFrame> } | null = null;
        if (atlasStr) {
          try { atlas = JSON.parse(atlasStr); } catch { /* ignore */ }
        }

        // Load all sprites in parallel
        const prefix = `map${area}${map}_`;
        const spritePromises: Promise<void>[] = [];
        const result: MapSprites = { routes: [] };

        // Background terrain
        if (info.bg?.[0]) {
          spritePromises.push(
            invoke<string>("get_map_sprite", { mapDisplay, frameName: info.bg[0] })
              .then((uri) => { if (uri) result.bg = uri; })
              .catch(() => { })
          );
        }

        // Point overlay (cell markers)
        if (info.bg?.[1]) {
          spritePromises.push(
            invoke<string>("get_map_sprite", { mapDisplay, frameName: info.bg[1] })
              .then((uri) => { if (uri) result.point = uri; })
              .catch(() => { })
          );
        }

        // Route sprites - each route_N connects to spots[N] using line offset
        const spotsWithLine = info.spots.filter((s) => s.line);

        // Build a lookup: spot no -> MapSpot
        const spotMap = new Map<number, MapSpot>();
        for (const spot of info.spots) {
          spotMap.set(spot.no, spot);
        }

        // Resolve visited cells to coordinates (in order)
        const visitedCells = nodes
          .map((node) => {
            const spot = spotMap.get(node.cell_no);
            if (!spot) return null;
            const label = getNodeLabel(mapDisplay, node.cell_no);
            return { ...spot, event_kind: node.event_kind, event_id: node.event_id, cell_no: node.cell_no, label };
          })
          .filter((c) => c != null);

        for (let i = 0; i < spotsWithLine.length; i++) {
          const spot = spotsWithLine[i];
          const routeN = i + 1;
          let frameName: string;
          if (spot.line?.img) {
            frameName = spot.line.img;
          } else {
            frameName = `route_${routeN}`;
          }
          const fullFrameName = `${prefix}${frameName}`;
          const frame = atlas?.frames[fullFrameName]?.frame;

          if (frame && spot.line) {
            const rx = spot.x + spot.line.x;
            const ry = spot.y + spot.line.y;
            const rw = frame.w;
            const rh = frame.h;
            const tintCyan = visitedCells.some((c) => c.cell_no === spot.no);

            spritePromises.push(
              invoke<string>("get_map_sprite", {
                mapDisplay,
                frameName,
                tintCyan: false,
                routeIdx: i + 1,
                spotNo: spot.no
              })
                .then((uri) => {
                  if (!uri) return;
                  result.routes.push({ uri, x: rx, y: ry, w: rw, h: rh, spotNo: spot.no, isVisited: tintCyan });
                })
                .catch(() => { })
            );
          }
        }

        await Promise.all(spritePromises);
        if (!cancelled) setSprites(result);
      } catch {
        // Cache miss
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [mapDisplay, nodes]);

  if (loading) return null;
  if (!mapInfo) {
    return (
      <div className="map-route-no-data">
        マップデータ未取得（該当マップに出撃するとキャッシュされます）
      </div>
    );
  }

  const hasBg = !!sprites.bg;

  const spotMap = new Map<number, MapSpot>();
  for (const spot of mapInfo.spots) {
    spotMap.set(spot.no, spot);
  }

  const renderedVisitedCells = nodes
    .map((node) => {
      const spot = spotMap.get(node.cell_no);
      if (!spot) return null;
      const label = getNodeLabel(mapDisplay, node.cell_no);
      return { ...spot, event_kind: node.event_kind, event_id: node.event_id, cell_no: node.cell_no, label, hasBattle: node.battle != null };
    })
    .filter((c) => c != null);

  if (renderedVisitedCells.length === 0 && !hasBg) {
    return (
      <div className="map-route-no-data">
        ルート座標データなし
      </div>
    );
  }

  // viewBox
  let vbX: number, vbY: number, vbW: number, vbH: number;
  if (hasBg) {
    vbX = 0; vbY = 0; vbW = 1200; vbH = 720;
  } else {
    const pad = 60;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const s of mapInfo.spots) {
      if (s.x < minX) minX = s.x;
      if (s.y < minY) minY = s.y;
      if (s.x > maxX) maxX = s.x;
      if (s.y > maxY) maxY = s.y;
    }
    vbX = minX - pad; vbY = minY - pad;
    vbW = maxX - minX + pad * 2; vbH = maxY - minY + pad * 2;
  }

  const startSpot = mapInfo.spots.find((s) => !s.line);

  const connectionLines = !hasBg ? mapInfo.spots
    .filter((s) => s.line)
    .map((s) => ({
      x1: s.x + s.line!.x, y1: s.y + s.line!.y,
      x2: s.x, y2: s.y,
      spotNo: s.no
    })) : [];

  return (
    <div className="map-route-container">
      <div className="map-route-wrapper" style={{ position: "relative", width: "100%", maxWidth: mapMaxWidth != null ? `${mapMaxWidth}px` : "600px", margin: "0 auto" }}>
        {sprites.bg && (
          <img src={sprites.bg} alt="" style={{ width: "100%", display: "block" }} draggable={false} />
        )}
        {!sprites.bg && (
          <div style={{ width: "100%", paddingBottom: `${(vbH / vbW) * 100}%` }} />
        )}

        {sprites.point && (
          <img src={sprites.point} alt="" draggable={false}
            style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", pointerEvents: "none", zIndex: 1 }} />
        )}

        {sprites.routes.map((r, i) => (
          <img
            key={`route-${r.spotNo}-${i}`}
            src={r.uri}
            alt=""
            draggable={false}
            style={{
              position: "absolute",
              left: `${(r.x / 1200) * 100}%`,
              top: `${(r.y / 720) * 100}%`,
              width: `${(r.w / 1200) * 100}%`,
              height: `${(r.h / 720) * 100}%`,
              pointerEvents: "none",
              zIndex: 2,
              filter: r.isVisited ? "none" : "grayscale(100%) opacity(40%)"
            }}
          />
        ))}

        <svg style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", zIndex: 3 }}
          viewBox={`${vbX} ${vbY} ${vbW} ${vbH}`} preserveAspectRatio="xMidYMid meet">
          {connectionLines.map((seg, i) => (
            <line key={`conn-${i}`} x1={seg.x1} y1={seg.y1} x2={seg.x2} y2={seg.y2}
              stroke="rgba(64,192,216,0.4)" strokeWidth="2" strokeDasharray="6 4" />
          ))}
          {startSpot && (
            <g>
              <circle cx={startSpot.x} cy={startSpot.y} r={14} fill="none" stroke="rgba(76,175,80,0.8)" strokeWidth="3" strokeDasharray="4 3" />
              <text x={startSpot.x} y={startSpot.y + 1} textAnchor="middle" dominantBaseline="central"
                fill="#4caf50" fontSize="11" fontWeight="bold" style={{ pointerEvents: "none" }}>
                出撃
              </text>
            </g>
          )}
          {renderedVisitedCells.map((cell, i) => {
            if (!cell) return null;
            const isBoss = cell.event_id === 5 || cell.event_kind === 5;
            const isNonBattle = !cell.hasBattle;
            const color = isNonBattle && (cell.event_kind === 4 || cell.event_kind === 5) ? "#b0bec5" : (CELL_COLORS[cell.event_kind] ?? "#78909c");
            const r = isBoss ? 18 : 14;
            const label = cell.label ?? String(cell.cell_no);
            return (
              <g key={`cell-${i}`} style={{ cursor: (onCellClick && !isNonBattle) ? "pointer" : "default" }}
                onClick={() => { if (!isNonBattle) onCellClick?.(cell.cell_no); }}>
                <circle cx={cell.x} cy={cell.y} r={r} fill={color} stroke="#fff" strokeWidth="1.5" />
                <text x={cell.x} y={cell.y} textAnchor="middle" dominantBaseline="central"
                  fill="#fff" fontSize={label.length > 2 ? "9" : isBoss ? "14" : "11"} fontWeight="bold"
                  style={{ pointerEvents: "none" }}>
                  {label}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}
