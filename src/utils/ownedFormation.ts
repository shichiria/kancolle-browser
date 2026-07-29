import LZString from "lz-string";
import referenceJson from "../data/kcWebReference.json";

export interface OwnedFormationItem {
  instance_id: number;
  master_id: number;
  remodel: number;
  proficiency: number;
}

export interface OwnedFormationInventory {
  hq_level: number;
  items: OwnedFormationItem[];
}

export interface OwnedFormationResult {
  url: string;
  assigned: number;
  missing: number;
  missing_items: Array<{ master_id: number; name: string; count: number }>;
}

interface MasterItem {
  id: number;
  type: number;
  itype: number;
  name: string;
  fire?: number;
  antiAir?: number;
  torpedo?: number;
  bomber?: number;
  armor?: number;
  asw?: number;
  antiBomber?: number;
  interception?: number;
  scout?: number;
  accuracy?: number;
  avoid2?: number;
  avoidId?: number;
  radius?: number;
  range?: number;
  sortieAntiAir?: number;
  defenseAntiAir?: number;
  isSpecial?: boolean;
  isFighter?: boolean;
  isAttacker?: boolean;
  isAswPlane?: boolean;
  isABAttacker?: boolean;
  isBakusen?: boolean;
  isRocket?: boolean;
  isRecon?: boolean;
  enabledAttackLandBase?: boolean;
  isStrictDepthCharge?: boolean;
  isTorpedoAttacker?: boolean;
  isNightAircraftItem?: boolean;
  isLateModelTorpedo?: boolean;
  tp?: number;
  tp2?: number;
  tp3?: number;
  reconCorr?: number;
  reconCorrDefense?: number;
  eventTags?: Array<{
    key: string;
    text: string[];
    isOnlyAB: boolean;
    isOnlyShip: boolean;
  }>;
}

interface MasterShip {
  id: number;
  type: number;
  type2: number;
  orig: number;
  ver: number;
}

interface MasterEnemy {
  id: number;
  type: number;
  name: string;
  hp: number;
  aa: number;
  armor: number;
  speed: number;
}

interface BonusStatus {
  firePower?: number;
  torpedo?: number;
  antiAir?: number;
  armor?: number;
  asw?: number;
  scout?: number;
  avoid?: number;
  accuracy?: number;
  bomber?: number;
  range?: number;
}

interface BonusRule {
  bonus: BonusStatus;
  shipBase?: number[];
  shipClass?: number[];
  shipCountry?: number[];
  shipType?: number[];
  shipId?: number[];
  requiresType?: number[];
  requiresSR?: number;
  requiresAR?: number;
  requiresAccR?: number;
  requiresId?: number[];
  requiresIdNum?: number;
  requiresIdLevel?: number;
  requiresId2?: number[];
  requiresIdLevel2?: number;
  remodel?: number;
  num?: number;
}

interface BonusGroup {
  types?: number[];
  ids?: number[];
  bonuses: BonusRule[];
}

interface KcWebReference {
  items: MasterItem[];
  ships: MasterShip[];
  enemies: MasterEnemy[];
  bonuses: BonusGroup[];
  equipShip: Record<string, { api_equip_type: Record<string, number[] | null> }>;
  equipExslotShip: Record<
    string,
    {
      api_ship_ids: Record<string, 1> | null;
      api_stypes: Record<string, 1> | null;
      api_ctypes: Record<string, 1> | null;
      api_req_level?: number;
    }
  >;
}

interface SavedItem {
  i: number;
  l?: number;
  r?: number;
  s?: number;
}

interface SavedShip {
  i: number;
  is: SavedItem[];
  ex?: SavedItem;
}

interface SavedFormation {
  airbaseInfo?: {
    airbases?: Array<{ items?: SavedItem[]; mode?: number }>;
  };
  fleetInfo?: {
    fleets?: Array<{ ships?: SavedShip[] }>;
    admiralLevel?: number;
  };
  battleInfo?: {
    fleets?: Array<{
      enemies?: Array<{ i: number; es?: boolean }>;
      area?: number;
      nodeName?: string;
    }>;
  };
  [key: string]: unknown;
}

interface EquippedItem {
  master: MasterItem;
  remodel: number;
  proficiency: number;
}

interface TargetSlot {
  saved: SavedItem;
  target: EquippedItem;
  ship?: MasterShip;
  shipItems?: EquippedItem[];
  shipItemIndex?: number;
  isExSlot: boolean;
  airbaseMode?: number;
  airbaseEnemies?: MasterEnemy[];
  airbaseItems?: EquippedItem[];
  airbaseItemIndex?: number;
  enemies?: MasterEnemy[];
  area?: number;
  nodeName?: string;
  slotSize: number;
}

type Metric =
  | "firePower"
  | "torpedo"
  | "antiAir"
  | "bomber"
  | "armor"
  | "asw"
  | "scout"
  | "accuracy"
  | "avoid"
  | "range"
  | "antiBomber"
  | "interception"
  | "radius";

type MetricVector = Record<Metric, number>;

