import { useMemo, useState } from "react";
import type { AirBase, AirBasePlane, AirBaseAttackWave } from "../../types";
import "./AirBaseTab.css";

interface AirBaseTabProps {
  bases: AirBase[];
}

// Game enum (per docs/KNOWLEDGE/battle.md): 0=均衡, 1=確保, 2=優勢, 3=劣勢, 4=喪失
const SEIKU_LABEL: Record<number, { text: string; className: string }> = {
  0: { text: "均衡", className: "seiku-kinkou" },
  1: { text: "確保", className: "seiku-kakuho" },
  2: { text: "優勢", className: "seiku-yuusei" },
  3: { text: "劣勢", className: "seiku-ressei" },
  4: { text: "喪失", className: "seiku-soshitsu" },
};

const ACTION_KIND_LABEL: Record<number, string> = {
  0: "待機",
  1: "出撃",
  2: "防空",
  3: "退避",
  4: "休息",
};

const ACTION_KIND_CLASS: Record<number, string> = {
  0: "idle",
  1: "attack",
  2: "defense",
  3: "retreat",
  4: "rest",
};

// State values per ElectronicObserver/Data/BaseAirCorpsSquadron.cs:
//   0 = 未配属, 1 = 配属済み, 2 = 配置転換中
const PLANE_STATE_LABEL: Record<number, string> = {
  0: "未配備",
  1: "配備済",
  2: "配置転換中",
};

function areaLabel(areaId: number): string {
  if (areaId >= 1 && areaId <= 7) return `海域${areaId}`;
  if (areaId >= 21) return `イベント海域 (${areaId})`;
  return `海域 ${areaId}`;
}

// Cond values are an enum (NOT 0-100 fatigue). Per ElectronicObserver/Core/Types/AirBaseCondition.cs:
//   0 = キラキラ (Sparkled, best), 1 = 通常, 2 = 橙疲労, 3 = 赤疲労
const COND_LABEL: Record<number, { text: string; className: string }> = {
  0: { text: "✨ キラ", className: "cond-sparkle" },
  1: { text: "通常", className: "cond-normal" },
  2: { text: "🟠 橙疲労", className: "cond-tired" },
  3: { text: "🔴 赤疲労", className: "cond-very-tired" },
};

function condLabel(cond: number): { text: string; className: string } {
  return COND_LABEL[cond] ?? { text: `cond=${cond}`, className: "cond-normal" };
}

function PlaneCard({ plane }: { plane: AirBasePlane }) {
  const isEquipped = plane.state !== 0 && plane.slotid > 0;
  const stateLabel = PLANE_STATE_LABEL[plane.state] ?? `状態${plane.state}`;

  if (!isEquipped) {
    return (
      <div className="airbase-plane empty">
        <div className="airbase-plane-squadron">隊{plane.squadron_id}</div>
        <div className="airbase-plane-empty">未配備</div>
      </div>
    );
  }

  const remainRatio = plane.max_count > 0 ? plane.count / plane.max_count : 0;
  let countClass = "count-full";
  if (remainRatio <= 0) countClass = "count-zero";
  else if (remainRatio < 0.5) countClass = "count-low";
  else if (remainRatio < 1) countClass = "count-mid";

  const cond = condLabel(plane.cond);
  const star = plane.level && plane.level > 0 ? `★${plane.level}` : null;
  const alv = plane.alv && plane.alv > 0 ? `>>${plane.alv}` : null;
  const isRelocating = plane.state === 2;

  return (
    <div className={`airbase-plane state-${plane.state}`}>
      <div className="airbase-plane-squadron">隊{plane.squadron_id}</div>
      <div className="airbase-plane-name" title={plane.name ?? ""}>
        {plane.name ?? `装備#${plane.slotitem_id ?? "?"}`}
      </div>
      <div className="airbase-plane-meta">
        {star && <span className="airbase-plane-star">{star}</span>}
        {alv && <span className="airbase-plane-alv">{alv}</span>}
      </div>
      <div className="airbase-plane-stats">
        <span className={`airbase-plane-count ${countClass}`}>
          {plane.count}/{plane.max_count}
        </span>
        <span className={`airbase-plane-cond ${cond.className}`}>{cond.text}</span>
      </div>
      {isRelocating && (
        <div className="airbase-plane-warning">{stateLabel}</div>
      )}
    </div>
  );
}

