import { useState, useRef, useEffect, useCallback } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { buildPredeckUrl } from "../../utils/map";
import "./BattleDetailView.css";
import type { SortieRecord } from "../../types";
import { MapRouteView } from "./MapRouteView";
import { BattleNodeDetail } from "./BattleNodeDetail";

export function BattleDetailView({
  record,
  onBack,
}: {
  record: SortieRecord;
  onBack: () => void;
}) {
  const [highlightCellNo, setHighlightCellNo] = useState<number | null>(null);
  const nodeRefs = useRef<Map<number, HTMLDivElement | null>>(new Map());
  const splitContainerRef = useRef<HTMLDivElement | null>(null);
  const splitTopRef = useRef<HTMLDivElement | null>(null);
  const [mapRatio, setMapRatio] = useState(0.4);
  const [mapMaxWidth, setMapMaxWidth] = useState(600);
  const draggingRef = useRef(false);

  useEffect(() => {
    const el = splitTopRef.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      const h = entries[0].contentRect.height;
      setMapMaxWidth(Math.min(600, Math.max(100, (h - 12) * (1200 / 720))));
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const openAircalc = async () => {
    const url = buildPredeckUrl(record);
    await openUrl(url);
  };

  const handleCellClick = useCallback((cellNo: number) => {
    setHighlightCellNo(cellNo);
    const el = nodeRefs.current.get(cellNo);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
    setTimeout(() => setHighlightCellNo(null), 1500);
  }, []);

  const handleSplitterMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    draggingRef.current = true;
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";

    const onMouseMove = (ev: MouseEvent) => {
      if (!draggingRef.current || !splitContainerRef.current) return;
      const rect = splitContainerRef.current.getBoundingClientRect();
      const y = ev.clientY - rect.top;
      const ratio = Math.min(0.8, Math.max(0.15, y / rect.height));
      setMapRatio(ratio);
    };

    const onMouseUp = () => {
      draggingRef.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }, []);

  return (
    <div className="battle-detail">
      <div className="battle-detail-header">
        <button className="battle-back-btn" onClick={onBack}>
          &larr; 戻る
        </button>
        <span className="battle-detail-map">{record.map_display}</span>
        <span className="battle-detail-fleet">第{record.fleet_id}艦隊</span>
        <span className="battle-detail-time">{record.start_time}</span>
        <button className="aircalc-btn" onClick={openAircalc} title="制空権計算機で開く">
          制空計算
        </button>
      </div>

      <div className="battle-detail-fleet-comp">
        <div className="battle-section-title">出撃艦隊</div>
        <div className="battle-fleet-ships">
          {record.ships.map((ship, i) => (
            <span key={i} className="battle-fleet-ship">
              {ship.name} <span className="battle-ship-lv">Lv{ship.lv}</span>
            </span>
          ))}
        </div>
      </div>

      <div className="battle-split-container" ref={splitContainerRef}>
        <div className="battle-split-top" ref={splitTopRef} style={{ flex: `0 0 ${mapRatio * 100}%` }}>
          <MapRouteView mapDisplay={record.map_display} nodes={record.nodes} onCellClick={handleCellClick} mapMaxWidth={mapMaxWidth} />
        </div>
        <div className="battle-splitter" onMouseDown={handleSplitterMouseDown}>
          <div className="battle-splitter-handle" />
        </div>
        <div className="battle-split-bottom">
          {record.nodes.filter((n) => n.battle != null).map((node) => (
            <div
              key={node.cell_no}
              ref={(el) => { nodeRefs.current.set(node.cell_no, el); }}
              className={highlightCellNo === node.cell_no ? "node-highlight" : ""}
            >
              <BattleNodeDetail node={node} ships={record.ships} mapDisplay={record.map_display} />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