const reference = referenceJson as unknown as KcWebReference;
const itemById = new Map(reference.items.map((item) => [item.id, item]));
const shipById = new Map(reference.ships.map((ship) => [ship.id, ship]));
const enemyById = new Map(reference.enemies.map((enemy) => [enemy.id, enemy]));
const PROFICIENCY = [0, 10, 25, 40, 55, 70, 85, 100, 120];
const FIGHTERS = new Set([6, 45, 48, 56]);
const RECONNAISSANCES = new Set([9, 10, 41, 49]);
const AB_ATTACKERS = new Set([47, 53]);
const STRICT_DEPTH_CHARGE = new Set([226, 227, 378, 439, 488]);
const BAKUSEN = new Set([60, 154, 219, 447, 487]);
const LATE_MODEL_TORPEDO = new Set([213, 214, 383, 441, 443, 457, 461, 512]);
const SUBMARINE_SCOUT = new Set([522, 523]);
const TOKU_4_TANK = new Set([525, 526]);
const ROCKET_INTERCEPTOR = new Set([350, 351, 352]);
const SMOKE_GENERATOR = new Set([500, 501]);
const COMMAND_FACILITY = new Set([107, 272, 413]);
const NIGHT_AIRCREW = new Set([258, 259]);
const DECK_AIRCREW = new Set([478]);
const TORPEDO_LOOKOUT = new Set([412]);
const STANDARD_LOOKOUT = new Set([129]);
const ROCKET_BARRAGE = new Set([274]);
const ARMED_LANDING_CRAFT = new Set([408, 409]);
const CHIHA_FAMILY = new Set([494, 495, 497, 498, 499]);
const EXPANDED_ITEM_TYPE = new Set([16, 17, 21, 23, 27, 28, 36, 39, 43, 44]);
const ALL_METRICS: Metric[] = [
  "firePower",
  "torpedo",
  "antiAir",
  "bomber",
  "armor",
  "asw",
  "scout",
  "accuracy",
  "avoid",
  "range",
  "antiBomber",
  "interception",
  "radius",
];
const CATEGORY_METRICS: Record<number, Metric[]> = {
  1: ["firePower", "accuracy", "antiAir", "armor", "range"],
  2: ["firePower", "accuracy", "antiAir", "armor", "range"],
  3: ["firePower", "accuracy", "antiAir", "armor", "range"],
  4: ["firePower", "accuracy", "antiAir", "armor", "range"],
  5: ["torpedo", "accuracy", "armor"],
  6: ["antiAir", "accuracy", "avoid", "scout", "radius"],
  7: ["bomber", "antiAir", "accuracy", "asw", "avoid", "scout", "radius"],
  8: ["torpedo", "antiAir", "accuracy", "asw", "avoid", "scout", "radius"],
  9: ["scout", "accuracy", "avoid", "antiAir", "radius"],
  10: ["scout", "accuracy", "avoid", "antiAir", "radius"],
  11: ["bomber", "antiAir", "accuracy", "avoid", "scout", "radius"],
  12: ["antiAir", "accuracy", "scout", "firePower", "armor"],
  13: ["antiAir", "accuracy", "scout", "firePower", "armor"],
  14: ["asw", "accuracy", "scout"],
  15: ["asw", "accuracy"],
  21: ["antiAir", "accuracy", "firePower", "armor", "avoid"],
  22: ["torpedo", "accuracy", "avoid", "scout"],
  24: ["firePower", "antiAir", "armor", "avoid"],
  32: ["torpedo", "accuracy", "avoid"],
  40: ["asw", "accuracy", "scout"],
  41: ["scout", "accuracy", "avoid", "antiAir", "radius"],
  45: ["antiAir", "avoid", "accuracy", "scout", "radius"],
  47: ["torpedo", "bomber", "antiAir", "asw", "accuracy", "avoid", "scout", "radius"],
  48: ["antiAir", "antiBomber", "interception", "radius"],
  49: ["scout", "accuracy", "avoid", "antiAir", "radius"],
  53: ["torpedo", "bomber", "antiAir", "accuracy", "avoid", "scout", "radius"],
  56: ["antiAir", "accuracy", "avoid", "scout", "radius"],
  57: ["bomber", "antiAir", "accuracy", "avoid", "scout", "radius"],
};

const zeroVector = (): MetricVector => ({
  firePower: 0,
  torpedo: 0,
  antiAir: 0,
  bomber: 0,
  armor: 0,
  asw: 0,
  scout: 0,
  accuracy: 0,
  avoid: 0,
  range: 0,
  antiBomber: 0,
  interception: 0,
  radius: 0,
});

function isBattleship(enemy: MasterEnemy): boolean {
  return [8, 9, 10].includes(enemy.type);
}

function isCarrier(enemy: MasterEnemy): boolean {
  return [7, 11].includes(enemy.type);
}

function isInstallation(enemy: MasterEnemy): boolean {
  return (
    enemy.speed === 0 ||
    enemy.type === 17 ||
    /砲台|集積|離島|飛行場|港湾|泊地水鬼|トーチカ|対空小鬼/.test(
      enemy.name,
    )
  );
}

function airbaseEffectiveAttack(
  item: EquippedItem,
  enemy: MasterEnemy,
): number {
  const vector = itemVector(item);
  if (enemy.type === 13) {
    return item.master.itype === 47 ? vector.asw : 0;
  }
  if (isInstallation(enemy)) {
    return vector.bomber * (item.master.id === 459 ? 0.9 : 1);
  }

  let attack = vector.torpedo;
  if (item.master.id === 224 && enemy.type === 2) return 25;
  if (item.master.id === 405 && enemy.type === 2) return attack * 1.1;
  if (item.master.id === 406 && isBattleship(enemy)) return attack * 1.5;
  if (item.master.id === 562 && enemy.type === 2) return attack * 1.25;
  if (
    item.master.id === 444 &&
    (isBattleship(enemy) || isCarrier(enemy))
  ) {
    return attack * 1.13;
  }
  if (
    (item.master.id === 444 || item.master.id === 484) &&
    enemy.type !== 15
  ) {
    return attack * 1.15;
  }
  if (
    item.master.id === 454 &&
    [2, 3, 5, 7].includes(enemy.type)
  ) {
    return attack * 1.16;
  }
  if (
    item.master.id === 454 &&
    (enemy.type === 7 || isBattleship(enemy))
  ) {
    return attack * 1.14;
  }
  if (item.master.id !== 459) return attack;

  if (enemy.type === 2) attack *= 1.9;
  else if ([3, 4, 16].includes(enemy.type)) attack *= 1.75;
  else if ([5, 6].includes(enemy.type)) attack *= 1.6;
  else if (enemy.type === 7 || isBattleship(enemy) || enemy.type === 15) {
    attack *= 1.3;
  } else if (enemy.type === 12) {
    attack *= 1.75;
  }
  return attack;
}

