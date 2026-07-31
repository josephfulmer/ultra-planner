use crate::plan::{TrainingPlan, WorkoutKind};
use chrono::NaiveDate;

fn fold_ics_date(d: NaiveDate) -> String {
    d.format("%Y%m%d").to_string()
}

fn escape_ics_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// Builds a full VCALENDAR document with one all-day VEVENT per training day.
/// Rest days are skipped to keep the calendar readable; race day gets a
/// distinct, prominent event.
pub fn generate_ics(plan: &TrainingPlan) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str("VERSION:2.0\r\n");
    out.push_str("PRODID:-//Ultra Planner//50K Training Plan//EN\r\n");
    out.push_str("CALSCALE:GREGORIAN\r\n");
    out.push_str("METHOD:PUBLISH\r\n");
    out.push_str("X-WR-CALNAME:50K Training Plan\r\n");

    let now_stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    for week in &plan.weeks {
        for day in &week.days {
            if matches!(day.kind, WorkoutKind::Rest) {
                continue;
            }

            let uid = format!(
                "{}-w{}-{:?}",
                fold_ics_date(day.date),
                week.week_number,
                day.kind
            );
            let summary = match day.distance_km {
                Some(km) if km > 0.0 => format!("{} \u{2014} {:.0} km", day.label, km),
                _ => day.label.clone(),
            };

            let description = format!(
                "{}\\n{}",
                escape_ics_text(&format!("Week {} ({} phase)", week.week_number, week.phase)),
                escape_ics_text(&day.notes)
            );

            out.push_str("BEGIN:VEVENT\r\n");
            out.push_str(&format!("UID:{uid}@ultra-planner\r\n"));
            out.push_str(&format!("DTSTAMP:{now_stamp}\r\n"));
            out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", fold_ics_date(day.date)));
            out.push_str(&format!(
                "DTEND;VALUE=DATE:{}\r\n",
                fold_ics_date(day.date + chrono::Duration::days(1))
            ));
            out.push_str(&format!("SUMMARY:{}\r\n", escape_ics_text(&summary)));
            out.push_str(&format!("DESCRIPTION:{}\r\n", description));
            if matches!(day.kind, WorkoutKind::RaceDay) {
                out.push_str("PRIORITY:1\r\n");
            }
            out.push_str("END:VEVENT\r\n");
        }
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}
