#!/usr/bin/env python3
"""
Update sortie_quests.json with fleet composition conditions extracted from
ElectronicObserver's ProgressSpecialBattle.cs.

Only updates quests that currently have empty conditions (to not overwrite
any manually-set conditions).

Also adds missing quests (limited-time/seasonal quests not yet in the JSON).
"""

import json
import sys
from pathlib import Path

JSON_PATH = Path(__file__).parent.parent / "src-tauri" / "data" / "sortie_quests.json"

# ============================================================
# Condition definitions for each quest ID
# ============================================================

CONDITIONS = {
    # ---- Quest 237 (B138): 「羽黒」「神風」、出撃せよ！ ----
    # Not in ProgressSpecialBattle.cs, but quest name makes requirement clear
    237: [
        {"type": "ContainsShipName", "names": ["羽黒", "神風"], "count": 2},
    ],

    # ---- Quest 854 (Bq2): 戦果拡張任務！「Z作戦」前段作戦 ----
    # Only requires first fleet (no ship composition requirement)
    # Leave conditions empty — nothing to enforce
}

# ============================================================
# New quest entries to ADD (missing from JSON)
# ============================================================

NEW_QUESTS = [
    {
        "id": 831,
        "quest_id": "SB43",
        "name": "【春限定】春の海上キラキラ作戦！",
        "area": "1-1/1-2/1-3/1-4",
        "rank": "S",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "FlagshipType", "ship_type": "軽巡/軽空母/水母", "stypes": [3, 7, 16]},
            {"type": "ShipTypeCount", "ship_type": "駆逐/海防", "stypes": [2, 1], "value": 4},
        ],
    },
    {
        "id": 832,
        "quest_id": "SB44",
        "name": "【春限定】精鋭「三一駆」、春のかぼちゃ祭り！",
        "area": "2-1/2-2/5-4",
        "rank": "S",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "ContainsShipName", "names": ["長波"], "count": 1},
            {"type": "FlagshipType", "ship_type": "駆逐", "stypes": [2]},
            {"type": "ContainsShipNameAny", "names": ["高波", "沖波", "岸波", "朝霜"], "count": 3},
        ],
    },
    {
        "id": 932,
        "quest_id": "2103B5",
        "name": "天津風の護り",
        "area": "2-2/2-3/7-3(2nd)",
        "rank": "A",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "ContainsShipName", "names": ["天津風"], "count": 1},
            {"type": "FlagshipType", "ship_type": "駆逐", "stypes": [2]},
            {"type": "ContainsShipNameAny", "names": ["雪風", "時津風", "初風"], "count": 1},
        ],
    },
    {
        "id": 234,
        "quest_id": "LQ1",
        "name": "海上護衛総隊、遠征開始！",
        "area": "1-4/2-1/2-2/2-3",
        "rank": "A",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2},
            {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1},
        ],
    },
    {
        "id": 238,
        "quest_id": "LQ2",
        "name": "巡洋艦戦隊、出撃せよ！",
        "area": "1-3/2-4/3-1/3-3/4-2",
        "rank": "S",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "FlagshipType", "ship_type": "巡洋艦", "stypes": [3, 5, 6]},
        ],
    },
    {
        "id": 906,
        "quest_id": "2103B1",
        "name": "【春の海上キラキラ】護衛艦隊、出撃！",
        "area": "1-2/1-3/1-5/1-6",
        "rank": "A",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "ShipTypeCount", "ship_type": "駆逐/海防", "stypes": [2, 1], "value": 3},
        ],
    },
    {
        "id": 907,
        "quest_id": "2103B2",
        "name": "【春の海上キラキラ】南西方面、出撃！",
        "area": "2-1/2-2/2-3",
        "rank": "S",
        "boss_only": True,
        "count": 2,
        "reset": "limited",
        "conditions": [
            {"type": "ShipTypeCount", "ship_type": "駆逐/海防", "stypes": [2, 1], "value": 4},
        ],
    },
    {
        "id": 908,
        "quest_id": "2103B3",
        "name": "【春の海上キラキラ】主力艦隊、出撃！",
        "area": "2-4/2-5/7-2(2nd)",
        "rank": "S",
        "boss_only": True,
        "count": 2,
        "reset": "limited",
        "conditions": [
            {"type": "ShipTypeCount", "ship_type": "空母", "stypes": [7, 11, 18], "value": 1},
            {"type": "ShipTypeCount", "ship_type": "重巡/航巡", "stypes": [5, 6], "value": 1},
            {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1},
        ],
    },
    {
        "id": 883,
        "quest_id": "7thAnvLB2",
        "name": "【七周年】七周年任務【拡張作戦】",
        "area": "2-3/3-1/3-2/3-3/3-4/3-5/7-2(2nd)",
        "rank": "S",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1},
            {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2},
        ],
    },
    {
        "id": 840,
        "quest_id": "SB40",
        "name": "【節分任務】令和三年節分作戦",
        "area": "2-1/2-2/2-3",
        "rank": "A",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "FlagshipType", "ship_type": "軽空母/軽巡/雷巡/練巡", "stypes": [7, 3, 4, 21]},
            {"type": "ShipTypeCount", "ship_type": "駆逐/海防", "stypes": [2, 1], "value": 3},
        ],
    },
    {
        "id": 841,
        "quest_id": "SB41",
        "name": "【節分任務】令和三年西方海域節分作戦",
        "area": "4-1/4-2/4-3",
        "rank": "S",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "FlagshipType", "ship_type": "水母/重巡/航巡", "stypes": [16, 5, 6]},
        ],
    },
    {
        "id": 843,
        "quest_id": "SB42",
        "name": "【節分拡張任務】令和三年節分作戦、全力出撃！",
        "area": "5-2/5-5/6-4",
        "rank": "S",
        "boss_only": True,
        "count": 1,
        "reset": "limited",
        "conditions": [
            {"type": "FlagshipType", "ship_type": "戦艦/空母", "stypes": [8, 9, 10, 7, 11, 18]},
            {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2},
        ],
    },
]


