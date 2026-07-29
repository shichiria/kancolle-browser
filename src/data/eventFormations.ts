export interface EventFormationLink {
  label: string;
  url: string;
}

const links = (
  entries: Array<[label: string, id: string]>,
): EventFormationLink[] =>
  entries.map(([label, id]) => ({
    label,
    url: `https://kc.noro6.net/s/${id}`,
  }));

// ぜかましねっと 2026年夏イベント攻略記事の「制空シミュ」完全版編成。
// Key: E海域番号:EventTab上の攻略段階ID
const EVENT_FORMATIONS: Record<string, EventFormationLink[]> = {
  "1:gimmick1": links([
    ["C2・C3マス", "R082"],
    ["F・Hマス", "9huG"],
  ]),
  "1:gauge1": links([
    ["攻略編成", "tTo3"],
  ]),
  "1:gimmick2": links([
    ["Lマス", "igQm"],
    ["基地防空", "mQvo"],
  ]),
  "1:gauge2": links([
    ["水雷戦隊", "c92i"],
    ["夜襲空母採用", "ilCb"],
  ]),
  "1:gauge3": links([
    ["攻略編成", "hiqx"],
  ]),

  "2:gimmick1": links([
    ["C2マス", "qKuZ"],
    ["Dマス", "U65O"],
  ]),
  "2:gimmick2": links([
    ["Bマス", "I3hw"],
    ["Hマス", "mbtW"],
    ["G2マス", "XjgC"],
    ["基地防空", "WGHl"],
  ]),
  "2:gauge1": links([
    ["攻略編成", "jF88"],
  ]),
  "2:gauge2": links([
    ["水上打撃部隊", "NIx3"],
  ]),
  "2:gauge3": links([
    ["大和・武蔵／破壊", "Dch9"],
    ["長門・陸奥／破壊", "dk2F"],
    ["長門・陸奥／削り", "LtOX"],
    ["長門・陸奥／基地航空隊", "ecbv"],
  ]),

  "3:gimmick1": links([
    ["B2マス", "7Pua"],
    ["C2マス", "eHSX"],
    ["D2マス", "EvZF"],
    ["E2マス", "V1PV"],
    ["基地航空隊", "1a63"],
  ]),
  "3:gauge1": links([
    ["ゲージ破壊", "Cnb6"],
    ["ゲージ削り", "t1qG"],
  ]),
  "3:gauge2": links([
    ["遊撃部隊", "nLtu"],
    ["高速＋編成", "3TxU"],
  ]),
  "3:gauge3": links([
    ["駆逐2・潜水4・潜母1", "fQr6"],
    ["潜水7", "VwY1"],
  ]),
  "3:gauge4": links([
    ["ゲージ破壊", "7X56"],
    ["ゲージ削り", "2iIc"],
  ]),

  "4:gauge1": links([
    ["攻略編成", "1NiR"],
  ]),
  "4:gimmick1": links([
    ["E2マス", "gnja"],
  ]),
  "4:gauge2": links([
    ["輸送護衛部隊", "PKbK"],
  ]),
  "4:gauge3": links([
    ["空母機動部隊", "Hd4I"],
  ]),
  "4:gauge4": links([
    ["水上打撃部隊", "9bBK"],
  ]),
  "4:gauge5": links([
    ["ゲージ破壊", "cnj4"],
    ["ゲージ破壊・基地劣勢案", "1PAC"],
    ["ゲージ削り", "xQ5l"],
  ]),

  "5:gimmick1": links([
    ["B2マス", "NXC2"],
    ["C2・Dマス", "Ut3A"],
  ]),
  "5:gimmick2": links([
    ["E2マス", "GIDL"],
    ["基地防空", "n0Zp"],
  ]),
  "5:gauge1": links([
    ["欧州棲姫／破壊", "eeZr"],
    ["潜水夏姫／削り", "Yk3m"],
  ]),
  "5:gauge2": links([
    ["水上打撃部隊", "LksQ"],
  ]),
  // E5ギミック3・4、ゲージ3は攻略記事側の完全版編成が準備中。
  "5:gauge4": links([
    ["Warspite・Valiant／破壊", "0QWQ"],
    ["Warspite・Valiant／削り", "f7CD"],
    ["Richelieu・Jean Bart", "h4p7"],
  ]),
};

// 装甲破砕は最終ゲージのラスダン時に併記する。
const ARMOR_BREAK_FORMATIONS: Record<number, EventFormationLink[]> = {
  1: links([
    ["Tマス（第二ボス）", "fYY1"],
    ["F・Iマス（第一ボス）", "2QEn"],
    ["Iマス（別編成）", "7mSn"],
    ["Lマス", "fdYT"],
    ["A2マス", "tA4Y"],
    ["基地防空", "mQvo"],
  ]),
  2: links([
    ["Pマス（第一ボス）", "cmso"],
    ["Vマス（水上打撃）", "HEwE"],
    ["W2マス（空母機動）", "xWEa"],
    ["Hマス", "Qjiy"],
    ["基地航空隊", "1S78"],
    ["基地防空", "UOpQ"],
  ]),
  3: links([
    ["Qマス", "lFzl"],
    ["Wマス", "9yHV"],
    ["Xマス", "WkdL"],
    ["Oマス", "XAjn"],
    ["Y3マス", "yrKA"],
    ["Xマス基地航空隊", "VwY1"],
    ["基地防空", "8ef7"],
  ]),
  4: links([
    ["Nマス", "4exI"],
    ["Xマス", "jz7u"],
    ["Sマス", "xi0R"],
    ["Y・Y1マス", "dYaB"],
    ["Dマス", "xx5W"],
    ["E2マス", "scX4"],
    ["基地防空", "n0Zp"],
  ]),
  5: links([
    ["J2マス（英国救援艦隊）", "6rMq"],
    ["J2マス（欧州連合艦隊）", "xugB"],
    ["C2・G1マス", "Yk3m"],
    ["E2マス", "fLKY"],
    ["基地防空", "n0Zp"],
  ]),
};

export function getEventFormationLinks(
  mapNo: number,
  stageId: string,
): EventFormationLink[] {
  return EVENT_FORMATIONS[`${mapNo}:${stageId}`] ?? [];
}

export function getArmorBreakFormationLinks(
  mapNo: number,
): EventFormationLink[] {
  return ARMOR_BREAK_FORMATIONS[mapNo] ?? [];
}

export interface EventFormationAuditEntry extends EventFormationLink {
  mapNo: number;
  stageId: string;
  armorBreak: boolean;
}

export function getAllEventFormationLinks(): EventFormationAuditEntry[] {
  const stages = Object.entries(EVENT_FORMATIONS).flatMap(
    ([key, formations]) => {
      const [mapNo, stageId] = key.split(":");
      return formations.map((formation) => ({
        ...formation,
        mapNo: Number(mapNo),
        stageId,
        armorBreak: false,
      }));
    },
  );
  const armorBreaks = Object.entries(ARMOR_BREAK_FORMATIONS).flatMap(
    ([mapNo, formations]) =>
      formations.map((formation) => ({
        ...formation,
        mapNo: Number(mapNo),
        stageId: "armorBreak",
        armorBreak: true,
      })),
  );
  return [...stages, ...armorBreaks];
}
