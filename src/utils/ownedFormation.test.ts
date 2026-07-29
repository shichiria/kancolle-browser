import LZString from "lz-string";
import { describe, expect, it } from "vitest";
import {
  adaptOwnedFormationData,
  type OwnedFormationInventory,
} from "./ownedFormation";

function formationWithGuns(shipId: number, itemIds: number[]): string {
  return LZString.compressToEncodedURIComponent(
    JSON.stringify({
      fleetInfo: {
        admiralLevel: 120,
        fleets: [
          {
            ships: [
              {
                i: shipId,
                is: itemIds.map((i) => ({ i })),
                ex: { i: 0 },
              },
            ],
          },
        ],
      },
    }),
  );
}

function formationWithExItem(shipId: number, itemId: number): string {
  return LZString.compressToEncodedURIComponent(
    JSON.stringify({
      fleetInfo: {
        admiralLevel: 120,
        fleets: [
          {
            ships: [{ i: shipId, is: [], ex: { i: itemId } }],
          },
        ],
      },
    }),
  );
}

function formationWithAirbase(
  itemIds: number[],
  enemyIds: number[] = [],
): string {
  return LZString.compressToEncodedURIComponent(
    JSON.stringify({
      airbaseInfo: {
        airbases: [
          {
            mode: 1,
            battleTarget: [0, 0],
            items: itemIds.map((i) => ({ i })),
          },
        ],
      },
      battleInfo: {
        fleets: [{ enemies: enemyIds.map((i) => ({ i })) }],
      },
    }),
  );
}

function decodedFormation(url: string) {
  const compressed = new URL(url).searchParams.get("data");
  return JSON.parse(
    LZString.decompressFromEncodedURIComponent(compressed ?? ""),
  );
}

describe("owned formation allocation", () => {
  it("never assigns a single improved 3号砲 instance more than once", () => {
    const inventory: OwnedFormationInventory = {
      hq_level: 120,
      items: [
        { instance_id: 1001, master_id: 50, remodel: 9, proficiency: 0 },
        { instance_id: 1002, master_id: 90, remodel: 0, proficiency: 0 },
      ],
    };
    const result = adaptOwnedFormationData(
      formationWithGuns(59, [50, 50, 50]),
      inventory,
    );
    expect(result.assigned).toBe(2);
    expect(result.missing).toBe(1);

    const decoded = decodedFormation(result.url);
    const items = decoded.fleetInfo.fleets[0].ships[0].is as Array<{
      i: number;
      r?: number;
    }>;
    expect(items.filter((item) => item.i === 50 && item.r === 9)).toHaveLength(
      1,
    );
  });

  it("does not consume equipment from another kc-web category", () => {
    const result = adaptOwnedFormationData(formationWithGuns(59, [50]), {
      hq_level: 120,
      items: [
        { instance_id: 2001, master_id: 1, remodel: 10, proficiency: 0 },
      ],
    });
    expect(result.assigned).toBe(0);
    expect(result.missing).toBe(1);
  });

  it("never lowers a sortie land attacker's source radius", () => {
    const result = adaptOwnedFormationData(formationWithAirbase([169]), {
      hq_level: 120,
      items: [
        { instance_id: 3001, master_id: 459, remodel: 2, proficiency: 7 },
      ],
    });

    expect(result.assigned).toBe(0);
    expect(result.missing).toBe(1);
    expect(
      decodedFormation(result.url).airbaseInfo.airbases[0].items[0].i,
    ).toBe(0);
  });

  it("keeps B-25 ahead of basic 一式陸攻 against small surface ships", () => {
    const result = adaptOwnedFormationData(
      formationWithAirbase([459], [1623, 1592]),
      {
        hq_level: 120,
        items: [
          { instance_id: 4001, master_id: 169, remodel: 10, proficiency: 7 },
          { instance_id: 4002, master_id: 459, remodel: 2, proficiency: 7 },
        ],
      },
    );

    const item =
      decodedFormation(result.url).airbaseInfo.airbases[0].items[0];
    expect(item.i).toBe(459);
    expect(item.r).toBe(2);
  });

  it("does not replace a submarine-event scout with Walrus", () => {
    const result = adaptOwnedFormationData(formationWithGuns(495, [522]), {
      hq_level: 120,
      items: [
        { instance_id: 5001, master_id: 510, remodel: 0, proficiency: 0 },
      ],
    });

    expect(result.assigned).toBe(0);
    expect(result.missing).toBe(1);
    expect(
      decodedFormation(result.url).fleetInfo.fleets[0].ships[0].is[0].i,
    ).toBe(0);
  });

  it("preserves high-angle, night, and strict-depth-charge roles", () => {
    const cases = [
      { ship: 716, source: 122, candidate: 366 },
      { ship: 495, source: 471, candidate: 118 },
      { ship: 713, source: 473, candidate: 56 },
      { ship: 716, source: 439, candidate: 377 },
      { ship: 716, source: 272, candidate: 107 },
    ];
    for (const [index, item] of cases.entries()) {
      const result = adaptOwnedFormationData(
        formationWithGuns(item.ship, [item.source]),
        {
          hq_level: 120,
          items: [{
            instance_id: 6000 + index,
            master_id: item.candidate,
            remodel: 0,
            proficiency: 7,
          }],
        },
      );
      expect(result.missing, `${item.source} -> ${item.candidate}`).toBe(1);
    }
  });

  it("allows a turbine in the reinforcement expansion", () => {
    const result = adaptOwnedFormationData(formationWithExItem(716, 33), {
      hq_level: 120,
      items: [
        { instance_id: 7001, master_id: 33, remodel: 0, proficiency: 0 },
      ],
    });
    expect(result.assigned).toBe(1);
    expect(result.missing).toBe(0);
    expect(
      decodedFormation(result.url).fleetInfo.fleets[0].ships[0].ex.i,
    ).toBe(33);
  });

  it("uses the documented carrier bomber fallback when jets are short", () => {
    const result = adaptOwnedFormationData(formationWithGuns(466, [200]), {
      hq_level: 120,
      items: [
        { instance_id: 8001, master_id: 319, remodel: 1, proficiency: 7 },
      ],
    });
    expect(result.assigned).toBe(1);
    expect(result.missing).toBe(0);
    expect(
      decodedFormation(result.url).fleetInfo.fleets[0].ships[0].is[0].i,
    ).toBe(319);
  });

  it("preserves repair crews and goddesses without applying owned counts", () => {
    const result = adaptOwnedFormationData(
      formationWithGuns(716, [42, 42, 43, 43]),
      { hq_level: 120, items: [] },
    );

    expect(result.assigned).toBe(4);
    expect(result.missing).toBe(0);
    expect(
      decodedFormation(result.url).fleetInfo.fleets[0].ships[0].is.map(
        (item: { i: number }) => item.i,
      ),
    ).toEqual([42, 42, 43, 43]);
  });
});
