import "./style.css";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Item, Kind, Settings, ItemEvent, AudioStatus, AudioDevice, PairRequest, Site } from "./types";

const native = isTauri();
const alertWindow = new URLSearchParams(location.search).has("alert");
let audioWarning = "";
let previousSnapshot = "";
let refreshing = false;
const app = document.querySelector<HTMLDivElement>("#app")!;
const escape = (v: unknown) =>
  String(v ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ]!,
  );
const icons: Record<string, string> = {
  clock: '<circle cx="12" cy="12" r="8"/><path d="M12 7v5l3 2"/>',
  bridge:
    '<path d="M3 17V7m18 10V7M3 13c5-8 13-8 18 0M7 10v7m10-7v7M2 17h20"/>',
  range: '<path d="M5 5v14m14-14v14M5 12h14m-9-4-4 4 4 4m4-8 4 4-4 4"/>',
  alarm:
    '<circle cx="12" cy="13" r="7"/><path d="m4 3-3 3m19-3 3 3M12 9v4l3 2M7 19l-2 2m12-2 2 2"/>',
  repeat:
    '<path d="M4 9a8 8 0 0 1 14-4l2 2m0-5v5h-5M20 15A8 8 0 0 1 6 19l-2-2m0 5v-5h5"/>',
  watch:
    '<circle cx="12" cy="14" r="7"/><path d="M12 10v4M9 3h6m-3 0v4m6 0 2-2"/>',
  history: '<path d="M3 11a9 9 0 1 1 2 7M3 4v7h7m2-4v6l4 2"/>',
  settings:
    '<path d="m9 3-1 3-3 1-2 3 2 3v4l4 1 3 3 3-3 4-1v-4l2-3-2-3-3-1-1-3z"/><circle cx="12" cy="12" r="3"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  pause: '<path d="M9 5v14M15 5v14"/>',
  play: '<path d="m8 5 11 7-11 7z"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  close: '<path d="m6 6 12 12M6 18 18 6"/>',
  arrow: '<path d="M5 12h14m-5-5 5 5-5 5"/>',
  external: '<path d="M14 3h7v7m0-7L10 14M10 3H4v17h17v-6"/>',
  shield:
    '<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6z"/><path d="m8 12 3 3 5-6"/>',
};
const icon = (name: string, cls = "") =>
  `<svg class="icon ${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] ?? icons.clock}</svg>`;
const kindIcon: Record<Kind, string> = {
  countdown: "clock",
  ranged_countdown: "range",
  alarm: "alarm",
  recurring_alarm: "repeat",
  stopwatch: "watch",
};
const itemIcon = (item: Item) => item.icon?.startsWith("data:image/png;base64,")
  ? `<img class="app-alarm-icon" src="${escape(item.icon)}" alt="" width="32" height="32">` : icon(kindIcon[item.kind]);
const kindLabel: Record<Kind, string> = {
  countdown: "Countdown",
  ranged_countdown: "Ranged timer",
  alarm: "Alarm",
  recurring_alarm: "Recurring alarm",
  stopwatch: "Stopwatch",
};
let items: Item[] = [];
let events: ItemEvent[] = [];
let settings: Settings = { sound: true, notifications: true, alarmVolumeOverride: false, alarmVolumePercent: 75, alarmOutputDevice: null, muteOtherApps: false };
let tab = "running";
const active = (i: Item) =>
  ["running", "scheduled", "paused", "ready_window"].includes(i.status);
const group = (i: Item) =>
  !active(i)
    ? "history"
    : i.kind === "stopwatch"
      ? "stopwatches"
      : ["alarm", "recurring_alarm"].includes(i.kind)
        ? "upcoming"
        : "running";
const date = (value: string) =>
  new Date(value).toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
