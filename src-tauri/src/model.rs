use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Schedule {
    Weekly {
        weekdays: Vec<u32>,
        time: String,
        timezone: String,
    },
    Dates {
        occurrences: Vec<DateTime<Utc>>,
        timezone: String,
    },
}

impl Schedule {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Weekly {
                weekdays,
                time,
                timezone,
            } => {
                timezone
                    .parse::<Tz>()
                    .map_err(|_| "Unknown IANA timezone")?;
                NaiveTime::parse_from_str(time, "%H:%M").map_err(|_| "Time must be HH:MM")?;
                if weekdays.is_empty() || weekdays.len() > 7 || weekdays.iter().any(|d| *d > 6) {
                    return Err("Choose at least one weekday (Monday = 0)".into());
                }
            }
            Self::Dates {
                occurrences,
                timezone,
            } => {
                timezone
                    .parse::<Tz>()
                    .map_err(|_| "Unknown IANA timezone")?;
                if occurrences.is_empty() || occurrences.len() > 366 {
                    return Err("A date list must have 1–366 occurrences".into());
                }
            }
        }
        Ok(())
    }

    // DST: skip gaps; use the first instant of a repeated wall-clock time.
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Dates { occurrences, .. } => {
                occurrences.iter().copied().filter(|d| *d > after).min()
            }
            Self::Weekly {
                weekdays,
                time,
                timezone,
            } => {
                let tz: Tz = timezone.parse().ok()?;
                let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
                let date = after.with_timezone(&tz).date_naive();
                for offset in 0..15 {
                    let day = date.checked_add_signed(Duration::days(offset))?;
                    if !weekdays.contains(&day.weekday().num_days_from_monday()) {
                        continue;
                    }
                    let candidate = match tz.from_local_datetime(&day.and_time(time)) {
                        LocalResult::Single(d) => d,
                        LocalResult::Ambiguous(a, b) => a.min(b),
                        LocalResult::None => continue,
                    }
                    .with_timezone(&Utc);
                    if candidate > after {
                        return Some(candidate);
                    }
                }
                None
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Countdown,
    RangedCountdown,
    Alarm,
    RecurringAlarm,
    Stopwatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Scheduled,
    Running,
    Paused,
    ReadyWindow,
    Completed,
    Cancelled,
    Dismissed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub name: String,
    pub origin: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateItem {
    pub icon: Option<String>,
    #[serde(default)]
    pub title: String,
    pub kind: Kind,
    pub duration_seconds: Option<i64>,
    pub minimum_seconds: Option<i64>,
    pub maximum_seconds: Option<i64>,
    pub fire_at: Option<DateTime<Utc>>,
    pub timezone: Option<String>,
    pub schedule: Option<Schedule>,
    pub source: Option<Source>,
    pub external_id: Option<String>,
    pub return_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(default)]
    pub icon: Option<String>,
    pub id: String,
    pub title: String,
    pub kind: Kind,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub minimum_at: Option<DateTime<Utc>>,
    pub maximum_at: Option<DateTime<Utc>>,
    pub fire_at: Option<DateTime<Utc>>,
    pub timezone: Option<String>,
    pub schedule: Option<Schedule>,
    pub source: Option<Source>,
    pub external_id: Option<String>,
    pub return_url: Option<String>,
    pub paused_at: Option<DateTime<Utc>>,
    pub accumulated_paused_ms: i64,
    pub completed_at: Option<DateTime<Utc>>,
    pub minimum_fired: bool,
    pub alert: Option<String>,
    #[serde(default)]
    pub alert_silenced: bool,
}

pub fn validate_icon(icon: Option<&str>) -> Result<()> {
    use base64::Engine;
    let Some(icon) = icon else { return Ok(()); };
    if icon.len() > 22_000 { return Err("Icon must be a PNG of at most 16 KiB (128 × 128 pixels)".into()); }
    let encoded = icon.strip_prefix("data:image/png;base64,").ok_or("Icon must be a PNG data URL")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|_| "Invalid icon base64")?;
    if bytes.len() > 16_384 { return Err("Icon PNG must be at most 16 KiB".into()); }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_limits(png::Limits { bytes: 1_048_576 });
    let mut reader = decoder.read_info().map_err(|_| "Invalid PNG icon")?;
    let info = reader.info();
    if info.width == 0 || info.height == 0 || info.width > 128 || info.height > 128 || info.animation_control.is_some() {
        return Err("Icon must be a static PNG no larger than 128 × 128 pixels".into());
    }
    let mut pixels = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut pixels).map_err(|_| "Invalid PNG image data")?;
    Ok(())
}

