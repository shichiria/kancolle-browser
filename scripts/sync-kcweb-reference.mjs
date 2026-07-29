import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { build } from "esbuild";

const root = resolve(import.meta.dirname, "..");
const kcWebRoot = join(root, "REF", "kc-web");
const outputPath = join(root, "src", "data", "kcWebReference.json");
const masterUrl =
  "https://firebasestorage.googleapis.com/v0/b/development-74af0.appspot.com/o/master.json?alt=media";

const tempDir = await mkdtemp(join(tmpdir(), "kancolle-kcweb-"));
const bundledBonusPath = join(tempDir, "item-bonus.cjs");
const bundledItemMasterPath = join(tempDir, "item-master.cjs");
const bundledItemPath = join(tempDir, "item.cjs");

try {
  await build({
    entryPoints: [join(kcWebRoot, "src", "classes", "item", "ItemBonus.ts")],
    bundle: true,
    platform: "node",
    format: "cjs",
    outfile: bundledBonusPath,
  });
  await build({
    entryPoints: [join(kcWebRoot, "src", "classes", "item", "itemMaster.ts")],
    bundle: true,
    platform: "node",
    format: "cjs",
    outfile: bundledItemMasterPath,
  });
  await build({
    entryPoints: [join(kcWebRoot, "src", "classes", "item", "item.ts")],
    bundle: true,
    platform: "node",
    format: "cjs",
    outfile: bundledItemPath,
  });

  const require = createRequire(import.meta.url);
  const itemBonus = require(bundledBonusPath).default;
  const ItemMaster = require(bundledItemMasterPath).default;
  const Item = require(bundledItemPath).default;
  const response = await fetch(masterUrl);
  if (!response.ok) {
    throw new Error(`kc-web master download failed: HTTP ${response.status}`);
  }
  const master = await response.json();

  const storeSource = await readFile(
    join(kcWebRoot, "src", "store", "index.ts"),
    "utf8",
  );
  const siteVersion = storeSource.match(/siteVersion:\s*'([^']+)'/)?.[1];
  if (!siteVersion) throw new Error("kc-web siteVersion was not found");
  const commit = execFileSync(
    "git",
    ["-C", kcWebRoot, "rev-parse", "HEAD"],
    { encoding: "utf8" },
  ).trim();

  const reference = {
    metadata: {
      kcWebVersion: siteVersion,
      kcWebCommit: commit,
      generatedAt: new Date().toISOString(),
      masterUrl,
    },
    items: master.items.map((rawItem) => {
      const item = new ItemMaster(rawItem);
      const equipped = new Item({ master: item, slot: item.airbaseMaxSlot });
      return {
        ...rawItem,
        avoidId: item.avoidId,
        sortieAntiAir: item.sortieAntiAir,
        defenseAntiAir: item.defenseAntiAir,
        isSpecial: item.isSpecial,
        isFighter: item.isFighter,
        isAttacker: item.isAttacker,
        isAswPlane: item.isAswPlane,
        isABAttacker: item.isABAttacker,
        isBakusen: item.isBakusen,
        isRocket: item.isRocket,
        isRecon: item.isRecon,
        enabledAttackLandBase: item.enabledAttackLandBase,
        isStrictDepthCharge: item.isStrictDepthCharge,
        isTorpedoAttacker: item.isTorpedoAttacker,
        isNightAircraftItem: item.isNightAircraftItem,
        isLateModelTorpedo: item.isLateModelTorpedo,
        eventTags: item.bonuses.map(
          ({ key, text, isOnlyAB, isOnlyShip }) => ({
            key,
            text,
            isOnlyAB,
            isOnlyShip,
          }),
        ),
        tp: equipped.tp,
        tp2: equipped.tp2,
        tp3: equipped.tp3,
        reconCorr: equipped.reconCorr,
        reconCorrDefense: equipped.reconCorrDefense,
        wikiUrl: ItemMaster.getWikiURL(item),
      };
    }),
    enemies: master.enemies.map(
      ({ id, type, name, hp, aa, armor, speed }) => ({
        id,
        type,
        name,
        hp,
        aa,
        armor,
        speed,
      }),
    ),
    ships: master.ships.map(
      ({ id, type, type2, orig, ver, name }) => ({
        id,
        type,
        type2,
        orig,
        ver,
        name,
      }),
    ),
    equipExslotShip: master.api_mst_equip_exslot_ship,
    equipShip: master.api_mst_equip_ship,
    bonuses: itemBonus.bonusData,
  };

  await writeFile(outputPath, `${JSON.stringify(reference)}\n`, "utf8");
  console.log(
    `Wrote ${outputPath} (${reference.items.length} items, ${reference.bonuses.length} bonus groups)`,
  );
} finally {
  await rm(tempDir, { recursive: true, force: true });
}
