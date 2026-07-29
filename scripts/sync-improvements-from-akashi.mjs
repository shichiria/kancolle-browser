import fs from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const OUTPUT = path.join(ROOT, "src-tauri", "data", "equipment_upgrades.json");
const UPSTREAM =
  "https://raw.githubusercontent.com/ElectronicObserverEN/Data/master/Data/EquipmentUpgrades.json";

const conversionOverrides = new Map([
  [
    3,
    {
      pathIndex: 1,
      target: 553,
      conv: {
        devmats: 7,
        devmats_sli: 9,
        screws: 5,
        screws_sli: 7,
        equips: [{ id: 3, eq_count: 2 }],
        consumable: [
          { id: 75, eq_count: 1 },
          { id: 94, eq_count: 1 },
        ],
      },
    },
  ],
  [
    91,
    {
      pathIndex: 0,
      target: 380,
      conv: {
        devmats: 12,
        devmats_sli: 20,
        screws: 7,
        screws_sli: 9,
        equips: [{ id: 553, eq_count: 1 }],
        consumable: [
          { id: 75, eq_count: 2 },
          { id: 104, eq_count: 8 },
        ],
      },
    },
  ],
  [
    229,
    {
      pathIndex: 0,
      target: 379,
      conv: {
        devmats: 10,
        devmats_sli: 17,
        screws: 4,
        screws_sli: 7,
        equips: [{ id: 553, eq_count: 1 }],
        consumable: [
          { id: 75, eq_count: 1 },
          { id: 104, eq_count: 7 },
        ],
      },
    },
  ],
  [
    289,
    {
      pathIndex: 0,
      target: 502,
      conv: {
        devmats: 36,
        devmats_sli: 48,
        screws: 12,
        screws_sli: 16,
        equips: [{ id: 329, eq_count: 2 }],
        consumable: [
          { id: 75, eq_count: 2 },
          { id: 94, eq_count: 2 },
        ],
      },
    },
  ],
  [
    340,
    {
      pathIndex: 0,
      target: 341,
      conv: {
        devmats: 8,
        devmats_sli: 12,
        screws: 6,
        screws_sli: 8,
        equips: [{ id: 5, eq_count: 3 }],
        consumable: [{ id: 75, eq_count: 1 }],
      },
    },
  ],
  [
    382,
    {
      pathIndex: 0,
      target: 509,
      conv: {
        devmats: 4,
        devmats_sli: 6,
        screws: 3,
        screws_sli: 5,
        equips: [{ id: 382, eq_count: 2 }],
        consumable: [{ id: 75, eq_count: 1 }],
      },
    },
  ],
  [
    427,
    {
      pathIndex: 0,
      target: 429,
      conv: {
        devmats: 10,
        devmats_sli: 20,
        screws: 8,
        screws_sli: 11,
        equips: [{ id: 7, eq_count: 3 }],
        consumable: [{ id: 75, eq_count: 1 }],
      },
    },
  ],
  [
    573,
    {
      pathIndex: 0,
      target: 574,
      conv: {
        devmats: 23,
        devmats_sli: 38,
        screws: 13,
        screws_sli: 18,
        equips: [{ id: 278, eq_count: 1 }],
        consumable: [{ id: 104, eq_count: 13 }],
      },
    },
  ],
]);

function entry(data, id) {
  const found = data.find((item) => item.eq_id === id);
  if (!found) throw new Error(`Equipment ${id} was not found in upstream data`);
  return found;
}

function recalculateConversions(item) {
  item.convert_to = item.improvement
    .filter((path) => path.convert?.id_after)
    .map((path) => ({
      id_after: path.convert.id_after,
      lvl_after: path.convert.lvl_after ?? 0,
    }));
}

