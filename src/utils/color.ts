// Color utility functions for HP and condition display

/** HP bar color based on remaining percentage */
export function hpColor(hp: number, maxhp: number): string {
  if (maxhp <= 0) return "#4caf50";
  const ratio = hp / maxhp;
  if (ratio > 0.75) return "#4caf50";
  if (ratio > 0.5) return "#ffeb3b";
  if (ratio > 0.25) return "#ff9800";
  return "#f44336";
}

/** Condition text color */
export function condColor(cond: number): string {
  if (cond >= 50) return "#ffb74d"; // orange sparkle
  if (cond >= 40) return "#e0e0e0"; // normal
  if (cond >= 30) return "#ffeb3b"; // yellow warning
  return "#f44336"; // red fatigue
}

export function condBgClass(cond: number): string {
  if (cond >= 50) return "cond-sparkle";
  if (cond >= 40) return "";
  if (cond >= 30) return "cond-tired";
  return "cond-red";
}