function itemVector(
  item: EquippedItem,
  airbaseMode?: number,
): MetricVector {
  const { master, remodel } = item;
  const vector = zeroVector();
  vector.firePower = master.fire ?? 0;
  vector.torpedo = master.torpedo ?? 0;
  vector.antiAir = master.antiAir ?? 0;
  vector.bomber = master.bomber ?? 0;
  vector.armor = master.armor ?? 0;
  vector.asw = master.asw ?? 0;
  vector.scout = master.scout ?? 0;
  vector.accuracy = master.accuracy ?? 0;
  vector.avoid = master.avoid2 ?? 0;
  vector.range = master.range ?? 0;
  vector.antiBomber = master.antiBomber ?? 0;
  vector.interception = master.interception ?? 0;
  vector.radius = master.radius ?? 0;

  if (master.type === 3) vector.firePower += 1.5 * Math.sqrt(remodel);
  else if ([1, 2, 4, 18, 19, 21, 24, 29, 32, 34, 35, 36, 37, 39, 42, 46, 54].includes(master.type)) {
    if ([10, 66, 220, 275, 464].includes(master.id)) vector.firePower += 0.2 * remodel;
    else if ([12, 234, 247, 467].includes(master.id)) vector.firePower += 0.3 * remodel;
    else vector.firePower += Math.sqrt(remodel);
  } else if ([14, 15, 40].includes(master.type) && !STRICT_DEPTH_CHARGE.has(master.id)) {
    vector.firePower += 0.75 * Math.sqrt(remodel);
  } else if ((master.type === 7 && !BAKUSEN.has(master.id)) || master.type === 8) {
    vector.firePower += 0.2 * remodel;
  }

  if (master.type === 8) vector.torpedo += 0.2 * remodel;
  else if (AB_ATTACKERS.has(master.type) && master.itype !== 47) {
    vector.torpedo += (master.id === 484 ? 0.75 : 0.7) * Math.sqrt(remodel);
  } else if (master.type === 5 || master.type === 21) vector.torpedo += 1.2 * Math.sqrt(remodel);
  else if (master.type === 32) vector.torpedo += 0.2 * remodel;

  if ((master.type === 7 && !BAKUSEN.has(master.id)) || master.type === 11) {
    vector.bomber += 0.2 * remodel;
  }
  if (master.id === 486 || master.id === 487) vector.antiAir += 0.3 * remodel;
  else if (FIGHTERS.has(master.type)) vector.antiAir += 0.2 * remodel;
  else if (master.type === 7 && BAKUSEN.has(master.id)) vector.antiAir += 0.25 * remodel;
  else if (AB_ATTACKERS.has(master.type)) vector.antiAir += 0.5 * Math.sqrt(remodel);
  else if (master.type === 49) vector.antiAir += 0.2 * remodel;
  else if (master.type === 41) vector.antiAir += 0.15 * remodel;

  if ([14, 15, 40].includes(master.type)) vector.asw += (2 / 3) * Math.sqrt(remodel);
  else if (master.type === 8 || (master.type === 7 && !BAKUSEN.has(master.id))) vector.asw += 0.2 * remodel;
  else if (master.type === 26) vector.asw += ((master.asw ?? 0) >= 8 ? 0.3 : 0.2) * remodel;
  else if (master.type === 25) vector.asw += ((master.asw ?? 0) > 10 ? 0.3 : 0.2) * remodel;

  if ((master.type === 12 || master.type === 13) && (master.scout ?? 0) >= 5) {
    vector.accuracy += 1.7 * Math.sqrt(remodel);
  } else if ([1, 2, 3, 4, 12, 13, 14, 15, 18, 19, 24, 29, 36, 37, 39, 40, 42].includes(master.type)) {
    vector.accuracy += Math.sqrt(remodel);
  }
  if (master.type === 27) vector.armor += 0.2 * remodel;
  else if (master.type === 28) vector.armor += 0.3 * remodel;
  if (master.type === 12) vector.scout += 1.25 * Math.sqrt(remodel);
  else if (master.type === 13) vector.scout += 1.4 * Math.sqrt(remodel);
  else if (RECONNAISSANCES.has(master.type)) vector.scout += 1.2 * Math.sqrt(remodel);
  else if (master.type === 11) vector.scout += 1.15 * Math.sqrt(remodel);

  if (airbaseMode === 2) {
    vector.antiAir = (master.antiAir ?? 0) + (master.interception ?? 0) + 2 * (master.antiBomber ?? 0);
  } else if (airbaseMode !== undefined) {
    vector.antiAir = (master.antiAir ?? 0) + 1.5 * (master.interception ?? 0);
  }
  return vector;
}

