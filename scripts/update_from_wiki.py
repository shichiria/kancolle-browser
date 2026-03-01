#!/usr/bin/env python3
"""
Update sortie_quests.json with fleet composition conditions and
area/rank/boss_only/count data based on KanColle wiki quest information.

Rules:
- Only modify conditions for quests that currently have empty conditions []
- Never modify existing non-empty conditions (from ProgressSpecialBattle.cs)
- Never modify the recommended field
- Always update area/rank/boss_only/count from wiki data (overwrite)

KEY: Quest name (任務名) — avoids Bxx numbering differences between Wiki and EO
"""

import json
from pathlib import Path

JSON_PATH = Path(__file__).parent.parent / "src-tauri" / "data" / "sortie_quests.json"

# ============================================================
# Ship type constants
# ============================================================
DE = 1    # 海防
DD = 2    # 駆逐
CL = 3    # 軽巡
CLT = 4   # 雷巡
CA = 5    # 重巡
CAV = 6   # 航巡
CVL = 7   # 軽空母
FBB = 8   # 高速戦艦
BB = 9    # 戦艦(低速)
BBV = 10  # 航戦
CV = 11   # 正規空母
SS = 13   # 潜水艦
SSV = 14  # 潜水空母
AV = 16   # 水母
LHA = 17  # 揚陸艦
CVB = 18  # 装甲空母
AR = 19   # 工作艦
AS = 20   # 潜水母艦
CT = 21   # 練巡
AO = 22   # 補給艦

# Compound type groups
CARRIER = [CVL, CV, CVB]          # 航空母艦
SEIKI_CV = [CV, CVB]              # 正規空母
BB_ALL = [FBB, BB, BBV]           # 戦艦級
BB_FAST = [FBB]                   # 高速戦艦
BB_SLOW = [BB, BBV]               # 低速戦艦+航戦
CA_CLASS = [CA, CAV]              # 重巡級
CL_CLASS = [CL, CLT, CT]         # 軽巡級
SUB = [SS, SSV]                   # 潜水艦
DD_DE = [DD, DE]                  # 駆逐/海防


# ============================================================
# Helper functions for building condition objects
# ============================================================

def ST(ship_type_name, stypes, value):
    """ShipTypeCount condition"""
    return {"type": "ShipTypeCount", "ship_type": ship_type_name, "stypes": stypes, "value": value}

def FS(ship_type_name, stypes):
    """FlagshipType condition"""
    return {"type": "FlagshipType", "ship_type": ship_type_name, "stypes": stypes}

def SN(names, count=None):
    """ContainsShipName condition - all names must be present"""
    if count is None:
        count = len(names)
    return {"type": "ContainsShipName", "names": names, "count": count}

def SNA(names, count):
    """ContainsShipNameAny condition - count of names must be present"""
    return {"type": "ContainsShipNameAny", "names": names, "count": count}

def SC(value):
    """ShipCount condition - max number of ships"""
    return {"type": "ShipCount", "value": value}

def ONLY(desc, stypes):
    """OnlyShipTypes condition"""
    return {"type": "OnlyShipTypes", "desc": desc, "stypes": stypes}


# ============================================================
# Wiki-sourced quest data
# Key: Quest NAME (任務名) — matches the "name" field in sortie_quests.json
# Value: dict with fields:
#   area, rank, boss_only, count — always set
#   conditions — only set for quests without ProgressSpecialBattle.cs conditions
# ============================================================

