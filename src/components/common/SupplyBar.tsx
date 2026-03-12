export function SupplyBar({ rate, type }: { rate: number; type: "fuel" | "ammo" }) {
  const color = type === "fuel" ? "#4caf50" : "#795548";
  const dimColor = rate < 100 ? 0.5 : 1;
  return (
    <div className="supply-bar-container">
      <div
        className="supply-bar-fill"
        style={{
          width: `${rate}%`,
          backgroundColor: color,
          opacity: dimColor,
        }}
      />
    </div>
  );
}