function totalShipBonus(ship: MasterShip, items: EquippedItem[]): BonusStatus {
  const result: BonusStatus = {};
  const antiAirRadar = items.some((item) => item.master.itype === 11 && (item.master.antiAir ?? 0) > 1);
  const surfaceRadar = items.some((item) => item.master.itype === 11 && (item.master.scout ?? 0) > 4);
  const accuracyRadar = items.some((item) => item.master.itype === 11 && (item.master.accuracy ?? 0) >= 8);

  for (const group of reference.bonuses) {
    const fitItems = group.types
      ? items.filter((item) => group.types?.includes(item.master.type))
      : items.filter((item) => group.ids?.includes(item.master.id));
    if (fitItems.length === 0) continue;

    for (const rule of group.bonuses) {
      if (rule.shipBase && !rule.shipBase.includes(ship.orig)) continue;
      if (rule.shipClass && !rule.shipClass.includes(ship.type2)) continue;
      if (rule.shipCountry && !rule.shipCountry.includes(ship.type2)) continue;
      if (rule.shipType && !rule.shipType.includes(ship.type)) continue;
      if (rule.shipId && !rule.shipId.includes(ship.id)) continue;
      if (rule.requiresAR && !antiAirRadar) continue;
      if (rule.requiresSR && !surfaceRadar) continue;
      if (rule.requiresAccR && !accuracyRadar) continue;
      if (rule.requiresType && !items.some((item) => rule.requiresType?.includes(item.master.type))) continue;
      if (rule.requiresId) {
        const required = items.filter((item) => rule.requiresId?.includes(item.master.id));
        if (required.length === 0) continue;
        if (rule.requiresIdNum && required.length < rule.requiresIdNum) continue;
        if (rule.requiresIdLevel && !required.some((item) => item.remodel >= (rule.requiresIdLevel ?? 0))) continue;
        if (rule.requiresId2) {
          const required2 = items.filter((item) => rule.requiresId2?.includes(item.master.id));
          if (required2.length === 0) continue;
          if (rule.requiresIdLevel2 && !required2.some((item) => item.remodel >= (rule.requiresIdLevel2 ?? 0))) continue;
        }
      }

      const remodeled = rule.remodel
        ? fitItems.filter((item) => item.remodel >= (rule.remodel ?? 0))
        : fitItems;
      if (remodeled.length === 0 || (rule.num && remodeled.length < rule.num)) continue;
      const multiplier = rule.num ? 1 : remodeled.length;
      for (const [key, value] of Object.entries(rule.bonus)) {
        const statusKey = key as keyof BonusStatus;
        result[statusKey] = (result[statusKey] ?? 0) + (value ?? 0) * multiplier;
      }
    }
  }
  return result;
}

function shipVector(ship: MasterShip, items: EquippedItem[]): MetricVector {
  const total = zeroVector();
  for (const item of items) {
    const vector = itemVector(item);
    for (const metric of ALL_METRICS) total[metric] += vector[metric];
  }
  const bonus = totalShipBonus(ship, items);
  total.firePower += bonus.firePower ?? 0;
  total.torpedo += bonus.torpedo ?? 0;
  total.antiAir += bonus.antiAir ?? 0;
  total.armor += bonus.armor ?? 0;
  total.asw += bonus.asw ?? 0;
  total.scout += bonus.scout ?? 0;
  total.avoid += bonus.avoid ?? 0;
  total.accuracy += bonus.accuracy ?? 0;
  total.bomber += bonus.bomber ?? 0;
  total.range += bonus.range ?? 0;
  return total;
}

function proficiencyAirPower(item: EquippedItem): number {
  if (!item.master.isFighter && item.master.type !== 11) {
    return Math.sqrt(item.proficiency / 10);
  }
  let fixed = 0;
  if (item.proficiency >= 100) fixed = item.master.isFighter ? 22 : 6;
  else if (item.proficiency >= 70) fixed = item.master.isFighter ? 14 : 3;
  else if (item.proficiency >= 55) fixed = item.master.isFighter ? 9 : 1;
  else if (item.proficiency >= 40) fixed = item.master.isFighter ? 5 : 1;
  else if (item.proficiency >= 25) fixed = item.master.isFighter ? 2 : 1;
  return fixed + Math.sqrt(item.proficiency / 10);
}

function eventTags(item: MasterItem, isAirbase: boolean): Set<string> {
  return new Set(
    (item.eventTags ?? [])
      .filter((tag) => (isAirbase ? !tag.isOnlyShip : !tag.isOnlyAB))
      .flatMap((tag) => tag.text.map((text) => `${tag.key}:${text}`)),
  );
}

function eventRolePenalty(slot: TargetSlot, candidate: MasterItem): number {
  const isAirbase = slot.airbaseMode !== undefined;
  const sourceItems = slot.shipItems ?? slot.airbaseItems ?? [slot.target];
  const sourceTags = new Set(
    sourceItems.flatMap((item) => [...eventTags(item.master, isAirbase)]),
  );
  if (sourceTags.size === 0) return 0;
  const replacement = [...sourceItems];
  const index = slot.shipItemIndex ?? slot.airbaseItemIndex ?? 0;
  replacement[index] = {
    master: candidate,
    remodel: 0,
    proficiency: 0,
  };
  const candidateTags = new Set(
    replacement.flatMap((item) => [...eventTags(item.master, isAirbase)]),
  );
  let missing = 0;
  for (const tag of sourceTags) {
    if (!candidateTags.has(tag)) missing += 1;
  }
  const targetTags = eventTags(slot.target.master, isAirbase);
  const directTags = eventTags(candidate, isAirbase);
  let directMissing = 0;
  for (const tag of targetTags) {
    if (!directTags.has(tag)) directMissing += 1;
  }
  return missing * 50_000_000 + directMissing * 5_000_000;
}

