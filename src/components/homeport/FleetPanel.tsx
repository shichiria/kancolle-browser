import { formatRemaining } from "../../utils/format";
import { condColor, condBgClass } from "../../utils/color";
import { HpBar } from "../common";
import "./FleetPanel.css";
import type {
  FleetData, ExpeditionDef, SortieQuestDef,
  MapRecommendationDef, ActiveQuestDetail, QuestProgressSummary,
} from "../../types";
import { ExpeditionChecker } from "./ExpeditionChecker";
import { MapRecommendationChecker } from "./MapRecommendationChecker";
import { SortieQuestChecker } from "./SortieQuestChecker";

export function FleetPanel({
  fleet,
  now,
  fleetIndex,
  expeditions,
  portDataVersion,
  sortieQuests,
  mapRecommendations,
  activeQuests,
  questProgress,
  weaponIconSheet,
}: {
  fleet: FleetData;
  now: number;
  fleetIndex: number;
  expeditions: ExpeditionDef[];
  portDataVersion: number;
  sortieQuests: SortieQuestDef[];
  mapRecommendations: MapRecommendationDef[];
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
  weaponIconSheet: string | null;
}) {
  const expedition = fleet.expedition;
  const isOnExpedition =
    expedition != null && expedition.return_time > 0;
  const ships = fleet.ships ?? [];
  // Legacy: count ship_ids if ships array is empty
  const shipCount =
    ships.length > 0
      ? ships.length
      : (fleet.ship_ids?.filter((id) => id > 0).length ?? 0);

  return (
    <div className="fleet-panel">
      <div className="fleet-header">
        <span className="fleet-name">
          <span className="fleet-id">#{fleet.id}</span> {fleet.name}
        </span>
        {ships.length > 0 && (() => {
          const minSoku = Math.min(...ships.map(s => s.soku));
          const tag = minSoku >= 20 ? { label: "最速", cls: "speed-fastest" }
            : minSoku >= 15 ? { label: "高速+", cls: "speed-fast-plus" }
            : minSoku >= 10 ? { label: "高速", cls: "speed-fast" }
            : { label: "低速混合", cls: "speed-slow" };
          return <span className={`fleet-speed-tag ${tag.cls}`}>{tag.label}</span>;
        })()}
        {isOnExpedition && expedition && (
          <span className="fleet-expedition">
            {expedition.mission_name} [{formatRemaining(expedition.return_time, now)}]
          </span>
        )}
      </div>
      {ships.length > 0 ? (
        <div className="fleet-ships">
          {ships.map((ship, i) => (
            <div key={i} className="ship-row">
              <span className="ship-name" title={ship.name}>
                {ship.name}
              </span>
              <span className="ship-lv">Lv{ship.lv}</span>
              <HpBar hp={ship.hp} maxhp={ship.maxhp} />
              <span
                className={`ship-cond ${condBgClass(ship.cond)}`}
                style={{ color: condColor(ship.cond) }}
              >
                {ship.cond}
              </span>
              {ship.damecon_name && (
                <span
                  className={weaponIconSheet ? "damecon-icon" : "mark-noimage"}
                  title={ship.damecon_name}
                  style={weaponIconSheet ? { backgroundImage: `url(${weaponIconSheet})` } : undefined}
                />
              )}
              {ship.command_facility_name && (
                <span className="command-facility-badge" title={ship.command_facility_name}>
                  司
                </span>
              )}
              {ship.special_equips.length > 0 && (
                ship.special_equips.map((eq, j) => (
                  <span
                    key={`seq-${j}`}
                    className={weaponIconSheet ? `special-equip-icon special-equip-icon-${eq.icon_type}` : "mark-noimage mark-noimage-sm"}
                    title={eq.name}
                    style={weaponIconSheet ? { backgroundImage: `url(${weaponIconSheet})` } : undefined}
                  />
                ))
              )}
              {ship.can_opening_asw && (
                <span
                  className={weaponIconSheet ? "opening-asw-icon" : "mark-noimage"}
                  title="先制対潜"
                  style={weaponIconSheet ? { backgroundImage: `url(${weaponIconSheet})` } : undefined}
                />
              )}
            </div>
          ))}
        </div>
      ) : shipCount > 0 ? (
        <div className="fleet-no-detail">{shipCount}隻 (詳細なし)</div>
      ) : null}
      {fleetIndex === 0 && (
        <>
          <MapRecommendationChecker
            mapRecommendations={mapRecommendations}
            portDataVersion={portDataVersion}
          />
          <SortieQuestChecker
            fleetIndex={fleetIndex}
            sortieQuests={sortieQuests}
            portDataVersion={portDataVersion}
            activeQuests={activeQuests}
            questProgress={questProgress}
          />
        </>
      )}
      {fleetIndex > 0 && (
        <ExpeditionChecker
          fleetIndex={fleetIndex}
          expeditions={expeditions}
          portDataVersion={portDataVersion}
          currentExpedition={fleet.expedition}
          now={now}
        />
      )}
    </div>
  );
}
