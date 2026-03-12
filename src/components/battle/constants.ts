// Battle-specific display constants

export const FORMATION_NAMES: Record<number, string> = {
  1: "単縦陣",
  2: "複縦陣",
  3: "輪形陣",
  4: "梯形陣",
  5: "単横陣",
  6: "警戒陣",
  11: "第一警戒航行序列",
  12: "第二警戒航行序列",
  13: "第三警戒航行序列",
  14: "第四警戒航行序列",
};

export const ENGAGEMENT_NAMES: Record<number, string> = {
  1: "同航戦",
  2: "反航戦",
  3: "T字有利",
  4: "T字不利",
};

export const RANK_COLORS: Record<string, string> = {
  S: "#ffd700",
  A: "#ff6b6b",
  B: "#ff9800",
  C: "#888",
  D: "#666",
  E: "#555",
};

// EVENT_LABELS keyed by api_color_no (fallback)
export const EVENT_LABELS: Record<number, string> = {
  0: "始点",
  2: "資源",
  3: "渦潮",
  4: "戦闘",
  5: "ボス",
  6: "気のせい",
  7: "航空戦",
  8: "空襲",
  9: "揚陸",
  10: "泊地",
};

// EVENT_ID_LABELS keyed by api_event_id (takes priority over color_no)
export const EVENT_ID_LABELS: Record<number, string> = {
  6: "航路選択",
};

export const AIR_SUPERIORITY_LABELS: Record<number, { text: string; color: string }> = {
  0: { text: "航空劣勢", color: "#f44336" },
  1: { text: "航空優勢", color: "#4caf50" },
  2: { text: "制空権確保", color: "#2196f3" },
  3: { text: "航空均衡", color: "#ff9800" },
  4: { text: "制空権喪失", color: "#d32f2f" },
};
