import fs from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const DATA_PATH = path.join(ROOT, "src-tauri", "data", "equipment_upgrades.json");
const REFERENCE_PATH = path.join(ROOT, "src", "data", "kcWebReference.json");
const REPORT_JSON = path.join(ROOT, ".reports", "akashi-improvement-audit.json");
const REPORT_MD = path.join(ROOT, ".reports", "akashi-improvement-audit.md");
const BASE_URL = "https://akashi-list.me";
const EO_DATA_URL =
  "https://raw.githubusercontent.com/ElectronicObserverEN/Data/master/Data/EquipmentUpgrades.json";
const unknownConsumableNames = new Set();

const USE_ITEM_NAMES = new Map([
  ["高速修復材", 1],
  ["高速建造材", 2],
  ["開発資材", 3],
  ["改修資材", 4],
  ["家具箱（小）", 10],
  ["家具箱（中）", 11],
  ["家具箱（大）", 12],
  ["応急修理要員", 42],
  ["応急修理女神", 43],
  ["熟練搭乗員", 70],
  ["ネ式エンジン", 71],
  ["勲章", 57],
  ["改修資材", 73],
  ["新型砲熕兵装資材", 75],
  ["戦闘詳報", 78],
  ["新型航空兵装資材", 77],
  ["新型兵装資材", 94],
  ["新型噴進装備開発資材", 92],
  ["緊急修理資材", 91],
  ["潜水艦補給物資", 95],
  ["海外艦最新技術", 100],
  ["工廠資源", 104],
]);

