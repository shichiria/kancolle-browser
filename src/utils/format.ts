// Formatting utility functions

const RANK_NAMES: Record<number, string> = {
  1: "元帥",
  2: "大将",
  3: "中将",
  4: "少将",
  5: "大佐",
  6: "中佐",
  7: "新米中佐",
  8: "少佐",
  9: "中堅少佐",
  10: "新米少佐",
};

export function getRankName(rank?: number): string {
  if (rank == null) return "";
  return RANK_NAMES[rank] ?? `Rank ${rank}`;
}

/** Format remaining milliseconds as HH:MM:SS or MM:SS */
export function formatRemaining(targetMs: number, now: number): string {
  const diff = targetMs - now;
  if (diff <= 0) return "完了";
  const totalSec = Math.floor(diff / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

export function formatDuration(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h > 0 && m > 0) return `${h}h${m.toString().padStart(2, "0")}m`;
  if (h > 0) return `${h}h`;
  return `${m}m`;
}

/** Format YYYY-MM-DD to compact display YYYY/MM/DD */
export function fmtDate(d: string) {
  const [y, m, dd] = d.split("-");
  return `${y}/${m}/${dd}`;
}

export function formatImprovements(improvements: [number, number][]): string {
  return improvements
    .filter(([level]) => level > 0)
    .map(([level, count]) => {
      const label = level >= 10 ? "★max" : `★${level}`;
      return `${label}×${count}`;
    })
    .join(" ");
}

/** Get days in month (0-indexed month) */
export function daysInMonth(year: number, month: number) {
  return new Date(year, month + 1, 0).getDate();
}

/** YYYY-MM-DD string from Date parts (0-indexed month) */
export function toDateStr(y: number, m: number, d: number) {
  return `${y}-${String(m + 1).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}
