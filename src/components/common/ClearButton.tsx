import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ClearButton({ label, desc, command, onSuccess }: {
  label: string;
  desc: string;
  command: string;
  onSuccess?: () => void;
}) {
  const [status, setStatus] = useState<"idle" | "confirm" | "busy" | "done" | "error">("idle");
  const [message, setMessage] = useState("");

  return (
    <div className="options-clear-row">
      <span className="options-clear-label">{label}</span>
      <span className="options-clear-desc">
        {status === "done" || status === "error" ? message : desc}
      </span>
      {status === "confirm" ? (
        <>
          <button className="options-clear-btn options-clear-btn-danger" onClick={async () => {
            setStatus("busy");
            try {
              const msg = await invoke<string>(command);
              setMessage(msg || "完了しました");
              setStatus("done");
              onSuccess?.();
            } catch (e) {
              setMessage(`${e}`);
              setStatus("error");
            }
            setTimeout(() => { setStatus("idle"); setMessage(""); }, 5000);
          }}>実行</button>
          <button className="options-clear-btn" onClick={() => setStatus("idle")}>取消</button>
        </>
      ) : (
        <button
          className="options-clear-btn"
          disabled={status === "busy"}
          onClick={() => setStatus("confirm")}
        >
          {status === "busy" ? "処理中..." : status === "done" ? "完了" : status === "error" ? "失敗" : "クリア"}
        </button>
      )}
    </div>
  );
}
