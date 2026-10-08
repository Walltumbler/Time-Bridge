export type Kind =
  "countdown" | "ranged_countdown" | "alarm" | "recurring_alarm" | "stopwatch";
export type Status =
  | "scheduled"
  | "running"
  | "paused"
  | "ready_window"
  | "completed"
  | "cancelled"
  | "dismissed";
export type Schedule =
  | { kind: "weekly"; weekdays: number[]; time: string; timezone: string }
  | { kind: "dates"; occurrences: string[]; timezone: string };
export interface Item {
  icon?: string | null;
  id: string;
  title: string;
  kind: Kind;
  status: Status;
  createdAt: string;
  startedAt: string;
  endsAt: string | null;
  minimumAt: string | null;
  maximumAt: string | null;
  fireAt: string | null;
  timezone: string | null;
  schedule: Schedule | null;
  source: { name: string; origin: string } | null;
  externalId: string | null;
  returnUrl: string | null;
  pausedAt: string | null;
  accumulatedPausedMs: number;
  completedAt: string | null;
  minimumFired: boolean;
  alert: string | null;
  alertSilenced: boolean;
}
export interface Settings {
  sound: boolean;
  notifications: boolean;
  alarmVolumeOverride: boolean;
  alarmVolumePercent: number;
  alarmOutputDevice: string | null;
  muteOtherApps: boolean;
}
export interface AudioDevice { id: string; name: string }
export interface AudioStatus { available: boolean; muted: boolean; volume: number | null; warning: string | null }
export interface PairRequest { id: string; origin: string; name: string; expiresAt: string; status: string }
export interface Site { origin: string; name: string; approvedAt: string }
export interface ItemEvent {
  id: number;
  itemId: string;
  event: string;
  occurredAt: string;
  recordedAt: string;
  origin: string | null;
  reason: string;
  title: string;
}
