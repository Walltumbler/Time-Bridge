use crate::{model::*, store::Store};
use chrono::{DateTime, Duration, Utc};
fn now() -> DateTime<Utc> {
    "2026-10-07T07:00:00Z".parse().unwrap()
}
#[test]
fn app_icons_validate_persist_and_follow_snoozes() {
    use base64::Engine;
    let icon = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(include_bytes!("../icons/32x32.png")));
    let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("icons.db");
    let mut store = Store::open(&path).unwrap();
    let mut request = input(Kind::Countdown); request.icon = Some(icon.clone());
    let item = store.create(request, now()).unwrap();
    store.tick(now() + Duration::minutes(2)).unwrap();
    assert_eq!(store.snooze(&item.id, now() + Duration::minutes(2)).unwrap().icon.as_deref(), Some(icon.as_str()));
    drop(store);
    assert!(Store::open(&path).unwrap().list().unwrap().iter().all(|i| i.icon.as_deref() == Some(icon.as_str())));
    assert!(validate_icon(Some("https://example.com/tracker.png")).is_err());
    assert!(validate_icon(Some("data:image/svg+xml,<svg/>")).is_err());
    assert!(validate_icon(Some("data:image/png;base64,bm90IGEgcG5n")).is_err());
    let large = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(include_bytes!("../icons/icon.png")));
    assert!(validate_icon(Some(&large)).is_err());
}

#[test]
fn localhost_return_urls_stay_on_the_exact_approved_origin() {
    let source = Source { name: "Example".into(), origin: "http://127.0.0.1:4790".into() };
    assert!(validate_return_url(Some(&source), Some("http://127.0.0.1:4790/demo/")).is_ok());
    assert!(validate_return_url(Some(&source), Some("http://127.0.0.1:4791/demo/")).is_err());
    assert!(validate_return_url(Some(&source), Some("http://other.example/demo/")).is_err());
}
#[test]
fn audio_preferences_migrate_validate_and_persist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.db");
    let store = Store::open(&path).unwrap();
    store.conn.execute("INSERT INTO settings(key,payload) VALUES('app',?1)", [r#"{"sound":true,"notifications":false}"#]).unwrap();
    let mut settings = store.settings().unwrap();
    assert_eq!(settings.alarm_volume_percent, 75);
    assert!(!settings.alarm_volume_override && !settings.mute_other_apps);
    assert!(settings.alarm_output_device.is_none());
    settings.alarm_volume_percent = 0;
    assert!(store.set_settings(settings.clone()).is_err());
    settings.alarm_volume_percent = 101;
    assert!(store.set_settings(settings.clone()).is_err());
    settings.alarm_volume_percent = 68;
    settings.alarm_volume_override = cfg!(windows);
    settings.mute_other_apps = cfg!(windows);
    settings.alarm_output_device = cfg!(windows).then(|| "speakers-endpoint-id".into());
    store.set_settings(settings).unwrap();
    drop(store);
    let settings = Store::open(&path).unwrap().settings().unwrap();
    assert_eq!(settings.alarm_volume_percent, 68);
    assert_eq!(settings.alarm_volume_override, cfg!(windows));
    assert_eq!(settings.mute_other_apps, cfg!(windows));
    assert_eq!(settings.alarm_output_device, cfg!(windows).then(|| "speakers-endpoint-id".into()));
    assert!(!settings.notifications);
}
fn input(kind: Kind) -> CreateItem {
    CreateItem {
        icon: None,
        title: "Test timer".into(),
        kind,
        duration_seconds: Some(60),
        minimum_seconds: Some(30),
        maximum_seconds: Some(90),
        fire_at: Some(now() + Duration::hours(1)),
        timezone: Some("Asia/Jerusalem".into()),
        schedule: None,
        source: None,
        external_id: None,
        return_url: None,
    }
}

#[test]
fn countdown_pause_and_resume_preserves_remaining() {
    let mut item = Item::create(input(Kind::Countdown), now()).unwrap();
    item.action("pause", now() + Duration::seconds(20)).unwrap();
    assert!(item.advance(now() + Duration::hours(1)).is_empty());
    item.action("resume", now() + Duration::seconds(120))
        .unwrap();
    assert_eq!(item.ends_at.unwrap(), now() + Duration::seconds(160));
    assert_eq!(item.advance(now() + Duration::seconds(160)).len(), 1);
    assert!(item.advance(now() + Duration::seconds(200)).is_empty());
}
#[test]
fn range_recovers_both_milestones_exactly_once() {
    let mut item = Item::create(input(Kind::RangedCountdown), now()).unwrap();
    let events = item.advance(now() + Duration::hours(1));
    assert_eq!(
        events.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
        vec![
            "timer.minimum_reached",
            "timer.maximum_reached",
            "item.completed"
        ]
    );
    assert!(item.advance(now() + Duration::hours(2)).is_empty());
}
#[test]
fn range_ready_window_and_extension() {
    let mut item = Item::create(input(Kind::RangedCountdown), now()).unwrap();
    item.advance(now() + Duration::seconds(30));
    assert_eq!(item.status, Status::ReadyWindow);
    item.action("extend1", now() + Duration::seconds(31))
        .unwrap();
    assert_eq!(item.minimum_at.unwrap(), now() + Duration::seconds(30));
    assert_eq!(item.maximum_at.unwrap(), now() + Duration::seconds(150));
}
#[test]
fn restart_commits_items_and_events_together() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    {
        let mut s = Store::open(&path).unwrap();
        s.create(input(Kind::Countdown), now()).unwrap();
    }
    let mut s = Store::open(&path).unwrap();
    s.tick(now() + Duration::minutes(6)).unwrap();
    assert_eq!(s.list().unwrap()[0].status, Status::Completed);
    assert_eq!(s.events().unwrap().len(), 2);
    s.tick(now() + Duration::minutes(7)).unwrap();
    assert_eq!(s.events().unwrap().len(), 2);
    assert_eq!(
        s.events().unwrap().iter().filter(|e| !e.notified).count(),
        1
    );
}

