import { useState, useRef, useEffect } from "react";
import { getRankName, formatRemaining } from "../../utils/format";
import { FleetPanel } from "./FleetPanel";
import "./HomeportTab.css";
import type {
  PortData, SenkaSummary, ApiLogEntry,
  ExpeditionDef, MapRecommendationDef,
  SortieQuestDef, ActiveQuestDetail, QuestProgressSummary,
} from "../../types";

export interface HomeportTabProps {
  portData: PortData | null;
  senkaData: SenkaSummary | null;
  senkaCheckpoint: boolean;
  now: number;
  expeditions: ExpeditionDef[];
  sortieQuests: SortieQuestDef[];
  mapRecommendations: MapRecommendationDef[];
  activeQuests: ActiveQuestDetail[];
  questProgress: Map<number, QuestProgressSummary>;
  portDataVersion: number;
  weaponIconSheet: string | null;
  caInstalled: boolean | null;
  gameOpen: boolean;
  showApiLog: boolean;
  apiLog: ApiLogEntry[];
}

export function HomeportTab({
  portData,
  senkaData,
  senkaCheckpoint,
  now,
  expeditions,
  sortieQuests,
  mapRecommendations,
  activeQuests,
  questProgress,
  portDataVersion,
  weaponIconSheet,
  caInstalled,
  gameOpen,
  showApiLog,
  apiLog,
}: HomeportTabProps) {
  const [logCollapsed, setLogCollapsed] = useState(false);
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [apiLog]);

  return (
    <>
      {portData ? (
        <>
          {/* Top bar: Admiral + Resources */}
          <div className="top-bar">
            {/* Admiral section */}
            <div className="admiral-section">
              <span className="admiral-name">{portData.admiral_name}</span>
              <span className="admiral-detail">
                Lv.{portData.admiral_level}
              </span>
              {portData.admiral_rank != null && (
                <span className="admiral-detail">
                  {getRankName(portData.admiral_rank)}
                </span>
              )}
              <span className="admiral-detail">
                艦:{portData.ship_count}
                {portData.ship_capacity != null && `/${portData.ship_capacity}`}
              </span>
              {senkaData && senkaData.tracking_active && (
                senkaData.is_confirmed_base ? (
                  <span className="admiral-detail senka-display" title={
                    `確認済み: ${senkaData.confirmed_senka ?? 0} (${senkaData.confirmed_cutoff ? new Date(senkaData.confirmed_cutoff).toLocaleTimeString('ja-JP', {hour: '2-digit', minute: '2-digit'}) + 'まで反映' : '?'})` +
                    `\n追加経験値: +${senkaData.exp_senka.toFixed(1)} (exp +${senkaData.monthly_exp_gain.toLocaleString()})` +
                    (senkaData.eo_bonus > 0 ? `\n追加EO: +${senkaData.eo_bonus}` : '') +
                    (senkaData.quest_bonus > 0 ? `\n追加任務: +${senkaData.quest_bonus}` : '')
                  }>
                    戦果:{senkaData.total.toFixed(1)}
                    <span className="senka-breakdown">
                      ({senkaData.confirmed_senka}+{senkaData.exp_senka.toFixed(1)}
                      {senkaData.eo_bonus > 0 && `+EO${senkaData.eo_bonus}`}
                      {senkaData.quest_bonus > 0 && `+任${senkaData.quest_bonus}`})
                    </span>
                  </span>
                ) : (
                  <span className="admiral-detail senka-unconfirmed">
                    戦果:ランキング画面で確認してください
                  </span>
                )
              )}
            </div>
            {senkaCheckpoint && senkaData?.is_confirmed_base && (
              <div className="senka-checkpoint-notice">
                ランキング更新を通過しました - ランキング画面で戦果を再確認してください
              </div>
            )}

            {/* Resources section */}
            <div className="resources-section">
              <div className="resource-row">
                <div className="res-item">
                  <span className="res-label fuel-color">燃</span>
                  <span className="res-value">{(portData.fuel ?? 0).toLocaleString()}</span>
                </div>
                <div className="res-item">
                  <span className="res-label ammo-color">弾</span>
                  <span className="res-value">{(portData.ammo ?? 0).toLocaleString()}</span>
                </div>
                <div className="res-item">
                  <span className="res-label steel-color">鋼</span>
                  <span className="res-value">{(portData.steel ?? 0).toLocaleString()}</span>
                </div>
                <div className="res-item">
                  <span className="res-label bauxite-color">ボ</span>
                  <span className="res-value">{(portData.bauxite ?? 0).toLocaleString()}</span>
                </div>
              </div>
              <div className="resource-row">
                <div className="res-item">
                  <span className="res-label repair-color">修</span>
                  <span className="res-value">{(portData.instant_repair ?? 0).toLocaleString()}</span>
                </div>
                <div className="res-item">
                  <span className="res-label build-color">建</span>
                  <span className="res-value">{(portData.instant_build ?? 0).toLocaleString()}</span>
                </div>
                <div className="res-item">
                  <span className="res-label dev-color">開</span>
                  <span className="res-value">{(portData.dev_material ?? 0).toLocaleString()}</span>
                </div>
                <div className="res-item">
                  <span className="res-label improve-color">改</span>
                  <span className="res-value">{(portData.improvement_material ?? 0).toLocaleString()}</span>
                </div>
              </div>
            </div>

            {/* Repair docks inline */}
            <div className="ndock-section">
              <span className="ndock-label">入渠</span>
              {(portData.ndock ?? []).map((dock) => (
                <div key={dock.id} className="ndock-item">
                  <span className="ndock-id">#{dock.id}</span>
                  {dock.state === 0 ? (
                    <span className="ndock-empty">-</span>
                  ) : dock.state === -1 ? (
                    <span className="ndock-locked">封鎖</span>
                  ) : (
                    <>
                      <span className="ndock-ship">
                        {dock.ship_name ?? `Ship#${dock.ship_id ?? "?"}`}
                      </span>
                      <span className="ndock-time">
                        {dock.complete_time > 0
                          ? formatRemaining(dock.complete_time, now)
                          : ""}
                      </span>
                    </>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Fleet panels */}
          <div className="fleets-area">
            {(portData.fleets ?? []).map((fleet, i) => (
              <FleetPanel key={fleet.id} fleet={fleet} now={now} fleetIndex={i} expeditions={expeditions} portDataVersion={portDataVersion} sortieQuests={sortieQuests} mapRecommendations={mapRecommendations} activeQuests={activeQuests} questProgress={questProgress} weaponIconSheet={weaponIconSheet} />
            ))}
          </div>
        </>
      ) : (
        <div className="no-data-panel">
          {caInstalled === false
            ? 'CA証明書をインストールしてください。「Install CA Cert」を押すとmacOSのパスワード入力を求められます。'
            : gameOpen
              ? "ゲームウィンドウを開きました。APIデータ待機中..."
              : '「Open Game」でゲームを起動してください。'}
        </div>
      )}

      {/* API Log - collapsible, hideable via settings */}
      {showApiLog && <div className={`api-log-panel ${logCollapsed ? "collapsed" : ""}`}>
        <div
          className="api-log-header"
          onClick={() => setLogCollapsed(!logCollapsed)}
        >
          <span>
            {logCollapsed ? "▸" : "▾"} API Log ({apiLog.length})
          </span>
        </div>
        {!logCollapsed && (
          <div className="api-log" ref={logRef}>
            {apiLog.length === 0 ? (
              <div className="no-data">API通信なし</div>
            ) : (
              apiLog.map((entry, i) => (
                <div key={i} className="api-log-entry">
                  <span className="time">{entry.time}</span>
                  <span className="endpoint">{entry.endpoint}</span>
                </div>
              ))
            )}
          </div>
        )}
      </div>}
    </>
  );
}