def main():
    # Read existing data
    with open(JSON_PATH, "r", encoding="utf-8") as f:
        quests = json.load(f)

    id_map = {q["id"]: q for q in quests}

    # --- Update existing quests with empty conditions ---
    updated = 0
    for qid, conditions in CONDITIONS.items():
        if qid not in id_map:
            print(f"  [SKIP] Quest {qid} not found in JSON")
            continue
        quest = id_map[qid]
        if quest["conditions"]:
            print(f"  [SKIP] Quest {qid} ({quest['quest_id']}) already has conditions")
            continue
        quest["conditions"] = conditions
        updated += 1
        print(f"  [UPDATE] Quest {qid} ({quest['quest_id']}): added {len(conditions)} conditions")

    # --- Add new quests ---
    added = 0
    for new_quest in NEW_QUESTS:
        if new_quest["id"] in id_map:
            existing = id_map[new_quest["id"]]
            if existing["conditions"]:
                print(f"  [SKIP] Quest {new_quest['id']} ({existing['quest_id']}) already has conditions")
            else:
                # Update conditions on existing quest
                existing["conditions"] = new_quest["conditions"]
                updated += 1
                print(f"  [UPDATE] Quest {new_quest['id']} ({existing['quest_id']}): added {len(new_quest['conditions'])} conditions")
        else:
            quests.append(new_quest)
            id_map[new_quest["id"]] = new_quest
            added += 1
            print(f"  [ADD] Quest {new_quest['id']} ({new_quest['quest_id']}): {new_quest['name']}")

    # Sort by id for consistency
    quests.sort(key=lambda q: q["id"])

    # Write back
    with open(JSON_PATH, "w", encoding="utf-8") as f:
        json.dump(quests, f, ensure_ascii=False, indent=2)
        f.write("\n")

    print(f"\nDone: {updated} updated, {added} added. Total quests: {len(quests)}")


if __name__ == "__main__":
    main()