function vectorQuality(vector: MetricVector, metrics: Metric[]): number {
  let quality = 0;
  for (let index = 0; index < metrics.length; index += 1) {
    const weight = index === 0 ? 4 : index === 1 ? 2 : 1;
    quality += vector[metrics[index]] * weight;
  }
  return quality;
}

function landEquipmentQuality(item: EquippedItem): number {
  const { id, type } = item.master;
  const remodel = item.remodel;
  if (type === 52) return id === 499 ? 110 : id === 498 ? 100 : 70;
  if (type === 46) {
    if (id === 167) return 75 + 3 * remodel;
    if (id === 526) return 98;
    if (id === 525) return 82;
  }
  const fixed: Record<number, number> = {
    514: 110,
    495: 105,
    482: 100,
    355: 96,
    576: 94,
    230: 92,
    436: 88,
    494: 84,
    449: 80,
  };
  if (fixed[id]) return fixed[id];
  if (id === 166) return 72 + 3 * remodel;
  if (ARMED_LANDING_CRAFT.has(id)) return 68 + 3 * remodel;
  if (type === 37) return (item.master.fire ?? 0) * 4 + remodel;
  return 30 + remodel;
}

function landCombinationPenalty(items: EquippedItem[]): number {
  const hasChiha = items.some((item) => CHIHA_FAMILY.has(item.master.id));
  const hasArmedCraft = items.some((item) =>
    ARMED_LANDING_CRAFT.has(item.master.id),
  );
  return hasChiha && hasArmedCraft ? 100_000_000 : 0;
}

function gunCaliber(item: MasterItem): number | undefined {
  const metric = item.name.match(/(\d+(?:\.\d+)?)cm/i);
  if (metric) return Number(metric[1]);
  const imperial = item.name.match(/(\d+(?:\.\d+)?)inch/i);
  if (imperial) return Number(imperial[1]) * 2.54;
  return undefined;
}

function gunFamily(item: MasterItem): string | undefined {
  const name = item.name;
  if (/13\.8cm/.test(name)) return "french-138";
  if (/130mm B-13/.test(name)) return "soviet-130";
  if (/12\.7cm連装砲D型/.test(name)) return "japanese-d";
  if (/12\.7cm連装砲C型/.test(name)) return "japanese-c";
  if (/12\.7cm連装砲A型|試製 長12\.7cm連装砲A型/.test(name)) {
    return "japanese-a";
  }
  if (/10cm連装高角砲/.test(name)) return "japanese-10aa";
  if (/8inch三連装砲 Mk\.9/.test(name)) return "american-8inch";
  if (/5inch.*両用砲/.test(name)) return "american-5inch-dp";
  if (/38cm四連装砲/.test(name)) return "french-38quad";
  return undefined;
}

function gunRolePenalty(target: MasterItem, candidate: MasterItem): number {
  if (![1, 2, 3].includes(target.type)) return 0;
  let penalty = 0;
  const targetCaliber = gunCaliber(target);
  const candidateCaliber = gunCaliber(candidate);
  if (
    target.type === 3 &&
    targetCaliber !== undefined &&
    candidateCaliber !== undefined
  ) {
    penalty += (targetCaliber - candidateCaliber) ** 2 * 500_000;
  }
  const targetFamily = gunFamily(target);
  const candidateFamily = gunFamily(candidate);
  if (targetFamily && targetFamily !== candidateFamily) penalty += 12_000_000;
  if (target.id === 468 && candidate.id !== 468) penalty += 20_000_000;
  return penalty;
}

function specialRolePenalty(target: MasterItem, candidate: MasterItem): number {
  if (target.id === 490 && candidate.id !== 490) return 30_000_000;
  if (target.type === 57 && candidate.type !== 57) return 30_000_000;
  return 0;
}

function scoreCandidate(slot: TargetSlot, candidate: EquippedItem): number {
  let candidateVector: MetricVector;
  if (slot.ship && slot.shipItems && slot.shipItemIndex !== undefined) {
    const replacement = [...slot.shipItems];
    replacement[slot.shipItemIndex] = candidate;
    candidateVector = shipVector(slot.ship, replacement);
  } else {
    candidateVector = itemVector(
      candidate,
      slot.airbaseMode,
    );
  }

  if (slot.airbaseMode === 1 && slot.target.master.type === 47) {
    const enemies = slot.airbaseEnemies ?? [];
    const attack =
      enemies.length > 0
        ? enemies.reduce(
            (sum, enemy) => sum + airbaseEffectiveAttack(candidate, enemy),
            0,
          ) / enemies.length
        : Math.max(candidateVector.torpedo, candidateVector.bomber);
    const survival = candidate.master.avoidId ?? 0;
    const quality =
      attack * 10 +
      candidateVector.accuracy * 2 +
      candidateVector.antiAir +
      survival * 3;
    return (
      1_000_000_000 -
      quality * 1000 +
      eventRolePenalty(slot, candidate.master)
    );
  }
  if (slot.airbaseMode === 2) {
    const quality =
      candidateVector.antiAir * 10 + (candidate.master.antiBomber ?? 0);
    return (
      1_000_000_000 -
      quality * 1000 +
      eventRolePenalty(slot, candidate.master)
    );
  }

  const metrics = CATEGORY_METRICS[slot.target.master.type] ?? ALL_METRICS;
  let quality = vectorQuality(candidateVector, metrics);
  if (
    slot.target.master.isFighter ||
    [7, 8, 11, 57].includes(slot.target.master.type)
  ) {
    quality +=
      ((candidateVector.antiAir ?? 0) * Math.sqrt(slot.slotSize) +
        proficiencyAirPower(candidate)) *
      4;
  }
  let combinationPenalty = 0;
  if (
    slot.ship &&
    slot.shipItems &&
    slot.shipItemIndex !== undefined &&
    [24, 37, 46, 52].includes(slot.target.master.type)
  ) {
    const replacement = [...slot.shipItems];
    replacement[slot.shipItemIndex] = candidate;
    const hasInstallation = (slot.enemies ?? []).some(isInstallation);
    quality += hasInstallation
      ? landEquipmentQuality(candidate) * 10
      : (candidate.master.tp2 ?? candidate.master.tp ?? 0) * 10;
    combinationPenalty = landCombinationPenalty(replacement);
  }
  return (
    1_000_000_000 -
    quality * 1000 +
    eventRolePenalty(slot, candidate.master) +
    gunRolePenalty(slot.target.master, candidate.master) +
    specialRolePenalty(slot.target.master, candidate.master) +
    combinationPenalty +
    (slot.target.master.id === candidate.master.id ? 0 : 1)
  );
}

