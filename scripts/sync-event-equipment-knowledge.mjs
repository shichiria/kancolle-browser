import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import LZString from "lz-string";

const root = resolve(import.meta.dirname, "..");
const formationsSource = await readFile(
  resolve(root, "src/data/eventFormations.ts"),
  "utf8",
);
const reference = JSON.parse(
  await readFile(resolve(root, "src/data/kcWebReference.json"), "utf8"),
);
const itemById = new Map(reference.items.map((item) => [item.id, item]));
const ids = [
  ...formationsSource.matchAll(
    /\["([^"]+)",\s*"([A-Za-z0-9]+)"\]/g,
  ),
].map((match) => match[2]);
const usage = new Map();

for (const id of ids) {
  const response = await fetch(`https://kc.noro6.net/s/${id}`, {
    redirect: "manual",
  });
  const location = response.headers.get("location");
  if (!location) throw new Error(`Formation ${id} did not redirect`);
  const data = new URL(location).searchParams.get("data");
  const json = JSON.parse(LZString.decompressFromEncodedURIComponent(data));
  for (const base of json.airbaseInfo?.airbases ?? []) {
    for (const item of base.items ?? []) {
      if (item.i > 0) usage.set(item.i, (usage.get(item.i) ?? 0) + 1);
    }
  }
  for (const fleet of json.fleetInfo?.fleets ?? []) {
    for (const ship of fleet.ships ?? []) {
      for (const item of [...(ship.is ?? []), ship.ex].filter(Boolean)) {
        if (item.i > 0) usage.set(item.i, (usage.get(item.i) ?? 0) + 1);
      }
    }
  }
}

const typeNames = new Map([
  [1, "小口径主砲"], [2, "中口径主砲"], [3, "大口径主砲"],
  [4, "副砲"], [5, "魚雷"], [6, "艦上戦闘機"],
  [7, "艦上爆撃機"], [8, "艦上攻撃機"], [9, "艦上偵察機"],
  [10, "水上偵察機"], [11, "水上爆撃機"], [12, "小型電探"],
  [13, "大型電探"], [14, "ソナー"], [15, "爆雷"],
  [17, "機関部強化"], [18, "対空強化弾"], [19, "対艦強化弾"],
  [21, "対空機銃"], [22, "特殊潜航艇"], [23, "応急修理要員"],
  [24, "上陸用舟艇"], [26, "対潜哨戒機"], [27, "追加装甲"],
  [29, "探照灯"], [30, "輸送部材"], [32, "潜水艦魚雷"],
  [34, "司令部施設"], [35, "航空要員"], [36, "高射装置"],
  [37, "対地装備"], [39, "水上艦要員"], [40, "大型ソナー"],
  [41, "大型飛行艇"], [42, "大型探照灯"], [45, "水上戦闘機"],
  [46, "特型内火艇"], [47, "陸上攻撃機"], [48, "局地戦闘機"],
  [49, "陸上偵察機"], [51, "潜水艦装備"], [52, "陸戦部隊"],
  [54, "発煙装置等"], [57, "噴式戦闘爆撃機"],
]);

function roles(item) {
  const result = [];
  if (item.itype === 16) result.push("高角砲");
  if (item.isNightAircraftItem) result.push("夜間機");
  if (item.enabledAttackLandBase) result.push("対地攻撃可");
  if (item.isStrictDepthCharge) result.push("狭義爆雷");
  if (item.isRocket) result.push("ロケット局戦");
  if (item.isLateModelTorpedo) result.push("後期型潜水魚雷");
  if (item.type === 47 && item.itype === 47) result.push("基地対潜");
  if ((item.tp2 ?? 0) > 0) result.push(`TP補正${item.tp2}`);
  if ((item.avoidId ?? 0) > 0) result.push(`撃墜回避${item.avoidId}`);
  for (const tag of item.eventTags ?? []) {
    result.push(`${tag.key}:${tag.text.join("/")}`);
  }
  return result.join("、") || "通常";
}

const usedItems = [...usage]
  .map(([id, count]) => ({ ...itemById.get(id), count }))
  .sort((left, right) => left.type - right.type || right.count - left.count);
const lines = [
  "# 2026夏イベント使用装備索引",
  "",
  `生成日時: ${new Date().toISOString()}`,
  "",
  `制空シミュ82編成で使われる装備${usedItems.length}種。Wikiリンクはkc-webの装備マスター規則から生成する。`,
  "",
  "| ID | 装備 | カテゴリ | 使用枠 | 機械判定できる役割 |",
  "|---:|---|---|---:|---|",
  ...usedItems.map(
    (item) =>
      `| ${item.id} | [${item.name}](${item.wikiUrl}) | ${typeNames.get(item.type) ?? item.type} | ${item.count} | ${roles(item)} |`,
  ),
  "",
];
await writeFile(
  resolve(root, "docs/KNOWLEDGE/event-equipment-index.md"),
  `${lines.join("\n")}\n`,
  "utf8",
);
await writeFile(
  resolve(root, "src/data/eventEquipmentUsage.json"),
  `${JSON.stringify({
    metadata: {
      event: "2026-summer",
      generatedAt: new Date().toISOString(),
      source: "docs/KNOWLEDGE/event-equipment-index.md",
    },
    items: usedItems.map((item) => ({ id: item.id, usage: item.count })),
  }, null, 2)}\n`,
  "utf8",
);
console.log(`Wrote event equipment index (${usedItems.length} items)`);
