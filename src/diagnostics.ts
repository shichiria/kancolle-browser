import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import sensitiveKeys from "./sensitive-keys.json";

type LogLevel = "info" | "warn" | "error" | "debug";
type FrontendLogEntry = {
  level: LogLevel;
  message: string;
  source: string;
};

const FLUSH_DELAY_MS = 100;
const MAX_BATCH_ENTRIES = 64;
const pending: FrontendLogEntry[] = [];
let flushTimer: ReturnType<typeof setTimeout> | undefined;

const originals = {
  log: console.log.bind(console),
  info: console.info.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
  debug: console.debug.bind(console),
};

function redact(value: string): string {
  const alternatives = sensitiveKeys
    .map((key) => key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("|");
  return value.replace(
    new RegExp(
      `((?:${alternatives})["']?\\s*[:=]\\s*["']?)[^&,"'}\\]\\s]+`,
      "gi",
    ),
    "$1<redacted>",
  );
}

function describe(value: unknown, seen = new WeakSet<object>()): string {
  if (value instanceof Error) {
    return `${value.name}: ${value.message}${value.stack ? `\n${value.stack}` : ""}`;
  }
  if (typeof value === "string") return value;
  if (typeof value === "bigint") return `${value}n`;
  if (typeof value === "function")
    return `[Function ${value.name || "anonymous"}]`;
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

function flush(): void {
  if (flushTimer !== undefined) {
    clearTimeout(flushTimer);
    flushTimer = undefined;
  }
  if (pending.length === 0) return;

  const entries = pending.splice(0, MAX_BATCH_ENTRIES);
  void invoke("log_frontend_events", { entries }).catch((error) =>
    originals.error("Failed to persist frontend logs", error),
  );
  if (pending.length > 0) flushTimer = setTimeout(flush, FLUSH_DELAY_MS);
}

function persist(level: LogLevel, args: unknown[], source = "console"): void {
  pending.push({
    level,
    message: redact(args.map((arg) => describe(arg)).join(" ")),
    source: `${getCurrentWindow().label}:${source}`,
  });
  if (level === "error" || pending.length >= MAX_BATCH_ENTRIES) {
    flush();
  } else if (flushTimer === undefined) {
    flushTimer = setTimeout(flush, FLUSH_DELAY_MS);
  }
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
  persist(
    "error",
    [event.error ?? event.message],
    `${event.filename}:${event.lineno}:${event.colno}`,
  );
});

window.addEventListener("unhandledrejection", (event) => {
  persist("error", [event.reason], "unhandledrejection");
});

window.addEventListener("beforeunload", () => {
  // invoke is asynchronous; this is best-effort and crash-critical errors are flushed immediately.
  flush();
});