function decodeHtml(value) {
  return value
    .replace(/<br\s*\/?>/gi, "")
    .replace(/<[^>]+>/g, "")
    .replace(/&nbsp;|&#160;/g, " ")
    .replace(/&#215;|&times;/g, "×")
    .replace(/&#9733;/g, "★")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&#(\d+);/g, (_, code) => String.fromCodePoint(Number(code)))
    .trim();
}

function phaseCost(rowHtml) {
  const cells = [...rowHtml.matchAll(/<td(?:\s[^>]*)?>([\s\S]*?)<\/td>/g)].map((m) => m[1]);
  if (cells.length < 3) return null;
  const pair = (html) => {
    const match = decodeHtml(html).match(/(\d+)\s*\/\s*(\d+)/);
    return match ? [Number(match[1]), Number(match[2])] : null;
  };
  const dev = pair(cells[0]);
  const screws = pair(cells[1]);
  if (!dev || !screws) return null;
  const required = parseRequiredItems(cells[2]);
  return {
    devmats: dev[0],
    devmats_sli: dev[1],
    screws: screws[0],
    screws_sli: screws[1],
    ...required,
  };
}

function countFromHtml(html) {
  const text = decodeHtml(html);
  const match = text.match(/×\s*(\d+)/);
  return match ? Number(match[1]) : 1;
}

function parseRequiredItems(html) {
  const equips = [];
  const consumable = [];
  const covered = [];

  for (const match of html.matchAll(/<a\s+data-wid=w(\d+)[^>]*>[\s\S]*?<\/a>/g)) {
    equips.push({ id: Number(match[1]), eq_count: countFromHtml(match[0]) });
    covered.push([match.index, match.index + match[0].length]);
  }

  for (const match of html.matchAll(/<(?:div|figure)[^>]*title=["']?(\d{3}):[^>]*>[\s\S]*?<\/(?:div|figure)>[^<]*/g)) {
    if (covered.some(([start, end]) => match.index >= start && match.index < end)) continue;
    equips.push({ id: Number(match[1]), eq_count: countFromHtml(match[0]) });
    covered.push([match.index, match.index + match[0].length]);
  }

  if (equips.length === 0) {
    const classId = html.match(/class=["'][^"']*\bw(\d{3})\b/) ?? html.match(/class=[^\s>]*\bw(\d{3})\b/);
    if (classId && /×\s*\d+/.test(decodeHtml(html))) {
      equips.push({ id: Number(classId[1]), eq_count: countFromHtml(html) });
    }
  }

  for (const match of html.matchAll(/<figure[^>]*class=["']?icon-item["']?[^>]*>[\s\S]*?<\/figure>/g)) {
    if (covered.some(([start, end]) => match.index >= start && match.index < end)) continue;
    const nameMatch = match[0].match(/(?:alt|title)=["']?([^"'>\s][^"'>]*?)(?:["']|\s|>)/);
    if (!nameMatch) continue;
    const name = decodeHtml(nameMatch[1]);
    const id = USE_ITEM_NAMES.get(name);
    if (id) consumable.push({ id, eq_count: countFromHtml(match[0]), name });
    else unknownConsumableNames.add(name);
  }

  const equipVariants = [];
  if (html.includes("class=sub")) {
    const sections = html.split(/<div class=sub>[\s\S]*?<\/div>/);
    for (const section of sections) {
      const variant = [];
      for (const match of section.matchAll(/<a\s+data-wid=w(\d+)[^>]*>[\s\S]*?<\/a>/g)) {
        variant.push({ id: Number(match[1]), eq_count: countFromHtml(match[0]) });
      }
      if (variant.length) equipVariants.push(aggregate(variant));
    }
  }

  return {
    equips: aggregate(equips),
    equipVariants,
    consumable: aggregate(consumable).map(({ id, eq_count }) => ({ id, eq_count })),
  };
}

function parseExtraCosts(rowHtml) {
  const extra = [];
  for (const match of rowHtml.matchAll(
    /<span>&#9733;(\d+)<\/span>\s*<span class=s-gap>([\s\S]*?)<\/span>/g,
  )) {
    const level = Number(match[1]) + 1;
    const materialHtml = match[2];
    const equipment = materialHtml.match(/data-wid=w(\d+)/);
    if (equipment) {
      extra.push({
        levels: [level],
        equips: [{ id: Number(equipment[1]), eq_count: countFromHtml(materialHtml) }],
        consumable: [],
      });
      continue;
    }
    const nameMatch = materialHtml.match(/title=["']?([^"'>]+?)(?:["']|\s|>)/);
    const name = nameMatch ? decodeHtml(nameMatch[1]) : "";
    const id = USE_ITEM_NAMES.get(name);
    if (id) {
      extra.push({
        levels: [level],
        equips: [],
        consumable: [{ id, eq_count: countFromHtml(materialHtml) }],
      });
    } else if (name) {
      unknownConsumableNames.add(name);
    }
  }
  return extra;
}

function aggregate(items) {
  const result = new Map();
  for (const item of items) result.set(item.id, (result.get(item.id) ?? 0) + item.eq_count);
  return [...result].map(([id, eq_count]) => ({ id, eq_count })).sort((a, b) => a.id - b.id);
}

function parseResources(html) {
  const result = {};
  for (const [field, className] of [
    ["fuel", "ri-fuel"],
    ["ammo", "ri-ammo"],
    ["steel", "ri-steel"],
    ["baux", "ri-bauxite"],
  ]) {
    const match = html.match(new RegExp(`<span class=["']?${className}["']?[^>]*>\\s*(\\d+)`));
    if (match) result[field] = Number(match[1]);
  }
  return result;
}

function parseDetail(id, html) {
  const tableMatch = html.match(/<div class=resource-table><table>([\s\S]*?)<\/table><\/div>/);
  if (!tableMatch) return { id, error: "resource-table not found" };

  const rows = [...tableMatch[1].matchAll(/<tr(?:\s[^>]*)?>([\s\S]*?)<\/tr>/g)].map((m) => m[0]);
  const parsed = { id, ...parseResources(html), p1: null, p2: null, extra: [], paths: [] };
  let currentKind = 0;

  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index];
    const kind = row.match(/<th class=["'][^"']*\bkind(\d+)\b/);
    if (kind) {
      currentKind = Number(kind[1]);
      continue;
    }
    const label = row.match(/<th class=border-right(?:\s+rowspan=\d+)?>([\s\S]*?)<\/th>/);
    if (!label) continue;
    const text = decodeHtml(label[1]);
    if (text === "0 ～ 5") {
      parsed.p1 = phaseCost(row);
      if (rows[index + 1]?.includes("add-material")) parsed.extra.push(...parseExtraCosts(rows[index + 1]));
    } else if (text === "6 ～ 9") {
      parsed.p2 = phaseCost(row);
      if (rows[index + 1]?.includes("add-material")) parsed.extra.push(...parseExtraCosts(rows[index + 1]));
    }
    else if (text === "MAX") {
      const cost = phaseCost(row);
      const nextRow = rows[index + 1]?.includes("class=upgrade") ? rows[index + 1] : "";
      const target = nextRow
        ? [...nextRow.matchAll(/data-wid=w(\d+)/g)].map((m) => Number(m[1])).find((targetId) => targetId !== id)
        : null;
      parsed.paths.push({ kind: currentKind || parsed.paths.length + 1, target: target ?? null, conv: cost });
    }
  }
  return parsed;
}

function sameItems(a = [], b = []) {
  return JSON.stringify(aggregate(a)) === JSON.stringify(aggregate(b));
}

function comparePhase(diffs, eqId, pathIndex, phase, local, remote) {
  if (!remote) {
    if (local) diffs.push({ eqId, pathIndex, phase, field: "phase", local, akashi: null });
    return;
  }
  if (!local) {
    diffs.push({ eqId, pathIndex, phase, field: "phase", local: null, akashi: remote });
    return;
  }
  for (const field of ["devmats", "devmats_sli", "screws", "screws_sli"]) {
    if (local[field] !== remote[field]) {
      diffs.push({ eqId, pathIndex, phase, field, local: local[field], akashi: remote[field] });
    }
  }
  const remoteEquipMatches =
    sameItems(local.equips, remote.equips) ||
    (remote.equipVariants ?? []).some((variant) => sameItems(local.equips, variant));
  if (!remoteEquipMatches) {
    diffs.push({ eqId, pathIndex, phase, field: "equips", local: local.equips ?? [], akashi: remote.equips ?? [] });
  }
  if (!sameItems(local.consumable, remote.consumable)) {
    diffs.push({
      eqId,
      pathIndex,
      phase,
      field: "consumable",
      local: local.consumable ?? [],
      akashi: remote.consumable ?? [],
    });
  }
}

function costsAtLevel(extra = [], level, field) {
  return aggregate(
    extra.filter((cost) => cost.levels?.includes(level)).flatMap((cost) => cost[field] ?? []),
  );
}

function compare(localEntries, remoteEntries, itemNames) {
  const diffs = [];
  const remoteById = new Map(remoteEntries.map((entry) => [entry.id, entry]));
  const localById = new Map(localEntries.map((entry) => [entry.eq_id, entry]));

  for (const entry of localEntries) {
    const remote = remoteById.get(entry.eq_id);
    if (!remote || remote.error) continue;
    const localTargets = aggregate(
      entry.improvement
        .filter((path) => path.convert?.id_after)
        .map((path) => ({ id: path.convert.id_after, eq_count: 1 })),
    );
    const remoteTargets = aggregate(
      remote.paths.filter((path) => path.target).map((path) => ({ id: path.target, eq_count: 1 })),
    );
    if (!sameItems(localTargets, remoteTargets)) {
      diffs.push({
        eqId: entry.eq_id,
        pathIndex: 0,
        phase: "更新先",
        field: "conversion_targets",
        local: localTargets,
        akashi: remoteTargets,
      });
    }
    for (const [pathIndex, improvement] of entry.improvement.entries()) {
      if (!improvement.costs) continue;
      for (const field of ["fuel", "ammo", "steel", "baux"]) {
        if (remote[field] !== undefined && improvement.costs[field] !== remote[field]) {
          diffs.push({
            eqId: entry.eq_id,
            pathIndex,
            phase: "base",
            field,
            local: improvement.costs[field],
            akashi: remote[field],
          });
        }
      }
      comparePhase(diffs, entry.eq_id, pathIndex, "p1", improvement.costs.p1, remote.p1);
      comparePhase(diffs, entry.eq_id, pathIndex, "p2", improvement.costs.p2, remote.p2);
      for (const level of [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]) {
        for (const field of ["equips", "consumable"]) {
          const localExtra = costsAtLevel(improvement.costs.extra, level, field);
          const remoteExtra = costsAtLevel(remote.extra, level, field);
          if (!sameItems(localExtra, remoteExtra)) {
            diffs.push({
              eqId: entry.eq_id,
              pathIndex,
              phase: `★${level - 1}→★${level}`,
              field: `extra_${field}`,
              local: localExtra,
              akashi: remoteExtra,
            });
          }
        }
      }

      if (improvement.convert && improvement.costs.conv) {
        let remotePath = remote.paths.find((path) => path.target === improvement.convert?.id_after);
        if (!remotePath && remote.paths.length === 1) remotePath = remote.paths[0];
        comparePhase(diffs, entry.eq_id, pathIndex, "conv", improvement.costs.conv, remotePath?.conv ?? null);
      }
    }
  }

  const localOnly = [...localById.keys()].filter((id) => !remoteById.has(id)).sort((a, b) => a - b);
  const remoteOnly = [...remoteById.keys()].filter((id) => !localById.has(id)).sort((a, b) => a - b);
  for (const diff of diffs) diff.name = itemNames.get(diff.eqId) ?? `装備ID ${diff.eqId}`;
  return { diffs, localOnly, remoteOnly };
}

async function mapLimit(values, limit, callback) {
  const output = new Array(values.length);
  let cursor = 0;
  async function worker() {
    while (cursor < values.length) {
      const index = cursor++;
      output[index] = await callback(values[index], index);
    }
  }
  await Promise.all(Array.from({ length: limit }, () => worker()));
  return output;
}

const USE_ITEM_IDS = new Map([...USE_ITEM_NAMES].map(([name, id]) => [id, name]));

function formatValue(value, itemNames, field) {
  if (!Array.isArray(value)) return value === null ? "なし" : String(value);
  if (value.length === 0) return "なし";
  const names = field.includes("consumable") ? USE_ITEM_IDS : itemNames;
  return value.map((item) => `${names.get(item.id) ?? `ID${item.id}`}×${item.eq_count}`).join("、");
}

function markdown(report, itemNames) {
  const lines = [
    "# 明石の改修工廠 全件照合レポート",
    "",
    `生成日時: ${report.generatedAt}`,
    "",
    `- ローカル改修装備: ${report.summary.localEntries}件`,
    `- 明石の改修工廠掲載装備: ${report.summary.akashiEntries}件`,
    `- 正常取得: ${report.summary.fetched}件`,
    `- 詳細ページなし（改修経路なし）: ${report.summary.fetchErrors}件`,
    `- 差分項目: ${report.summary.differences}件（対象装備 ${report.summary.affectedEquipment}件）`,
    "",
    "## 差分",
    "",
    "| ID | 装備 | 経路 | 段階 | 項目 | ローカル | 明石の改修工廠 |",
    "|---:|---|---:|---|---|---|---|",
  ];
  for (const diff of report.differences) {
    lines.push(
      `| ${diff.eqId} | ${diff.name.replaceAll("|", "\\|")} | ${diff.pathIndex + 1} | ${diff.phase} | ${diff.field} | ${formatValue(diff.local, itemNames, diff.field)} | ${formatValue(diff.akashi, itemNames, diff.field)} |`,
    );
  }
  if (report.differences.length === 0) lines.push("| - | 差分なし | - | - | - | - | - |");
  lines.push(
    "",
    "## 片側にのみ存在",
    "",
    `- ローカルのみ: ${report.localOnly.join(", ") || "なし"}`,
    `- 明石の改修工廠のみ: ${report.remoteOnly.join(", ") || "なし"}`,
    "",
    "## 注記",
    "",
    "- 比較対象は燃料・弾薬・鋼材・ボーキ、★0～5、★6～9、★別追加素材、更新先、更新時の開発資材・改修資材・消費装備・特殊資材です。",
    "- 複数更新先は更新先装備IDで照合しています。",
    "- 「ローカルのみ」は改修経路を持たず、別レシピの消費素材参照のためだけに残しているエントリです。",
    `- 出典: ${BASE_URL}/`,
    "",
  );
  return `${lines.join("\n")}\n`;
}

async function main() {
  const useUpstream = process.argv.includes("--upstream");
  const [localText, referenceText, indexHtml] = await Promise.all([
    useUpstream ? fetch(EO_DATA_URL).then((response) => response.text()) : fs.readFile(DATA_PATH, "utf8"),
    fs.readFile(REFERENCE_PATH, "utf8"),
    fetch(`${BASE_URL}/`).then((response) => response.text()),
  ]);
  const localEntries = JSON.parse(localText);
  const reference = JSON.parse(referenceText);
  const itemNames = new Map(reference.items.map((item) => [item.id, item.name]));
  for (const [name, id] of USE_ITEM_NAMES) if (!itemNames.has(id)) itemNames.set(id, name);

  const remoteIds = [...new Set([...indexHtml.matchAll(/weaponWeeks\.w(\d+)=/g)].map((match) => Number(match[1])))].sort(
    (a, b) => a - b,
  );
  const fetchIds = [...new Set([...remoteIds, ...localEntries.map((entry) => entry.eq_id)])].sort((a, b) => a - b);
  const remoteEntries = await mapLimit(fetchIds, 12, async (id) => {
    const padded = String(id).padStart(3, "0");
    try {
      const response = await fetch(`${BASE_URL}/detail/w${padded}.html`);
      if (!response.ok) return { id, error: `HTTP ${response.status}` };
      return parseDetail(id, await response.text());
    } catch (error) {
      return { id, error: String(error) };
    }
  });

  const comparison = compare(localEntries, remoteEntries, itemNames);
  const dayNames = ["日", "月", "火", "水", "木", "金", "土"];
  const dayIds = { sun: 0, mon: 1, tue: 2, wed: 3, thu: 4, fry: 5, sat: 6 };
  const akashiWeeks = new Map(
    [...indexHtml.matchAll(/weaponWeeks\.w(\d+)="([^"]*)"/g)].map((match) => [
      Number(match[1]),
      [...new Set(match[2].split(",").filter((day) => day in dayIds).map((day) => dayIds[day]))].sort(),
    ]),
  );
  for (const item of localEntries) {
    if (!akashiWeeks.has(item.eq_id) || item.improvement.length === 0) continue;
    const localDays = [
      ...new Set(item.improvement.flatMap((improvement) => improvement.helpers.flatMap((helper) => helper.days))),
    ].sort();
    const remoteDays = akashiWeeks.get(item.eq_id);
    if (JSON.stringify(localDays) !== JSON.stringify(remoteDays)) {
      comparison.diffs.push({
        eqId: item.eq_id,
        pathIndex: 0,
        phase: "曜日",
        field: "availability_days",
        local: localDays.map((day) => dayNames[day]).join("・") || "なし",
        akashi: remoteDays.map((day) => dayNames[day]).join("・") || "なし",
        name: itemNames.get(item.eq_id) ?? `装備ID ${item.eq_id}`,
      });
    }
  }
  const localIds = new Set(localEntries.map((entry) => entry.eq_id));
  const listedIds = new Set(remoteIds);
  const notListedOnAkashi = [...localIds].filter((id) => !listedIds.has(id)).sort((a, b) => a - b);
  const missingLocally = [...listedIds].filter((id) => !localIds.has(id)).sort((a, b) => a - b);
  const report = {
    generatedAt: new Date().toISOString(),
    source: BASE_URL,
    input: useUpstream ? EO_DATA_URL : DATA_PATH,
    summary: {
      localEntries: localEntries.length,
      akashiEntries: remoteIds.length,
      fetched: remoteEntries.filter((entry) => !entry.error).length,
      fetchErrors: remoteEntries.filter((entry) => entry.error).length,
      differences: comparison.diffs.length,
      affectedEquipment: new Set(comparison.diffs.map((diff) => diff.eqId)).size,
    },
    differences: comparison.diffs,
    localOnly: notListedOnAkashi,
    remoteOnly: missingLocally,
    fetchErrors: remoteEntries.filter((entry) => entry.error),
    unknownConsumableNames: [...unknownConsumableNames].sort(),
    remoteEntries,
  };

  await fs.mkdir(path.dirname(REPORT_JSON), { recursive: true });
  await Promise.all([
    fs.writeFile(REPORT_JSON, `${JSON.stringify(report, null, 2)}\n`),
    fs.writeFile(REPORT_MD, markdown(report, itemNames)),
  ]);
  console.log(JSON.stringify(report.summary, null, 2));
  console.log(`JSON: ${REPORT_JSON}`);
  console.log(`Markdown: ${REPORT_MD}`);
}

await main();