function isEquipable(slot: TargetSlot, candidate: OwnedFormationItem): boolean {
  const master = itemById.get(candidate.master_id);
  if (!master) return false;
  const target = slot.target.master;
  if (
    target.type !== master.type &&
    !(target.type === 57 && [7, 8].includes(master.type))
  ) {
    return false;
  }
  if (
    slot.airbaseMode === 1 &&
    (master.radius ?? 0) < (target.radius ?? 0)
  ) {
    return false;
  }
  if (
    SUBMARINE_SCOUT.has(target.id) &&
    !SUBMARINE_SCOUT.has(master.id)
  ) {
    return false;
  }
  if (target.type === 10 && target.itype === 50 && master.itype !== 50) {
    return false;
  }
  if (
    slot.area === 623 &&
    TOKU_4_TANK.has(target.id) &&
    !TOKU_4_TANK.has(master.id)
  ) {
    return false;
  }
  if (
    LATE_MODEL_TORPEDO.has(target.id) &&
    !LATE_MODEL_TORPEDO.has(master.id)
  ) {
    return false;
  }
  if (
    ROCKET_INTERCEPTOR.has(target.id) &&
    !ROCKET_INTERCEPTOR.has(master.id)
  ) {
    return false;
  }
  if (
    [1, 2, 4].includes(target.type) &&
    (target.itype === 16) !== (master.itype === 16)
  ) {
    return false;
  }
  if (
    [6, 7, 8].includes(target.type) &&
    target.isNightAircraftItem &&
    !master.isNightAircraftItem
  ) {
    return false;
  }
  if (
    target.type === 7 &&
    target.enabledAttackLandBase !== master.enabledAttackLandBase
  ) {
    return false;
  }
  if (
    target.type === 15 &&
    target.isStrictDepthCharge !== master.isStrictDepthCharge
  ) {
    return false;
  }
  if (target.type === 17 && (target.id === 33) !== (master.id === 33)) {
    return false;
  }
  if (
    target.type === 21 &&
    ROCKET_BARRAGE.has(target.id) &&
    !ROCKET_BARRAGE.has(master.id)
  ) {
    return false;
  }
  if (target.type === 23 && target.id !== master.id) return false;
  if (
    target.type === 24 &&
    ARMED_LANDING_CRAFT.has(target.id) &&
    !ARMED_LANDING_CRAFT.has(master.id)
  ) {
    return false;
  }
  if (
    target.type === 34 &&
    COMMAND_FACILITY.has(target.id) &&
    target.id !== master.id
  ) {
    return false;
  }
  if (
    target.type === 35 &&
    ((NIGHT_AIRCREW.has(target.id) && !NIGHT_AIRCREW.has(master.id)) ||
      (DECK_AIRCREW.has(target.id) && !DECK_AIRCREW.has(master.id)))
  ) {
    return false;
  }
  if (
    target.type === 39 &&
    ((TORPEDO_LOOKOUT.has(target.id) && !TORPEDO_LOOKOUT.has(master.id)) ||
      (STANDARD_LOOKOUT.has(target.id) && !STANDARD_LOOKOUT.has(master.id)))
  ) {
    return false;
  }
  if (
    target.type === 47 &&
    (target.itype === 47) !== (master.itype === 47)
  ) {
    return false;
  }
  if (
    target.type === 54 &&
    ((SMOKE_GENERATOR.has(target.id) && !SMOKE_GENERATOR.has(master.id)) ||
      (!SMOKE_GENERATOR.has(target.id) && SMOKE_GENERATOR.has(master.id)))
  ) {
    return false;
  }
  if ([12, 13].includes(target.type)) {
    if ((target.antiAir ?? 0) > 1 && (master.antiAir ?? 0) <= 1) return false;
    if (
      (target.scout ?? 0) > 4 &&
      (target.antiAir ?? 0) <= 1 &&
      (master.scout ?? 0) <= 4
    ) {
      return false;
    }
    if ((target.accuracy ?? 0) >= 8 && (master.accuracy ?? 0) < 8) return false;
  }
  if (!slot.ship) return true;
  if (master.id === 561) return false;
  if (master.id === 151 && slot.ship.type !== 18) return false;
  if (
    (master.id === 142 || master.id === 460) &&
    ![8, 9, 10].includes(slot.ship.type)
  ) {
    return false;
  }
  if (
    [128, 281, 465].includes(master.id) &&
    slot.ship.type2 !== 37 &&
    !(slot.ship.type2 === 19 && slot.ship.ver > 0)
  ) {
    return false;
  }
  if (
    master.id === 467 &&
    ![5, 8, 9, 10, 11, 18].includes(slot.ship.type)
  ) {
    return false;
  }
  const shipOverride = reference.equipShip[String(slot.ship.id)]?.api_equip_type;
  const itemRestriction = shipOverride?.[String(master.type)];
  if (Array.isArray(itemRestriction) && !itemRestriction.includes(master.id)) return false;
  if (!slot.isExSlot) return true;

  if (EXPANDED_ITEM_TYPE.has(master.type)) return true;
  const exslot = reference.equipExslotShip[String(master.id)];
  if (!exslot || (exslot.api_req_level ?? 0) > candidate.remodel) return false;
  return (
    Boolean(exslot.api_ship_ids?.[String(slot.ship.id)]) ||
    Boolean(exslot.api_stypes?.[String(slot.ship.type)]) ||
    Boolean(exslot.api_ctypes?.[String(slot.ship.type2)])
  );
}