pub fn validate_origin(origin: &str) -> Result<()> {
    if origin.len() > 512 { return Err("Origin too long".into()); }
    let url = url::Url::parse(origin).map_err(|_| "Invalid origin")?;
    let loopback = url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    if (url.scheme() != "https" && !loopback) || !url.username().is_empty() || url.password().is_some()
        || url.origin().ascii_serialization() != origin { return Err("Use an exact HTTPS origin (HTTP allowed only for localhost development)".into()); }
    Ok(())
}

pub fn validate_return_url(source: Option<&Source>, value: Option<&str>) -> Result<()> {
    let origin = if let Some(source) = source {
        if source.name.trim().is_empty() || source.name.len() > 120 {
            return Err("Invalid source name".into());
        }
        let url = url::Url::parse(&source.origin).map_err(|_| "Invalid source origin")?;
        validate_origin(&source.origin)?;
        Some(url)
    } else {
        None
    };
    if let Some(value) = value {
        if value.len() > 2048 {
            return Err("Return URL too long".into());
        }
        let url = url::Url::parse(value).map_err(|_| "Invalid return URL")?;
        let local = url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
        if (url.scheme() != "https" && !local)
            || !url.username().is_empty()
            || url.password().is_some()
            || origin.as_ref().map(|o| o.origin()) != Some(url.origin())
        {
            return Err("Return URL must match the source origin and use HTTPS (HTTP only for localhost)".into());
        }
    }
    Ok(())
}

impl Item {
    pub fn active(&self) -> bool {
        matches!(
            self.status,
            Status::Running | Status::Scheduled | Status::ReadyWindow | Status::Paused
        )
    }

    pub fn create(input: CreateItem, now: DateTime<Utc>) -> Result<Self> {
        validate_icon(input.icon.as_deref())?;
        if input.title.len() > 200 {
            return Err("Title must be at most 200 bytes".into());
        }
        validate_return_url(input.source.as_ref(), input.return_url.as_deref())?;
        if input.external_id.as_ref().is_some_and(|s| s.len() > 200) {
            return Err("External ID too long".into());
        }
        let seconds = |s: Option<i64>| -> Result<i64> {
            s.filter(|v| *v > 0 && *v <= 31_536_000)
                .ok_or("Duration must be between 1 second and 365 days".into())
        };
        let default_title = match input.kind {
            Kind::Countdown => match input.duration_seconds { Some(s) if s % 60 == 0 => format!("{} minute timer", s / 60), Some(s) => format!("{s} second timer"), None => "Timer".into() },
            Kind::RangedCountdown => "Ranged timer".into(), Kind::Alarm => "Alarm".into(),
            Kind::RecurringAlarm => "Recurring alarm".into(), Kind::Stopwatch => "Stopwatch".into(),
        };
        let mut item = Self {
            icon: input.icon,
            id: uuid::Uuid::new_v4().to_string(),
            title: if input.title.trim().is_empty() { default_title } else { input.title.trim().into() },
            kind: input.kind,
            status: Status::Running,
            created_at: now,
            started_at: now,
            ends_at: None,
            minimum_at: None,
            maximum_at: None,
            fire_at: None,
            timezone: input.timezone,
            schedule: input.schedule,
            source: input.source,
            external_id: input.external_id,
            return_url: input.return_url,
            paused_at: None,
            accumulated_paused_ms: 0,
            completed_at: None,
            minimum_fired: false,
            alert: None,
            alert_silenced: false,
        };
        match item.kind {
            Kind::Countdown => {
                item.ends_at = Some(now + Duration::seconds(seconds(input.duration_seconds)?))
            }
            Kind::RangedCountdown => {
                let min = seconds(input.minimum_seconds)?;
                let max = seconds(input.maximum_seconds)?;
                if min >= max {
                    return Err("Maximum must be greater than minimum".into());
                }
                item.minimum_at = Some(now + Duration::seconds(min));
                item.maximum_at = Some(now + Duration::seconds(max));
            }
            Kind::Alarm => {
                item.timezone
                    .as_ref()
                    .ok_or("Timezone required")?
                    .parse::<Tz>()
                    .map_err(|_| "Unknown IANA timezone")?;
                item.fire_at = Some(
                    input
                        .fire_at
                        .filter(|d| *d > now)
                        .ok_or("Alarm must be in the future")?,
                );
                item.status = Status::Scheduled;
            }
            Kind::RecurringAlarm => {
                let schedule = item.schedule.as_ref().ok_or("Schedule required")?;
                schedule.validate()?;
                item.fire_at = Some(
                    schedule
                        .next_after(now)
                        .ok_or("Schedule has no future occurrence")?,
                );
                item.status = Status::Scheduled;
            }
            Kind::Stopwatch => {}
        }
        Ok(item)
    }

