import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type LogLevel = "info" | "warn" | "error" | "debug";

const originals = {
  log: console.log.bind(console),
  info: console.info.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
  debug: console.debug.bind(console),
};

function redact(value: string): string {
  return value.replace(
    /((?:api_token|authorization|password|client_secret|access_token|refresh_token|cookie|rpctoken|st)["']?\s*[:=]\s*["']?)[^&,"'}\]\s]+/gi,
    "$1<redacted>",
  );
}

function describe(value: unknown, seen = new WeakSet<object>()): string {
  if (value instanceof Error) {
    return `${value.name}: ${value.message}${value.stack ? `\n${value.stack}` : ""}`;
  }
  if (typeof value === "string") return value;
  if (typeof value === "bigint") return `${value}n`;
  if (typeof value === "function") return `[Function ${value.name || "anonymous"}]`;
  if (typeof value === "object" && value !== null) {
    if (seen.has(value)) return "[Circular]";
    seen.add(value);
    try {
      return JSON.stringify(value, (_key, nested) => {
        if (typeof nested === "bigint") return `${nested}n`;
        if (typeof nested === "object" && nested !== null) {
          if (seen.has(nested) && nested !== value) return "[Circular]";
          seen.add(nested);
        }
        return nested;
      });
    } catch {
      return Object.prototype.toString.call(value);
    }
  }
  return String(value);
}

function persist(level: LogLevel, args: unknown[], source = "console"): void {
  const message = redact(args.map((arg) => describe(arg)).join(" "));
  void invoke("log_frontend_event", {
    level,
    message,
    source: `${getCurrentWindow().label}:${source}`,
  }).catch((error) => originals.error("Failed to persist frontend log", error));
}

console.log = (...args: unknown[]) => {
  originals.log(...args);
  persist("info", args);
};
console.info = (...args: unknown[]) => {
  originals.info(...args);
  persist("info", args);
};
console.warn = (...args: unknown[]) => {
  originals.warn(...args);
  persist("warn", args);
};
console.error = (...args: unknown[]) => {
  originals.error(...args);
  persist("error", args);
};
console.debug = (...args: unknown[]) => {
  originals.debug(...args);
  persist("debug", args);
};

window.addEventListener("error", (event) => {
  persist("error", [event.error ?? event.message], `${event.filename}:${event.lineno}:${event.colno}`);
});

window.addEventListener("unhandledrejection", (event) => {
  persist("error", [event.reason], "unhandledrejection");
});
