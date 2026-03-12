import { hpColor } from "../../utils/color";

/** HP bar used in battle detail view (wider, with before->after display) */
export function BattleHpBar({
  before,
  after,
  max,
  shipName,
}: {
  before: number;
  after: number;
  max: number;
  shipName?: string;
}) {
  const afterClamped = Math.max(0, after);
  const pctBefore = max > 0 ? (before / max) * 100 : 100;
  const pctAfter = max > 0 ? (afterClamped / max) * 100 : 100;
  const damage = before - afterClamped;
  const isSunk = afterClamped <= 0;

  return (
    <div className="battle-hp-row">
      {shipName && (
        <span className={`battle-hp-name ${isSunk ? "sunk" : ""}`}>{shipName}</span>
      )}
      <div className="battle-hp-bar-wrap">
        <div className="battle-hp-bar-bg">
          {/* Ghost bar showing pre-battle HP */}
          <div
            className="battle-hp-bar-ghost"
            style={{ width: `${pctBefore}%` }}
          />
          {/* Actual after-battle HP */}
          <div
            className="battle-hp-bar-fill"
            style={{
              width: `${pctAfter}%`,
              backgroundColor: hpColor(afterClamped, max),
            }}
          />
        </div>
        <span className="battle-hp-text">
          {afterClamped}/{max}
          {damage > 0 && <span className="battle-hp-dmg"> (-{damage})</span>}
        </span>
      </div>
    </div>
  );
}