WIKI_DATA = {
    # ================================================================
    # DAILY QUESTS (Bd series)
    # ================================================================

    "敵艦隊を撃破せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "敵艦隊主力を撃滅せよ！": {
        "area": "任意", "rank": "B", "boss_only": True, "count": 1,
    },
    "敵艦隊を10回邀撃せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 10,
    },
    "敵空母を３隻撃沈せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "敵補給艦を3隻撃沈せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "敵輸送船団を叩け！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "南西諸島海域の制海権を握れ！": {
        "area": "2-1/2-2/2-3/2-4/2-5", "rank": "B", "boss_only": True, "count": 5,
    },
    "敵潜水艦を制圧せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },

    # ================================================================
    # WEEKLY QUESTS (Bw series)
    # ================================================================

    "あ号作戦": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "い号作戦": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "海上通商破壊作戦": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "ろ号作戦": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "海上護衛戦": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
    },
    "敵北方艦隊主力を撃滅せよ！": {
        "area": "3-3/3-4/3-5", "rank": "B", "boss_only": True, "count": 5,
    },
    "敵北方母港を叩け！": {
        "area": "3-3/3-4/3-5", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            ST("空母", CARRIER, 2),
        ],
    },
    "海上輸送路の安全確保に努めよ！": {
        "area": "1-5", "rank": "A", "boss_only": True, "count": 3,
    },
    "海上護衛を強化せよ！": {
        "area": "1-5", "rank": "A", "boss_only": True, "count": 10,
    },
    "南方海域珊瑚諸島沖の制空権を握れ！": {
        "area": "5-2", "rank": "S", "boss_only": True, "count": 2,
    },

    # ================================================================
    # MONTHLY QUESTS (Bm series) — most have CS conditions already
    # ================================================================

    "「第五戦隊」出撃せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions: 那智+妙高+羽黒
    },
    "「潜水艦隊」出撃せよ！": {
        "area": "6-1", "rank": "S", "boss_only": True, "count": 3,
        # Duplicate name with B17; this is Bm2 (monthly)
    },
    "「水雷戦隊」南西へ！": {
        "area": "1-4", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "「水上打撃部隊」南方へ！": {
        "area": "5-1", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "海上護衛強化月間": {
        "area": "1-5", "rank": "A", "boss_only": True, "count": 10,
    },
    "「空母機動部隊」西へ！": {
        "area": "4-2", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "「水上反撃部隊」突入せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "兵站線確保！海上警備を強化実施せよ！": {
        "area": "1-2/1-3/1-4/2-1", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },

    # ================================================================
    # QUARTERLY QUESTS (Bq series) — most have CS conditions
    # ================================================================

    "沖ノ島海域迎撃戦": {
        "area": "2-4", "rank": "S", "boss_only": True, "count": 2,
    },
    "戦果拡張任務！「Z作戦」前段作戦": {
        "area": "2-4/6-1/6-3/6-4", "rank": "A", "boss_only": True, "count": 1,
    },
    "強行輸送艦隊、抜錨！": {
        "area": "1-6", "rank": "", "boss_only": False, "count": 2,
        # CS has conditions
    },
    "前線の航空偵察を実施せよ！": {
        "area": "6-3", "rank": "A", "boss_only": True, "count": 2,
        # CS has conditions
    },
    "北方海域警備を実施せよ！": {
        "area": "3-1/3-2/3-3", "rank": "A", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "精鋭「三一駆」、鉄底海域に突入せよ！": {
        "area": "5-4", "rank": "S", "boss_only": True, "count": 2,
        # CS has conditions
    },
    "南西方面の兵站航路の安全を図れ！": {
        "area": "1-4/1-6/2-1/2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽空母/軽巡/練巡", [CVL, CL, CT], 1),
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    "新編成「三川艦隊」、鉄底海峡に突入せよ！": {
        "area": "5-1/5-3/5-4", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "空母戦力の投入による兵站線戦闘哨戒": {
        "area": "1-3/1-4/2-1/2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "戦果拡張任務！「Z作戦」後段作戦": {
        "area": "5-5/6-2/6-5/7-2(2nd)", "rank": "S", "boss_only": True, "count": 1,
    },
    "南西諸島方面「海上警備行動」発令！": {
        "area": "1-4/2-1/2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "発令！「西方海域作戦」": {
        "area": "4-1/4-2/4-3/4-4/4-5", "rank": "S", "boss_only": True, "count": 1,
    },
    "拡張「六水戦」、最前線へ！": {
        "area": "5-1/5-4/6-4/6-5", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },

    # ================================================================
    # YEARLY QUESTS (By series) — some have CS conditions
    # ================================================================

    "精鋭「十九駆」、躍り出る！": {
        "area": "2-5/3-4/4-5/5-3", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "「海防艦」、海を護る！": {
        "area": "1-1/1-2/1-3/1-5/1-6", "rank": "A", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "工作艦「明石」護衛任務": {
        "area": "1-3/2-1/2-2/2-3/1-6", "rank": "A", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "重巡戦隊、西へ！": {
        "area": "4-1/4-2/4-3/4-4", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "歴戦「第十方面艦隊」、全力出撃！": {
        "area": "4-2/7-2/7-3", "rank": "S", "boss_only": True, "count": 1,
        # CS has conditions
    },
    "鎮守府近海海域の哨戒を実施せよ！": {
        "area": "1-2/1-3/1-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽巡/練巡", [CL, CT], 1),
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    "南西方面の兵站航路の安全を図れ！": {
        # Duplicate name with Bq7 — handled by position in JSON
        "area": "1-5/1-6/2-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽巡/練巡", [CL, CT], 1),
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    "空母機動部隊、出撃！敵艦隊を迎撃せよ！": {
        "area": "2-4/2-5/3-5/4-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航空母艦", CARRIER),
            ST("重巡級", CA_CLASS, 2),
        ],
    },
    "AL作戦": {
        "area": "3-1/3-3/3-4/3-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽空母", [CVL], 2),
        ],
    },
    "機動部隊決戦": {
        "area": "5-2/5-5/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航空母艦", CARRIER),
        ],
    },
    "日英米合同水上艦隊、抜錨せよ！": {
        "area": "2-4/2-5/5-5/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            ST("海防", [DE], 3),
            SC(5),
        ],
    },
    "精鋭「第十九駆逐隊」、全力出撃！": {
        "area": "2-3/3-5/4-5/5-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            ST("駆逐", [DD], 3),
        ],
    },
    "精強「第七駆逐隊」緊急出動！": {
        "area": "1-5/2-3/2-4/7-2(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡級", CA_CLASS),
            ST("駆逐", [DD], 2),
        ],
    },
    "鵜来型海防艦、静かな海を防衛せよ！": {
        "area": "1-2/1-3/1-5/7-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("戦艦級", BB_ALL),
            ST("駆逐", [DD], 2),
        ],
    },
    "「第三戦隊」第二小隊、鉄底海峡へ！": {
        "area": "5-1/5-3/5-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽巡級", CL_CLASS, 1),
            ST("駆逐", [DD], 3),
        ],
    },

    # ================================================================
    # ONE-TIME QUESTS B11-B50
    # ================================================================

    "「三川艦隊」出撃せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
        "conditions": [
            SN(["鳥海", "青葉", "衣笠", "古鷹", "加古", "天龍"], 6),
        ],
    },
    "「第六駆逐隊」出撃せよ！": {
        "area": "任意", "rank": "", "boss_only": False, "count": 1,
        "conditions": [
            SN(["暁", "響", "雷", "電"], 4),
            SC(4),
        ],
    },
    "「第四戦隊」出撃せよ！": {
        "area": "2-2", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SN(["愛宕", "高雄", "鳥海", "摩耶"], 4),
        ],
    },
    "「西村艦隊」出撃せよ！": {
        "area": "2-3", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["扶桑", "山城", "最上", "時雨", "満潮", "朝雲", "山雲"], 4),
        ],
    },
    "「第五航空戦隊」出撃せよ！": {
        "area": "3-1", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SN(["翔鶴", "瑞鶴"], 2),
        ],
    },
    "新「三川艦隊」出撃せよ！": {
        "area": "2-3", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SN(["鳥海", "青葉", "衣笠", "加古", "古鷹", "天龍"], 6),
            SC(6),
        ],
    },
    # Note: B17「潜水艦隊」出撃せよ！ shares name with Bm2; handled by quest_id
    "「航空水上打撃艦隊」出撃せよ！": {
        "area": "4-2", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            ST("航巡", [CAV], 2),
            ST("航戦", [BBV], 2),
        ],
    },
    "「第六戦隊」出撃せよ！": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["古鷹", "加古", "青葉", "衣笠"], 4),
        ],
    },
    "「第八駆逐隊」出撃せよ！": {
        "area": "2-3", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SN(["朝潮", "満潮", "大潮", "荒潮"], 4),
        ],
    },
    "「第十八駆逐隊」出撃せよ！": {
        "area": "3-1", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SN(["霰", "霞", "陽炎", "不知火"], 4),
        ],
    },
    "「第三十駆逐隊(第一次)」出撃せよ！": {
        "area": "3-2", "rank": "C", "boss_only": True, "count": 1,
        "conditions": [
            SN(["睦月", "如月", "弥生", "望月"], 4),
            SC(6),
        ],
    },
    "「航空戦艦」抜錨せよ！": {
        "area": "4-4", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            ST("航戦", [BBV], 2),
        ],
    },
    "「第三十駆逐隊」対潜哨戒！": {
        "area": "1-5", "rank": "C", "boss_only": True, "count": 1,
        "conditions": [
            SN(["睦月", "卯月", "弥生", "望月"], 4),
            SC(4),
        ],
    },
    "新編「第二航空戦隊」出撃せよ！": {
        "area": "5-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["飛龍改二"]),
            SN(["蒼龍"]),
            ST("駆逐", [DD], 2),
        ],
    },
    # B26 精鋭「第二航空戦隊」抜錨せよ！ — has CS conditions
    "戦艦「榛名」出撃せよ！": {
        "area": "5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["榛名改二"]),
        ],
    },
    "「第六〇一航空隊」出撃せよ！": {
        "area": "5-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["雲龍改"]),
        ],
    },
    "「軽空母」戦隊、出撃せよ！": {
        "area": "2-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽空母", [CVL], 1),
            ST("軽巡", [CL], 1),
            ONLY("軽空母+軽巡+駆逐", [CVL, CL, DD]),
        ],
    },
    "「水雷戦隊」バシー島沖緊急展開": {
        "area": "2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            ONLY("軽巡+駆逐", [CL, DD]),
        ],
    },
    "「第二戦隊」抜錨！": {
        "area": "4-2", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SN(["長門", "陸奥", "扶桑", "山城"], 4),
        ],
    },
    "「戦艦部隊」北方海域に突入せよ！": {
        "area": "3-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("戦艦", BB_ALL, 2),
            ST("軽空母", [CVL], 1),
        ],
    },
    # B33 西村艦隊南方 — has CS conditions
    "「第六戦隊」南西海域へ出撃せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["古鷹", "加古", "青葉", "衣笠"], 4),
        ],
    },
    "「第十一駆逐隊」出撃せよ！": {
        "area": "2-3", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["吹雪", "白雪", "初雪", "深雪", "叢雲"], 4),
        ],
    },
    "「第十一駆逐隊」対潜哨戒！": {
        "area": "1-5", "rank": "C", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["吹雪", "白雪", "初雪", "深雪", "叢雲"], 4),
            SC(4),
        ],
    },
    "「第二一駆逐隊」出撃せよ！": {
        "area": "3-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["初春", "子日", "若葉", "初霜"], 4),
        ],
    },
    "「那智戦隊」抜錨せよ！": {
        "area": "2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡", [CA]),
            SN(["那智"]),
            SNA(["初霜", "霞", "潮", "曙"], 4),
        ],
    },
    "「第二二駆逐隊」出撃せよ！": {
        "area": "1-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["皐月", "文月", "長月"], 3),
            ST("駆逐", [DD], 4),
        ],
    },
    "「改装防空重巡」出撃せよ！": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["摩耶改"]),
            ST("軽巡", [CL], 1),
            ST("駆逐", [DD], 2),
        ],
    },
    "新編「三川艦隊」ソロモン方面へ！": {
        "area": "5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡", [CA]),
            SN(["鳥海改二"]),
            SNA(["古鷹", "加古", "青葉", "衣笠", "夕張", "天龍"], 5),
            SC(6),
        ],
    },
    "「第六駆逐隊」対潜哨戒なのです！": {
        "area": "1-5", "rank": "C", "boss_only": True, "count": 1,
        "conditions": [
            SN(["暁", "響", "雷", "電"], 4),
            SC(4),
        ],
    },
    "抜錨！「第十八戦隊」": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["天龍", "龍田"], 2),
        ],
    },
    # B44 海上突入部隊、進発せよ！ — has CS conditions
    "「第六駆逐隊」対潜哨戒を徹底なのです！": {
        "area": "1-5", "rank": "A", "boss_only": True, "count": 4,
        "conditions": [
            SN(["暁", "響", "雷", "電"], 4),
            SC(4),
        ],
    },
    "「第一水雷戦隊」ケ号作戦、突入せよ！": {
        "area": "3-2", "rank": "B", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["阿武隈"]),
            SN(["響", "初霜", "若葉", "五月雨", "島風"], 5),
            SC(6),
        ],
    },
    "「第一水雷戦隊」北方ケ号作戦、再突入！": {
        "area": "3-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["阿武隈改二"]),
            SN(["響", "夕雲", "長波", "秋雲", "島風"], 5),
            SC(6),
        ],
    },
    "鎮守府正面の対潜哨戒を強化せよ！": {
        "area": "1-5", "rank": "A", "boss_only": True, "count": 4,
    },
    "「空母機動部隊」北方海域に進出せよ！": {
        "area": "3-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航空母艦", CARRIER),
        ],
    },
    "「第五航空戦隊」珊瑚諸島沖に出撃せよ！": {
        "area": "5-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["翔鶴", "瑞鶴", "朧", "秋雲"], 4),
        ],
    },

    # ================================================================
    # ONE-TIME QUESTS B51-B100
    # ================================================================

    "新編「第二一戦隊」北方へ出撃せよ！": {
        "area": "3-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["那智改二", "足柄改二", "多摩", "木曾"], 4),
        ],
    },
    "「第十六戦隊(第一次)」出撃せよ！": {
        "area": "2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡", [CA]),
            SN(["足柄"]),
            SN(["球磨", "長良"], 2),
        ],
    },
    "「第三航空戦隊」南西諸島防衛線に出撃！": {
        "area": "1-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["瑞鶴改"]),
            SN(["千歳航", "千代田航", "瑞鳳"], 3),
        ],
    },
    "「小沢艦隊」出撃せよ！": {
        "area": "2-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["瑞鶴改"]),
            SN(["瑞鳳改", "千歳航", "千代田航", "伊勢改", "日向改"], 5),
            SC(6),
        ],
    },
    "「第十六戦隊(第二次)」出撃せよ！": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["名取"]),
            SN(["五十鈴", "鬼怒"], 2),
        ],
    },
    "新編成航空戦隊、北方へ進出せよ！": {
        "area": "3-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("空母", CARRIER, 2),
            ST("航戦/航巡", [BBV, CAV], 2),
            ST("駆逐", [DD], 2),
        ],
    },
    "「礼号作戦」実施せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["霞"]),
            SN(["足柄", "大淀", "朝霜", "清霜"], 4),
        ],
    },
    "旗艦「霞」北方海域を哨戒せよ！": {
        "area": "3-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["霞改二"]),
            ST("駆逐", [DD], 4),
        ],
    },
    "旗艦「霞」出撃！敵艦隊を撃滅せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["霞改二"]),
            ST("駆逐", [DD], 3),
        ],
    },
    "「第三十一戦隊」出撃せよ！": {
        "area": "1-6", "rank": "", "boss_only": False, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["五十鈴改二"]),
            SN(["皐月改二", "卯月改"], 2),
        ],
    },
    "「第二七駆逐隊」出撃せよ！": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["白露改"]),
            SN(["時雨", "春雨", "五月雨"], 3),
        ],
    },
    "強襲上陸作戦用戦力を増強せよ！": {
        "area": "6-3", "rank": "A", "boss_only": True, "count": 1,
    },
    "製油所地帯を防衛せよ！": {
        "area": "1-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽巡", [CL], 1),
            ONLY("軽巡+駆逐", [CL, DD]),
        ],
    },
    "南西諸島防衛線を強化せよ！": {
        "area": "1-4", "rank": "S", "boss_only": True, "count": 5,
    },
    "オリョール海の制海権を確保せよ！": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 6,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["大潮"]),
        ],
    },
    "旗艦「大潮」出撃せよ！": {
        "area": "3-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["大潮改二"]),
        ],
    },
    "艦隊、三周年！": {
        "area": "2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
    },
    "強行高速輸送部隊、出撃せよ！": {
        "area": "4-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["川内改二", "江風改二", "時雨改二"], 3),
            ST("駆逐", [DD], 4),
        ],
    },
    "「第一航空戦隊」西へ！": {
        "area": "4-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["赤城"]),
            SN(["加賀"]),
        ],
    },
    "新編艦隊、南西諸島防衛線へ急行せよ！": {
        "area": "1-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡級", CL_CLASS),
            ST("駆逐", [DD], 4),
        ],
    },
    "鎮守府近海航路の安全確保を強化せよ！": {
        "area": "1-6", "rank": "", "boss_only": False, "count": 1,
        "conditions": [
            FS("軽巡/練巡", [CL, CT]),
            ST("駆逐", [DD], 4),
        ],
    },
    "「第三十一戦隊」敵潜を制圧せよ！": {
        "area": "1-6", "rank": "", "boss_only": False, "count": 2,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["五十鈴改二"]),
            SN(["皐月改二", "卯月改"], 2),
        ],
    },
    "新編「第八駆逐隊」出撃せよ！": {
        "area": "1-6", "rank": "", "boss_only": False, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["朝潮改二"]),
            SN(["満潮", "大潮", "荒潮"], 3),
        ],
    },
    "精鋭「八駆第一小隊」対潜哨戒！": {
        "area": "1-5", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SN(["朝潮改二丁", "大潮改二"], 2),
            SC(4),
        ],
    },
    "水雷戦隊、南西防衛線に反復出撃せよ！": {
        "area": "1-4", "rank": "A", "boss_only": True, "count": 2,
        "conditions": [
            FS("軽巡級", CL_CLASS),
            ST("駆逐", [DD], 4),
        ],
    },
    "製油所地帯沿岸の哨戒を実施せよ！": {
        "area": "1-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽空母", [CVL]),
            ST("駆逐", [DD], 3),
        ],
    },
    "水雷戦隊、南西諸島海域を哨戒せよ！": {
        "area": "2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡級", CL_CLASS),
            ST("駆逐", [DD], 4),
        ],
    },
    "「第十九駆逐隊」出撃せよ！": {
        "area": "1-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["磯波", "浦波", "綾波", "敷波"], 4),
            SC(4),
        ],
    },
    "「第十九駆逐隊」敵主力に突入せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["磯波", "浦波", "綾波", "敷波"], 4),
        ],
    },
    "飛行場設営の準備を実施せよ！": {
        "area": "6-3", "rank": "A", "boss_only": True, "count": 1,
    },
    "夜間突入！敵上陸部隊を叩け！": {
        "area": "5-3", "rank": "S", "boss_only": True, "count": 1,
    },
    "夜の海を照らす「灯り」を入手せよ！": {
        "area": "2-1", "rank": "S", "boss_only": True, "count": 1,
    },
    "南西諸島防衛線を増強せよ！": {
        "area": "1-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("水母/航巡", [AV, CAV]),
        ],
    },
    "「第十六戦隊(第三次)」出撃せよ！": {
        "area": "2-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["鬼怒", "青葉", "北上", "大井"], 4),
        ],
    },
    "精鋭「第十六戦隊」突入せよ！": {
        "area": "2-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["鬼怒改二"]),
            SNA(["北上改二", "大井改二", "球磨改", "青葉改", "浦波改", "敷波改"], 5),
            SC(6),
        ],
    },
    "輸送作戦を成功させ、帰還せよ！": {
        "area": "2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["鬼怒改二"]),
            SN(["浦波改"]),
            ST("駆逐", [DD], 4),
        ],
    },
    "重巡戦隊、抜錨せよ！": {
        "area": "2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡級", CA_CLASS),
            ST("重巡級", CA_CLASS, 4),
        ],
    },
    "戦艦戦隊、出撃せよ！": {
        "area": "3-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("戦艦級", BB_ALL),
            ST("戦艦級", BB_ALL, 2),
        ],
    },
    "主力戦艦戦隊、抜錨せよ！": {
        "area": "2-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("低速戦艦/航戦", [BB, BBV], 2),
        ],
    },
    "精鋭「第八駆逐隊」突入せよ！": {
        "area": "5-5", "rank": "A", "boss_only": True, "count": 2,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["荒潮改二"]),
            SNA(["朝潮", "大潮", "満潮"], 1),
        ],
    },
    "潜水艦隊、中部海域の哨戒を実施せよ！": {
        "area": "6-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("潜水", SUB),
            ST("潜水", SUB, 4),
        ],
    },
    "重装甲巡洋艦、鉄底海峡に突入せよ！": {
        "area": "5-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡", [CA]),
            SN(["Zara due"]),
        ],
    },
    "南西諸島方面の敵艦隊を撃破せよ！": {
        "area": "1-4/2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
        ],
    },
    "洋上航空戦力を拡充せよ！": {
        "area": "3-5/4-4/6-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("空母/水母", [*CARRIER, AV]),
        ],
    },
    "改装航空巡洋艦、出撃！": {
        "area": "5-1/5-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航巡", [CAV]),
            SN(["鈴谷改二"]),
        ],
    },
    "改装攻撃型軽空母、前線展開せよ！": {
        "area": "6-2/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽空母", [CVL]),
            SN(["鈴谷航改二"]),
        ],
    },
    "鎮守府海域警戒を厳とせよ！": {
        "area": "1-2/1-3/1-4/1-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("巡洋艦", [*CA_CLASS, *CL_CLASS]),
            ST("駆逐", [DD], 2),
        ],
    },
    "海上護衛体制の強化に努めよ！": {
        "area": "1-3/1-4/1-5/1-6", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    # B99 新編「第一戦隊」、抜錨せよ！ — has CS conditions
    "増強海上護衛総隊、抜錨せよ！": {
        "area": "2-2/2-3/2-4/2-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("軽巡", [CL], 1),
            ST("駆逐/海防", DD_DE, 2),
            ST("航巡/軽空母", [CAV, CVL], 1),
        ],
    },

    # ================================================================
    # ONE-TIME QUESTS B101-B150
    # ================================================================

    "新編「第七戦隊」、出撃せよ！": {
        "area": "4-5/6-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("重巡/航巡", CA_CLASS),
            SN(["熊野改二"]),
            SN(["鈴谷改二"]),
            SN(["最上改", "三隈改"], 2),
        ],
    },
    # B102 精鋭「第四航空戦隊」 — has CS conditions
    "旗艦「由良」、抜錨！": {
        "area": "2-3/5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["由良改二"]),
            SNA(["村雨", "夕立", "春雨", "五月雨", "秋月"], 2),
        ],
    },
    # B104 精鋭「第二二駆逐隊」出撃せよ！ — has CS conditions
    "精強大型航空母艦、抜錨！": {
        "area": "5-5/6-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["Saratoga Mk.II"]),
            ST("軽巡", [CL], 1),
            ST("駆逐", [DD], 2),
        ],
    },
    # B106 夜間作戦空母 — has CS conditions
    "補給線の安全を確保せよ！": {
        "area": "1-3/1-4/1-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡級", CL_CLASS),
            ST("駆逐/海防", DD_DE, 2),
        ],
    },
    "「第八駆逐隊」、南西へ！": {
        "area": "1-2/2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["朝潮", "満潮", "大潮", "荒潮"], 4),
        ],
    },
    "最精鋭「第八駆逐隊」、全力出撃！": {
        "area": "3-2/5-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["朝潮改二", "大潮改二", "荒潮改二", "満潮改二"], 4),
        ],
    },
    # B110 北方海域戦闘哨戒 — has CS conditions
    # B111 松輸送作戦、開始せよ！ — has CS conditions
    # B112 精鋭「四水戦」 — has CS conditions
    "松輸送作戦、継続実施せよ！": {
        "area": "1-4/1-6", "rank": "A", "boss_only": True, "count": 3,
        "conditions": [
            FS("軽巡級/駆逐", [*CL_CLASS, DD]),
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    "新編「四航戦」、全力出撃！": {
        "area": "1-6/2-5/3-5/4-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["伊勢改", "日向改", "大淀改"], 3),
            ST("駆逐", [DD], 1),
        ],
    },
    # B115 精鋭駆逐隊、獅子奮迅！ — has CS conditions
    "「十八駆」、北方海域キス島へ！": {
        "area": "3-2", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SN(["霰改二", "霞改二", "陽炎改", "不知火改"], 4),
        ],
    },
    "最精鋭甲型駆逐艦、突入！敵中突破！": {
        "area": "4-2/3-2/5-3", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("駆逐", [DD]),
            SNA(["陽炎改二", "不知火改二", "黒潮改二"], 1),
        ],
    },
    # B118 戦闘航空母艦 — has CS conditions
    "「伊勢改二」、敵機動部隊を迎撃せよ！": {
        "area": "6-5", "rank": "S", "boss_only": True, "count": 3,
        "conditions": [
            FS("航戦", [BBV]),
            SN(["伊勢改二"]),
            ST("駆逐", [DD], 2),
        ],
    },
    # B120 精鋭「第十八戦隊」 — has CS conditions
    "精鋭「二七駆」第一小隊、出撃せよ！": {
        "area": "2-3/4-1/5-5/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["白露改二", "時雨改二"], 2),
        ],
    },
    # B122 精鋭「四戦隊」第二小隊 — has CS conditions
    # B123 精強「十七駆」 — has CS conditions
    # B124 「第七駆逐隊」 — has CS conditions
    "近海の警戒監視と哨戒活動を強化せよ！": {
        "area": "1-2/1-3/1-4/2-1/2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            ST("駆逐/海防", DD_DE, 2),
        ],
    },
    # B126 主力オブ主力、抜錨開始！ — has CS conditions
    # B127 冬季北方海域作戦 — has CS conditions
    # B128 「比叡」の出撃 — has CS conditions
    "精鋭無比「第一戦隊」まかり通る！": {
        "area": "2-2/3-5/4-5/5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("戦艦級", BB_ALL),
            SNA(["長門改二", "陸奥改二"], 2),
        ],
    },
    "精鋭無比「第一戦隊」まかり通る！【拡張作戦】": {
        "area": "2-5/5-5/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("戦艦級", BB_ALL),
            SNA(["長門改二", "陸奥改二"], 2),
        ],
    },
    # B131 航空戦艦戦隊、戦闘哨戒！ — has CS conditions
    # B132 最精鋭「第四航空戦隊｣ — has CS conditions
    "重改装高速戦艦「金剛改二丙」、南方突入！": {
        "area": "5-1/5-3/5-4/5-5", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("高速戦艦", [FBB]),
            SN(["金剛改二丙"]),
            SNA(["比叡", "榛名", "霧島"], 1),
            ST("駆逐", [DD], 2),
        ],
    },
    "艦隊司令部の強化 【実施段階】": {
        "area": "2-3/3-3/4-1", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["大淀"]),
        ],
    },
    # B135 近海哨戒を実施せよ！ — has CS conditions
    "精鋭「二四駆逐隊」出撃せよ！": {
        "area": "2-3/2-4/5-1/5-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["海風改二"]),
            SNA(["山風", "江風", "涼風"], 2),
        ],
    },
    "精強！「第一航空戦隊」出撃せよ！": {
        "area": "4-5/5-2/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["赤城改二"]),
            SN(["加賀"]),
        ],
    },
    "「羽黒」「神風」、出撃せよ！": {
        "area": "2-1/2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["羽黒", "神風"], 2),
        ],
    },
    # B139 陸戦用装備 — has CS conditions
    # B140 「夕張改二」試してみてもいいかしら？ — has CS conditions
    # B141 新編「六水戦」出撃！ — has CS conditions
    "再編「第三一駆逐隊」、抜錨せよ！": {
        "area": "1-3/1-4/1-5/2-2/2-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["沖波改二", "長波", "岸波", "朝霜"], 4),
        ],
    },
    # B143 「第五航空戦隊」、縦横無尽！ — has CS conditions
    "「比叡改二丙」見参！第三戦隊、南方突入！": {
        "area": "5-1/5-2/5-3/5-4/5-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["比叡改二丙"]),
            SNA(["金剛", "榛名", "霧島"], 1),
            ST("軽巡級", CL_CLASS, 1),
            ST("駆逐", [DD], 1),
        ],
    },
    # B145 改装航空軽巡「Gotland andra」 — has CS conditions
    "「Gotland」戦隊、進撃せよ！": {
        "area": "2-5/6-3/6-4/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["Gotland andra"]),
            ST("駆逐", [DD], 1),
        ],
    },
    "南西諸島海域合同哨戒": {
        "area": "2-2/2-3/2-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["Fletcher", "Johnston", "Samuel B.Roberts", "Gambier Bay", "Intrepid", "Hornet", "Langley", "Houston", "Northampton", "Perth", "De Ruyter"], 2),
        ],
    },
    "合同艦隊旗艦、改装「Fletcher」、抜錨！": {
        "area": "1-4/2-5/3-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["Fletcher改 Mod.2"]),
            SNA(["Fletcher", "Johnston", "Samuel B.Roberts", "Gambier Bay", "Intrepid", "Hornet", "Langley", "Houston", "Northampton", "Perth", "De Ruyter"], 2),
        ],
    },
    "改装護衛駆逐艦「Fletcher Mk.II」作戦開始！": {
        "area": "1-5/7-1/6-2/6-5", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["Fletcher Mk.II"]),
            SNA(["Fletcher", "Johnston", "Samuel B.Roberts", "Gambier Bay", "Intrepid", "Hornet", "Langley", "Houston", "Northampton", "Perth", "De Ruyter"], 2),
        ],
    },
    "合同艦隊作戦任務【拡張作戦】": {
        "area": "4-5/5-5/6-4", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SN(["Fletcher Mk.II"]),
            SNA(["Fletcher", "Johnston", "Samuel B.Roberts", "Gambier Bay", "Intrepid", "Hornet", "Langley", "Houston", "Northampton", "Perth", "De Ruyter"], 3),
        ],
    },

    # ================================================================
    # ONE-TIME QUESTS B151-B214
    # ================================================================

    "合同艦隊機動部隊、出撃せよ！": {
        "area": "4-3/3-4/5-2/7-2(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["Intrepid", "Hornet", "Ark Royal", "Victorious", "Saratoga"], 1),
        ],
    },
    # B152 【航空母艦特別任務】 — has CS conditions
    "改加賀型航空母艦「加賀改二」、抜錨！": {
        "area": "3-4/4-4/4-5/5-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("正規空母", SEIKI_CV),
            SN(["加賀改二"]),
            ST("正規空母", SEIKI_CV, 2),
        ],
    },
    "最精鋭「第一航空戦隊」、出撃！鎧袖一触！": {
        "area": "5-5/7-2(2nd)/6-2/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["赤城改二"]),
            SN(["加賀改二"]),
        ],
    },
    # B155 重巡「羽黒」 — has CS conditions
    # B156 静かな海を護る「鯨」 — has CS conditions
    "主力オブ主力、縦横無尽ッ！": {
        "area": "1-4/2-2/3-2/4-1/7-3(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["秋雲改二", "夕雲改二", "巻雲改二", "風雲改二"], 4),
        ],
    },
    "精鋭「二七駆」、回避運動は気をつけて！": {
        "area": "1-5/2-5/7-1/5-5/6-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["白露改二", "時雨改二"], 2),
        ],
    },
    "【艦隊司令部強化】艦隊旗艦、出撃せよ！": {
        "area": "1-3/1-4/2-1/2-2", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("軽巡", [CL]),
            SNA(["大淀", "丹陽"], 1),
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    "奇跡の駆逐艦「雪風」、再び出撃す！": {
        "area": "2-3/2-4/2-5/3-3/7-3(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SNA(["丹陽", "雪風改二"], 1),
        ],
    },
    "最精強！「呉の雪風」「佐世保の時雨」": {
        "area": "5-3/5-5/4-5/6-4/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["雪風改二", "時雨改二"], 2),
        ],
    },
    "北の海から愛をこめて": {
        "area": "3-1/3-2/3-3/3-4/3-5", "rank": "S", "boss_only": True, "count": 1,
    },
    "球磨型軽巡一番艦、出撃だクマ!": {
        "area": "2-2/3-2/7-3(2nd)/1-6", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["球磨改二"]),
        ],
    },
    # B164 改装最新鋭軽巡「能代改二」 — has CS conditions
    # B165 精鋭「第七駆逐隊」 — has CS conditions
    "改装航空巡洋艦「最上」、抜錨せよ！": {
        "area": "2-2/2-4/4-5/5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航巡", [CAV]),
            SN(["最上改二"]),
        ],
    },
    "西村艦隊、精鋭先行掃討隊、前進せよ！": {
        "area": "2-3/6-4/7-2(2nd)/7-3(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航巡", [CAV]),
            SN(["最上改二"]),
            SNA(["時雨", "満潮", "朝雲", "山雲"], 2),
        ],
    },
    "二水戦旗艦、この「矢矧」が預かります！": {
        "area": "1-4/2-5/5-3/5-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽巡", [CL]),
            SN(["矢矧改二"]),
            ST("駆逐", [DD], 2),
        ],
    },
    "新しき翼。改装航空母艦「龍鳳」、出撃せよ！": {
        "area": "2-2/2-3/2-4/2-5/7-2(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽空母", [CVL]),
            SN(["龍鳳改二"]),
            SN(["時雨改二"]),
        ],
    },
    "改装特務空母「Gambier Bay Mk.II」抜錨！": {
        "area": "2-4/3-5/6-4", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("軽空母", [CVL]),
            SN(["Gambier Bay Mk.II"]),
            SNA(["Fletcher", "Johnston", "Samuel B.Roberts"], 1),
        ],
    },
    "【作戦準備】第二段階任務(対地/対空整備)": {
        "area": "1-3/1-4/2-1/2-2", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("駆逐", [DD], 3),
        ],
    },
    "「山風改二」、抜錨せよ！": {
        "area": "1-2/1-3/1-4/1-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["山風改二"]),
            ST("駆逐/海防", DD_DE, 3),
        ],
    },
    "改白露型駆逐艦「山風改二」、奮戦す！": {
        "area": "2-2/7-2(2nd)/5-1/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["山風改二", "江風改二", "海風改二"], 2),
        ],
    },
    "奮戦！精鋭「第十五駆逐隊」第一小隊": {
        "area": "2-4/5-4/7-2(2nd)", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SN(["親潮改二", "黒潮改二"], 2),
        ],
    },
    "南西海域「基地航空隊」開設！": {
        "area": "2-1/2-2/2-3/7-3(2nd)/7-4", "rank": "S", "boss_only": True, "count": 1,
    },
    "海上護衛！ヒ船団を護り抜け！": {
        "area": "7-4", "rank": "S", "boss_only": True, "count": 1,
    },
    "航空母艦「雲鷹」、抜錨せよ！": {
        "area": "2-5/7-2(2nd)/7-4/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽空母", [CVL]),
            SN(["雲鷹"]),
        ],
    },
    "改特型駆逐艦「天霧改二」、出撃す！": {
        "area": "2-2/7-3(2nd)/5-1/5-4/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["天霧改二"]),
            SNA(["青葉", "大井", "狭霧"], 2),
        ],
    },
    "第十六戦隊、改装「浦波改二」出撃します！": {
        "area": "1-4/2-3/2-5/7-2(2nd)/7-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["浦波改二", "青葉", "鬼怒"], 3),
        ],
    },
    "「磯波改二」、抜錨せよ！": {
        "area": "1-2/1-3/5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["磯波改二"]),
        ],
    },
    "改大和型戦艦「大和改二」、出撃せよ！": {
        "area": "1-4/2-5/5-1/4-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("戦艦級", BB_ALL),
            SN(["大和改二"]),
            ST("軽巡", [CL], 1),
            ST("駆逐", [DD], 2),
        ],
    },
    "見敵必殺！最精鋭大和型「第一戦隊」抜錨！": {
        "area": "4-5/5-3/7-2(2nd)/6-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["大和改二"]),
            SN(["武蔵改二"]),
            ST("駆逐", [DD], 2),
        ],
    },
    "【拡張作戦】重改装「大和改二重」、出撃！": {
        "area": "7-3(2nd)/7-4/5-5/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("戦艦級", BB_ALL),
            SN(["大和改二重"]),
        ],
    },
    "抜錨！精強「第十五駆逐隊」": {
        "area": "2-4/5-4/5-5/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["早潮改二", "親潮改二", "黒潮改二"], 2),
        ],
    },
    "米駆逐艦部隊の奮戦": {
        "area": "2-3/6-4/7-3(2nd)/7-4", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SNA(["Fletcher", "Johnston", "Samuel B.Roberts", "Heywood L.Edwards", "Richard P.Leary"], 2),
        ],
    },
    "Samuel B.Roberts Mk.II、抜錨せよ！": {
        "area": "1-5/2-2/3-5/1-6", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["Samuel B.Roberts Mk.II"]),
        ],
    },
    "不屈敢闘「Taffy Ⅲ」、Weigh anchor!": {
        "area": "2-3/2-5/4-5/5-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["Gambier Bay", "Johnston", "Samuel B.Roberts"], 3),
        ],
    },
    "機動部隊旗艦「鳳翔改二」、前線に出撃せよ！": {
        "area": "2-5/3-5/4-5/5-2/6-4/7-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("軽空母", [CVL]),
            SN(["鳳翔改二"]),
        ],
    },
    "改装特I型駆逐艦「深雪改二」、出撃せよ！": {
        "area": "3-2/5-3/6-4/7-3(2nd)", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["深雪改二"]),
            SNA(["吹雪", "白雪", "初雪", "叢雲", "磯波"], 1),
        ],
    },
    "改金剛型高速戦艦「榛名改二乙/丙」、抜錨！": {
        "area": "2-2/2-4/7-5(3rd)/4-3/5-3/5-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["榛名改二乙"]),
            SN(["金剛改二丙"]),
            ST("駆逐", [DD], 2),
        ],
    },
    "【海上護衛作戦】海上補給線を確保せよ！": {
        "area": "1-3/1-5/2-2/1-6", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            ST("海防", [DE], 3),
        ],
    },
    "改装白露型精鋭駆逐艦「時雨改三」出撃す！": {
        "area": "2-5/7-4/4-5/5-5/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["時雨改三"]),
            SNA(["白露", "有明", "夕暮"], 1),
        ],
    },
    "改装駆逐艦「天津風改二」、抜錨せよ！": {
        "area": "2-5/7-4/5-3/5-4/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["天津風改二"]),
        ],
    },
    "主力オブ主力「清霜改二」、出撃せよ！": {
        "area": "2-2/2-3/2-4/7-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["清霜改二"]),
            SNA(["霞", "朝霜", "大淀", "足柄"], 2),
        ],
    },
    "【潜水艦任務】潜水艦、戦闘哨戒！": {
        "area": "1-2/1-3/2-3/3-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("潜水", SUB),
            SNA(["Salmon", "Scamp"], 1),
        ],
    },
    "改装航空巡洋艦「三隈」、進発せよ！": {
        "area": "2-3/2-4/4-5/6-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("航巡", [CAV]),
            SN(["三隈改二"]),
        ],
    },
    "「第二駆逐隊」抜錨！": {
        "area": "1-2/1-3/1-4/1-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["村雨", "夕立", "春雨", "五月雨"], 3),
        ],
    },
    "改装白露型「春雨改二」出撃です！": {
        "area": "2-2/2-3/5-1", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["春雨改二"]),
            ST("軽巡", [CL], 1),
            ST("駆逐", [DD], 2),
        ],
    },
    "夕立姉さん！今度は一緒について行きますっ！": {
        "area": "4-5/5-3/5-4/5-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SN(["夕立改二", "春雨改二"], 2),
        ],
    },
    "八戸の盾「稲木改二」、抜錨ッ！": {
        "area": "1-3/1-4/2-2/2-3/7-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("海防", [DE]),
            SN(["稲木改二"]),
        ],
    },
    "哨戒部隊で近海及び南西諸島を警戒せよ！": {
        "area": "1-1/1-2/1-5/2-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            ST("海防", [DE], 3),
        ],
    },
    "防空水上艦、出撃せよッ！": {
        "area": "1-3/1-4/2-2/2-3/5-1", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            SNA(["秋月", "照月", "初月", "涼月", "摩耶改", "天龍改二", "龍田改二", "五十鈴改二"], 3),
        ],
    },
    "防空駆逐艦「初月改二」、推して参る！": {
        "area": "2-4/3-5/7-4/5-4/5-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["初月改二"]),
        ],
    },
    "「第二駆逐隊(後期編成)」、出撃せよ！": {
        "area": "1-2/1-5/2-3", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            FS("駆逐", [DD]),
            SNA(["早霜", "秋霜", "朝霜", "清霜"], 3),
        ],
    },
    "激闘！「第三戦隊」精鋭第二小隊！": {
        "area": "5-5/6-5/4-5", "rank": "S", "boss_only": True, "count": 2,
        "conditions": [
            SN(["比叡改二丙", "霧島改二丙"], 2),
        ],
    },
    "「早霜改二」見ているだけでは…ありません！": {
        "area": "2-5/7-2(2nd)/7-4/5-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["早霜改二"]),
            SNA(["清霜", "秋霜"], 1),
        ],
    },
    "三十二駆「藤波改二」、鳥海を護衛せよ！": {
        "area": "2-4/2-5/7-2(2nd)/5-5", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["藤波改二"]),
            SN(["鳥海"]),
            SNA(["玉波", "涼波", "早波", "浜波"], 1),
        ],
    },
    "精鋭十一駆「白雪改二」、抜錨します！": {
        "area": "2-3/2-5/4-3/5-3", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["白雪改二"]),
            SNA(["吹雪", "初雪", "深雪", "叢雲", "磯波"], 1),
        ],
    },
    "三十二駆「浜波改二」抜錨！敵中を突破せよ！": {
        "area": "1-6/2-2/2-3/2-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["浜波改二"]),
            SNA(["玉波", "涼波", "藤波", "早波", "朝霜"], 2),
        ],
    },
    "二等輸送艦の積極運用": {
        "area": "1-1/1-2/1-3/1-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("揚陸艦", [LHA]),
            SN(["第百一号輸送艦"]),
            ST("軽巡級", CL_CLASS, 1),
            ST("駆逐", [DD], 2),
        ],
    },
    "輸送船団護衛部隊、出撃せよ！": {
        "area": "1-2/1-3/1-4/1-5", "rank": "A", "boss_only": True, "count": 2,
        "conditions": [
            ST("揚陸艦/補給艦", [LHA, AO], 2),
            ST("海防", [DE], 2),
        ],
    },
    "防空駆逐艦「秋月改二」、推参します！": {
        "area": "1-4/2-4/7-1/7-2(2nd)/5-4", "rank": "S", "boss_only": True, "count": 1,
        "conditions": [
            FS("駆逐", [DD]),
            SN(["秋月改二"]),
        ],
    },

    # ================================================================
    # LIMITED QUESTS — most have CS conditions already
    # ================================================================

    # LQ1, LQ2, SB40-SB44, 7thAnvLB2, 2103B1-B3, 2103B5 all have CS conditions
}