function applyAkashiCorrections(data) {
  entry(data, 2).improvement[0].helpers = [
    { ship_ids: [], days: [0, 1, 2, 3, 4, 5, 6] },
  ];
  entry(data, 4).improvement[0].helpers = [
    { ship_ids: [], days: [0, 1, 2, 3, 4, 5, 6] },
  ];
  entry(data, 14).improvement[0].helpers = [
    { ship_ids: [], days: [0, 1, 2, 5, 6] },
  ];
  entry(data, 44).improvement[0].helpers = [{ ship_ids: [], days: [3, 4] }];

  entry(data, 6).improvement[0].costs.conv.devmats = 2;

  Object.assign(entry(data, 10).improvement[0].costs.p2, {
    screws: 3,
    screws_sli: 4,
  });

  Object.assign(entry(data, 19).improvement[0].costs.conv, {
    devmats_sli: 6,
    screws: 2,
    equips: [{ id: 19, eq_count: 2 }],
  });

  Object.assign(entry(data, 91).improvement[0].costs.p2, {
    devmats_sli: 5,
    screws_sli: 4,
  });

  const item121 = entry(data, 121);
  item121.improvement = item121.improvement.filter((path) => path.convert?.id_after !== 130);

  Object.assign(entry(data, 297).improvement[1].costs.conv, {
    devmats: 5,
    devmats_sli: 6,
    screws_sli: 5,
  });

  entry(data, 386).improvement[0].costs.p2.equips = [{ id: 5, eq_count: 2 }];
  entry(data, 511).improvement[0].costs.conv.screws = 4;

  entry(data, 118).improvement[0].helpers = [
    { ship_ids: [183], days: [1, 2, 3, 4] },
    { ship_ids: [321], days: [1, 3, 4] },
  ];
  entry(data, 252).improvement[0].helpers = [{ ship_ids: [713], days: [0, 4, 5, 6] }];
  entry(data, 395).improvement[0].helpers = entry(data, 395).improvement[0].helpers.filter(
    (helper) => !helper.ship_ids.includes(183),
  );

  delete entry(data, 288).improvement[0].costs.extra;
  const item573 = entry(data, 573);
  item573.improvement[0].costs.extra = item573.improvement[0].costs.extra
    .map((cost) => ({ ...cost, levels: cost.levels.filter((level) => level !== 8) }))
    .filter((cost) => cost.levels.length > 0);

  for (const [id, override] of conversionOverrides) {
    const item = entry(data, id);
    const improvement = item.improvement[override.pathIndex];
    improvement.convert = { id_after: override.target, lvl_after: 0 };
    improvement.costs.conv = override.conv;
  }

  const item379 = entry(data, 379);
  const base379 = item379.improvement[0];
  base379.convert = null;
  delete base379.costs.conv;
  base379.helpers = [
    { ship_ids: [702], days: [0, 1, 2, 3, 4, 5, 6] },
    { ship_ids: [641], days: [0, 1, 6] },
  ];
  const conversion379 = structuredClone(base379);
  conversion379.convert = { id_after: 572, lvl_after: 0 };
  conversion379.helpers = [
    { ship_ids: [641], days: [2, 3, 4, 5] },
    { ship_ids: [997, 1035], days: [0, 4, 5, 6] },
  ];
  conversion379.costs.conv = {
    devmats: 12,
    devmats_sli: 24,
    screws: 7,
    screws_sli: 14,
    equips: [{ id: 553, eq_count: 1 }],
    consumable: [
      { id: 75, eq_count: 2 },
      { id: 104, eq_count: 10 },
    ],
  };
  item379.improvement = [base379, conversion379];

  for (const item of data) recalculateConversions(item);
  return data;
}

async function main() {
  const response = await fetch(UPSTREAM);
  if (!response.ok) throw new Error(`Failed to fetch upstream data: HTTP ${response.status}`);
  const data = applyAkashiCorrections(await response.json());
  await fs.writeFile(OUTPUT, `${JSON.stringify(data, null, 2)}\n`);
  console.log(`Synced ${data.length} equipment entries to ${OUTPUT}`);
}

await main();
