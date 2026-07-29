import { describe, expect, it } from "vitest";
import type { SortieRecord } from "../../types";
import {
  countGimmickResults,
  EVENT_FORMATION_HEADING,
  EVENT_FORMATION_NOTE,
  gaugeProgress,
  getEventGimmickRequirements,
  isLastDance,
  latestBossObservation,
} from "./EventTab";
import {
  getArmorBreakFormationLinks,
  getEventFormationLinks,
} from "../../data/eventFormations";

describe("event progress", () => {
  it("labels owned formations as recommended and shows the adjustment note", () => {
    expect(EVENT_FORMATION_HEADING).toBe("推奨編成");
    expect(EVENT_FORMATION_NOTE).toBe(
      "注意：索敵や制空が不足する場合は、自分で調整してください。",
    );
  });

  it("provides the published complete formation links for each event stage", () => {
    expect(getEventFormationLinks(3, "gauge3")).toContainEqual({
      label: "駆逐2・潜水4・潜母1",
      url: "https://kc.noro6.net/s/fQr6",
    });
    expect(getEventFormationLinks(1, "gimmick1")).toHaveLength(2);
    expect(getEventFormationLinks(4, "gauge5")).toHaveLength(3);
    expect(getArmorBreakFormationLinks(3)).toHaveLength(7);
  });

  it("uses the published E3 hard-mode gimmick requirements", () => {
    expect(getEventGimmickRequirements(3, "gimmick1", "甲")).toEqual([
      {
        id: "E2",
        node: "E2",
        kind: "victory",
        cellNos: [15],
        victory: "S",
        required: 2,
      },
      {
        id: "C2",
        node: "C2",
        kind: "victory",
        cellNos: [9, 40],
        victory: "A",
        required: 2,
      },
      {
        id: "B2",
        node: "B2",
        kind: "victory",
        cellNos: [6],
        victory: "S",
        required: 2,
      },
      {
        id: "D2",
        node: "D2",
        kind: "victory",
        cellNos: [12],
        victory: "A",
        required: 2,
      },
    ]);
  });

  it("provides difficulty-specific E1 and E5 conditions", () => {
    expect(getEventGimmickRequirements(1, "gimmick1", "乙")).toHaveLength(3);
    expect(getEventGimmickRequirements(5, "gimmick4", "甲")).toMatchObject([
      { node: "P3", victory: "A", required: 2 },
      { node: "P", victory: "A", required: 3 },
    ]);
  });

  it("counts qualifying E3 battle results automatically", () => {
    const requirements = getEventGimmickRequirements(3, "gimmick1", "甲");
    const records: SortieRecord[] = [{
      id: "sortie-1",
      fleet_id: 3,
      map_area: 62,
      map_no: 3,
      map_display: "62-3",
      gauge_num: 1,
      ships: [],
      start_time: "2026-07-26T00:00:00+09:00",
      nodes: [
        { cell_no: 15, event_kind: 1, rank: "S" },
        { cell_no: 9, event_kind: 1, rank: "A" },
        { cell_no: 6, event_kind: 1, rank: "A" },
      ],
    }];

    expect(countGimmickResults(records, 3, requirements, 1)).toEqual({
      E2: 1,
      C2: 1,
      B2: 0,
      D2: 0,
    });
  });

  it("counts air superiority, arrivals, and base defense", () => {
    const requirements = getEventGimmickRequirements(1, "gimmick1", "甲");
    const defense = getEventGimmickRequirements(1, "gimmick2", "甲")
      .find((item) => item.kind === "defense")!;
    const records: SortieRecord[] = [{
      id: "sortie-air",
      fleet_id: 3,
      map_area: 62,
      map_no: 1,
      map_display: "62-1",
      gauge_num: 1,
      ships: [],
      start_time: "2026-07-26T00:00:00+09:00",
      nodes: [
        {
          cell_no: 11,
          event_kind: 1,
          battle: {
            rank: "A",
            enemy_name: "",
            enemy_ships: [],
            formation: [4, 3, 1],
            air_battle: { air_superiority: 2 },
            friendly_hp: [],
            enemy_hp: [],
            ship_exp: [],
            night_battle: false,
          },
        },
        { cell_no: 13, event_kind: 1 },
        {
          cell_no: 16,
          event_kind: 1,
          base_air_defense: {
            occurred_at: "2026-07-26T00:01:00+09:00",
            air_superiority: 1,
          },
        },
      ],
    }];

    expect(countGimmickResults(records, 1, requirements, 1)).toMatchObject({
      F: 1,
      H: 1,
    });
    expect(countGimmickResults(records, 1, [defense], 1)).toEqual({
      defense: 1,
    });
  });

  it("calculates and clamps boss HP progress", () => {
    expect(gaugeProgress({
      map_id: 623,
      current_hp: 750,
      max_hp: 1000,
      cleared: false,
      provisional: false,
    })).toBe(25);
    expect(gaugeProgress({
      map_id: 623,
      current_hp: -1,
      max_hp: 1000,
      cleared: true,
      provisional: false,
    })).toBe(100);
  });

  it("enters last dance using the final-form HP threshold", () => {
    const base = {
      map_id: 623,
      gauge_num: 1,
      gauge_type: 2,
      max_hp: 4410,
      selected_rank: 4,
      state: 1,
      cleared: false,
      provisional: false,
    };
    expect(isLastDance({ ...base, current_hp: 1166 }, 980)).toBe(false);
    expect(isLastDance({ ...base, current_hp: 980 }, 980)).toBe(true);
    expect(isLastDance({ ...base, current_hp: 1050 }, 1080)).toBe(true);
    expect(
      isLastDance({ ...base, gauge_type: 3, current_hp: 500 }, 1080),
    ).toBe(false);
  });

  it("confirms the target boss final form from the observed name", () => {
    const records: SortieRecord[] = [{
      id: "sortie-final",
      fleet_id: 1,
      map_area: 62,
      map_no: 3,
      map_display: "62-3",
      gauge_num: 1,
      ships: [],
      start_time: "2026-07-26T04:00:00+09:00",
      nodes: [
        {
          cell_no: 12,
          event_kind: 1,
          event_id: 5,
          battle: {
            rank: "S",
            enemy_name: "別の艦隊",
            enemy_ships: [{ ship_id: 1, level: 1, name: "別のボス-壊" }],
            formation: [4, 3, 1],
            friendly_hp: [],
            enemy_hp: [{ before: 500, after: 0, max: 500 }],
            ship_exp: [],
            night_battle: false,
          },
        },
        {
          cell_no: 32,
          event_kind: 1,
          event_id: 5,
          battle: {
            rank: "A",
            enemy_name: "敵主力艦隊",
            enemy_ships: [
              { ship_id: 2162, level: 1, name: "深海擱座揚陸姫-壊" },
            ],
            formation: [4, 3, 1],
            friendly_hp: [],
            enemy_hp: [{ before: 980, after: 120, max: 980 }],
            ship_exp: [],
            night_battle: false,
          },
        },
      ],
    }];

    expect(latestBossObservation(records, 3, 1, [32])).toEqual({
      maxHp: 980,
      finalForm: true,
    });
  });
});