#[test]
fn alert_silence_migration_converts_sqlite_integers_to_json_booleans() {
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("legacy.db");
    let id;
    {
        let mut store=Store::open(&path).unwrap();
        id=store.create(input(Kind::Countdown),now()).unwrap().id;
        store.conn.execute("UPDATE items SET payload=json_set(payload,'$.alertSilenced',0) WHERE id=?1",[&id]).unwrap();
    }
    let store=Store::open(&path).unwrap();
    assert!(!store.list().unwrap()[0].alert_silenced);
    store.conn.execute("UPDATE items SET payload=json_set(payload,'$.alertSilenced',1) WHERE id=?1",[&id]).unwrap();
    drop(store);
    let store=Store::open(&path).unwrap();
    assert!(store.list().unwrap()[0].alert_silenced);
}
#[test]
fn cancellation_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    {
        let mut s = Store::open(&path).unwrap();
        let i = s.create(input(Kind::Countdown), now()).unwrap();
        s.action(&i.id, "cancel", now()).unwrap();
    }
    let mut s = Store::open(&path).unwrap();
    s.tick(now() + Duration::days(1)).unwrap();
    assert_eq!(s.list().unwrap()[0].status, Status::Cancelled);
    assert_eq!(s.events().unwrap()[0].event, "item.cancelled");
}
#[test]
fn safe_return_urls_only() {
    let source = Source {
        name: "Example app".into(),
        origin: "https://example.com".into(),
    };
    assert!(validate_return_url(Some(&source), Some("https://example.com/task/123")).is_ok());
    for value in [
        "https://evil.example/",
        "javascript:alert(1)",
        "file:///C:/x",
        "https://example.com.evil.test",
        "https://user@example.com",
        "http://example.com",
        "https://example.com:444/path",
    ] {
        assert!(
            validate_return_url(Some(&source), Some(value)).is_err(),
            "{value}"
        );
    }
    assert!(validate_return_url(None, Some("https://example.com")).is_err());
}
#[test]
fn malformed_inputs_rejected() {
    for seconds in [-1, 0, 31_536_001] {
        let mut i = input(Kind::Countdown);
        i.duration_seconds = Some(seconds);
        assert!(Item::create(i, now()).is_err());
    }
    let mut i = input(Kind::RangedCountdown);
    i.minimum_seconds = Some(100);
    assert!(Item::create(i, now()).is_err());
    let mut i = input(Kind::Alarm);
    i.timezone = Some("invalid".into());
    assert!(Item::create(i, now()).is_err());
    assert!(serde_json::from_str::<CreateItem>(
        r#"{"title":"x","kind":"stopwatch","command":"rm"}"#
    )
    .is_err());
}
#[test]
fn weekly_uses_iana_zone() {
    let s = Schedule::Weekly {
        weekdays: vec![0, 2, 4],
        time: "07:30".into(),
        timezone: "Asia/Jerusalem".into(),
    };
    assert_eq!(
        s.next_after(now()).unwrap(),
        "2026-10-09T04:30:00Z".parse::<DateTime<Utc>>().unwrap()
    );
}
#[test]
fn dst_gap_is_skipped_and_fold_fires_once() {
    let s = Schedule::Weekly {
        weekdays: vec![6],
        time: "02:30".into(),
        timezone: "America/New_York".into(),
    };
    assert_eq!(
        s.next_after("2026-03-08T00:00:00Z".parse().unwrap())
            .unwrap(),
        "2026-03-15T06:30:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    let s = Schedule::Weekly {
        weekdays: vec![6],
        time: "01:30".into(),
        timezone: "America/New_York".into(),
    };
    let first = s
        .next_after("2026-11-01T00:00:00Z".parse().unwrap())
        .unwrap();
    assert_eq!(
        first,
        "2026-11-01T05:30:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(
        s.next_after(first).unwrap(),
        "2026-11-08T06:30:00Z".parse::<DateTime<Utc>>().unwrap()
    );
}
#[test]
fn date_list_coalesces_overdue_and_finishes() {
    let mut i = input(Kind::RecurringAlarm);
    i.schedule = Some(Schedule::Dates {
        timezone: "UTC".into(),
        occurrences: vec![
            now() + Duration::hours(1),
            now() + Duration::hours(2),
            now() + Duration::hours(3),
        ],
    });
    let mut item = Item::create(i, now()).unwrap();
    let events = item.advance(now() + Duration::minutes(150));
    assert_eq!(events.len(), 1);
    assert_eq!(item.fire_at, Some(now() + Duration::hours(3)));
    item.advance(now() + Duration::hours(4));
    assert_eq!(item.status, Status::Completed);
}
#[test]
fn stopwatch_elapsed_excludes_pause() {
    let mut i = Item::create(input(Kind::Stopwatch), now()).unwrap();
    i.action("pause", now() + Duration::seconds(10)).unwrap();
    i.action("resume", now() + Duration::seconds(100)).unwrap();
    assert_eq!(i.accumulated_paused_ms, 90_000);
    assert_eq!(
        i.action("done", now() + Duration::seconds(110)).unwrap(),
        "stopwatch.stopped"
    );
}

