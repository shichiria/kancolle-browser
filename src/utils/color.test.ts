import { describe, test, expect } from "vitest";
import { hpColor, condColor, condBgClass } from "./color";

describe("hpColor", () => {
  test("green when > 75%", () => {
    expect(hpColor(80, 100)).toBe("#4caf50");
  });
  test("green at boundary 76%", () => {
    expect(hpColor(76, 100)).toBe("#4caf50");
  });
  test("yellow at 75% boundary", () => {
    expect(hpColor(75, 100)).toBe("#ffeb3b");
  });
  test("yellow when > 50%", () => {
    expect(hpColor(70, 100)).toBe("#ffeb3b");
  });
  test("yellow at 51%", () => {
    expect(hpColor(51, 100)).toBe("#ffeb3b");
  });
  test("orange at 50% boundary", () => {
    expect(hpColor(50, 100)).toBe("#ff9800");
  });
  test("orange when > 25%", () => {
    expect(hpColor(49, 100)).toBe("#ff9800");
  });
  test("orange at 26%", () => {
    expect(hpColor(26, 100)).toBe("#ff9800");
  });
  test("red at 25% boundary", () => {
    expect(hpColor(25, 100)).toBe("#f44336");
  });
  test("red when <= 25%", () => {
    expect(hpColor(24, 100)).toBe("#f44336");
  });
  test("red at 0 HP", () => {
    expect(hpColor(0, 100)).toBe("#f44336");
  });
  test("green when maxhp is 0", () => {
    expect(hpColor(0, 0)).toBe("#4caf50");
  });
});

describe("condColor", () => {
  test("sparkle color when >= 50", () => {
    expect(condColor(50)).toBe("#ffb74d");
    expect(condColor(85)).toBe("#ffb74d");
  });
  test("normal color when 40-49", () => {
    expect(condColor(49)).toBe("#e0e0e0");
    expect(condColor(40)).toBe("#e0e0e0");
  });
  test("warning color when 30-39", () => {
    expect(condColor(39)).toBe("#ffeb3b");
    expect(condColor(30)).toBe("#ffeb3b");
  });
  test("fatigue color when < 30", () => {
    expect(condColor(29)).toBe("#f44336");
    expect(condColor(0)).toBe("#f44336");
  });
});

describe("condBgClass", () => {
  test("sparkle class when >= 50", () => {
    expect(condBgClass(50)).toBe("cond-sparkle");
  });
  test("empty class when 40-49", () => {
    expect(condBgClass(40)).toBe("");
  });
  test("tired class when 30-39", () => {
    expect(condBgClass(30)).toBe("cond-tired");
  });
  test("red class when < 30", () => {
    expect(condBgClass(29)).toBe("cond-red");
  });
});
