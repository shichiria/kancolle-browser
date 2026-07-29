import { execFileSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const DATA_PATH = path.join(ROOT, "src-tauri", "data", "equipment_upgrades.json");
const REFERENCE_PATH = path.join(ROOT, "src", "data", "kcWebReference.json");
const REPORT_MD = path.join(ROOT, ".reports", "akashi-material-cost-differences.md");
const REPORT_JSON = path.join(ROOT, ".reports", "akashi-material-cost-differences.json");

const USE_ITEM_NAMES = new Map([
  [2, "高速建造材"],
  [57, "勲章"],
  [70, "熟練搭乗員"],
  [75, "新型砲熕兵装資材"],
  [77, "新型航空兵装資材"],
  [78, "戦闘詳報"],
  [91, "緊急修理資材"],
  [92, "新型噴進装備開発資材"],
  [94, "新型兵装資材"],
  [95, "潜水艦補給物資"],
  [100, "海外艦最新技術"],
  [104, "工廠資源"],
]);

function target(pathData) {
  return pathData?.convert?.id_after ?? null;
}

function pairPaths(before = [], after = []) {
  const pairs = [];
  const usedBefore = new Set();
  const usedAfter = new Set();

  // First pair paths whose update target is unchanged.
  for (let beforeIndex = 0; beforeIndex < before.length; beforeIndex += 1) {
    const afterIndex = after.findIndex(
      (candidate, index) =>
        !usedAfter.has(index) && target(candidate) === target(before[beforeIndex]),
    );
    if (afterIndex >= 0) {
      pairs.push({ beforeIndex, afterIndex });
      usedBefore.add(beforeIndex);
      usedAfter.add(afterIndex);
    }
  }

  // A newly-added update target often replaces a formerly non-converting path.
  const remainingBefore = before.map((_, index) => index).filter((index) => !usedBefore.has(index));
  const remainingAfter = after.map((_, index) => index).filter((index) => !usedAfter.has(index));
  const pairedCount = Math.min(remainingBefore.length, remainingAfter.length);
  for (let index = 0; index < pairedCount; index += 1) {
    pairs.push({
      beforeIndex: remainingBefore[index],
      afterIndex: remainingAfter[index],
    });
    usedBefore.add(remainingBefore[index]);
    usedAfter.add(remainingAfter[index]);
  }

  for (const beforeIndex of before.map((_, index) => index).filter((index) => !usedBefore.has(index))) {
    pairs.push({ beforeIndex, afterIndex: null });
  }
  for (const afterIndex of after.map((_, index) => index).filter((index) => !usedAfter.has(index))) {
    pairs.push({ beforeIndex: null, afterIndex });
  }
  return pairs;
}

function itemList(items, names) {
  if (!items?.length) return "なし";
  return [...items]
    .sort((a, b) => a.id - b.id)
    .map((item) => `${names.get(item.id) ?? `ID ${item.id}`} [${item.id}] ×${item.eq_count}`)
    .join("、");
}

function materialPair(cost) {
  if (!cost) return "—";
  return `${cost.devmats ?? 0}/${cost.devmats_sli ?? 0}`;
}

function screwPair(cost) {
  if (!cost) return "—";
  return `${cost.screws ?? 0}/${cost.screws_sli ?? 0}`;
}

function extraByLevel(costs, level, field) {
  const aggregated = new Map();
  for (const extra of costs?.extra ?? []) {
    if (!extra.levels?.includes(level)) continue;
    for (const item of extra[field] ?? []) {
      aggregated.set(item.id, (aggregated.get(item.id) ?? 0) + item.eq_count);
    }
  }
  return [...aggregated].map(([id, eq_count]) => ({ id, eq_count }));
}

function routeLabel(beforePath, afterPath, names) {
  const beforeTarget = target(beforePath);
  const afterTarget = target(afterPath);
  const label = (id) => (id ? `${names.get(id) ?? "装備"} [${id}]` : "更新なし");
  return beforeTarget === afterTarget
    ? label(afterTarget)
    : `${label(beforeTarget)} → ${label(afterTarget)}`;
}

function addDiff(rows, context, phase, category, before, after) {
  if (before === after) return;
  rows.push({ ...context, phase, category, before, after });
}

function comparePath(rows, context, beforePath, afterPath, equipmentNames) {
  const beforeCosts = beforePath?.costs ?? null;
  const afterCosts = afterPath?.costs ?? null;
  for (const [phaseKey, phaseName] of [
    ["p1", "★0～5"],
    ["p2", "★6～9"],
    ["conv", "更新時"],
  ]) {
    const beforePhase = beforeCosts?.[phaseKey] ?? null;
    const afterPhase = afterCosts?.[phaseKey] ?? null;
    addDiff(
      rows,
      context,
      phaseName,
      "開発資材（通常/確実化）",
      materialPair(beforePhase),
      materialPair(afterPhase),
    );
    addDiff(
      rows,
      context,
      phaseName,
      "改修資材（通常/確実化）",
      screwPair(beforePhase),
      screwPair(afterPhase),
    );
    addDiff(
      rows,
      context,
      phaseName,
      "消費装備",
      beforePhase ? itemList(beforePhase.equips, equipmentNames) : "—",
      afterPhase ? itemList(afterPhase.equips, equipmentNames) : "—",
    );
    addDiff(
      rows,
      context,
      phaseName,
      "特殊素材",
      beforePhase ? itemList(beforePhase.consumable, USE_ITEM_NAMES) : "—",
      afterPhase ? itemList(afterPhase.consumable, USE_ITEM_NAMES) : "—",
    );
  }

  for (const level of [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]) {
    const phase = `★${level - 1}→★${level}追加`;
    addDiff(
      rows,
      context,
      phase,
      "消費装備",
      itemList(extraByLevel(beforeCosts, level, "equips"), equipmentNames),
      itemList(extraByLevel(afterCosts, level, "equips"), equipmentNames),
    );
    addDiff(
      rows,
      context,
      phase,
      "特殊素材",
      itemList(extraByLevel(beforeCosts, level, "consumable"), USE_ITEM_NAMES),
      itemList(extraByLevel(afterCosts, level, "consumable"), USE_ITEM_NAMES),
    );
  }
}

function escapeCell(value) {
  return String(value).replaceAll("|", "\\|").replaceAll("\n", "<br>");
}

function markdown(report) {
  const lines = [
    "# 改修コスト変更差分",
    "",
    `生成日時: ${report.generatedAt}`,
    "",
    `- 変更対象装備: ${report.summary.equipment}件`,
    `- 変更対象経路: ${report.summary.paths}件`,
    `- 差分項目: ${report.summary.rows}件`,
    `- 開発資材: ${report.summary.categories["開発資材（通常/確実化）"] ?? 0}件`,
    `- 改修資材: ${report.summary.categories["改修資材（通常/確実化）"] ?? 0}件`,
    `- 消費装備: ${report.summary.categories["消費装備"] ?? 0}件`,
    `- 特殊素材: ${report.summary.categories["特殊素材"] ?? 0}件`,
    "",
    "- 修正前: Git管理中の更新前 `src-tauri/data/equipment_upgrades.json`",
    "- 修正後: 現在の `src-tauri/data/equipment_upgrades.json`",
    "- `—` は、その改修段階または経路自体が追加・削除されたことを表します。",
    "",
    "表記 `通常/確実化` は、確実化しない場合と確実化した場合の必要数です。",
    "",
    "| ID | 装備 | 経路 | 段階 | 項目 | 修正前 | 修正後 |",
    "|---:|---|---|---|---|---|---|",
  ];
  for (const row of report.rows) {
    lines.push(
      `| ${row.eqId} | ${escapeCell(row.name)} | ${escapeCell(row.route)} | ${row.phase} | ${row.category} | ${escapeCell(row.before)} | ${escapeCell(row.after)} |`,
    );
  }
  return `${lines.join("\n")}\n`;
}

async function main() {
  const [afterText, referenceText] = await Promise.all([
    fs.readFile(DATA_PATH, "utf8"),
    fs.readFile(REFERENCE_PATH, "utf8"),
  ]);
  const beforeText = execFileSync(
    "git",
    ["show", "HEAD:src-tauri/data/equipment_upgrades.json"],
    { cwd: ROOT, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
  );
  const before = JSON.parse(beforeText);
  const after = JSON.parse(afterText);
  const reference = JSON.parse(referenceText);
  const equipmentNames = new Map(reference.items.map((item) => [item.id, item.name]));
  const beforeById = new Map(before.map((item) => [item.eq_id, item]));
  const afterById = new Map(after.map((item) => [item.eq_id, item]));
  const ids = [...new Set([...beforeById.keys(), ...afterById.keys()])].sort((a, b) => a - b);
  const rows = [];
  const changedPaths = new Set();

  for (const eqId of ids) {
    const beforeEntry = beforeById.get(eqId);
    const afterEntry = afterById.get(eqId);
    const pairs = pairPaths(beforeEntry?.improvement, afterEntry?.improvement);
    for (const pair of pairs) {
      const beforePath =
        pair.beforeIndex === null ? null : beforeEntry?.improvement[pair.beforeIndex] ?? null;
      const afterPath =
        pair.afterIndex === null ? null : afterEntry?.improvement[pair.afterIndex] ?? null;
      const rowStart = rows.length;
      comparePath(
        rows,
        {
          eqId,
          name: equipmentNames.get(eqId) ?? `装備ID ${eqId}`,
          route: routeLabel(beforePath, afterPath, equipmentNames),
          beforePathIndex: pair.beforeIndex,
          afterPathIndex: pair.afterIndex,
        },
        beforePath,
        afterPath,
        equipmentNames,
      );
      if (rows.length > rowStart) {
        changedPaths.add(`${eqId}:${pair.beforeIndex ?? "-"}:${pair.afterIndex ?? "-"}`);
      }
    }
  }

  const categories = {};
  for (const row of rows) categories[row.category] = (categories[row.category] ?? 0) + 1;
  const report = {
    generatedAt: new Date().toISOString(),
    before: "git HEAD:src-tauri/data/equipment_upgrades.json",
    after: DATA_PATH,
    summary: {
      equipment: new Set(rows.map((row) => row.eqId)).size,
      paths: changedPaths.size,
      rows: rows.length,
      categories,
    },
    rows,
  };
  await fs.mkdir(path.dirname(REPORT_MD), { recursive: true });
  await Promise.all([
    fs.writeFile(REPORT_MD, markdown(report)),
    fs.writeFile(REPORT_JSON, `${JSON.stringify(report, null, 2)}\n`),
  ]);
  console.log(JSON.stringify(report.summary, null, 2));
  console.log(`Markdown: ${REPORT_MD}`);
  console.log(`JSON: ${REPORT_JSON}`);
}

await main();
