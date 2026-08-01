import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { EVENTS } from "../../constants";
import {
  getKonekoArmorBreakFormationLinks,
  getKonekoEventFormationLinks,
  getShimakazeArmorBreakFormationLinks,
  getShimakazeEventFormationLinks,
  type EventFormationLink,
} from "../../data/eventFormations";
import type { OwnedFormationInventory } from "../../utils/ownedFormation";
import type {
  BattleLogsResponse,
  EventMapStatus,
  SortieRecord,
} from "../../types";
import "./EventTab.css";

export type Difficulty = "甲" | "乙" | "丙" | "丁";
export const EVENT_SHIMAKAZE_FORMATION_HEADING = "島風編成";
export const EVENT_KONEKO_FORMATION_HEADING = "子猫編成";
export const EVENT_FORMATION_NOTE =
  "注意：索敵や制空が不足する場合は、自分で調整してください。";
type VictoryRank = "S" | "A";
type RequirementKind = "victory" | "air" | "arrival" | "defense";

interface DifficultyRequirement {
  required: number;
  victory?: VictoryRank;
}

interface GimmickRequirementTemplate {
  id: string;
  label: string;
  kind: RequirementKind;
  cellNos?: number[];
  difficulties: Partial<Record<Difficulty, DifficultyRequirement>>;
}

export interface GimmickRequirement {
  id: string;
  node: string;
  kind: RequirementKind;
  cellNos: number[];
  victory?: VictoryRank;
  required: number;
}

interface GimmickStage {
  kind: "gimmick";
  id: string;
  label: string;
  detail: string;
  recordGauge: number;
  unlocksGauge: number;
  requirements: GimmickRequirementTemplate[];
}

interface GaugeStage {
  kind: "gauge";
  id: string;
  label: string;
  detail: string;
  gaugeNumber: number;
  bossCellNos: number[];
  finalBossHp?: Partial<Record<Difficulty, number>>;
}

type EventStage = GimmickStage | GaugeStage;

interface EventMapConfig {
  mapNo: number;
  mapId: number;
  area: string;
  operation: string;
  stages: EventStage[];
}

interface GimmickProgress {
  counts: Record<string, number>;
  complete: boolean;
  completedAt: string | null;
}

function FormationLinkButton({
  formation,
}: {
  formation: EventFormationLink;
}) {
  const [status, setStatus] = useState<"idle" | "loading" | "opened" | "error">(
    "idle",
  );
  const [detail, setDetail] = useState("");

  const openOwnedFormation = useCallback(async () => {
    if (status === "loading") return;
    setStatus("loading");
    setDetail("");
    try {
      const [compressedData, inventory] = await Promise.all([
        invoke<string>("resolve_event_formation_data", {
          sourceUrl: formation.url,
        }),
        invoke<OwnedFormationInventory>("get_owned_formation_inventory"),
      ]);
      const { adaptOwnedFormationData } = await import(
        "../../utils/ownedFormation"
      );
      const result = adaptOwnedFormationData(compressedData, inventory);
      await openUrl(result.url);
      setStatus("opened");
      const missingNames = result.missing_items
        .map((item) => `${item.name}${item.count > 1 ? `×${item.count}` : ""}`)
        .join("、");
      setDetail(
        result.missing > 0
          ? `不足 ${result.missing}枠: ${missingNames}`
          : `${result.assigned}枠を所持装備で再現`,
      );
    } catch (error) {
      setStatus("error");
      setDetail(error instanceof Error ? error.message : String(error));
    }
  }, [formation.url, status]);

  return (
    <span className="event-formation-link">
      <button
        disabled={status === "loading"}
        onClick={() => void openOwnedFormation()}
        title="この編成を、現在の所持数を超えない範囲で同カテゴリの最適装備に置き換えます"
        type="button"
      >
        {status === "loading" ? "編成中…" : formation.label}
        <b aria-hidden="true">↗</b>
      </button>
      {detail && (
        <small
          aria-live="polite"
          className={status === "error" ? "is-error" : ""}
          title={detail}
        >
          {detail}
        </small>
      )}
    </span>
  );
}

function FormationLinks({
  eyebrow,
  formations,
}: {
  eyebrow: string;
  formations: EventFormationLink[];
}) {
  if (formations.length === 0) return null;

  return (
    <div className="event-formation-links">
      <span>{eyebrow}</span>
      <div>
        {formations.map((formation) => (
          <FormationLinkButton formation={formation} key={formation.url} />
        ))}
      </div>
    </div>
  );
}

const RANK_VALUE: Record<string, number> = {
  S: 5,
  A: 4,
  B: 3,
  C: 2,
  D: 1,
  E: 0,
};

