import { describe, test, expect } from "vitest";
import {
  getRankName,
  formatRemaining,
  formatDuration,
  fmtDate,
  daysInMonth,
  toDateStr,
  formatImprovements,
} from "./format";

describe("getRankName", () => {
  test("returns 元帥 for rank 1", () => {
    expect(getRankName(1)).toBe("元帥");
  });
  test("returns 大将 for rank 2", () => {
    expect(getRankName(2)).toBe("大将");
  });
  test("returns 新米少佐 for rank 10", () => {
    expect(getRankName(10)).toBe("新米少佐");
  });
  test("returns fallback for unknown rank", () => {
    expect(getRankName(99)).toBe("Rank 99");
  });
  test("returns empty for null", () => {
    expect(getRankName(undefined)).toBe("");
  });
});

describe("formatRemaining", () => {
  test("returns HH:MM:SS when hours > 0", () => {
    const now = 0;
    const target = 3661000; // 1h 1m 1s
    expect(formatRemaining(target, now)).toBe("1:01:01");
  });
  test("returns MM:SS when no hours", () => {
    expect(formatRemaining(65000, 0)).toBe("01:05");
  });
  test("returns 完了 when past", () => {
    expect(formatRemaining(0, 1000)).toBe("完了");
  });
  test("returns 完了 when equal", () => {
    expect(formatRemaining(1000, 1000)).toBe("完了");
  });
});

describe("formatDuration", () => {
  test("hours and minutes", () => {
    expect(formatDuration(150)).toBe("2h30m");
  });
  test("minutes only", () => {
    expect(formatDuration(45)).toBe("45m");
  });
  test("exact hours", () => {
    expect(formatDuration(120)).toBe("2h");
  });
  test("zero minutes", () => {
    expect(formatDuration(0)).toBe("0m");
  });
});

describe("fmtDate", () => {
  test("formats YYYY-MM-DD to YYYY/MM/DD", () => {
    expect(fmtDate("2025-03-18")).toBe("2025/03/18");
  });
});

describe("toDateStr", () => {
  test("creates date string with 0-indexed month", () => {
    expect(toDateStr(2025, 2, 18)).toBe("2025-03-18");
  });
  test("handles January (month 0)", () => {
    expect(toDateStr(2025, 0, 1)).toBe("2025-01-01");
  });
  test("handles December (month 11)", () => {
    expect(toDateStr(2025, 11, 31)).toBe("2025-12-31");
  });
});

describe("daysInMonth", () => {
  test("February non-leap year", () => {
    expect(daysInMonth(2025, 1)).toBe(28);
  });
  test("February leap year", () => {
    expect(daysInMonth(2024, 1)).toBe(29);
  });
  test("January", () => {
    expect(daysInMonth(2025, 0)).toBe(31);
  });
  test("April", () => {
    expect(daysInMonth(2025, 3)).toBe(30);
  });
});

describe("formatImprovements", () => {
  test("single improvement", () => {
    expect(formatImprovements([[6, 2]])).toBe("★6×2");
  });
  test("max level", () => {
    expect(formatImprovements([[10, 1]])).toBe("★max×1");
  });
  test("multiple levels", () => {
    expect(formatImprovements([[4, 3], [7, 1]])).toBe("★4×3 ★7×1");
  });
  test("filters out level 0", () => {
    expect(formatImprovements([[0, 5], [3, 2]])).toBe("★3×2");
  });
});
