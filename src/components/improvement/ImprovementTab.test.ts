import { describe, expect, it } from "vitest";
import { eventEquipmentUseCount } from "./ImprovementTab";

describe("improvement event equipment highlighting", () => {
  it("recognizes equipment listed in the event equipment index", () => {
    expect(eventEquipmentUseCount(533)).toBe(89);
    expect(eventEquipmentUseCount(122)).toBe(71);
  });

  it("does not highlight equipment absent from the event equipment index", () => {
    expect(eventEquipmentUseCount(-1)).toBe(0);
  });
});