function hungarian(cost: number[][]): number[] {
  const rows = cost.length;
  const columns = cost[0]?.length ?? 0;
  const u = new Array(rows + 1).fill(0);
  const v = new Array(columns + 1).fill(0);
  const p = new Array(columns + 1).fill(0);
  const way = new Array(columns + 1).fill(0);
  for (let row = 1; row <= rows; row += 1) {
    p[0] = row;
    let column0 = 0;
    const min = new Array(columns + 1).fill(Number.POSITIVE_INFINITY);
    const used = new Array(columns + 1).fill(false);
    do {
      used[column0] = true;
      const row0 = p[column0];
      let delta = Number.POSITIVE_INFINITY;
      let column1 = 0;
      for (let column = 1; column <= columns; column += 1) {
        if (used[column]) continue;
        const current = cost[row0 - 1][column - 1] - u[row0] - v[column];
        if (current < min[column]) {
          min[column] = current;
          way[column] = column0;
        }
        if (min[column] < delta) {
          delta = min[column];
          column1 = column;
        }
      }
      for (let column = 0; column <= columns; column += 1) {
        if (used[column]) {
          u[p[column]] += delta;
          v[column] -= delta;
        } else {
          min[column] -= delta;
        }
      }
      column0 = column1;
    } while (p[column0] !== 0);
    do {
      const column1 = way[column0];
      p[column0] = p[column1];
      column0 = column1;
    } while (column0 !== 0);
  }
  const assignment = new Array(rows).fill(-1);
  for (let column = 1; column <= columns; column += 1) {
    if (p[column] > 0) assignment[p[column] - 1] = column - 1;
  }
  return assignment;
}

function savedToEquipped(saved: SavedItem): EquippedItem | undefined {
  const master = itemById.get(saved.i);
  if (!master) return undefined;
  return { master, remodel: saved.r ?? 0, proficiency: saved.l ?? 0 };
}

function collectTargetSlots(formation: SavedFormation): TargetSlot[] {
  const slots: TargetSlot[] = [];
  const battles = formation.battleInfo?.fleets ?? [];
  const finalBattle = [...battles]
    .reverse()
    .find((battle) => (battle.enemies ?? []).some((enemy) => enemy.i > 0));
  const formationEnemies = (finalBattle?.enemies ?? [])
    .map((enemy) => enemyById.get(enemy.i))
    .filter((enemy): enemy is MasterEnemy => Boolean(enemy));
  for (const airbase of formation.airbaseInfo?.airbases ?? []) {
    const targetIndexes = Array.from(
      new Set(
        (
          airbase as {
            battleTarget?: number[];
          }
        ).battleTarget ?? [],
      ),
    );
    const airbaseEnemies = targetIndexes.flatMap((index) =>
      (battles[index]?.enemies ?? [])
        .map((enemy) => enemyById.get(enemy.i))
        .filter((enemy): enemy is MasterEnemy => Boolean(enemy)),
    );
    const targetBattle = battles[targetIndexes[0]];
    const airbaseItems = (airbase.items ?? [])
      .map(savedToEquipped)
      .filter((item): item is EquippedItem => Boolean(item));
    let airbaseItemIndex = 0;
    for (const saved of airbase.items ?? []) {
      const target = savedToEquipped(saved);
      if (target) {
        slots.push({
          saved,
          target,
          isExSlot: false,
          airbaseMode: airbase.mode,
          airbaseEnemies,
          airbaseItems,
          airbaseItemIndex,
          enemies: airbaseEnemies,
          area: targetBattle?.area,
          nodeName: targetBattle?.nodeName,
          slotSize: saved.s ?? (target.master.type === 49 ? 4 : 18),
        });
        airbaseItemIndex += 1;
      }
    }
  }
  for (const fleet of formation.fleetInfo?.fleets ?? []) {
    for (const savedShip of fleet.ships ?? []) {
      const ship = shipById.get(savedShip.i);
      if (!ship) continue;
      const allSaved = [...(savedShip.is ?? [])];
      if (savedShip.ex?.i) allSaved.push(savedShip.ex);
      const shipItems = allSaved.map(savedToEquipped).filter((item): item is EquippedItem => Boolean(item));
      let itemIndex = 0;
      for (const saved of savedShip.is ?? []) {
        const target = savedToEquipped(saved);
        if (!target) continue;
        slots.push({
          saved,
          target,
          ship,
          shipItems,
          shipItemIndex: itemIndex,
          isExSlot: false,
          enemies: formationEnemies,
          area: finalBattle?.area,
          nodeName: finalBattle?.nodeName,
          slotSize: saved.s ?? 1,
        });
        itemIndex += 1;
      }
      if (savedShip.ex?.i) {
        const target = savedToEquipped(savedShip.ex);
        if (target) {
          slots.push({
            saved: savedShip.ex,
            target,
            ship,
            shipItems,
            shipItemIndex: itemIndex,
            isExSlot: true,
            enemies: formationEnemies,
            area: finalBattle?.area,
            nodeName: finalBattle?.nodeName,
            slotSize: savedShip.ex.s ?? 1,
          });
        }
      }
    }
  }
  return slots;
}