const duration = (ms: number) => {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${
    s >= 3600
      ? `${Math.floor(s / 3600)
          .toString()
          .padStart(2, "0")}:`
      : ""
  }${Math.floor((s % 3600) / 60)
    .toString()
    .padStart(2, "0")}:${(s % 60).toString().padStart(2, "0")}`;
};
function timing(i: Item) {
  const now = i.pausedAt
    ? +new Date(i.pausedAt)
    : !active(i) && i.completedAt
      ? +new Date(i.completedAt)
      : Date.now();
  if (i.kind === "stopwatch")
    return {
      value: duration(now - +new Date(i.startedAt) - i.accumulatedPausedMs),
      label:
        i.status === "paused" ? "paused" : active(i) ? "elapsed" : "total time",
      progress: 0,
    };
  const end =
    i.endsAt ?? (i.minimumFired ? i.maximumAt : i.minimumAt) ?? i.fireAt;
  if (!end)
    return { value: "—", label: "No upcoming occurrence", progress: 100 };
  if (["alarm", "recurring_alarm"].includes(i.kind))
    return {
      value: new Date(end).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        timeZone: i.timezone ?? i.schedule?.timezone ?? undefined,
      }),
      label: `${date(end)} · ${i.timezone ?? i.schedule?.timezone ?? "Local time"}`,
      progress: 0,
    };
  const remaining = +new Date(end) - now;
  return {
    value: duration(remaining),
    label:
      i.status === "paused"
        ? "paused"
        : !active(i)
          ? i.status.replace("_", " ")
          : i.minimumFired
            ? "until maximum time"
            : i.kind === "ranged_countdown"
              ? "until ready window"
              : "remaining",
    progress: Math.max(
      0,
      Math.min(
        100,
        100 -
          (remaining / Math.max(1, +new Date(end) - +new Date(i.startedAt))) *
            100,
      ),
    ),
  };
}
function demoItems(): Item[] {
  const now = Date.now();
  const base = (
    id: string,
    title: string,
    kind: Kind,
    minutes: number,
  ): Item => ({
    id,
    title,
    kind,
    status: "running",
    createdAt: new Date(now - 300000).toISOString(),
    startedAt: new Date(now - 300000).toISOString(),
    endsAt: new Date(now + minutes * 60000).toISOString(),
    minimumAt: null,
    maximumAt: null,
    fireAt: null,
    timezone: null,
    schedule: null,
    source: null,
    externalId: null,
    returnUrl: null,
    pausedAt: null,
    accumulatedPausedMs: 0,
    completedAt: null,
    minimumFired: false,
    alert: null,
    alertSilenced: false,
  });
  const a = base("preview-1", "A little time to focus", "countdown", 22.23);
  const b = base("preview-2", "Let the dough rest", "ranged_countdown", 0);
  Object.assign(b, {
    endsAt: null,
    minimumAt: new Date(now - 60000).toISOString(),
    maximumAt: new Date(now + 252000).toISOString(),
    minimumFired: true,
    status: "ready_window",
  });
  const c = base("preview-3", "Morning reset", "recurring_alarm", 0);
  Object.assign(c, {
    status: "scheduled",
    endsAt: null,
    fireAt: new Date(now + 86400000).toISOString(),
    schedule: {
      kind: "weekly",
      weekdays: [0, 1, 2, 3, 4],
      time: "07:30",
      timezone: "Asia/Jerusalem",
    },
  });
  const d = base("preview-4", "An afternoon walk", "stopwatch", 0);
  Object.assign(d, {
    endsAt: null,
    startedAt: new Date(now - 1397000).toISOString(),
  });
  return [a, b, c, d];
}
function toast(message: string, error = false) {
  const el = document.querySelector<HTMLDivElement>("#toast")!;
  el.textContent = message;
  el.className = `toast visible ${error ? "error" : ""}`;
  setTimeout(() => el.classList.remove("visible"), 5500);
}
async function refresh() {
  if (refreshing) return;
  refreshing = true;
  try {
    if (native) [items, events, settings] = await Promise.all([invoke<Item[]>("list_items"), invoke<ItemEvent[]>("list_events"), invoke<Settings>("get_settings")]);
    const snapshot = JSON.stringify([items, events, settings]);
    if (snapshot !== previousSnapshot) { previousSnapshot = snapshot; if (alertWindow) renderAlerts(); else renderContent(); }
  } finally { refreshing = false; }
}
function shell() {
  app.innerHTML = `<aside class="sidebar"><a class="brand" href="#" aria-label="Timebridge home"><span class="brand-mark">${icon("bridge")}</span>Timebridge<span class="version">BETA</span></a><div class="workspace-label">YOUR TIME, IN ONE PLACE</div><nav aria-label="Main navigation">${[
    ["running", "clock", "Running"],
    ["upcoming", "alarm", "Upcoming"],
    ["stopwatches", "watch", "Stopwatches"],
    ["history", "history", "History"],
  ]
    .map(
      ([id, ic, label]) =>
        `<button class="nav-item" data-tab="${id}">${icon(ic)}<span>${label}</span><span class="nav-count" data-count="${id}">0</span></button>`,
    )
    .join(
      "",
    )}</nav><div class="sidebar-bottom"><div class="local-note">${icon("shield")}<div><strong>On this device. Always.</strong><p>Your time stays yours.<br>No account. No cloud.</p></div></div><button class="nav-item" id="settings">${icon("settings")}<span>Settings</span></button><div class="sidebar-footer"><span class="status-dot"></span>${native ? "Desktop service active" : "Design preview"}<span>v0.2</span></div></div></aside><main><header class="topbar"><div class="breadcrumb">Workspace <span>/</span> <strong id="breadcrumb">Running</strong></div><div class="today">${new Date().toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" })}</div></header><div class="main-body"><div id="preview-note">${native ? "" : "DESIGN PREVIEW · Sample items only. Run the desktop app for persistent timers and alerts."}</div><section class="page-heading"><div><div class="eyebrow">MAKE ROOM FOR THE MOMENT</div><h1 id="page-title">Time that keeps going<span>.</span></h1><p id="page-description">Set it here. Get on with your day.</p></div><button class="button primary" id="new-item">${icon("plus")} New item</button></section><div id="audio-warning" role="status"></div><div id="pairing-notice"></div><div id="content"></div><footer class="main-footer"><span>${icon("bridge")} Built to keep going.</span><span>${native ? "Closing this window keeps Timebridge in your tray." : "Windows & macOS · Local first"}</span></footer></div></main><dialog id="dialog"></dialog><div id="toast" class="toast" role="status" aria-live="polite"></div>`;
  app.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach(
    (b) =>
      (b.onclick = () => {
        tab = b.dataset.tab!;
        renderContent();
      }),
  );
  app.querySelector<HTMLAnchorElement>(".brand")!.onclick = (e) => {
    e.preventDefault();
    tab = "running";
    renderContent();
  };
  document.querySelector<HTMLButtonElement>("#new-item")!.onclick = () =>
    createDialog();
  document.querySelector<HTMLButtonElement>("#settings")!.onclick =
    settingsDialog;
}
function card(i: Item) {
  const t = timing(i);
  const running = active(i);
  const ready = i.status === "ready_window";
  return `<article class="timer-card ${ready ? "ready" : ""} ${!running ? "finished" : ""}" data-item="${escape(i.id)}"><div class="card-top"><span class="type-icon ${ready ? "amber" : ""}">${itemIcon(i)}</span><span class="type-label">${kindLabel[i.kind]}</span><span class="badge ${ready ? "amber" : ""}">${ready ? "● READY WINDOW" : i.status === "running" ? "● RUNNING" : escape(i.status.replace("_", " ").toUpperCase())}</span><button class="icon-button detail-button" data-id="${escape(i.id)}" title="Item details" aria-label="Details for ${escape(i.title)}">···</button></div><h3>${escape(i.title)}</h3><div class="time-display" data-time="${escape(i.id)}">${escape(t.value)}</div><div class="time-label" data-label="${escape(i.id)}">${escape(t.label)}</div>${i.kind === "countdown" || i.kind === "ranged_countdown" ? `<div class="progress-track"><div data-progress="${escape(i.id)}" style="width:${t.progress}%"></div></div>` : '<div class="card-spacer"></div>'}<div class="card-source">${icon(i.source ? "external" : "shield")}<span>${i.source ? `${escape(i.source.name)} <small>${escape(i.source.origin)}</small>` : "Created locally"}</span></div><div class="card-actions">${running ? `<button class="button secondary small" data-action="${i.status === "paused" ? "resume" : "pause"}" data-id="${escape(i.id)}">${icon(i.status === "paused" ? "play" : "pause")}${i.status === "paused" ? "Resume" : "Pause"}</button>${["countdown", "ranged_countdown"].includes(i.kind) ? `<button class="button quiet small" data-action="extend5" data-id="${escape(i.id)}">+5 min</button>` : ""}<button class="button quiet small push-right" data-action="done" data-id="${escape(i.id)}">${icon("check")}Done</button>` : `<span class="finished-date">${i.completedAt ? date(i.completedAt) : ""}</span>${i.alert ? `<button class="button secondary small push-right" data-action="dismiss" data-id="${escape(i.id)}">Dismiss</button>` : ""}`}</div></article>`;
}
function renderContent() {
  const titles: Record<string, string> = {
    running: "Time that keeps going<span>.</span>",
    upcoming: "A little ahead of time<span>.</span>",
    stopwatches: "Every moment counts<span>.</span>",
    history: "Time, well spent<span>.</span>",
  };
  const subtitles: Record<string, string> = {
    running: "Set it here. Get on with your day.",
    upcoming: "Your next moments, taken care of.",
    stopwatches: "Start when you’re ready. Stop when you’re done.",
    history: "A quiet record of what you’ve made time for.",
  };
  document.querySelector("#page-title")!.innerHTML = titles[tab];
  document.querySelector("#page-description")!.textContent = subtitles[tab];
  document.querySelector("#breadcrumb")!.textContent =
    tab.charAt(0).toUpperCase() + tab.slice(1);
  app
    .querySelectorAll<HTMLElement>("[data-tab]")
    .forEach((b) => b.classList.toggle("selected", b.dataset.tab === tab));
  app
    .querySelectorAll<HTMLElement>("[data-count]")
    .forEach(
      (b) =>
        (b.textContent = String(
          items.filter((i) => group(i) === b.dataset.count).length,
        )),
    );
  const list = items.filter((i) => group(i) === tab);
  const alerts = items.filter((i) => i.alert);
  const upcoming = items
    .filter((i) => group(i) === "upcoming" && i.fireAt)
    .sort((a, b) => a.fireAt!.localeCompare(b.fireAt!));
  document.querySelector("#content")!.innerHTML =
    `${alerts.map(alertMarkup).join("")}${tab === "running" ? `<div class="overview"><div><span class="overview-label">${icon("clock")} IN MOTION</span><strong>${items.filter((i) => group(i) === "running").length}<small>active timers</small></strong></div><div><span class="overview-label">${icon("alarm")} UP NEXT</span><strong class="overview-next">${upcoming.length ? escape(upcoming[0].title) : "Nothing scheduled"}<small>${upcoming.length ? date(upcoming[0].fireAt!) : "A little breathing room"}</small></strong></div><div class="overview-status"><span class="service-orbit">${icon("bridge")}</span><span><strong>${native ? "We’ll keep the time." : "Meet your new timekeeper."}</strong><small>${native ? "Even when this window is closed." : "Independent of your browser."}</small></span></div></div>` : ""}<div class="section-heading"><h2>${tab === "running" ? "Your timers" : tab.charAt(0).toUpperCase() + tab.slice(1)} <span>${list.length.toString().padStart(2, "0")}</span></h2>${tab === "history" ? '<button class="text-button" id="clear-history">Clear dismissed history</button>' : "<span>All on this device</span>"}</div>${list.length ? `<div class="card-grid">${list.map(card).join("")}</div>` : `<div class="empty-state"><div class="empty-clock">${icon(tab === "history" ? "history" : tab === "upcoming" ? "alarm" : tab === "stopwatches" ? "watch" : "clock")}</div><h3>${tab === "history" ? "A fresh start." : tab === "upcoming" ? "Something to look forward to?" : tab === "stopwatches" ? "Take it one second at a time." : "What will you make time for?"}</h3><p>${tab === "history" ? "Completed and cancelled items will appear here." : "A focused hour, a well-earned break, or something all your own."}</p>${tab === "history" ? "" : `<button class="button secondary" id="empty-create">${icon("plus")}Create ${tab === "upcoming" ? "an alarm" : tab === "stopwatches" ? "a stopwatch" : "your first timer"}</button>`}</div>`}${
      tab === "running"
        ? `<section class="quick-start"><div class="section-heading"><h2>A little time for…</h2><span>QUICK START</span></div><div class="preset-grid">${[
            ["5", "A breather", "Step away. Come back fresh.", "☕"],
            ["25", "Deep focus", "One thing at a time.", "◎"],
            ["10", "A fresh start", "Make a little room.", "✧"],
          ]
            .map(
              ([mins, title, desc, ic]) =>
                `<button class="preset" data-preset="${mins}" data-title="${title}"><span class="preset-icon">${ic}</span><span><strong>${title}</strong><small>${desc}</small></span><b>${mins}<small>MIN</small></b>${icon("arrow")}</button>`,
            )
            .join("")}</div></section>`
        : ""
    }`;
  document
    .querySelector<HTMLButtonElement>("#empty-create")
    ?.addEventListener("click", () =>
      createDialog(
        tab === "upcoming"
          ? "alarm"
          : tab === "stopwatches"
            ? "stopwatch"
            : "countdown",
      ),
    );
  document
    .querySelectorAll<HTMLButtonElement>("[data-preset]")
    .forEach(
      (b) =>
        (b.onclick = () =>
          createDialog("countdown", b.dataset.title, Number(b.dataset.preset))),
    );
  document
    .querySelectorAll<HTMLButtonElement>("[data-action]")
    .forEach(
      (b) => (b.onclick = () => void action(b.dataset.id!, b.dataset.action!)),
    );
  document
    .querySelectorAll<HTMLButtonElement>(".detail-button")
    .forEach((b) => (b.onclick = () => detailDialog(b.dataset.id!)));
  document
    .querySelector<HTMLButtonElement>("#clear-history")
    ?.addEventListener("click", () => confirmClear());
}
async function action(id: string, action: string) {
  if (!native) {
    toast(
      "This is a design preview. Open Timebridge Desktop to manage timers.",
    );
    return;
  }
  try {
    await invoke(action === "snooze" ? "snooze_item" : "item_action", {
      id,
      action,
    });
    await refresh();
  } catch (e) {
    toast(String(e), true);
  }
}
function dialog(title: string, subtitle: string, body: string) {
  const d = document.querySelector<HTMLDialogElement>("#dialog")!;
  d.innerHTML = `<div class="dialog-heading"><span class="eyebrow">TIMEBRIDGE</span><button class="icon-button" id="close-dialog" aria-label="Close dialog">${icon("close")}</button><h2>${title}</h2><p>${subtitle}</p></div>${body}`;
  if (!d.open) d.showModal();
  document.querySelector<HTMLButtonElement>("#close-dialog")!.onclick = () =>
    d.close();
  return d;
}
function createDialog(kind: Kind = "countdown", title = "", minutes = 25) {
  const localZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const d = dialog(
    "Make time for something.",
    "Choose a timer, set your moment, and leave the rest to us.",
    `<form id="create-form"><label>Item type<select name="kind">${Object.entries(
      kindLabel,
    )
      .map(
        ([v, t]) =>
          `<option value="${v}" ${v === kind ? "selected" : ""}>${t}</option>`,
      )
      .join(
        "",
      )}<option value="dates">Date-list alarm</option></select></label><label>Name (optional)<input name="title" maxlength="200" placeholder="Leave blank for an automatic name" value="${escape(title)}"></label><div id="type-fields"></div><div class="form-audio-warning">${escape(audioWarning)}</div><div class="form-error" role="alert"></div><div class="form-note">${icon("shield")}Saved on this device. No account needed.</div><button class="button primary full" type="submit">${icon("plus")}Create item</button></form>`,
  );
  const f = document.querySelector<HTMLFormElement>("#create-form")!;
  const select = f.elements.namedItem("kind") as HTMLSelectElement;
  function fields() {
    const k = select.value;
    document.querySelector("#type-fields")!.innerHTML =
      k === "countdown"
        ? `<div class="form-row"><label>Minutes<input name="minutes" type="number" min="0" max="525600" value="${minutes}" required></label><label>Seconds<input name="seconds" type="number" min="0" max="59" value="0" required></label></div>`
        : k === "ranged_countdown"
          ? '<div class="form-row"><label>Minimum (minutes)<input name="min" type="number" min="1" value="10" required></label><label>Maximum (minutes)<input name="max" type="number" min="2" value="15" required></label></div>'
          : k === "alarm"
            ? `<label>Date & time (this device’s local time)<input name="date" type="datetime-local" required></label><label>Timezone<input value="${escape(localZone)}" disabled></label>`
            : k === "recurring_alarm"
              ? `<label>Time<input name="time" type="time" value="07:30" required></label><label>Repeat on</label><div class="weekday-picker">${["M", "T", "W", "T", "F", "S", "S"].map((s, i) => `<label><input type="checkbox" name="day" value="${i}" ${i < 5 ? "checked" : ""}><span>${s}</span></label>`).join("")}</div><label>IANA timezone<input name="timezone" value="${escape(localZone)}" required></label><p class="field-help">During daylight saving changes, skipped times are omitted and repeated times ring once.</p>`
              : k === "dates"
                ? `<label>Occurrences (one per line, with UTC offset)<textarea name="dates" rows="4" placeholder="2026-10-12T09:00:00+03:00" required></textarea></label><label>IANA timezone<input name="timezone" value="${escape(localZone)}" required></label>`
                : '<div class="stopwatch-note">Your stopwatch starts as soon as you create it. Pause and resume whenever you need.</div>';
  }
  select.onchange = fields;
  fields();
  (f.querySelector("#type-fields input") as HTMLInputElement | null)?.focus();
  f.onsubmit = async (e) => {
    e.preventDefault();
    const error = f.querySelector<HTMLElement>(".form-error")!;
    if (!native) {
      error.textContent =
        "This is a design preview. Launch the desktop app to create a real timer.";
      return;
    }
    const data = new FormData(f);
    const k = String(data.get("kind"));
    const input: Record<string, unknown> = {
      title: data.get("title"),
      kind: k === "dates" ? "recurring_alarm" : k,
    };
    try {
      if (k === "countdown")
        input.durationSeconds =
          Number(data.get("minutes")) * 60 + Number(data.get("seconds"));
      if (k === "ranged_countdown") {
        input.minimumSeconds = Number(data.get("min")) * 60;
        input.maximumSeconds = Number(data.get("max")) * 60;
      }
      if (k === "alarm") {
        input.fireAt = new Date(String(data.get("date"))).toISOString();
        input.timezone = localZone;
      }
      if (k === "recurring_alarm")
        input.schedule = {
          kind: "weekly",
          weekdays: data.getAll("day").map(Number),
          time: data.get("time"),
          timezone: data.get("timezone"),
        };
      if (k === "dates") {
        const occurrences = String(data.get("dates"))
          .split("\n")
          .map((s) => s.trim())
          .filter(Boolean);
        if (occurrences.some((s) => !/(Z|[+-]\d{2}:\d{2})$/.test(s)))
          throw new Error(
            "Each occurrence needs an explicit offset, such as +03:00 or Z.",
          );
        input.schedule = {
          kind: "dates",
          occurrences,
          timezone: data.get("timezone"),
        };
      }
      (f.querySelector('[type="submit"]') as HTMLButtonElement).disabled = true;
      const item = await invoke<Item>("create_item", { input });
      tab = group(item);
      d.close();
      await refresh();
      toast("A little time, taken care of.");
    } catch (err) {
      error.textContent = String(err);
      (f.querySelector('[type="submit"]') as HTMLButtonElement).disabled =
        false;
    }
  };
}
function detailDialog(id: string) {
  const i = items.find((i) => i.id === id)!;
  const history = events.filter((e) => e.itemId === id);
  const d = dialog(
    escape(i.title),
    `${kindLabel[i.kind]} · ${escape(i.status.replace("_", " "))}`,
    `<form id="rename-form"><label>Name<input name="title" value="${escape(i.title)}" maxlength="200" required></label><button class="button secondary small" type="submit">Save name</button></form><dl class="details"><dt>Source</dt><dd>${i.source ? `${escape(i.source.name)}<br>${escape(i.source.origin)}` : "Created locally"}</dd><dt>Created</dt><dd>${date(i.createdAt)}</dd>${i.schedule ? `<dt>Schedule</dt><dd>${i.schedule.kind === "weekly" ? `${i.schedule.weekdays.map((n) => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][n]).join(", ")} at ${escape(i.schedule.time)}` : `${i.schedule.occurrences.length} explicit dates`}<br>${escape(i.schedule.timezone)}</dd>` : ""}</dl>${i.returnUrl ? '<button class="button secondary" id="open-source">Return to source ↗</button>' : ""}<h3 class="timeline-heading">Activity</h3><div class="timeline">${
      history.length
        ? history
            .slice(0, 12)
            .map(
              (e) =>
                `<div><span class="timeline-dot"></span><strong>${escape(e.event.replaceAll(".", " · ").replaceAll("_", " "))}</strong><small>${date(e.occurredAt)} · ${escape(e.reason)}</small></div>`,
            )
            .join("")
        : "<p>No recorded activity in preview.</p>"
    }</div>${active(i) ? '<button class="button danger full" id="cancel-item">Cancel this item</button>' : ""}`,
  );
  document.querySelector<HTMLFormElement>("#rename-form")!.onsubmit = async (
    e,
  ) => {
    e.preventDefault();
    if (!native) return toast("Editing is available in the desktop app.");
    try {
      const data = new FormData(e.currentTarget as HTMLFormElement);
      await invoke("rename_item", { id, title: data.get("title") });
      d.close();
      await refresh();
    } catch (e) {
      toast(String(e), true);
    }
  };
  document
    .querySelector<HTMLButtonElement>("#cancel-item")
    ?.addEventListener("click", async () => {
      await action(id, "cancel");
      d.close();
    });
  document
    .querySelector<HTMLButtonElement>("#open-source")
    ?.addEventListener("click", async () => {
      try {
        await invoke("open_source", { id });
      } catch (e) {
        toast(String(e), true);
      }
    });
}
async function settingsDialog() {
  const devices = native ? await invoke<AudioDevice[]>("audio_devices").catch(() => []) : [];
  const sites = native ? await invoke<Site[]>("approved_sites").catch(() => []) : [];
  const bridge = native ? await invoke<{error:string|null;helperError:string|null}>("bridge_status") : null;
  const d = dialog(
    "Quietly in your corner.",
    "A few preferences to make Timebridge yours.",
    `<form id="settings-form"><label class="setting-row"><span><strong>Notification sounds</strong><small>A softer cue for ready windows, a stronger cue when time is up.</small></span><input name="sound" type="checkbox" ${settings.sound ? "checked" : ""}></label><label class="setting-row"><span><strong>Unmute and set alarm volume</strong><small>Temporarily change Windows output and Timebridge mixer volume only while an alarm sounds. Restore previous volume and mute settings when all alarms stop.</small></span><input name="alarmVolumeOverride" type="checkbox" ${settings.alarmVolumeOverride ? "checked" : ""}></label><label class="field">Alarm volume (%)<input name="alarmVolumePercent" type="number" min="1" max="100" step="1" required value="${settings.alarmVolumePercent}"></label><p class="field-help">Default: 75%. Requires notification sounds.</p><label class="field">Alarm output device<select name="alarmOutputDevice"><option value="">Use current Windows output</option>${settings.alarmOutputDevice && !devices.some(d => d.id === settings.alarmOutputDevice) ? `<option value="${escape(settings.alarmOutputDevice)}" selected>Saved device (disconnected)</option>` : ""}${devices.map(device => `<option value="${escape(device.id)}" ${device.id === settings.alarmOutputDevice ? "selected" : ""}>${escape(device.name)}</option>`).join("")}</select></label><p class="field-help">Send alarms to speakers even while headphones are the normal output. Your normal output stays unchanged. If the selected device is disconnected, alarms use the current output.</p><label class="setting-row"><span><strong>Mute other apps during alarms</strong><small>Temporarily mute other apps across output devices. Restore their previous mute settings after the last alarm stops.</small></span><input name="muteOtherApps" type="checkbox" ${settings.muteOtherApps ? "checked" : ""}></label><p class="field-help">Test sound uses the selected device at its current volume. It does not unmute, raise volume, or mute other apps.</p><label class="setting-row"><span><strong>Desktop notifications</strong><small>Show native notifications without opening the window.</small></span><input name="notifications" type="checkbox" ${settings.notifications ? "checked" : ""}></label><div class="settings-note">${icon("shield")}<div><strong>Local by design</strong><p>Items, settings, and event history stay in SQLite on this device. Websites need your explicit approval. Revoke access below at any time.</p></div></div><p class="field-help">Closing the window keeps timers in the tray. Quitting the app stops alerts until you reopen it. Overdue timers recover on launch. Your computer cannot play alerts while powered off or asleep.</p><button class="button secondary" type="button" id="test-sound">Test sound</button><p id="settings-audio-warning" class="field-help">${escape(audioWarning || "Output volume is on. Use Test sound to confirm you can hear it.")}</p><button class="button primary full" type="submit">Save preferences</button></form><section class="sites"><h3>Connect apps & websites</h3><button class="button secondary" type="button" id="open-setup">Open setup guide & test alarm</button><p class="field-help">${escape(bridge?.error || "Local bridge is available. A website can request approval from the Timebridge SDK or extension.")}</p>${bridge?.helperError ? `<p class="form-error">${escape(bridge.helperError)}</p>` : ""}${sites.length ? sites.map(site => `<div class="site-row"><span><strong>${escape(site.name)}</strong><small>${escape(site.origin)}</small></span><button class="button small danger" data-revoke="${escape(site.origin)}">Revoke</button></div>`).join("") : "<p class=field-help>No approved websites yet.</p>"}</section>`,
  );
  document.querySelector<HTMLButtonElement>("#test-sound")!.onclick = () => { if (native) void invoke("test_sound", {device: d.querySelector<HTMLSelectElement>("[name=alarmOutputDevice]")!.value || null}).catch(e=>toast(String(e),true)); };
  d.querySelector<HTMLButtonElement>("#open-setup")!.onclick = () => { if (native) void invoke("open_setup").catch(e=>toast(String(e),true)); };
  d.querySelectorAll<HTMLButtonElement>("[data-revoke]").forEach(b=>b.onclick=async()=>{try{await invoke("revoke_site",{origin:b.dataset.revoke});await settingsDialog();toast("Site access revoked. Existing timers remain under your control.");}catch(e){toast(String(e),true);}});
  document.querySelector<HTMLFormElement>("#settings-form")!.onsubmit = async (
    e,
  ) => {
    e.preventDefault();
    const f = new FormData(e.currentTarget as HTMLFormElement);
    if (!native) return toast("Preferences are available in the desktop app.");
    try {
      await invoke("set_settings", {
        settings: {
          sound: f.has("sound"),
          notifications: f.has("notifications"),
          alarmVolumeOverride: f.has("alarmVolumeOverride"),
          alarmVolumePercent: Number(f.get("alarmVolumePercent")),
          alarmOutputDevice: String(f.get("alarmOutputDevice") || "") || null,
          muteOtherApps: f.has("muteOtherApps"),
        },
      });
      d.close();
      await refresh();
      toast("Preferences saved.");
    } catch (e) {
      toast(String(e), true);
    }
  };
}
function confirmClear() {
  const d = dialog(
    "Clear your history?",
    "This deletes finished, non-ringing items and their events on this device.",
    '<button class="button danger full" id="confirm-clear">Delete history</button>',
  );
  document.querySelector<HTMLButtonElement>("#confirm-clear")!.onclick =
    async () => {
      if (!native) return toast("History is available in the desktop app.");
      try {
        await invoke("clear_history");
        d.close();
        await refresh();
      } catch (e) {
        toast(String(e), true);
      }
    };
}
if (alertWindow) { document.body.classList.add("alert-window"); app.innerHTML = '<div id="alert-content"></div><div id="toast" class="toast" role="status"></div>'; } else shell();
if (!native) items = demoItems();
void refresh().then(() => { if (native && !alertWindow && !localStorage.getItem("timebridge-welcome-v1")) welcome(); }).catch((e) => toast(`Could not load Timebridge: ${e}`, true));
if (native) {
  void listen(
    "items-changed",
    () => void refresh().catch((e) => toast(String(e), true)),
  );
  if (!alertWindow) { void listen("new-item", () => createDialog()); void listen("pairing-changed", () => void refreshPairing()); }
  void checkAudio();
  if (!alertWindow) void refreshPairing();
  setInterval(() => void refresh().catch(e=>toast(String(e),true)), 1500);
  setInterval(() => { void checkAudio(); if (!alertWindow) void refreshPairing(); }, 5000);
  void listen<string>("service-error", (e) =>
    toast(`Timebridge needs attention: ${e.payload}`, true),
  );
}
setInterval(() => {
  for (const i of items) {
    const t = timing(i);
    const el = document.querySelector(`[data-time="${i.id}"]`);
    if (el) el.textContent = t.value;
    const label = document.querySelector(`[data-label="${i.id}"]`);
    if (label) label.textContent = t.label;
    const progress = document.querySelector<HTMLElement>(
      `[data-progress="${i.id}"]`,
    );
    if (progress) progress.style.width = `${t.progress}%`;
  }
}, 250);

function alertMarkup(i: Item) {
  return `<div class="alert-banner ringing" role="alert">${itemIcon(i)}<div><strong>${escape(i.title)} · ${escape(i.alert)}</strong><span>${i.source ? escape(i.source.origin) : "Created locally"}</span></div><div class="alert-buttons">${!i.alertSilenced ? `<button class="button small secondary" data-action="silence" data-id="${escape(i.id)}">Silence</button>` : '<span class="silenced-label">Sound silenced</span>'}<button class="button small secondary" data-action="snooze" data-id="${escape(i.id)}">Snooze 5 min</button><button class="button small primary" data-action="dismiss" data-id="${escape(i.id)}">Dismiss</button></div></div>`;
}
function renderAlerts() {
  const alerts=items.filter(i=>i.alert);
  document.querySelector("#alert-content")!.innerHTML=`<div class="alert-window-heading"><span class="brand-mark">${icon("bridge")}</span><span>TIMEBRIDGE</span></div><h1>${alerts.length ? "Time’s up." : "All taken care of."}</h1><p>${alerts.length ? "Your timer needs your attention." : "You can close this window."}</p><div id="audio-warning" role="status">${escape(audioWarning)}</div>${alerts.map(alertMarkup).join("")}<button class="button quiet full" id="open-main">Open Timebridge</button>`;
  document.querySelectorAll<HTMLButtonElement>("[data-action]").forEach(b=>b.onclick=()=>void action(b.dataset.id!,b.dataset.action!));
  document.querySelector<HTMLButtonElement>("#open-main")!.onclick=()=>void invoke("open_main");
}
async function checkAudio() {
  if (!native) return;
  try {
    const status=await invoke<AudioStatus>("audio_status");
    audioWarning=!settings.sound ? "Sound is turned off in Timebridge. Visual alerts are still enabled." : status.warning ? `${status.warning}${settings.alarmVolumeOverride ? ` Automatic alarm volume is enabled: Timebridge will unmute and set volume to ${settings.alarmVolumePercent}% when an alarm sounds, then restore it.` : ""}` : "";
    const el=document.querySelector<HTMLElement>("#audio-warning");
    if(el){el.textContent=audioWarning;el.classList.toggle("volume-warning",Boolean(audioWarning));}
    const settingsWarning=document.querySelector<HTMLElement>("#settings-audio-warning");
    if(settingsWarning){settingsWarning.textContent=audioWarning||"Output volume is on. Use Test sound to confirm you can hear it.";settingsWarning.classList.toggle("volume-warning",Boolean(audioWarning));}
  } catch { /* Volume is advisory; alert scheduling remains independent. */ }
}
let lastPairing="";
async function refreshPairing() {
  if (!native || alertWindow) return;
  const requests=await invoke<PairRequest[]>("pairing_requests").catch(()=>[]);
  requests.sort((a,b)=>a.id.localeCompare(b.id));const key=JSON.stringify(requests);if(key===lastPairing)return;lastPairing=key;
  document.querySelector("#pairing-notice")!.innerHTML=requests.map(p=>`<section class="pairing-card"><span class="eyebrow">WEBSITE PERMISSION REQUEST</span><h3>${escape(p.origin)}</h3><p>Requests to connect as “${escape(p.name)}”. Allow this origin to create timers and alarms, manage only its own items, and receive its own events?</p>${p.origin.startsWith("http:") ? '<p class="field-help">Local development origin · HTTP</p>' : ""}<div><button class="button primary" data-pair="${escape(p.id)}" data-allow="true">Allow site</button><button class="button secondary" data-pair="${escape(p.id)}" data-allow="false">Deny</button></div><small>Expires ${new Date(p.expiresAt).toLocaleTimeString()}</small></section>`).join("");
  document.querySelectorAll<HTMLButtonElement>("[data-pair]").forEach(b=>b.onclick=async()=>{b.disabled=true;try{await invoke("approve_site",{id:b.dataset.pair,allow:b.dataset.allow==="true"});await refreshPairing();}catch(e){toast(String(e),true);b.disabled=false;}});
}

function welcome() {
  const d=dialog("Welcome to Timebridge.", "Your alarms have a home, even after the browser closes.", '<p class="field-help">Create your own timers here, or connect a website or app. The setup guide walks you through installation, approval, and a 10-second test alarm.</p><button class="button primary full" id="welcome-setup">Connect an app or website</button><button class="button secondary full" id="welcome-local">Start with a local timer</button>');
  d.addEventListener("close",()=>localStorage.setItem("timebridge-welcome-v1","seen"),{once:true});
  d.querySelector<HTMLButtonElement>("#welcome-setup")!.onclick=async()=>{try{await invoke("open_setup");d.close();}catch(e){toast(String(e),true);}};
  d.querySelector<HTMLButtonElement>("#welcome-local")!.onclick=()=>{d.close();document.querySelector<HTMLButtonElement>("#new-item")?.click();};
}