#[test]
fn pending_alerts_are_not_lost_outside_recent_history() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("test.db")).unwrap();
    let i = s.create(input(Kind::Countdown), now()).unwrap();
    s.tick(now() + Duration::seconds(61)).unwrap();
    for _ in 0..510 {
        s.rename(&i.id, "Renamed", now()).unwrap();
    }
    assert_eq!(s.events().unwrap().len(), 500);
    assert_eq!(s.pending_notifications().unwrap().len(), 1);
    s.action(&i.id, "dismiss", now() + Duration::seconds(62))
        .unwrap();
    assert!(s.pending_notifications().unwrap().is_empty());
}

#[test]
fn snooze_and_history_preserve_future_recurrence() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("test.db")).unwrap();
    let mut input = input(Kind::RecurringAlarm);
    input.schedule = Some(Schedule::Dates {
        timezone: "UTC".into(),
        occurrences: vec![now() + Duration::seconds(10), now() + Duration::hours(1)],
    });
    let i = s.create(input, now()).unwrap();
    s.tick(now() + Duration::seconds(11)).unwrap();
    let snooze = s.snooze(&i.id, now() + Duration::seconds(12)).unwrap();
    assert_eq!(snooze.ends_at, Some(now() + Duration::seconds(312)));
    let original = s
        .list()
        .unwrap()
        .into_iter()
        .find(|v| v.id == i.id)
        .unwrap();
    assert_eq!(original.fire_at, Some(now() + Duration::hours(1)));
    assert!(original.alert.is_none());
    s.clear_history().unwrap();
    assert_eq!(s.list().unwrap().len(), 2);
}
