import { hpColor } from "../../utils/color";

export function HpBar({ hp, maxhp }: { hp: number; maxhp: number }) {
  const pct = maxhp > 0 ? (hp / maxhp) * 100 : 100;
  return (
    <div className="hp-bar-container">
      <div
        className="hp-bar-fill"
        style={{ width: `${pct}%`, backgroundColor: hpColor(hp, maxhp) }}
      />
      <span className="hp-bar-text">
        {hp}/{maxhp}
      </span>
    </div>
  );
}
