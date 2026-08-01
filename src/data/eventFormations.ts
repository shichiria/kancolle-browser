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
// UIでは出典名を反転した「島風編成」として表示する。
// Key: E海域番号:EventTab上の攻略段階ID
const SHIMAKAZE_EVENT_FORMATIONS: Record<string, EventFormationLink[]> = {
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
const SHIMAKAZE_ARMOR_BREAK_FORMATIONS: Record<number, EventFormationLink[]> = {
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

// キトンの艦これ攻略ブログ 2026年夏イベント攻略記事の編成。
// UIでは「子猫編成」として表示する。
const KONEKO_EVENT_FORMATIONS: Record<string, EventFormationLink[]> = {
  "1:gimmick1": links([
    ["C2マス", "0EsH"],
    ["C3マス", "j5x0"],
    ["F・Hマス", "Nwez"],
  ]),
  "1:gauge1": links([["攻略編成", "pSv1"]]),
  "1:gimmick2": links([
    ["Lマス", "yzoD"],
    ["基地防空", "i5t0"],
  ]),
  "1:gauge2": links([["輸送編成", "NXJW"]]),
  "1:gauge3": links([["攻略編成", "aAJC"]]),

  "2:gimmick1": links([
    ["Dマス", "ql8G"],
    ["C2マス", "SR7x"],
  ]),
  "2:gimmick2": links([
    ["Hマス", "Oq9I"],
    ["G2マス", "FYA3"],
    ["Bマス", "aS1i"],
    ["基地防空", "Wbrh"],
  ]),
  "2:gauge1": links([["輸送編成", "FJrb"]]),
  "2:gauge2": links([["攻略編成", "vllj"]]),
  "2:gauge3": links([
    ["ゲージ破壊", "bKlC"],
    ["ゲージ削り", "X5yO"],
  ]),

  "3:gimmick1": links([
    ["B2マス", "s61f"],
    ["C2マス", "Ns5b"],
    ["E2マス", "XkB4"],
    ["D2マス", "6wqm"],
  ]),
  "3:gauge1": links([["攻略編成", "X2eq"]]),
  "3:gauge2": links([["輸送編成", "CqoV"]]),
  "3:gauge3": links([["攻略編成", "Kz0a"]]),
  "3:gauge4": links([["攻略編成", "AbNB"]]),

  "4:gauge1": links([["攻略編成", "2Z4u"]]),
  "4:gimmick1": links([["E2マス", "32pn"]]),
  "4:gauge2": links([["輸送編成", "4PiN"]]),
  "4:gauge3": links([["攻略編成", "MfKy"]]),
  "4:gauge4": links([["攻略編成", "faey"]]),
  "4:gauge5": links([["攻略編成", "sKG2"]]),

  "5:gimmick1": links([
    ["B2マス", "Jh26"],
    ["C2マス", "8koJ"],
    ["Dマス", "bAxe"],
  ]),
  "5:gimmick2": links([["E2マス", "691x"]]),
  "5:gauge1": links([
    ["ゲージ削り", "8p03"],
    ["ゲージ破壊", "3d0D"],
  ]),
  "5:gauge2": links([["輸送編成", "qawU"]]),
  "5:gimmick3": links([
    ["L2マス", "H2Du"],
    ["L1マス", "iIjd"],
  ]),
  "5:gimmick4": links([
    ["Pマス", "ngTo"],
    ["P3マス", "6N28"],
  ]),
  "5:gauge3": links([["攻略編成", "9Mfg"]]),
  "5:gauge4": links([["攻略編成", "jp9z"]]),
};

const KONEKO_ARMOR_BREAK_FORMATIONS: Record<number, EventFormationLink[]> = {
  1: links([
    ["Tマス", "7Fij"],
    ["Iマス", "zm2h"],
    ["Lマス", "yzoD"],
    ["A2マス", "vcBN"],
    ["基地防空", "i5t0"],
  ]),
  2: links([
    ["Vマス", "AHkW"],
    ["Hマス", "R2n8"],
    ["Pマス", "6mtV"],
    ["W2マス", "MfzP"],
    ["基地防空", "nSCi"],
  ]),
  3: links([
    ["Oマス", "ybFI"],
    ["Qマス", "VB2X"],
    ["Xマス", "nr6W"],
    ["Wマス", "ck96"],
    ["Y3マス", "0rhn"],
    ["基地防空", "9EdJ"],
    ["E2マス", "XkB4"],
  ]),
  4: links([
    ["Sマス", "ADYJ"],
    ["X・T1マス", "T6mA"],
    ["Nマス", "q4y7"],
    ["Dマス", "iLsT"],
    ["E2マス", "VhHP"],
    ["Y・Y1マス", "qHIq"],
    ["基地防空", "yLuO"],
  ]),
  5: links([
    ["P3・P2マス", "YmE5"],
    ["J2マス", "NePo"],
    ["Sマス", "dTcN"],
    ["L1マス", "8MvQ"],
    ["L2マス", "l1lq"],
    ["Gマス", "Ropy"],
    ["C2マス", "QXpj"],
    ["E2マス", "jckt"],
    ["Y2マス", "RQdH"],
    ["基地防空", "sCZX"],
  ]),
};

export function getShimakazeEventFormationLinks(
  mapNo: number,
  stageId: string,
): EventFormationLink[] {
  return SHIMAKAZE_EVENT_FORMATIONS[`${mapNo}:${stageId}`] ?? [];
}

export function getKonekoEventFormationLinks(
  mapNo: number,
  stageId: string,
): EventFormationLink[] {
  return KONEKO_EVENT_FORMATIONS[`${mapNo}:${stageId}`] ?? [];
}

export function getShimakazeArmorBreakFormationLinks(
  mapNo: number,
): EventFormationLink[] {
  return SHIMAKAZE_ARMOR_BREAK_FORMATIONS[mapNo] ?? [];
}

export function getKonekoArmorBreakFormationLinks(
  mapNo: number,
): EventFormationLink[] {
  return KONEKO_ARMOR_BREAK_FORMATIONS[mapNo] ?? [];
}

// 既存の呼び出し元向け。従来の編成系列は島風編成として維持する。
export const getEventFormationLinks = getShimakazeEventFormationLinks;
export const getArmorBreakFormationLinks =
  getShimakazeArmorBreakFormationLinks;

export type EventFormationSource = "shimakaze" | "koneko";

export interface EventFormationAuditEntry extends EventFormationLink {
  mapNo: number;
  stageId: string;
  armorBreak: boolean;
  source: EventFormationSource;
}

export function getAllEventFormationLinks(): EventFormationAuditEntry[] {
  const collect = (
    source: EventFormationSource,
    stages: Record<string, EventFormationLink[]>,
    armorBreaks: Record<number, EventFormationLink[]>,
  ): EventFormationAuditEntry[] => [
    ...Object.entries(stages).flatMap(([key, formations]) => {
      const [mapNo, stageId] = key.split(":");
      return formations.map((formation) => ({
        ...formation,
        mapNo: Number(mapNo),
        stageId,
        armorBreak: false,
        source,
      }));
    }),
    ...Object.entries(armorBreaks).flatMap(([mapNo, formations]) =>
      formations.map((formation) => ({
        ...formation,
        mapNo: Number(mapNo),
        stageId: "armorBreak",
        armorBreak: true,
        source,
      })),
    ),
  ];

  return [
    ...collect(
      "shimakaze",
      SHIMAKAZE_EVENT_FORMATIONS,
      SHIMAKAZE_ARMOR_BREAK_FORMATIONS,
    ),
    ...collect(
      "koneko",
      KONEKO_EVENT_FORMATIONS,
      KONEKO_ARMOR_BREAK_FORMATIONS,
    ),
  ];
}