function allocationGroup(type: number): number {
  return [7, 8, 57].includes(type) ? 700 : type;
}

function applyOwnedItems(
  slots: TargetSlot[],
  inventory: OwnedFormationInventory,
): {
  assigned: number;
  missing: number;
  missing_items: Array<{ master_id: number; name: string; count: number }>;
} {
  let assigned = 0;
  let missing = 0;
  const missingItems = new Map<number, number>();
  const recordMissing = (slot: TargetSlot) => {
    const id = slot.target.master.id;
    missingItems.set(id, (missingItems.get(id) ?? 0) + 1);
  };
  const slotsByType = new Map<number, TargetSlot[]>();
  for (const slot of slots) {
    const type = allocationGroup(slot.target.master.type);
    slotsByType.set(type, [...(slotsByType.get(type) ?? []), slot]);
  }
  const inventoryByType = new Map<number, OwnedFormationItem[]>();
  for (const item of inventory.items) {
    const masterType = itemById.get(item.master_id)?.type;
    if (masterType === undefined) continue;
    const type = allocationGroup(masterType);
    inventoryByType.set(type, [...(inventoryByType.get(type) ?? []), item]);
  }

  for (const [type, typedSlots] of slotsByType) {
    // Damage-control items are consumables whose usable stock is managed by the
    // player outside this optimizer. Preserve the source formation exactly,
    // including repeated repair crews or goddesses, without consuming owned
    // equipment instances.
    if (type === 23) {
      assigned += typedSlots.length;
      continue;
    }
    const candidates = [...(inventoryByType.get(type) ?? [])].sort((a, b) => {
      const aMaster = itemById.get(a.master_id);
      const bMaster = itemById.get(b.master_id);
      const primary = CATEGORY_METRICS[aMaster?.type ?? type]?.[0] ?? "firePower";
      const quality =
        (itemVector({ master: bMaster!, remodel: b.remodel, proficiency: b.proficiency })[primary] -
          itemVector({ master: aMaster!, remodel: a.remodel, proficiency: a.proficiency })[primary]);
      return quality || b.remodel - a.remodel || a.instance_id - b.instance_id;
    });
    // Always include an empty choice. Some owned items can still be invalid for a
    // ship-specific or expansion-slot restriction.
    const dummyCount = typedSlots.length;
    const columnCount = candidates.length + dummyCount;
    if (columnCount === 0) {
      for (const slot of typedSlots) slot.saved.i = 0;
      missing += typedSlots.length;
      for (const slot of typedSlots) recordMissing(slot);
      continue;
    }
    const impossible = 1e15;
    const costs = typedSlots.map((slot) => [
      ...candidates.map((candidate) => {
        if (!isEquipable(slot, candidate)) return impossible;
        const master = itemById.get(candidate.master_id)!;
        return scoreCandidate(slot, {
          master,
          remodel: candidate.remodel,
          proficiency: PROFICIENCY[candidate.proficiency] ?? 0,
        });
      }),
      ...new Array(dummyCount).fill(impossible / 2),
    ]);
    const assignment = hungarian(costs);
    typedSlots.forEach((slot, row) => {
      const column = assignment[row];
      const candidate = column >= 0 ? candidates[column] : undefined;
      if (!candidate || costs[row][column] >= impossible / 2) {
        slot.saved.i = 0;
        delete slot.saved.r;
        delete slot.saved.l;
        missing += 1;
        recordMissing(slot);
        return;
      }
      slot.saved.i = candidate.master_id;
      if (candidate.remodel > 0) slot.saved.r = candidate.remodel;
      else delete slot.saved.r;
      const master = itemById.get(candidate.master_id)!;
      if ([6, 7, 8, 9, 10, 11, 25, 26, 41, 45, 47, 48, 49, 53, 56, 57].includes(master.type)) {
        slot.saved.l = PROFICIENCY[candidate.proficiency] ?? 0;
      } else {
        delete slot.saved.l;
      }
      assigned += 1;
    });
  }
  return {
    assigned,
    missing,
    missing_items: [...missingItems].map(([master_id, count]) => ({
      master_id,
      name: itemById.get(master_id)?.name ?? `ID ${master_id}`,
      count,
    })),
  };
}

export function adaptOwnedFormationData(
  compressedData: string,
  inventory: OwnedFormationInventory,
): OwnedFormationResult {
  const json = LZString.decompressFromEncodedURIComponent(compressedData);
  if (!json) throw new Error("制空シミュの編成データを展開できませんでした");
  const formation = JSON.parse(json) as SavedFormation;
  if (formation.fleetInfo) formation.fleetInfo.admiralLevel = inventory.hq_level;
  const slots = collectTargetSlots(formation);
  const result = applyOwnedItems(slots, inventory);
  const compressed = LZString.compressToEncodedURIComponent(JSON.stringify(formation));
  return {
    url: `https://noro6.github.io/kc-web/?data=${compressed}`,
    ...result,
  };
}
