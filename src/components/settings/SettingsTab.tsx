import type { Dispatch, SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ClearButton } from "../common";
import "./SettingsTab.css";
import type { DriveStatus, SortieRecord } from "../../types";

export function SettingsTab({
  uiZoom,
  setUiZoom,
  driveStatus,
  setDriveStatus,
  driveLoading,
  setDriveLoading,
  showApiLog,
  setShowApiLog,
  rawApiEnabled,
  setRawApiEnabled,
  setBattleLogs,
  setBattleLogsTotal,
}: {
  uiZoom: number;
  setUiZoom: (v: number) => void;
  driveStatus: DriveStatus;
  setDriveStatus: Dispatch<SetStateAction<DriveStatus>>;
  driveLoading: boolean;
  setDriveLoading: (v: boolean) => void;
  showApiLog: boolean;
  setShowApiLog: (v: boolean) => void;
  rawApiEnabled: boolean;
  setRawApiEnabled: (v: boolean) => void;
  setBattleLogs: Dispatch<SetStateAction<SortieRecord[]>>;
  setBattleLogsTotal: Dispatch<SetStateAction<number>>;
}) {
  return (
    <div className="options-tab">
      <div className="options-section">
        <div className="options-section-title">表示</div>
        <div className="options-row">
          <label className="options-label">UIサイズ</label>
          <input
            type="range"
            min={50}
            max={200}
            step={5}
            value={uiZoom}
            onChange={(e) => {
              const v = Number(e.target.value);
              setUiZoom(v);
              localStorage.setItem("ui-zoom", String(v));
            }}
            className="options-slider"
          />
          <span className="options-value">{uiZoom}%</span>
          <button
            className="options-reset-btn"
            onClick={() => {
              setUiZoom(135);
              localStorage.setItem("ui-zoom", "135");
            }}
          >
            リセット
          </button>
        </div>
      </div>

      <div className="options-section">
        <div className="options-section-title">Google Drive 同期</div>
        {!driveStatus.authenticated ? (
          <div className="drive-sync-content">
            <p className="drive-sync-desc">
              Google Driveと同期して、複数端末間でデータを共有できます。
            </p>
            {driveStatus.error && (
              <p className="drive-sync-error">{driveStatus.error}</p>
            )}
            <button
              className="drive-sync-btn"
              disabled={driveLoading}
              onClick={async () => {
                setDriveLoading(true);
                try {
                  await invoke("drive_login");
                  const status = await invoke<DriveStatus>("get_drive_status");
                  setDriveStatus(status);
                } catch (e) {
                  console.error("Drive login failed:", e);
                  setDriveStatus((prev) => ({
                    ...prev,
                    error: String(e),
                  }));
                } finally {
                  setDriveLoading(false);
                }
              }}
            >
              {driveLoading ? "認証中" : "Googleでログイン"}
            </button>
          </div>
        ) : (
          <div className="drive-sync-content">
            <div className="drive-sync-row">
              <span className="drive-sync-email">{driveStatus.email || "認証済み"}</span>
              <span className={`drive-sync-status-value ${driveStatus.syncing ? "syncing" : driveStatus.error ? "error" : ""}`}>
                {driveStatus.syncing ? "同期中" : driveStatus.error ? `エラー: ${driveStatus.error}` : "変更待機中"}
              </span>
              <button
                className="drive-sync-btn drive-sync-btn-sm"
                disabled={driveLoading || driveStatus.syncing}
                onClick={async () => {
                  setDriveLoading(true);
                  try {
                    await invoke("drive_force_sync");
                  } catch (e) {
                    console.error("Force sync failed:", e);
                  } finally {
                    setDriveLoading(false);
                  }
                }}
              >
                手動同期
              </button>
              <button
                className="drive-sync-btn drive-sync-btn-sm"
                onClick={async () => {
                  setDriveLoading(true);
                  try {
                    await invoke("drive_logout");
                    setDriveStatus({ authenticated: false, syncing: false });
                  } catch (e) {
                    console.error("Drive logout failed:", e);
                  } finally {
                    setDriveLoading(false);
                  }
                }}
                disabled={driveLoading}
              >
                ログアウト
              </button>
            </div>
            {driveStatus.last_sync && (
              <div className="drive-sync-status-row">
                <span className="drive-sync-status-label">最終同期:</span>
                <span className="drive-sync-status-value">{driveStatus.last_sync}</span>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="options-section">
        <div className="options-section-title">開発者オプション</div>
        <div className="options-row">
          <label className="options-label">APIログ表示</label>
          <label className="options-toggle">
            <input
              type="checkbox"
              checked={showApiLog}
              onChange={(e) => {
                setShowApiLog(e.target.checked);
                localStorage.setItem("show-api-log", String(e.target.checked));
              }}
            />
            <span className="options-toggle-text">母港にAPIログを表示</span>
          </label>
        </div>
        <div className="options-row">
          <label className="options-label">全ログ保存</label>
          <label className="options-toggle">
            <input
              type="checkbox"
              checked={rawApiEnabled}
              onChange={async (e) => {
                const enabled = e.target.checked;
                setRawApiEnabled(enabled);
                localStorage.setItem("raw-api-enabled", String(enabled));
                await invoke("set_raw_api_enabled", { enabled });
              }}
            />
            <span className="options-toggle-text">全APIレスポンスをディスクに保存</span>
          </label>
        </div>
      </div>

      <div className="options-section">
        <div className="options-section-title">データクリア</div>
        <div className="options-clear-list">
          <ClearButton
            label="改修履歴"
            desc="改修した装備の記録"
            command="clear_improved_history"
          />
          <ClearButton
            label="戦闘ログ"
            desc="出撃・戦闘の記録"
            command="clear_battle_logs"
            onSuccess={() => { setBattleLogs([]); setBattleLogsTotal(0); }}
          />
          <ClearButton
            label="生APIダンプ"
            desc="傍受したAPIの生データ"
            command="clear_raw_api"
          />
          <ClearButton
            label="任務進捗"
            desc="任務の進捗データ"
            command="clear_quest_progress"
          />
          <ClearButton
            label="ブラウザキャッシュ"
            desc="WebViewのHTTP/GPUキャッシュ"
            command="clear_browser_cache"
          />
          <ClearButton
            label="保存リソース"
            desc="プロキシ経由で保存したマップ画像等"
            command="clear_resource_cache"
          />
          <ClearButton
            label="Cookie"
            desc="DMM保存Cookie（再ログイン必要）"
            command="clear_cookies"
          />
          <ClearButton
            label="ブラウザデータ全削除"
            desc="Cookie・セッション・キャッシュ等を全て削除"
            command="reset_browser_data"
          />
        </div>
      </div>
    </div>
  );
}