const all = (
  required: number,
  victory?: VictoryRank,
): Record<Difficulty, DifficultyRequirement> => ({
  甲: { required, victory },
  乙: { required, victory },
  丙: { required, victory },
  丁: { required, victory },
});

const EVENT_MAPS: EventMapConfig[] = [
  {
    mapNo: 1,
    mapId: 621,
    area: "九州沖／南西諸島沖",
    operation: "第三十一戦隊駆逐艦の出撃",
    stages: [
      {
        kind: "gimmick",
        id: "gimmick1",
        label: "ギミック1",
        detail: "第一ボス出現",
        recordGauge: 1,
        unlocksGauge: 1,
        requirements: [
          {
            id: "C2",
            label: "C2",
            kind: "victory",
            cellNos: [7],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 1, victory: "S" },
              丙: { required: 1, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "C3",
            label: "C3",
            kind: "victory",
            cellNos: [8],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 1, victory: "S" },
              丙: { required: 1, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "F",
            label: "F",
            kind: "air",
            cellNos: [11],
            difficulties: { 甲: { required: 2 } },
          },
          {
            id: "H",
            label: "H",
            kind: "arrival",
            cellNos: [13, 36],
            difficulties: all(1),
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge1",
        label: "ゲージ1",
        detail: "戦力ゲージ",
        gaugeNumber: 1,
        bossCellNos: [18],
      },
      {
        kind: "gimmick",
        id: "gimmick2",
        label: "ギミック2",
        detail: "遊撃部隊出撃地点解放",
        recordGauge: 2,
        unlocksGauge: 2,
        requirements: [
          {
            id: "L",
            label: "L",
            kind: "arrival",
            cellNos: [21],
            difficulties: {
              甲: { required: 2 },
              乙: { required: 2 },
              丙: { required: 1 },
              丁: { required: 1 },
            },
          },
          {
            id: "defense",
            label: "基地防空",
            kind: "defense",
            difficulties: {
              甲: { required: 2 },
              乙: { required: 1 },
              丙: { required: 1 },
            },
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge2",
        label: "ゲージ2",
        detail: "輸送ゲージ",
        gaugeNumber: 2,
        bossCellNos: [32],
      },
      {
        kind: "gauge",
        id: "gauge3",
        label: "ゲージ3",
        detail: "戦力ゲージ",
        gaugeNumber: 3,
        bossCellNos: [46],
        finalBossHp: { 甲: 660, 乙: 440, 丙: 330, 丁: 330 },
      },
    ],
  },
  {
    mapNo: 2,
    mapId: 622,
    area: "南沙諸島沖／オルモック沖／サンベルナルジノ海峡沖",
    operation: "サンベルナルジノ海峡 通峡",
    stages: [
      {
        kind: "gimmick",
        id: "gimmick1",
        label: "ギミック1",
        detail: "ルート解放",
        recordGauge: 1,
        unlocksGauge: 1,
        requirements: [
          {
            id: "D",
            label: "D",
            kind: "air",
            cellNos: [11, 45],
            difficulties: {
              甲: { required: 2 },
              乙: { required: 2 },
              丙: { required: 1 },
            },
          },
          {
            id: "C2",
            label: "C2",
            kind: "victory",
            cellNos: [10],
            difficulties: {
              甲: { required: 3, victory: "S" },
              乙: { required: 2, victory: "S" },
              丙: { required: 2, victory: "A" },
              丁: { required: 2, victory: "A" },
            },
          },
        ],
      },
      {
        kind: "gimmick",
        id: "gimmick2",
        label: "ギミック2",
        detail: "第一ボス・出撃地点解放",
        recordGauge: 1,
        unlocksGauge: 1,
        requirements: [
          {
            id: "H",
            label: "H",
            kind: "victory",
            cellNos: [20],
            difficulties: {
              甲: { required: 3, victory: "S" },
              乙: { required: 2, victory: "S" },
              丙: { required: 2, victory: "A" },
              丁: { required: 2, victory: "A" },
            },
          },
          {
            id: "G2",
            label: "G2",
            kind: "victory",
            cellNos: [19],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 2, victory: "A" },
              丙: { required: 2, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "B",
            label: "B",
            kind: "victory",
            cellNos: [5],
            difficulties: { 甲: { required: 1, victory: "S" } },
          },
          {
            id: "defense",
            label: "基地防空",
            kind: "defense",
            difficulties: {
              甲: { required: 2 },
              乙: { required: 2 },
              丙: { required: 1 },
            },
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge1",
        label: "ゲージ1",
        detail: "輸送ゲージ",
        gaugeNumber: 1,
        bossCellNos: [32],
      },
      {
        kind: "gauge",
        id: "gauge2",
        label: "ゲージ2",
        detail: "戦力ゲージ",
        gaugeNumber: 2,
        bossCellNos: [43],
        finalBossHp: { 甲: 920, 乙: 720, 丙: 620, 丁: 620 },
      },
      {
        kind: "gauge",
        id: "gauge3",
        label: "ゲージ3",
        detail: "戦力ゲージ",
        gaugeNumber: 3,
        bossCellNos: [55],
        finalBossHp: { 甲: 880, 乙: 820, 丙: 770, 丁: 770 },
      },
    ],
  },
  {
    mapNo: 3,
    mapId: 623,
    area: "パラオ沖／ウルシー泊地沖／中部太平洋",
    operation: "ウルシー泊地への本格反攻",
    stages: [
      {
        kind: "gimmick",
        id: "gimmick1",
        label: "ギミック1",
        detail: "出撃地点解放",
        recordGauge: 1,
        unlocksGauge: 1,
        requirements: [
          {
            id: "E2",
            label: "E2",
            kind: "victory",
            cellNos: [15],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 1, victory: "A" },
              丙: { required: 1, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "C2",
            label: "C2",
            kind: "victory",
            cellNos: [9, 40],
            difficulties: {
              甲: { required: 2, victory: "A" },
              乙: { required: 1, victory: "A" },
              丙: { required: 1, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "B2",
            label: "B2",
            kind: "victory",
            cellNos: [6],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 1, victory: "A" },
              丙: { required: 1, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "D2",
            label: "D2",
            kind: "victory",
            cellNos: [12],
            difficulties: {
              甲: { required: 2, victory: "A" },
              乙: { required: 1, victory: "A" },
              丙: { required: 1, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge1",
        label: "ゲージ1",
        detail: "戦力ゲージ",
        gaugeNumber: 1,
        bossCellNos: [32],
        finalBossHp: { 甲: 980, 乙: 880, 丙: 880, 丁: 880 },
      },
      {
        kind: "gauge",
        id: "gauge2",
        label: "ゲージ2",
        detail: "輸送ゲージ",
        gaugeNumber: 2,
        bossCellNos: [39],
      },
      {
        kind: "gauge",
        id: "gauge3",
        label: "ゲージ3",
        detail: "戦力ゲージ",
        gaugeNumber: 3,
        bossCellNos: [50],
        finalBossHp: { 甲: 1000, 乙: 900, 丙: 900, 丁: 700 },
      },
      {
        kind: "gauge",
        id: "gauge4",
        label: "ゲージ4",
        detail: "戦力ゲージ",
        gaugeNumber: 4,
        bossCellNos: [62],
        finalBossHp: { 甲: 1150, 乙: 950, 丙: 900, 丁: 900 },
      },
    ],
  },
  {
    mapNo: 4,
    mapId: 624,
    area: "地中海南仏沖／アルジェリア沖／イタリア半島沖",
    operation: "Opération Vado -ヴァード作戦-",
    stages: [
      {
        kind: "gauge",
        id: "gauge1",
        label: "ゲージ1",
        detail: "戦力ゲージ",
        gaugeNumber: 1,
        bossCellNos: [4],
      },
      {
        kind: "gimmick",
        id: "gimmick1",
        label: "ギミック1",
        detail: "E2マス到達（甲のみ）",
        recordGauge: 2,
        unlocksGauge: 2,
        requirements: [
          {
            id: "E2",
            label: "E2",
            kind: "arrival",
            cellNos: [7],
            difficulties: { 甲: { required: 1 } },
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge2",
        label: "ゲージ2",
        detail: "輸送ゲージ",
        gaugeNumber: 2,
        bossCellNos: [17],
      },
      {
        kind: "gauge",
        id: "gauge3",
        label: "ゲージ3",
        detail: "戦力ゲージ",
        gaugeNumber: 3,
        bossCellNos: [29],
        finalBossHp: { 甲: 790, 乙: 740, 丙: 740, 丁: 700 },
      },
      {
        kind: "gauge",
        id: "gauge4",
        label: "ゲージ4",
        detail: "戦力ゲージ",
        gaugeNumber: 4,
        bossCellNos: [40],
        finalBossHp: { 甲: 1550, 乙: 1050, 丙: 1050, 丁: 750 },
      },
      {
        kind: "gauge",
        id: "gauge5",
        label: "ゲージ5",
        detail: "戦力ゲージ",
        gaugeNumber: 5,
        bossCellNos: [47],
        finalBossHp: { 甲: 1200, 乙: 1000, 丙: 800, 丁: 800 },
      },
    ],
  },
  {
    mapNo: 5,
    mapId: 625,
    area: "ブレスト沖／大西洋／イギリス本土沖／バルト海",
    operation: "Au-delà du Destin Cruel -フランス艦隊／欧州連合艦隊の躍動-",
    stages: [
      {
        kind: "gimmick",
        id: "gimmick1",
        label: "ギミック1",
        detail: "出撃範囲解放",
        recordGauge: 1,
        unlocksGauge: 1,
        requirements: [
          {
            id: "B2",
            label: "B2",
            kind: "victory",
            cellNos: [7],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 2, victory: "A" },
              丙: { required: 2, victory: "A" },
              丁: { required: 2, victory: "A" },
            },
          },
          {
            id: "D",
            label: "D",
            kind: "victory",
            cellNos: [11],
            difficulties: all(2, "S"),
          },
          {
            id: "C2",
            label: "C2",
            kind: "arrival",
            cellNos: [10],
            difficulties: {
              甲: { required: 2 },
              乙: { required: 1 },
              丙: { required: 1 },
              丁: { required: 1 },
            },
          },
          {
            id: "defense",
            label: "基地防空",
            kind: "defense",
            difficulties: {
              甲: { required: 2 },
              丙: { required: 1 },
              丁: { required: 1 },
            },
          },
        ],
      },
      {
        kind: "gimmick",
        id: "gimmick2",
        label: "ギミック2",
        detail: "第一ボス出現",
        recordGauge: 1,
        unlocksGauge: 1,
        requirements: [
          {
            id: "E2",
            label: "E2",
            kind: "victory",
            cellNos: [15],
            difficulties: {
              甲: { required: 2, victory: "S" },
              乙: { required: 2, victory: "A" },
              丙: { required: 2, victory: "A" },
              丁: { required: 2, victory: "A" },
            },
          },
          {
            id: "defense",
            label: "基地防空",
            kind: "defense",
            difficulties: {
              甲: { required: 2 },
              乙: { required: 1 },
              丙: { required: 1 },
            },
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge1",
        label: "ゲージ1",
        detail: "戦力ゲージ",
        gaugeNumber: 1,
        bossCellNos: [18],
      },
      {
        kind: "gauge",
        id: "gauge2",
        label: "ゲージ2",
        detail: "輸送ゲージ",
        gaugeNumber: 2,
        bossCellNos: [29],
      },
      {
        kind: "gimmick",
        id: "gimmick3",
        label: "ギミック3",
        detail: "第三出撃地点解放",
        recordGauge: 3,
        unlocksGauge: 3,
        requirements: [
          {
            id: "L1",
            label: "L1",
            kind: "victory",
            cellNos: [34, 38],
            difficulties: {
              甲: { required: 2, victory: "A" },
              乙: { required: 2, victory: "A" },
              丙: { required: 2, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
          {
            id: "L2",
            label: "L2",
            kind: "victory",
            cellNos: [36],
            difficulties: {
              甲: { required: 2, victory: "A" },
              乙: { required: 2, victory: "A" },
              丙: { required: 2, victory: "A" },
              丁: { required: 1, victory: "A" },
            },
          },
        ],
      },
      {
        kind: "gimmick",
        id: "gimmick4",
        label: "ギミック4",
        detail: "第三ボス出現",
        recordGauge: 3,
        unlocksGauge: 3,
        requirements: [
          {
            id: "P3",
            label: "P3",
            kind: "victory",
            cellNos: [48, 51],
            difficulties: all(2, "A"),
          },
          {
            id: "P",
            label: "P",
            kind: "victory",
            cellNos: [45],
            difficulties: {
              甲: { required: 3, victory: "A" },
              乙: { required: 2, victory: "A" },
              丙: { required: 2, victory: "A" },
              丁: { required: 2, victory: "A" },
            },
          },
        ],
      },
      {
        kind: "gauge",
        id: "gauge3",
        label: "ゲージ3",
        detail: "戦力ゲージ",
        gaugeNumber: 3,
        bossCellNos: [55],
        finalBossHp: { 甲: 1600, 乙: 1000, 丙: 1000, 丁: 1000 },
      },
      {
        kind: "gauge",
        id: "gauge4",
        label: "ゲージ4",
        detail: "戦力ゲージ",
        gaugeNumber: 4,
        bossCellNos: [76],
        finalBossHp: { 甲: 1220, 乙: 1020, 丙: 920, 丁: 920 },
      },
    ],
  },
];

function rankToDifficulty(rank?: number): Difficulty | null {
  return ({ 4: "甲", 3: "乙", 2: "丙", 1: "丁" } as Record<number, Difficulty>)[
    rank ?? 0
  ] ?? null;
}

function resolveRequirements(
  stage: GimmickStage,
  difficulty: Difficulty,
): GimmickRequirement[] {
  return stage.requirements.flatMap((template) => {
    const value = template.difficulties[difficulty];
    if (!value?.required) return [];
    return [{
      id: template.id,
      node: template.label,
      kind: template.kind,
      cellNos: template.cellNos ?? [],
      victory: value.victory,
      required: value.required,
    }];
  });
}

export function getEventGimmickRequirements(
  mapNo: number,
  stageId: string,
  difficulty: Difficulty,
): GimmickRequirement[] {
  const stage = EVENT_MAPS
    .find((map) => map.mapNo === mapNo)
    ?.stages.find((candidate) => candidate.id === stageId);
  return stage?.kind === "gimmick" ? resolveRequirements(stage, difficulty) : [];
}

export function gaugeProgress(status?: EventMapStatus): number | null {
  if (!status?.max_hp || status.current_hp == null) return null;
  return Math.max(
    0,
    Math.min(100, ((status.max_hp - status.current_hp) / status.max_hp) * 100),
  );
}

interface BossObservation {
  maxHp: number;
  finalForm: boolean;
}

export function latestBossObservation(
  records: SortieRecord[],
  mapNo: number,
  gaugeNumber?: number,
  bossCellNos: number[] = [],
): BossObservation | null {
  const ordered = [...records].sort((a, b) =>
    b.start_time.localeCompare(a.start_time),
  );
  for (const record of ordered) {
    const isTargetMap =
      (record.map_area === 62 && record.map_no === mapNo) ||
      record.map_display === `62-${mapNo}`;
    if (
      !isTargetMap ||
      (gaugeNumber != null &&
        record.gauge_num != null &&
        record.gauge_num !== gaugeNumber)
    ) {
      continue;
    }
    for (let index = record.nodes.length - 1; index >= 0; index -= 1) {
      const node = record.nodes[index];
      if (bossCellNos.length > 0 && !bossCellNos.includes(node.cell_no)) continue;
      const flagship = node.battle?.enemy_ships[0];
      const maxHp = node.battle?.enemy_hp[0]?.max;
      if (node.event_id !== 5 || !flagship || !maxHp) continue;
      return {
        maxHp,
        finalForm: /(?:-|－)?壊$/.test(flagship.name ?? ""),
      };
    }
  }
  return null;
}

export function isLastDance(
  status: EventMapStatus | undefined,
  finalBossHp: number | undefined,
  finalFormObserved = false,
): boolean {
  if (
    !status ||
    status.gauge_type !== 2 ||
    status.cleared ||
    status.current_hp == null ||
    status.current_hp <= 0
  ) {
    return false;
  }
  return finalFormObserved ||
    (finalBossHp != null && status.current_hp <= finalBossHp);
}

function isAirSuperiority(value?: number): boolean {
  return value === 1 || value === 2;
}

function recordMatchesMap(record: SortieRecord, mapNo: number): boolean {
  return (
    (record.map_area === 62 && record.map_no === mapNo) ||
    record.map_display === `62-${mapNo}`
  );
}

function calculateGimmickProgress(
  records: SortieRecord[],
  mapNo: number,
  requirements: GimmickRequirement[],
  recordGauge?: number,
  afterTime?: string | null,
): GimmickProgress {
  const counts = Object.fromEntries(
    requirements.map((requirement) => [requirement.id, 0]),
  );
  let completedAt: string | null = null;
  const ordered = [...records].sort((a, b) =>
    a.start_time.localeCompare(b.start_time),
  );

  for (const record of ordered) {
    if (
      !recordMatchesMap(record, mapNo) ||
      (recordGauge != null &&
        record.gauge_num != null &&
        record.gauge_num !== recordGauge) ||
      (afterTime && record.start_time <= afterTime)
    ) {
      continue;
    }

    for (const requirement of requirements) {
      if (counts[requirement.id] >= requirement.required) continue;
      if (requirement.kind === "defense") {
        const successes = record.nodes.filter((node) =>
          isAirSuperiority(node.base_air_defense?.air_superiority)
        ).length;
        counts[requirement.id] += successes;
        continue;
      }

      for (const node of record.nodes) {
        if (!requirement.cellNos.includes(node.cell_no)) continue;
        if (requirement.kind === "arrival") {
          counts[requirement.id] += 1;
          break;
        }
        if (requirement.kind === "air") {
          if (isAirSuperiority(node.battle?.air_battle?.air_superiority)) {
            counts[requirement.id] += 1;
          }
          break;
        }
        const rank = node.battle?.rank ?? node.rank;
        if (
          rank &&
          requirement.victory &&
          (RANK_VALUE[rank] ?? -1) >= RANK_VALUE[requirement.victory]
        ) {
          counts[requirement.id] += 1;
        }
        break;
      }
    }

    if (
      requirements.every(
        (requirement) => counts[requirement.id] >= requirement.required,
      )
    ) {
      completedAt = record.start_time;
      break;
    }
  }

  return {
    counts,
    complete: requirements.every(
      (requirement) => counts[requirement.id] >= requirement.required,
    ),
    completedAt,
  };
}

export function countGimmickResults(
  records: SortieRecord[],
  mapNo: number,
  requirements: GimmickRequirement[],
  recordGauge?: number,
): Record<string, number> {
  return calculateGimmickProgress(
    records,
    mapNo,
    requirements,
    recordGauge,
  ).counts;
}

function buildGimmickProgress(
  map: EventMapConfig,
  difficulty: Difficulty,
  records: SortieRecord[],
): Map<string, GimmickProgress> {
  const result = new Map<string, GimmickProgress>();
  let previousGimmickGauge: number | null = null;
  let previousCompletedAt: string | null = null;

  for (const stage of map.stages) {
    if (stage.kind !== "gimmick") {
      previousGimmickGauge = null;
      previousCompletedAt = null;
      continue;
    }
    const afterTime =
      previousGimmickGauge === stage.recordGauge ? previousCompletedAt : null;
    const progress = calculateGimmickProgress(
      records,
      map.mapNo,
      resolveRequirements(stage, difficulty),
      stage.recordGauge,
      afterTime,
    );
    result.set(stage.id, progress);
    previousGimmickGauge = stage.recordGauge;
    previousCompletedAt = progress.complete ? progress.completedAt : null;
  }
  return result;
}

function currentStageFor(
  map: EventMapConfig,
  status: EventMapStatus | undefined,
  progress: Map<string, GimmickProgress>,
): EventStage {
  const lastStage = map.stages[map.stages.length - 1];
  if (status?.cleared) return lastStage;

  for (const stage of map.stages) {
    if (stage.kind === "gimmick") {
      const stageProgress = progress.get(stage.id);
      if (stageProgress?.complete) continue;
      if ((status?.gauge_num ?? 0) > stage.unlocksGauge) continue;
      return stage;
    }

    if ((status?.gauge_num ?? 0) > stage.gaugeNumber) continue;
    if (
      status?.gauge_num === stage.gaugeNumber &&
      status.current_hp != null &&
      status.current_hp <= 0
    ) {
      continue;
    }
    return stage;
  }
  return lastStage;
}

function upsertSortie(records: SortieRecord[], record: SortieRecord): SortieRecord[] {
  const index = records.findIndex((item) => item.id === record.id);
  if (index < 0) return [record, ...records];
  const next = [...records];
  next[index] = record;
  return next;
}

function conditionLabel(requirement: GimmickRequirement): string {
  if (requirement.kind === "victory") {
    return `${requirement.victory}勝利以上`;
  }
  if (requirement.kind === "air") return "航空優勢以上";
  if (requirement.kind === "defense") return "航空優勢以上";
  return "到達";
}

export function EventTab() {
  const [eventMaps, setEventMaps] = useState<EventMapStatus[]>([]);
  const [sorties, setSorties] = useState<SortieRecord[]>([]);
  const [selectedMapNo, setSelectedMapNo] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [statusError, setStatusError] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [maps, logs] = await Promise.all([
        invoke<EventMapStatus[]>("get_event_map_statuses"),
        invoke<BattleLogsResponse>("get_battle_logs", { limit: 3000, offset: 0 }),
      ]);
      setEventMaps(maps);
      setSorties(logs.records);
      setStatusError("");
    } catch (error) {
      setStatusError(String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const unlistenMap = listen<EventMapStatus[]>(
      EVENTS.EVENT_MAP_UPDATED,
      (event) => {
        setEventMaps(event.payload);
        setLoading(false);
      },
    );
    const unlistenUpdate = listen<SortieRecord>(
      EVENTS.SORTIE_UPDATE,
      (event) => setSorties((previous) => upsertSortie(previous, event.payload)),
    );
    const unlistenComplete = listen<SortieRecord>(
      EVENTS.SORTIE_COMPLETE,
      (event) => setSorties((previous) => upsertSortie(previous, event.payload)),
    );
    return () => {
      unlistenMap.then((dispose) => dispose());
      unlistenUpdate.then((dispose) => dispose());
      unlistenComplete.then((dispose) => dispose());
    };
  }, [refresh]);

  useEffect(() => {
    if (selectedMapNo != null || loading) return;
    const active = eventMaps
      .filter((status) => !status.cleared && (status.selected_rank ?? 0) > 0)
      .sort((a, b) => b.map_id - a.map_id)[0];
    setSelectedMapNo(active ? active.map_id % 10 : 1);
  }, [eventMaps, loading, selectedMapNo]);

  const selectedMap =
    EVENT_MAPS.find((map) => map.mapNo === selectedMapNo) ?? EVENT_MAPS[0];
  const selectedStatus = eventMaps.find(
    (status) => status.map_id === selectedMap.mapId,
  );
  const selectedDifficulty =
    rankToDifficulty(selectedStatus?.selected_rank) ?? "甲";
  const gimmickProgress = useMemo(
    () => buildGimmickProgress(selectedMap, selectedDifficulty, sorties),
    [selectedMap, selectedDifficulty, sorties],
  );
  const currentStage = currentStageFor(
    selectedMap,
    selectedStatus,
    gimmickProgress,
  );

  const requirements =
    currentStage.kind === "gimmick"
      ? resolveRequirements(currentStage, selectedDifficulty)
      : [];
  const currentGimmick = gimmickProgress.get(currentStage.id);
  const pendingRequirements = requirements.filter(
    (requirement) =>
      (currentGimmick?.counts[requirement.id] ?? 0) < requirement.required,
  );
  const bossObservation = useMemo(
    () =>
      currentStage.kind === "gauge"
        ? latestBossObservation(
            sorties,
            selectedMap.mapNo,
            currentStage.gaugeNumber,
            currentStage.bossCellNos,
          )
        : null,
    [currentStage, selectedMap.mapNo, sorties],
  );
  const finalBossHp =
    currentStage.kind === "gauge"
      ? currentStage.finalBossHp?.[selectedDifficulty] ?? bossObservation?.maxHp
      : undefined;
  const lastDance = isLastDance(
    selectedStatus,
    finalBossHp,
    bossObservation?.finalForm,
  );
  const hpProgress = gaugeProgress(selectedStatus);
  const mapCleared = selectedStatus?.cleared === true;
  const shimakazeFormations = getShimakazeEventFormationLinks(
    selectedMap.mapNo,
    currentStage.id,
  );
  const konekoFormations = getKonekoEventFormationLinks(
    selectedMap.mapNo,
    currentStage.id,
  );
  const showArmorBreakFormations =
    currentStage.kind === "gauge" &&
    currentStage === selectedMap.stages[selectedMap.stages.length - 1] &&
    lastDance;
  const shimakazeArmorBreakFormations = showArmorBreakFormations
    ? getShimakazeArmorBreakFormationLinks(selectedMap.mapNo)
    : [];
  const konekoArmorBreakFormations = showArmorBreakFormations
    ? getKonekoArmorBreakFormationLinks(selectedMap.mapNo)
    : [];

  return (
    <div className="event-tab">
      <nav className="event-map-selector" aria-label="イベント海域">
        {EVENT_MAPS.map((map) => {
          const status = eventMaps.find((candidate) => candidate.map_id === map.mapId);
          const active = map.mapNo === selectedMap.mapNo;
          return (
            <button
              className={`${active ? "active" : ""} ${status?.cleared ? "cleared" : ""}`}
              key={map.mapNo}
              onClick={() => setSelectedMapNo(map.mapNo)}
              type="button"
            >
              <strong>E{map.mapNo}</strong>
              <span>
                {status?.cleared
                  ? "突破済み"
                  : (status?.selected_rank ?? 0) > 0
                    ? `${rankToDifficulty(status?.selected_rank)}作戦`
                    : "未選択"}
              </span>
            </button>
          );
        })}
      </nav>

      <header className="event-header">
        <div>
          <div className="event-kicker">
            2026年夏イベント・E{selectedMap.mapNo}
          </div>
          <h1>{selectedMap.area}</h1>
          <p>{selectedMap.operation}</p>
        </div>
        <div className={`current-stage-badge ${mapCleared ? "completed" : ""}`}>
          <small>{mapCleared ? "海域" : "現在"}</small>
          <strong>{mapCleared ? "突破済み" : currentStage.label}</strong>
          <span>
            {(selectedStatus?.selected_rank ?? 0) > 0
              ? `${selectedDifficulty}作戦`
              : "甲条件を仮表示"}
          </span>
        </div>
      </header>

      <main className="event-current">
        <section className="event-card">
          <div className="event-card-title">
            <span className="event-card-eyebrow">現在やること</span>
            <h2>{mapCleared ? "海域突破済み" : currentStage.detail}</h2>
          </div>

          {!mapCleared && (
            <>
              <FormationLinks
                eyebrow={EVENT_SHIMAKAZE_FORMATION_HEADING}
                formations={shimakazeFormations}
              />
              <FormationLinks
                eyebrow={EVENT_KONEKO_FORMATION_HEADING}
                formations={konekoFormations}
              />
              <FormationLinks
                eyebrow="装甲破砕・島風編成"
                formations={shimakazeArmorBreakFormations}
              />
              <FormationLinks
                eyebrow="装甲破砕・子猫編成"
                formations={konekoArmorBreakFormations}
              />
              {(shimakazeFormations.length > 0 ||
                konekoFormations.length > 0 ||
                shimakazeArmorBreakFormations.length > 0 ||
                konekoArmorBreakFormations.length > 0) && (
                <p className="event-formation-note">
                  {EVENT_FORMATION_NOTE}
                </p>
              )}
            </>
          )}

          {mapCleared ? (
            <div className="gauge-status">
              <div className="gauge-cleared">E{selectedMap.mapNo} 突破済み</div>
            </div>
          ) : currentStage.kind === "gimmick" ? (
            <>
              <div className="auto-status">
                戦闘・航空戦・基地防空ログから自動判定中
                <span>
                  達成済み {requirements.length - pendingRequirements.length} /{" "}
                  {requirements.length}条件
                </span>
              </div>
              <div className="gimmick-list">
                {requirements.map((requirement) => {
                  const count = currentGimmick?.counts[requirement.id] ?? 0;
                  const completed = count >= requirement.required;
                  return (
                    <div
                      className={`gimmick-row ${completed ? "completed" : ""}`}
                      key={requirement.id}
                    >
                      <div className="gimmick-node">{requirement.node}</div>
                      <div className="gimmick-condition">
                        <strong>{conditionLabel(requirement)}</strong>
                        <small>
                          {completed ? "✓ 達成済み" : "保存ログを自動集計"}
                        </small>
                      </div>
                      <div className="automatic-count">
                        <strong>{Math.min(count, requirement.required)}</strong>
                        <span> / {requirement.required}回</span>
                      </div>
                    </div>
                  );
                })}
              </div>
              <p className="event-note">
                達成した条件も一覧に残ります。全条件達成後は次の攻略段階へ自動で切り替わります。
                {(selectedStatus?.selected_rank ?? 0) === 0 &&
                  " 難易度選択後、実際の条件へ自動更新します。"}
              </p>
            </>
          ) : (
            <div className={`gauge-status ${lastDance ? "last-dance" : ""}`}>
              <div className="gauge-label">
                <span>{currentStage.detail}</span>
                <span>
                  {lastDance && <b className="last-dance-badge">ラスダン</b>}
                  {selectedStatus?.provisional && (
                    <b className="provisional-badge">暫定</b>
                  )}
                  第{currentStage.gaugeNumber}ゲージ
                </span>
              </div>
              {loading ? (
                <p className="muted">ゲージ情報を取得中…</p>
              ) : statusError ? (
                <p className="status-error">取得できませんでした: {statusError}</p>
              ) : selectedStatus?.current_hp != null &&
                selectedStatus.max_hp != null ? (
                <>
                  <div className="gauge-numbers">
                    <strong>{selectedStatus.current_hp.toLocaleString()}</strong>
                    <span> / {selectedStatus.max_hp.toLocaleString()} HP</span>
                    {hpProgress != null && <em>{Math.round(hpProgress)}% 進行</em>}
                  </div>
                  <div className="gauge-bar">
                    <span style={{ width: `${hpProgress ?? 0}%` }} />
                  </div>
                  {lastDance && (
                    <div className="last-dance-callout">
                      <strong>最終形態（ラスダン）</strong>
                      <span>
                        ボス撃破でゲージ破壊
                        {finalBossHp != null &&
                          `・最終ボスHP ${finalBossHp.toLocaleString()}`}
                      </span>
                    </div>
                  )}
                  {selectedStatus.provisional && (
                    <p className="provisional-note">
                      ボス戦結果から計算した暫定値です。次の出撃画面で正式値に更新します。
                    </p>
                  )}
                </>
              ) : (
                <p className="muted">
                  難易度選択または出撃画面の更新後、現在のボスHPと進行度を自動表示します。
                </p>
              )}
            </div>
          )}
        </section>
      </main>
    </div>
  );
}