def main():
    with open(JSON_PATH, "r", encoding="utf-8") as f:
        quests = json.load(f)

    def normalize_name(name):
        """Normalize quest name for matching (handle ! vs ！, \/ vs / etc)"""
        return name.replace("！", "!").replace("\\/", "/").replace("\\", "")

    name_map = {}
    for q in quests:
        norm = normalize_name(q["name"])
        name_map.setdefault(norm, []).append(q)

    updated_conditions = 0
    updated_metadata = 0
    skipped_has_conditions = 0
    not_found = 0

    for quest_name, wiki_info in WIKI_DATA.items():
        matches = name_map.get(normalize_name(quest_name), [])
        if not matches:
            print(f"  [NOT FOUND] Quest '{quest_name}' not in JSON")
            not_found += 1
            continue

        # Handle duplicate names (e.g., 「潜水艦隊」出撃せよ！ appears as B17 and Bm2)
        for quest in matches:
            quest_label = f"{quest['quest_id']} (id={quest['id']})"

            # --- Always update area/rank/boss_only/count from wiki ---
            changed_meta = False
            for field in ["area", "rank", "boss_only", "count"]:
                if field not in wiki_info:
                    continue
                wiki_val = wiki_info[field]
                current_val = quest.get(field)
                if current_val != wiki_val:
                    quest[field] = wiki_val
                    changed_meta = True

            if changed_meta:
                updated_metadata += 1

            # --- Update conditions only if currently empty ---
            if "conditions" in wiki_info and wiki_info["conditions"]:
                if quest.get("conditions"):
                    skipped_has_conditions += 1
                    continue
                quest["conditions"] = wiki_info["conditions"]
                updated_conditions += 1
                print(f"  [CONDITIONS] {quest_label}: added {len(wiki_info['conditions'])} conditions")

    # Sort by id
    quests.sort(key=lambda q: q["id"])

    with open(JSON_PATH, "w", encoding="utf-8") as f:
        json.dump(quests, f, ensure_ascii=False, indent=2)
        f.write("\n")

    print(f"\n{'='*60}")
    print(f"Summary:")
    print(f"  Conditions added: {updated_conditions}")
    print(f"  Metadata updated: {updated_metadata}")
    print(f"  Skipped (already has conditions): {skipped_has_conditions}")
    print(f"  Not found in JSON: {not_found}")
    print(f"  Total quests in JSON: {len(quests)}")


if __name__ == "__main__":
    main()