    pub fn advance(&mut self, now: DateTime<Utc>) -> Vec<(String, DateTime<Utc>)> {
        if !self.active() || self.status == Status::Paused {
            return vec![];
        }
        let mut events = vec![];
        if self.kind == Kind::RangedCountdown
            && !self.minimum_fired
            && self.minimum_at.is_some_and(|t| t <= now)
        {
            self.minimum_fired = true;
            self.status = Status::ReadyWindow;
            self.alert = Some("Ready window".into());
            self.alert_silenced = false;
            events.push(("timer.minimum_reached".into(), self.minimum_at.unwrap()));
        }
        let due = match self.kind {
            Kind::Countdown => self.ends_at,
            Kind::RangedCountdown => self.maximum_at,
            Kind::Alarm | Kind::RecurringAlarm => self.fire_at,
            Kind::Stopwatch => None,
        };
        if let Some(due) = due.filter(|d| *d <= now) {
            let event = match self.kind {
                Kind::RangedCountdown => "timer.maximum_reached",
                Kind::Alarm | Kind::RecurringAlarm => "alarm.fired",
                _ => "item.completed",
            };
            events.push((event.into(), due));
            self.alert_silenced = false;
            self.alert = Some(
                if self.kind == Kind::RangedCountdown {
                    "Maximum time reached"
                } else {
                    "Time’s up"
                }
                .into(),
            );
            if self.kind == Kind::RecurringAlarm {
                // Coalesce missed recurrences into one overdue alert, then schedule the next future one.
                self.fire_at = self.schedule.as_ref().and_then(|s| s.next_after(now));
            }
            if self.kind != Kind::RecurringAlarm || self.fire_at.is_none() {
                self.status = Status::Completed;
                self.completed_at = Some(due);
                if event != "item.completed" {
                    events.push(("item.completed".into(), due));
                }
            }
        }
        events
    }

    pub fn action(&mut self, action: &str, now: DateTime<Utc>) -> Result<&'static str> {
        match action {
            "silence" if self.alert.is_some() => { self.alert_silenced = true; }
            "pause" if self.active() && self.status != Status::Paused => {
                self.paused_at = Some(now);
                self.status = Status::Paused;
            }
            "resume" if self.status == Status::Paused => {
                let delta = now
                    .signed_duration_since(self.paused_at.take().ok_or("Missing pause timestamp")?)
                    .max(Duration::zero());
                self.accumulated_paused_ms += delta.num_milliseconds();
                match self.kind {
                    Kind::Countdown | Kind::RangedCountdown => {
                        self.ends_at = self.ends_at.map(|d| d + delta);
                        self.minimum_at = self.minimum_at.map(|d| d + delta);
                        self.maximum_at = self.maximum_at.map(|d| d + delta);
                    }
                    Kind::RecurringAlarm => {
                        self.fire_at = self.schedule.as_ref().and_then(|s| s.next_after(now))
                    }
                    _ => {}
                }
                self.status = if matches!(self.kind, Kind::Alarm | Kind::RecurringAlarm) {
                    Status::Scheduled
                } else if self.minimum_fired {
                    Status::ReadyWindow
                } else {
                    Status::Running
                };
                if self.kind == Kind::RecurringAlarm && self.fire_at.is_none() {
                    self.status = Status::Completed;
                    self.completed_at = Some(now);
                }
            }
            "cancel" if self.active() => {
                self.status = Status::Cancelled;
                self.completed_at = Some(now);
                self.alert = None;
                return Ok("item.cancelled");
            }
            "done" if self.active() => {
                self.status = Status::Completed;
                self.completed_at = Some(now);
                self.alert = None;
                return Ok(if self.kind == Kind::Stopwatch {
                    "stopwatch.stopped"
                } else {
                    "item.completed"
                });
            }
            "dismiss" => {
                self.alert = None;
                if !self.active() {
                    self.status = Status::Dismissed;
                }
                return Ok("item.dismissed");
            }
            "extend1" | "extend5"
                if matches!(self.kind, Kind::Countdown | Kind::RangedCountdown)
                    && self.active() =>
            {
                let delta = Duration::minutes(if action == "extend1" { 1 } else { 5 });
                self.ends_at = self.ends_at.map(|d| d + delta);
                if !self.minimum_fired {
                    self.minimum_at = self.minimum_at.map(|d| d + delta);
                }
                self.maximum_at = self.maximum_at.map(|d| d + delta);
            }
            _ => return Err("This action is not available for the item’s current state".into()),
        }
        Ok("item.updated")
    }
}
