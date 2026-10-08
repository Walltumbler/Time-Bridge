import { TimebridgeError } from "./types.js";
import type { ClientOptions, Connection, CountdownOptions, RangedCountdownOptions, AlarmOptions, WeeklyAlarmOptions, DateListAlarmOptions, StopwatchOptions, TimebridgeItem, TimebridgeEvent, ItemAction } from "./types.js";
const endpoint = "http://127.0.0.1:47832/v1";
type Request = Record<string, unknown>;

/** Independent browser or native client. No global DOM access occurs at import time. */
export function createTimebridge(options: ClientOptions = {}) {
  const browser = typeof window !== "undefined" && typeof document !== "undefined";
  const origin = browser ? window.location.origin : options.origin;
  if (!origin) throw new TimebridgeError("Native clients must provide their application origin.", "INVALID_ORIGIN");
  const parsed = new URL(origin);
  if (parsed.origin !== origin || parsed.username || parsed.password || !(parsed.protocol === "https:" || parsed.protocol === "http:" && ["localhost", "127.0.0.1"].includes(parsed.hostname))) throw new TimebridgeError("Use an HTTPS origin or localhost development origin.", "INVALID_ORIGIN");
  const appName = (options.appName || (browser ? document.querySelector('meta[name="application-name"]')?.getAttribute("content") || document.title : undefined) || parsed.hostname).trim().slice(0, 120);
  const preference = options.transport ?? "auto";
  if (!browser && preference === "extension") throw new TimebridgeError("Extension transport requires a browser page.", "EXTENSION_UNAVAILABLE");
  const timeoutMs = options.timeoutMs ?? 5000;
  let transport: "extension" | "loopback" | null = null;
  let credential = options.credential;
  let pairing: Promise<void> | null = null;
  let disposed = false;
  const pending = new Map<string, { resolve: (v: unknown) => void; reject: (e: TimebridgeError) => void; timer: ReturnType<typeof setTimeout> }>();
  const check = () => { if (disposed) throw new TimebridgeError("Client disposed.", "CLIENT_DISPOSED"); };
  const response = <T>(value: unknown): T => {
    if (value && typeof value === "object" && "error" in value && typeof value.error === "string") throw new TimebridgeError(value.error, /permission|paired|credential/i.test(value.error) ? "PAIRING_REQUIRED" : "BRIDGE_ERROR");
    return value as T;
  };
  const listener = (event: MessageEvent) => {
    if (event.source !== window || event.origin !== origin || event.data?.source !== "timebridge-extension") return;
    const entry = pending.get(event.data.id); if (!entry) return;
    clearTimeout(entry.timer); pending.delete(event.data.id);
    if (event.data.error) entry.reject(new TimebridgeError(String(event.data.error), "EXTENSION_ERROR"));
    else { try { entry.resolve(response(event.data.result)); } catch (error) { entry.reject(error as TimebridgeError); } }
  };
  if (browser) window.addEventListener("message", listener);
  function extensionCall<T>(request: Request, timeout = timeoutMs): Promise<T> {
    check();
    if (!browser) return Promise.reject(new TimebridgeError("No browser extension transport.", "EXTENSION_UNAVAILABLE"));
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { pending.delete(id); reject(new TimebridgeError("The extension did not respond. Open its toolbar button for this website. A timed-out write may have completed; check your items before retrying.", "EXTENSION_TIMEOUT")); }, timeout);
      pending.set(id, { resolve: resolve as (v: unknown) => void, reject, timer });
      window.postMessage({ source: "timebridge-sdk", id, request }, origin!);
    });
  }
  async function localCall<T>(request: Request): Promise<T> {
    check();
    const controller = new AbortController(); const timeout = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const headers: Record<string, string> = { "Content-Type": "application/json" };
      if (!browser) headers.Origin = origin!;
      if (credential) headers.Authorization = `Bearer ${credential}`;
      const result = await (options.fetch ?? fetch)(endpoint, { method: "POST", headers, body: JSON.stringify(request), signal: controller.signal, credentials: "omit", cache: "no-store", redirect: "error" });
      const value = await result.json();
      if (!result.ok) throw new TimebridgeError(value.error || `Timebridge returned ${result.status}`, result.status === 401 ? "PAIRING_REQUIRED" : result.status === 429 ? "RATE_LIMITED" : "BRIDGE_ERROR");
      return response<T>(value);
    } catch (error) {
      if (error instanceof TimebridgeError) throw error;
      throw new TimebridgeError("Cannot reach Timebridge Desktop. Open the app; in a browser, connect the extension or allow local-network access when prompted. A timed-out write may have completed; check your items before retrying.", "DESKTOP_UNAVAILABLE");
    } finally { clearTimeout(timeout); }
  }
  async function connect(): Promise<Connection> {
    check();
    if (browser && preference !== "loopback") {
      try { const status = await extensionCall<Connection>({ op: "detect" }, Math.min(timeoutMs, 1500)); if (status.desktop) { transport = "extension"; return { ...status, extension: true, transport }; } }
      catch { /* Detection is read-only: another transport is safe here. */ }
      if (preference === "extension") { transport = null; return { desktop: false, extension: false, transport: null }; }
    }
    try { const status = await localCall<Connection>({ op: "detect" }); transport = "loopback"; return { ...status, extension: false, transport }; }
    catch { transport = null; return { desktop: false, extension: false, transport: null }; }
  }
  async function call<T>(request: Request): Promise<T> {
    check();
    if (!transport && !(await connect()).desktop) throw new TimebridgeError("Open Timebridge Desktop and connect this app first.", "DESKTOP_UNAVAILABLE");
    // A timed-out mutation must never be automatically replayed via another transport.
    return transport === "extension" ? extensionCall<T>(request) : localCall<T>(request);
  }
  async function pair(): Promise<void> {
    try { if ((await call<{ approved: boolean }>({ op: "permissions.status" })).approved) return; }
    catch (error) { if (!(error instanceof TimebridgeError) || error.code !== "PAIRING_REQUIRED") throw error; }
    const pollSecret = [...crypto.getRandomValues(new Uint8Array(32))].map(b => b.toString(16).padStart(2, "0")).join("");
    const request = await call<{ id: string }>({ op: "permissions.request", name: appName, pollSecret });
    const deadline = Date.now() + 120000;
    while (Date.now() < deadline) {
      const state = await call<{ status: string; credential?: string }>({ op: "permissions.poll", id: request.id, pollSecret });
      if (state.status === "approved") {
        if (transport === "loopback") {
          if (!state.credential) throw new TimebridgeError("No pairing credential returned.", "PAIRING_FAILED");
          credential = state.credential; await options.onCredential?.(credential);
        }
        return;
      }
      if (state.status === "denied") throw new TimebridgeError("Access was declined in Timebridge Desktop.", "PAIRING_DENIED");
      await new Promise(resolve => setTimeout(resolve, 1500));
    }
    throw new TimebridgeError("Approval expired. Connect again and approve in Desktop.", "PAIRING_EXPIRED");
  }
  function requestPermission(): Promise<void> { check(); return pairing ??= pair().finally(() => { pairing = null; }); }
  function withReturnUrl<T extends { returnUrl?: string }>(value: T): T & { returnUrl?: string } {
    const url = value.returnUrl ?? (browser ? window.location.href : undefined); if (!url) return value;
    const safe = new URL(url, origin);
    if (safe.origin !== origin || safe.username || safe.password) throw new TimebridgeError("Return URL must stay on the paired application's origin.", "INVALID_RETURN_URL");
    return { ...value, returnUrl: safe.href };
  }
  return {
    connect, requestPermission,
    createCountdown: (value: CountdownOptions) => call<TimebridgeItem>({ op: "item.create", input: { ...withReturnUrl(value), kind: "countdown" } }),
    createRangedCountdown: (value: RangedCountdownOptions) => call<TimebridgeItem>({ op: "item.create", input: { ...withReturnUrl(value), kind: "ranged_countdown" } }),
    createAlarm: (value: AlarmOptions) => call<TimebridgeItem>({ op: "item.create", input: { ...withReturnUrl(value), kind: "alarm" } }),
    createWeeklyAlarm: ({ weekdays, time, timezone, ...rest }: WeeklyAlarmOptions) => call<TimebridgeItem>({ op: "item.create", input: { ...withReturnUrl(rest), kind: "recurring_alarm", schedule: { kind: "weekly", weekdays, time, timezone } } }),
    createDateListAlarm: ({ occurrences, timezone, ...rest }: DateListAlarmOptions) => call<TimebridgeItem>({ op: "item.create", input: { ...withReturnUrl(rest), kind: "recurring_alarm", schedule: { kind: "dates", occurrences, timezone } } }),
    createStopwatch: (value: StopwatchOptions = {}) => call<TimebridgeItem>({ op: "item.create", input: { ...value, kind: "stopwatch" } }),
    listItems: () => call<TimebridgeItem[]>({ op: "items.list" }),
    getItem: (id: string) => call<TimebridgeItem>({ op: "item.get", id }),
    updateItem: (id: string, title: string) => call<TimebridgeItem>({ op: "item.update", id, title }),
    actOnItem: (id: string, action: ItemAction) => call<TimebridgeItem>({ op: "item.action", id, action }),
    listUndeliveredEvents: () => call<TimebridgeEvent[]>({ op: "events.listUndelivered" }),
    acknowledgeEvents: (ids: number[]) => call<{ ok: boolean }>({ op: "events.acknowledge", ids }),
    dispose() { disposed = true; if (browser) window.removeEventListener("message", listener); for (const entry of pending.values()) { clearTimeout(entry.timer); entry.reject(new TimebridgeError("Client disposed.", "CLIENT_DISPOSED")); } pending.clear(); },
  };
}
