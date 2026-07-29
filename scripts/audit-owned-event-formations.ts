import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import LZString from "lz-string";
import {
  getAllEventFormationLinks,
  type EventFormationAuditEntry,
} from "../src/data/eventFormations";
import reference from "../src/data/kcWebReference.json";
import {
  adaptOwnedFormationData,
  type OwnedFormationInventory,
} from "../src/utils/ownedFormation";

interface RawApiDump {
  response_body?: {
    api_data?: {
      api_basic?: { api_level?: number };
      api_slot_item?: Array<{
        api_id: number;
        api_slotitem_id: number;
        api_level?: number;
        api_alv?: number;
      }>;
    };
  };
}

interface SavedItem {
  i: number;
  r?: number;
  l?: number;
}

interface SavedFormation {
  airbaseInfo?: { airbases?: Array<{ items?: SavedItem[] }> };
  fleetInfo?: {
    fleets?: Array<{
      ships?: Array<{ is?: SavedItem[]; ex?: SavedItem }>;
    }>;
  };
}

interface AuditResult {
  formation: EventFormationAuditEntry;
  ownedUrl: string;
  assigned: number;
  missing: number;
  changed: number;
  replacements: Array<{
    sourceId: number;
    source: string;
    selectedId: number;
    selected: string;
  }>;
  missingItems: Array<{ sourceId: number; source: string }>;
}

function option(name: string): string | undefined {
  const prefix = `--${name}=`;
  return process.argv.find((value) => value.startsWith(prefix))?.slice(prefix.length);
}

function decodeFormation(compressed: string): SavedFormation {
  const json = LZString.decompressFromEncodedURIComponent(compressed);
  if (!json) throw new Error("Formation data could not be decompressed");
  return JSON.parse(json) as SavedFormation;
}

function formationItems(formation: SavedFormation): SavedItem[] {
  const items: SavedItem[] = [];
  for (const base of formation.airbaseInfo?.airbases ?? []) {
    items.push(...(base.items ?? []));
  }
  for (const fleet of formation.fleetInfo?.fleets ?? []) {
    for (const ship of fleet.ships ?? []) {
      items.push(...(ship.is ?? []));
      if (ship.ex) items.push(ship.ex);
    }
  }
  return items;
}

async function resolveFormation(sourceUrl: string): Promise<string> {
  const response = await fetch(sourceUrl, { redirect: "manual" });
  const location = response.headers.get("location");
  if (!location) {
    throw new Error(`${sourceUrl} did not return a formation redirect`);
  }
  const data = new URL(location).searchParams.get("data");
  if (!data) throw new Error(`${sourceUrl} did not include formation data`);
  return data;
}

function itemName(id: number): string {
  return reference.items.find((item) => item.id === id)?.name ?? `ID ${id}`;
}

async function auditOne(
  formation: EventFormationAuditEntry,
  inventory: OwnedFormationInventory,
): Promise<AuditResult> {
  const compressed = await resolveFormation(formation.url);
  const sourceItems = formationItems(decodeFormation(compressed));
  const adapted = adaptOwnedFormationData(compressed, inventory);
  const resultData = new URL(adapted.url).searchParams.get("data") ?? "";
  const selectedItems = formationItems(decodeFormation(resultData));
  const replacements = sourceItems.flatMap((source, index) => {
    const selected = selectedItems[index];
    if (
      source.i <= 0 ||
      !selected ||
      selected.i <= 0 ||
      source.i === selected.i
    ) {
      return [];
    }
    return [{
      sourceId: source.i,
      source: itemName(source.i),
      selectedId: selected.i,
      selected: itemName(selected.i),
    }];
  });
  const missingItems = sourceItems.flatMap((source, index) => {
    const selected = selectedItems[index];
    if (source.i <= 0 || (selected && selected.i > 0)) return [];
    return [{ sourceId: source.i, source: itemName(source.i) }];
  });
  return {
    formation,
    ownedUrl: adapted.url,
    assigned: adapted.assigned,
    missing: adapted.missing,
    changed: replacements.length,
    replacements,
    missingItems,
  };
}

const rawPath = option("raw");
if (!rawPath) {
  throw new Error(
    "Use --raw=<api_get_member_require_info.json> to provide a local inventory snapshot",
  );
}
const outputPath = resolve(option("output") ?? ".reports/event-formation-audit.json");
const markdownPath = outputPath.replace(/\.json$/i, ".md");
const raw = JSON.parse(await readFile(resolve(rawPath), "utf8")) as RawApiDump;
const apiData = raw.response_body?.api_data;
const slotItems = apiData?.api_slot_item;
if (!slotItems) throw new Error("The raw API dump did not contain api_slot_item");
const inventory: OwnedFormationInventory = {
  hq_level: apiData.api_basic?.api_level ?? 120,
  items: slotItems.map((item) => ({
    instance_id: item.api_id,
    master_id: item.api_slotitem_id,
    remodel: item.api_level ?? 0,
    proficiency: item.api_alv ?? 0,
  })),
};

const results: AuditResult[] = [];
for (const formation of getAllEventFormationLinks()) {
  results.push(await auditOne(formation, inventory));
}

const replacementCounts = new Map<string, number>();
for (const result of results) {
  for (const replacement of result.replacements) {
    const key = `${replacement.source} → ${replacement.selected}`;
    replacementCounts.set(key, (replacementCounts.get(key) ?? 0) + 1);
  }
}
const summary = {
  generatedAt: new Date().toISOString(),
  formations: results.length,
  complete: results.filter((result) => result.missing === 0).length,
  withMissing: results.filter((result) => result.missing > 0).length,
  assigned: results.reduce((sum, result) => sum + result.assigned, 0),
  missing: results.reduce((sum, result) => sum + result.missing, 0),
};
const report = {
  summary,
  results,
  replacementCounts: [...replacementCounts]
    .sort((left, right) => right[1] - left[1])
    .map(([replacement, count]) => ({ replacement, count })),
};

const markdown = [
  "# 所持装備によるイベント編成監査",
  "",
  `生成日時: ${summary.generatedAt}`,
  "",
  `全${summary.formations}編成 / 完全編成${summary.complete} / 不足あり${summary.withMissing} / 不足${summary.missing}枠`,
  "",
  "| 海域 | 段階 | 編成 | 割当 | 不足 | 変更 |",
  "|---|---|---|---:|---:|---:|",
  ...results.map(
    (result) =>
      `| E${result.formation.mapNo} | ${result.formation.stageId} | ${result.formation.label} | ${result.assigned} | ${result.missing} | ${result.changed} |`,
  ),
  "",
  "## 多い置換",
  "",
  ...report.replacementCounts
    .slice(0, 50)
    .map((replacement) => `- ${replacement.replacement}: ${replacement.count}枠`),
  "",
  "完全な制空シミュリンクは同名JSONの `results[].ownedUrl` に保存する。",
  "",
].join("\n");

await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
await writeFile(markdownPath, markdown, "utf8");
console.log(JSON.stringify(summary));
