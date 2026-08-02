use chrono::{DateTime, TimeZone, Utc};
use rrule::{RRuleSet, Tz};
use std::str::FromStr;

use crate::error::{CoreError, Result};
use crate::event_exceptions::{self, EventException};
use crate::events::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OccurrenceScope {
    This,
    ThisAndFollowing,
    All,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventOccurrence {
    pub event_id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub original_start_ms: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub all_day: bool,
    pub recurring: bool,
}

fn local_tz() -> Tz {
    Tz::LOCAL
}

fn ms_to_datetime(tz: &Tz, ms: i64) -> Result<DateTime<Tz>> {
    tz.timestamp_millis_opt(ms)
        .single()
        .ok_or_else(|| CoreError::Message("invalid timestamp".into()))
}

fn datetime_to_ms(dt: &DateTime<Tz>) -> i64 {
    dt.timestamp_millis()
}

pub fn build_rrule_set(event: &Event) -> Result<RRuleSet> {
    let rrule = event
        .rrule
        .as_deref()
        .ok_or_else(|| CoreError::Message("event has no rrule".into()))?;
    let tz = local_tz();
    let dt_start = ms_to_datetime(&tz, event.start_ms)?;
    let body = rrule.trim();
    let rrule_line = if body.starts_with("RRULE:") {
        body.to_string()
    } else {
        format!("RRULE:{body}")
    };
    let ical = format!(
        "DTSTART:{}\n{}",
        dt_start.format("%Y%m%dT%H%M%S"),
        rrule_line
    );
    RRuleSet::from_str(&ical).map_err(|e| CoreError::Message(format!("invalid rrule: {e}")))
}

pub fn expand_recurring(
    event: &Event,
    range_start_ms: i64,
    range_end_ms: i64,
    exceptions: &[EventException],
) -> Result<Vec<EventOccurrence>> {
    let duration_ms = event.end_ms.saturating_sub(event.start_ms);
    let all_day = event.all_day != 0;
    let tz = local_tz();
    let after = ms_to_datetime(&tz, range_start_ms)?;
    let before = ms_to_datetime(&tz, range_end_ms)?;

    let set = build_rrule_set(event)?
        .after(after)
        .before(before);
    let result = set.all(u16::MAX);
    let exc_map = event_exceptions::exception_map(exceptions);

    let mut out = Vec::new();
    for dt in result.dates {
        let original_start_ms = datetime_to_ms(&dt);
        let key = (event.id.clone(), original_start_ms);
        if let Some(exc) = exc_map.get(&key) {
            if let Some((start_ms, end_ms)) =
                event_exceptions::apply_exception(exc, duration_ms, original_start_ms)?
            {
                if start_ms < range_end_ms && end_ms > range_start_ms {
                    out.push(EventOccurrence {
                        event_id: event.id.clone(),
                        calendar_id: event.calendar_id.clone(),
                        title: event.title.clone(),
                        description: event.description.clone(),
                        original_start_ms,
                        start_ms,
                        end_ms,
                        all_day,
                        recurring: true,
                    });
                }
            }
            continue;
        }

        let end_ms = original_start_ms.saturating_add(duration_ms);
        if original_start_ms < range_end_ms && end_ms > range_start_ms {
            out.push(EventOccurrence {
                event_id: event.id.clone(),
                calendar_id: event.calendar_id.clone(),
                title: event.title.clone(),
                description: event.description.clone(),
                original_start_ms,
                start_ms: original_start_ms,
                end_ms,
                all_day,
                recurring: true,
            });
        }
    }
    Ok(out)
}

pub fn single_occurrence(event: &Event) -> EventOccurrence {
    EventOccurrence {
        event_id: event.id.clone(),
        calendar_id: event.calendar_id.clone(),
        title: event.title.clone(),
        description: event.description.clone(),
        original_start_ms: event.start_ms,
        start_ms: event.start_ms,
        end_ms: event.end_ms,
        all_day: event.all_day != 0,
        recurring: event.rrule.is_some(),
    }
}

pub fn truncate_rrule_until(rrule: &str, until_ms: i64) -> Result<String> {
    let line = rrule
        .trim()
        .trim_start_matches("RRULE:")
        .split(';')
        .filter(|part| !part.starts_with("UNTIL=") && !part.starts_with("COUNT="))
        .collect::<Vec<_>>()
        .join(";");
    let until_utc: DateTime<Utc> = Utc
        .timestamp_millis_opt(until_ms)
        .single()
        .ok_or_else(|| CoreError::Message("invalid until timestamp".into()))?;
    let until_str = until_utc.format("%Y%m%dT%H%M%SZ");
    if line.is_empty() {
        return Err(CoreError::Message("empty rrule".into()));
    }
    Ok(format!("{line};UNTIL={until_str}"))
}
