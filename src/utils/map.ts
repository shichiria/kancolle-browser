// Map and battle-related utility functions

import edgesData from "../data/edges.json";
import type { SortieRecord } from "../types";

/** Lookup node label from KC3Kai edges data. Returns destination node label for given edge ID. */
export function getNodeLabel(mapDisplay: string, edgeId: number): string | null {
  const key = `World ${mapDisplay}`;
  const edges = edgesData as Record<string, Record<string, string[]>>;
  const mapEdges = edges[key];
  if (!mapEdges) return null;
  const edge = mapEdges[String(edgeId)];
  if (!edge || edge.length < 2) return null;
  return edge[1]; // [source, destination] - we want destination
}

/** Build a predeck JSON for kc-web aircalc */
export function buildPredeckUrl(record: SortieRecord): string {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const predeck: any = { version: 4, hqlv: 120 };

  // Fleet 1 (the sortie fleet)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const fleet: any = {};
  record.ships.forEach((ship, idx) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const s: any = { id: ship.ship_id, lv: ship.lv, luck: -1, items: {} };
    if (ship.slots) {
      ship.slots.forEach((slot, si) => {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const item: any = { id: slot.id, rf: slot.rf ?? 0 };
        if (slot.mas != null) item.mas = slot.mas;
        s.items[`i${si + 1}`] = item;
      });
    }
    if (ship.slot_ex) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const exItem: any = { id: ship.slot_ex.id, rf: ship.slot_ex.rf ?? 0 };
      if (ship.slot_ex.mas != null) exItem.mas = ship.slot_ex.mas;
      s.items.ix = exItem;
    }
    fleet[`s${idx + 1}`] = s;
  });
  predeck.f1 = fleet;

  // Sortie data (enemy compositions per cell)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const cells: any[] = [];
  for (const node of record.nodes) {
    const b = node.battle;
    if (!b) continue;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const enemyShips: any[] = b.enemy_ships.map((e) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const items: any[] = (e.slots ?? []).map((eid) => ({ id: eid }));
      return { id: e.ship_id, items };
    });
    cells.push({
      c: node.cell_no,
      pf: b.formation[0],
      ef: b.formation[1],
      f1: { s: enemyShips },
    });
  }

  // Parse map_display "X-Y" -> world X, map Y
  const [worldStr, mapStr] = record.map_display.split("-");
  const world = parseInt(worldStr, 10) || 0;
  const mapId = parseInt(mapStr, 10) || 0;

  if (cells.length > 0) {
    predeck.s = { a: world, i: mapId, c: cells };
  }

  const json = JSON.stringify(predeck);
  return `https://noro6.github.io/kc-web/?predeck=${encodeURIComponent(json)}`;
}

/** Map route colors by event_kind */
export const CELL_COLORS: Record<number, string> = {
  0: "#4caf50",  // start - green
  2: "#ffc107",  // resource - yellow
  3: "#9c27b0",  // maelstrom - purple
  4: "#f44336",  // battle - red
  5: "#d32f2f",  // boss - dark red
  6: "#78909c",  // nothing - grey
  7: "#29b6f6",  // aerial - light blue
  8: "#ff7043",  // air raid - orange
  9: "#8d6e63",  // landing - brown
  10: "#26a69a", // anchorage - teal
};
