#!/usr/bin/env python3
"""Fix recommended fleet compositions to use correct areas from sortie_quests.json.

For each quest where the recommended areas don't match the quest's area field,
regenerate recommended entries using the correct areas, preserving the quest-specific
ship conditions from the fleet data.
"""

import json

JSON_PATH = "src-tauri/data/sortie_quests.json"

# Map-specific filler compositions (to add alongside quest-specific conditions)
# These are reasonable fillers based on common map routing requirements
MAP_FILLERS = {
    # 1-x maps: easy, minimal requirements
    "1-1": [],
    "1-2": [{"type": "ShipTypeCount", "ship_type": "軽空母", "stypes": [7], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "1-3": [{"type": "ShipTypeCount", "ship_type": "軽空母", "stypes": [7], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "1-4": [{"type": "ShipTypeCount", "ship_type": "軽空母", "stypes": [7], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "1-5": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "1-6": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 4}],
    # 2-x maps
    "2-1": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "2-2": [{"type": "ShipTypeCount", "ship_type": "空母", "stypes": [7, 11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}],
    "2-3": [{"type": "ShipTypeCount", "ship_type": "空母", "stypes": [7, 11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}],
    "2-4": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}],
    "2-5": [{"type": "ShipTypeCount", "ship_type": "航巡", "stypes": [6], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    # 3-x maps
    "3-1": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽空母", "stypes": [7], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "3-2": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 4}],
    "3-3": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽空母", "stypes": [7], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "3-4": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}],
    "3-5": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "重巡級", "stypes": [5, 6], "value": 2}],
    # 4-x maps
    "4-1": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "4-2": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}],
    "4-3": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "4-4": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}],
    "4-5": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    # 5-x maps
    "5-1": [{"type": "ShipTypeCount", "ship_type": "航巡", "stypes": [6], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "5-2": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "航巡", "stypes": [6], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "5-3": [{"type": "ShipTypeCount", "ship_type": "戦艦級", "stypes": [8, 9, 10], "value": 1}, {"type": "ShipTypeCount", "ship_type": "重巡級", "stypes": [5, 6], "value": 2}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "5-4": [{"type": "ShipTypeCount", "ship_type": "航巡", "stypes": [6], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "5-5": [{"type": "ShipTypeCount", "ship_type": "戦艦級", "stypes": [8, 9, 10], "value": 2}, {"type": "ShipTypeCount", "ship_type": "航巡", "stypes": [6], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    # 6-x maps
    "6-2": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 4}],
    "6-3": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 4}],
    "6-4": [{"type": "ShipTypeCount", "ship_type": "航戦", "stypes": [10], "value": 1}, {"type": "ShipTypeCount", "ship_type": "航巡", "stypes": [6], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "6-5": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 2}, {"type": "ShipTypeCount", "ship_type": "戦艦級", "stypes": [8, 9, 10], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    # 7-x maps
    "7-1": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "7-2": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "7-2(1st)": [{"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 3}],
    "7-2(2nd)": [{"type": "ShipTypeCount", "ship_type": "正規空母", "stypes": [11, 18], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "7-3": [{"type": "ShipTypeCount", "ship_type": "重巡級", "stypes": [5, 6], "value": 2}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "7-3(2nd)": [{"type": "ShipTypeCount", "ship_type": "重巡級", "stypes": [5, 6], "value": 2}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "7-4": [{"type": "ShipTypeCount", "ship_type": "軽空母", "stypes": [7], "value": 1}, {"type": "ShipTypeCount", "ship_type": "軽巡", "stypes": [3], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "7-5": [{"type": "ShipTypeCount", "ship_type": "戦艦級", "stypes": [8, 9, 10], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
    "7-5(3rd)": [{"type": "ShipTypeCount", "ship_type": "戦艦級", "stypes": [8, 9, 10], "value": 1}, {"type": "ShipTypeCount", "ship_type": "駆逐", "stypes": [2], "value": 2}],
}


def get_stype_names(stypes):
    """Get the type names referenced in a condition"""
    return set(stypes)


def has_stype_overlap(cond, filler):
    """Check if a condition and filler both reference the same ship type"""
    if cond.get("type") != "ShipTypeCount" or filler.get("type") != "ShipTypeCount":
        return False
    return bool(set(cond["stypes"]) & set(filler["stypes"]))


def build_recommended_for_area(area, conditions):
    """Build a recommended fleet entry for a given area using quest conditions + map fillers."""
    # Start with quest conditions
    fleet = list(conditions)

    # Add map-specific fillers that don't overlap with quest conditions
    fillers = MAP_FILLERS.get(area, [])
    for filler in fillers:
        # Skip if this type is already covered by quest conditions
        overlap = any(has_stype_overlap(c, filler) for c in conditions)
        if not overlap:
            fleet.append(filler)

    return {"area": area, "fleet": fleet}


def main():
    with open(JSON_PATH, "r", encoding="utf-8") as f:
        quests = json.load(f)

    fixed_count = 0
    for q in quests:
        if not q.get("recommended"):
            continue

        quest_areas = set(q["area"].split("/"))
        rec_areas = set(r["area"] for r in q["recommended"])

        # Skip quests with free area
        if "任意" in quest_areas:
            continue

        missing = quest_areas - rec_areas
        extra = rec_areas - quest_areas

        if not missing and not extra:
            continue

        # This quest needs fixing
        areas = q["area"].split("/")
        conditions = q.get("conditions", [])

        new_recommended = []
        for area in areas:
            new_recommended.append(build_recommended_for_area(area, conditions))

        q["recommended"] = new_recommended
        fixed_count += 1
        print(f"Fixed {q['quest_id']}: {q['name']} -> {[r['area'] for r in new_recommended]}")

    with open(JSON_PATH, "w", encoding="utf-8") as f:
        json.dump(quests, f, ensure_ascii=False, indent=2)
        f.write("\n")

    print(f"\nFixed {fixed_count} quests")

    # Verify
    with open(JSON_PATH, "r", encoding="utf-8") as f:
        quests = json.load(f)

    remaining = 0
    for q in quests:
        if not q.get("recommended"):
            continue
        quest_areas = set(q["area"].split("/"))
        rec_areas = set(r["area"] for r in q["recommended"])
        if "任意" in quest_areas:
            continue
        if quest_areas != rec_areas:
            remaining += 1
            print(f"  STILL MISMATCHED: {q['quest_id']}")

    print(f"Remaining mismatches: {remaining}")


if __name__ == "__main__":
    main()
