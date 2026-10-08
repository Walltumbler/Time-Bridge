export type CountdownOptions = { icon?: string; title?: string; durationSeconds: number; externalId?: string; returnUrl?: string };
export type RangedCountdownOptions = { icon?: string; title?: string; minimumSeconds: number; maximumSeconds: number; externalId?: string; returnUrl?: string };
export type AlarmOptions = { icon?: string; title?: string; fireAt: string; timezone: string; externalId?: string; returnUrl?: string };
export type WeeklyAlarmOptions = { icon?: string; title?: string; weekdays: number[]; time: string; timezone: string; externalId?: string; returnUrl?: string };
export type DateListAlarmOptions = { icon?: string; title?: string; occurrences: string[]; timezone: string; externalId?: string; returnUrl?: string };
export type StopwatchOptions = { icon?: string; title?: string; externalId?: string };
export type TimebridgeItem = {
  icon: string|null; id: string; title: string; kind: string; status: string; createdAt: string;
  startedAt: string; endsAt: string|null; minimumAt: string|null; maximumAt: string|null;
  fireAt: string|null; externalId: string|null; alert: string|null;
  source: {name:string;origin:string}|null; returnUrl:string|null;
};
export type TimebridgeEvent = {id:number;itemId:string;event:string;occurredAt:string;recordedAt:string;origin:string|null;externalId:string|null;reason:string;title:string};
export type Connection = {desktop:boolean;extension:boolean;transport:"extension"|"loopback"|null;version?:string;protocol?:number;capabilities?:string[]};
export class TimebridgeError extends Error {
  constructor(message:string, public readonly code:string) { super(message);this.name="TimebridgeError"; }
}

export type ItemAction = "pause" | "resume" | "cancel" | "done" | "dismiss" | "silence" | "extend1" | "extend5";
export type ClientOptions = {
  appName?: string;
  /** Required outside a browser. Use a stable HTTPS origin identifying your application. */
  origin?: string;
  transport?: "auto" | "extension" | "loopback";
  timeoutMs?: number;
  /** Native apps may restore this from secure OS storage. Never put it in a URL. */
  credential?: string;
  onCredential?: (credential: string) => void | Promise<void>;
  fetch?: typeof fetch;
};