function AttackWaveRow({ wave }: { wave: AirBaseAttackWave }) {
  const seiku = SEIKU_LABEL[wave.disp_seiku] ?? {
    text: `?${wave.disp_seiku}`,
    className: "seiku-unknown",
  };
  const totalLost = wave.stage1_lost + wave.stage2_lost;
  const perSquadron = wave.per_squadron_lost
    .map((n, i) => `隊${i + 1}:-${n}`)
    .join(" ");
  return (
    <tr>
      <td className="airbase-wave-num">{wave.wave}波</td>
      <td className={`airbase-wave-seiku ${seiku.className}`}>{seiku.text}</td>
      <td className="airbase-wave-lost">
        <span className="airbase-wave-total">-{totalLost}</span>
        <span className="airbase-wave-stage">
          (S1:{wave.stage1_lost} / S2:{wave.stage2_lost})
        </span>
      </td>
      <td className="airbase-wave-per" title={perSquadron}>{perSquadron}</td>
      <td className="airbase-wave-edam">
        {wave.edam_total > 0 ? `→敵 ${wave.edam_total}` : "-"}
      </td>
    </tr>
  );
}

function BaseCard({ base }: { base: AirBase }) {
  const action = ACTION_KIND_LABEL[base.action_kind] ?? `動作${base.action_kind}`;
  const actionClass = ACTION_KIND_CLASS[base.action_kind] ?? "idle";
  const distance =
    base.distance.bonus > 0
      ? `${base.distance.base}+${base.distance.bonus}`
      : `${base.distance.base}`;

  // Always render 4 squadrons; backfill missing ones as empty.
  const squadrons: AirBasePlane[] = [];
  for (let id = 1; id <= 4; id++) {
    const found = base.planes.find((p) => p.squadron_id === id);
    squadrons.push(
      found ?? {
        squadron_id: id,
        slotid: 0,
        state: 0,
        count: 0,
        max_count: 0,
        cond: 0,
      },
    );
  }

  const attacks = base.recent_attacks;

  return (
    <div className="airbase-base">
      <div className="airbase-base-header">
        <span className="airbase-base-rid">第{base.rid}</span>
        <span className="airbase-base-name">{base.name}</span>
        <span className={`airbase-base-action action-${actionClass}`}>{action}</span>
        <span className="airbase-base-distance" title="戦闘行動半径(基本+ボーナス)">
          距離 {distance}
        </span>
      </div>
      <div className="airbase-planes">
        {squadrons.map((p) => (
          <PlaneCard key={p.squadron_id} plane={p} />
        ))}
      </div>
      {attacks.length > 0 && (
        <div className="airbase-attacks">
          <div className="airbase-attacks-title">📊 最新出撃結果 ({attacks.length}波)</div>
          <table className="airbase-attacks-table">
            <thead>
              <tr>
                <th>波</th>
                <th>制空</th>
                <th>機ロス (合計)</th>
                <th>隊別</th>
                <th>与ダメ</th>
              </tr>
            </thead>
            <tbody>
              {attacks.map((w, i) => (
                <AttackWaveRow key={i} wave={w} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export function AirBaseTab({ bases }: AirBaseTabProps) {
  // Filter to areas that have at least one equipped squadron — empty area-7
  // entries from mapinfo would otherwise be confusing noise.
  const populatedAreas = useMemo(() => {
    const areas = new Map<number, AirBase[]>();
    for (const base of bases) {
      const hasPlanes = base.planes.some((p) => p.state !== 0 && p.slotid > 0);
      if (!hasPlanes && base.action_kind === 0) continue;
      if (!areas.has(base.area_id)) areas.set(base.area_id, []);
      areas.get(base.area_id)!.push(base);
    }
    for (const list of areas.values()) {
      list.sort((a, b) => a.rid - b.rid);
    }
    return areas;
  }, [bases]);

  const areaIds = useMemo(
    () => Array.from(populatedAreas.keys()).sort((a, b) => a - b),
    [populatedAreas],
  );

  const [selectedArea, setSelectedArea] = useState<number | null>(null);
  const activeArea =
    selectedArea !== null && populatedAreas.has(selectedArea)
      ? selectedArea
      : areaIds[0] ?? null;
  const activeBases =
    activeArea !== null ? populatedAreas.get(activeArea) ?? [] : [];

  if (bases.length === 0) {
    return (
      <div className="airbase-empty">
        基地航空隊データがありません。
        <br />
        ゲーム内で出撃 (海域選択) 画面を開いて mapinfo を取得してください。
      </div>
    );
  }

  if (areaIds.length === 0) {
    return (
      <div className="airbase-empty">
        現在、いずれの基地にも航空機が配備されていません。
      </div>
    );
  }

  return (
    <div className="airbase-tab">
      {areaIds.length > 1 && (
        <div className="airbase-area-tabs">
          {areaIds.map((area) => (
            <button
              key={area}
              className={`airbase-area-tab ${
                activeArea === area ? "active" : ""
              }`}
              onClick={() => setSelectedArea(area)}
            >
              {areaLabel(area)}
            </button>
          ))}
        </div>
      )}
      <div className="airbase-list">
        {activeBases.map((base) => (
          <BaseCard key={`${base.area_id}-${base.rid}`} base={base} />
        ))}
      </div>
    </div>
  );
}
