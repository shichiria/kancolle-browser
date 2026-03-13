import { getNodeLabel } from "../../utils/map";
import { BattleHpBar } from "../common";
import {
  FORMATION_NAMES, ENGAGEMENT_NAMES, RANK_COLORS,
  EVENT_LABELS, EVENT_ID_LABELS, AIR_SUPERIORITY_LABELS,
} from "./constants";
import type { BattleNode, SortieShip } from "../../types";

/** Detailed view of a single battle node */
export function BattleNodeDetail({
  node,
  ships,
  mapDisplay,
}: {
  node: BattleNode;
  ships: SortieShip[];
  mapDisplay: string;
}) {
  const b = node.battle;
  const isBattle = b != null;
  const eventLabel = (node.event_id != null && EVENT_ID_LABELS[node.event_id]) || EVENT_LABELS[node.event_kind] || `Event ${node.event_kind}`;
  const cellLabel = node.cell_no > 0 ? getNodeLabel(mapDisplay, node.cell_no) : null;

  return (
    <div className={`battle-node-detail ${isBattle ? "has-battle" : "no-battle"}`}>
      {/* Node header */}
      <div className="battle-node-header">
        <span className="battle-node-cell">
          {node.cell_no > 0 ? (cellLabel ? `${cellLabel}` : `${node.cell_no}マス`) : "出撃"}
        </span>
        <span className={`battle-node-event event-${node.event_kind}`}>
          {eventLabel}
        </span>
        {b?.rank && (
          <span
            className="battle-node-rank"
            style={{ color: RANK_COLORS[b.rank] ?? "#888" }}
          >
            {b.rank}
          </span>
        )}
        {b?.enemy_name && (
          <span className="battle-node-enemy">{b.enemy_name}</span>
        )}
        {b?.mvp != null && b.mvp > 0 && (
          <span className="battle-node-mvp">
            MVP: {ships[b.mvp - 1]?.name ?? `#${b.mvp}`}
          </span>
        )}
        {b?.base_exp != null && (
          <span className="battle-node-exp">+{b.base_exp} exp</span>
        )}
        {b?.night_battle && (
          <span className="battle-node-night">夜戦</span>
        )}
        {b?.drop_ship && (
          <span className="battle-node-drop">
            drop: {b.drop_ship}
          </span>
        )}
      </div>

      {/* Battle details */}
      {b && (
        <div className="battle-node-body">
          {/* Formation info */}
          {b.formation && (
            <div className="battle-formation-row">
              <span className="formation-label">陣形:</span>
              <span className="formation-friendly">
                {FORMATION_NAMES[b.formation[0]] ?? `F${b.formation[0]}`}
              </span>
              <span className="formation-vs">vs</span>
              <span className="formation-enemy">
                {FORMATION_NAMES[b.formation[1]] ?? `F${b.formation[1]}`}
              </span>
              <span className="formation-sep">|</span>
              <span className="formation-engagement">
                {ENGAGEMENT_NAMES[b.formation[2]] ?? `E${b.formation[2]}`}
              </span>
            </div>
          )}

          {/* Air battle result */}
          {b.air_battle && (
            <div className="battle-air-row">
              {b.air_battle.air_superiority != null && (
                <span
                  className="air-superiority"
                  style={{ color: AIR_SUPERIORITY_LABELS[b.air_battle.air_superiority]?.color ?? "#888" }}
                >
                  {AIR_SUPERIORITY_LABELS[b.air_battle.air_superiority]?.text ?? `制空${b.air_battle.air_superiority}`}
                </span>
              )}
              {b.air_battle.friendly_plane_count && (
                <span className="air-planes friendly">
                  味方 {b.air_battle.friendly_plane_count[0] - b.air_battle.friendly_plane_count[1]}/{b.air_battle.friendly_plane_count[0]}
                </span>
              )}
              {b.air_battle.enemy_plane_count && (
                <span className="air-planes enemy">
                  敵 {b.air_battle.enemy_plane_count[0] - b.air_battle.enemy_plane_count[1]}/{b.air_battle.enemy_plane_count[0]}
                </span>
              )}
            </div>
          )}

          {/* Fleet HP side-by-side: friendly left, enemy right */}
          <div className="battle-fleets-row">
            {b.friendly_hp.length > 0 && (
              <div className="battle-hp-section battle-hp-friendly">
                <div className="battle-hp-label">味方艦隊</div>
                <div className="battle-hp-list">
                  {b.friendly_hp.map((hp, idx) => (
                    <BattleHpBar
                      key={idx}
                      before={hp.before}
                      after={hp.after}
                      max={hp.max}
                      shipName={ships[idx]?.name}
                    />
                  ))}
                </div>
              </div>
            )}

            {b.enemy_hp.length > 0 && (
              <div className="battle-hp-section battle-hp-enemy">
                <div className="battle-hp-label">{b.enemy_name || "敵艦隊"}</div>
                <div className="battle-hp-list">
                  {b.enemy_hp.map((hp, idx) => {
                    const enemy = b.enemy_ships[idx];
                    const enemyName = enemy?.name ?? (enemy ? `ID:${enemy.ship_id}` : undefined);
                    return (
                      <BattleHpBar
                        key={idx}
                        before={hp.before}
                        after={hp.after}
                        max={hp.max}
                        shipName={enemyName}
                      />
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
