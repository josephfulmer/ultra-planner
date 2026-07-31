use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Experience {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Debug, Deserialize)]
pub struct AthleteInput {
    /// Average km run per week right now
    pub current_weekly_km: f32,
    /// Longest single run in the last 4 weeks, in km
    pub longest_recent_run_km: f32,
    pub experience: Experience,
    /// How many days a week the athlete can train (3-6)
    pub days_per_week: u8,
    /// Race date, used both to size the plan and to date the calendar
    pub race_date: NaiveDate,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum WorkoutKind {
    // (Debug already derived above; kept as a single enum block)
    Rest,
    Easy,
    MediumLong,
    Long,
    BackToBackLong,
    Tempo,
    Hills,
    Strides,
    CrossTrain,
    Strength,
    ShakeOut,
    RaceDay,
}

impl WorkoutKind {
    pub fn label(&self) -> &'static str {
        match self {
            WorkoutKind::Rest => "Rest",
            WorkoutKind::Easy => "Easy run",
            WorkoutKind::MediumLong => "Medium-long run",
            WorkoutKind::Long => "Long run",
            WorkoutKind::BackToBackLong => "Back-to-back long run",
            WorkoutKind::Tempo => "Tempo run",
            WorkoutKind::Hills => "Hill repeats",
            WorkoutKind::Strides => "Easy run + strides",
            WorkoutKind::CrossTrain => "Cross-train",
            WorkoutKind::Strength => "Strength + mobility",
            WorkoutKind::ShakeOut => "Shake-out jog",
            WorkoutKind::RaceDay => "RACE DAY \u{1F3C1} 50K",
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DayPlan {
    pub date: NaiveDate,
    pub weekday: String,
    pub kind: WorkoutKind,
    pub distance_km: Option<f32>,
    pub label: String,
    pub notes: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct WeekPlan {
    pub week_number: u32,
    pub phase: String,
    pub start_date: NaiveDate,
    pub target_km: f32,
    pub long_run_km: f32,
    pub days: Vec<DayPlan>,
}

#[derive(Debug, Serialize)]
pub struct TrainingPlan {
    pub weeks: Vec<WeekPlan>,
    pub total_weeks: u32,
    pub race_date: NaiveDate,
    pub peak_weekly_km: f32,
    pub peak_long_run_km: f32,
    pub compressed_warning: Option<String>,
}

fn weekday_index(w: Weekday) -> usize {
    // 0 = Mon ... 6 = Sun
    w.num_days_from_monday() as usize
}

/// Monday on/after `d` (or `d` itself if it's already Monday).
fn next_monday_on_or_after(d: NaiveDate) -> NaiveDate {
    let offset = d.weekday().num_days_from_monday();
    if offset == 0 {
        d
    } else {
        d + Duration::days((7 - offset) as i64)
    }
}

pub fn generate_plan(input: &AthleteInput, today: NaiveDate) -> TrainingPlan {
    let days_per_week = input.days_per_week.clamp(3, 6);

    let start_monday = next_monday_on_or_after(today);
    // Monday of the week that contains race day, so the last generated week
    // always spans the actual race date.
    let race_week_monday = input.race_date - Duration::days(
        input.race_date.weekday().num_days_from_monday() as i64,
    );
    let mut total_weeks = (((race_week_monday - start_monday).num_days() / 7) + 1).max(1) as u32;

    let mut compressed_warning = None;
    if total_weeks < 8 {
        compressed_warning = Some(format!(
            "Only {total_weeks} weeks until race day \u{2014} that's tight for a 50K. This plan compresses base building, so prioritize recovery and consider a run/walk strategy on race day."
        ));
    }
    total_weeks = total_weeks.clamp(3, 40);

    // ---- Phase split: Base 40% / Build 35% / Peak 15% / Taper 10% (min 2wk taper) ----
    let mut taper_weeks = ((total_weeks as f32) * 0.10).round().max(2.0) as u32;
    if taper_weeks > total_weeks / 3 {
        taper_weeks = (total_weeks / 3).max(1);
    }
    let remaining = total_weeks - taper_weeks;
    let mut peak_weeks = ((remaining as f32) * 0.20).round().max(1.0) as u32;
    let mut build_weeks = ((remaining as f32) * 0.45).round().max(1.0) as u32;
    let mut base_weeks = remaining.saturating_sub(peak_weeks + build_weeks);
    if base_weeks == 0 {
        base_weeks = 1;
        if build_weeks > 1 {
            build_weeks -= 1;
        } else if peak_weeks > 1 {
            peak_weeks -= 1;
        }
    }
    // Reconcile rounding so it always sums exactly to total_weeks
    let sum = base_weeks + build_weeks + peak_weeks + taper_weeks;
    if sum < total_weeks {
        base_weeks += total_weeks - sum;
    } else if sum > total_weeks {
        let mut excess = sum - total_weeks;
        while excess > 0 {
            if build_weeks > 1 {
                build_weeks -= 1;
            } else if base_weeks > 1 {
                base_weeks -= 1;
            } else if peak_weeks > 1 {
                peak_weeks -= 1;
            } else {
                break;
            }
            excess -= 1;
        }
    }
    let ramp_weeks = base_weeks + build_weeks + peak_weeks; // everything before taper

    // ---- Target volumes ----
    let peak_multiplier = match input.experience {
        Experience::Beginner => 1.55,
        Experience::Intermediate => 1.85,
        Experience::Advanced => 2.15,
    };
    let peak_weekly_km = (input.current_weekly_km * peak_multiplier)
        .max(input.current_weekly_km * 1.15)
        .min(100.0);

    let long_run_base_target = match input.experience {
        Experience::Beginner => 28.0_f32,
        Experience::Intermediate => 33.0_f32,
        Experience::Advanced => 37.0_f32,
    };
    let peak_long_run_km = long_run_base_target
        .max(input.longest_recent_run_km + 5.0)
        .min(40.0);

    let mut weeks = Vec::with_capacity(total_weeks as usize);

    for i in 0..total_weeks {
        let week_number = i + 1;
        let phase = if i < base_weeks {
            "Base"
        } else if i < base_weeks + build_weeks {
            "Build"
        } else if i < base_weeks + build_weeks + peak_weeks {
            "Peak"
        } else {
            "Taper"
        };

        let week_start = start_monday + Duration::days(7 * i as i64);

        let (target_km, long_run_km, back_to_back) = if phase == "Taper" {
            let taper_idx = i - (base_weeks + build_weeks + peak_weeks); // 0-based within taper
            let remaining_taper = taper_weeks - taper_idx; // counts down to 1 on race week
            if remaining_taper == 1 {
                // Race week: short shake-outs only, race counted separately
                (peak_weekly_km * 0.25, 8.0_f32.min(peak_long_run_km * 0.3), false)
            } else {
                let t = taper_idx as f32 / (taper_weeks.saturating_sub(1).max(1)) as f32;
                let factor = 0.80 - 0.35 * t; // decreasing from ~0.80 down toward ~0.45
                (peak_weekly_km * factor, peak_long_run_km * (0.6 - 0.15 * t), false)
            }
        } else {
            let ramp_t = if ramp_weeks <= 1 {
                1.0
            } else {
                i as f32 / (ramp_weeks - 1) as f32
            };
            let mut target = input.current_weekly_km
                + (peak_weekly_km - input.current_weekly_km) * ramp_t;
            let mut long_run = input.longest_recent_run_km
                + (peak_long_run_km - input.longest_recent_run_km) * ramp_t.min(0.92);

            // Cutback (recovery) week every 4th week, but never the very last ramp week
            let is_last_ramp_week = i == ramp_weeks.saturating_sub(1);
            if (week_number % 4 == 0) && !is_last_ramp_week {
                target *= 0.75;
                long_run *= 0.80;
            }

            // Introduce back-to-back long weekends in the last stretch of Build and all of Peak
            let in_specificity_window = phase == "Peak"
                || (phase == "Build" && i + 2 >= base_weeks + build_weeks);
            let back_to_back = in_specificity_window
                && days_per_week >= 4
                && !((week_number % 4 == 0) && !is_last_ramp_week);

            (target, long_run, back_to_back)
        };

        let days = build_week_days(
            week_start,
            phase,
            days_per_week,
            target_km,
            long_run_km,
            back_to_back,
            input.race_date,
        );

        weeks.push(WeekPlan {
            week_number,
            phase: phase.to_string(),
            start_date: week_start,
            target_km,
            long_run_km,
            days,
        });
    }

    if let Some(race_week_idx) = weeks
        .iter()
        .position(|w| w.days.iter().any(|d| matches!(d.kind, WorkoutKind::RaceDay)))
    {
        weeks.truncate(race_week_idx + 1);
    }
    let total_weeks = weeks.len() as u32;

    TrainingPlan {
        weeks,
        total_weeks,
        race_date: input.race_date,
        peak_weekly_km,
        peak_long_run_km,
        compressed_warning,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_week_days(
    week_start: NaiveDate,
    phase: &str,
    days_per_week: u8,
    target_km: f32,
    long_run_km: f32,
    back_to_back: bool,
    race_date: NaiveDate,
) -> Vec<DayPlan> {
    // weekday indices: 0 Mon ... 6 Sun
    // Priority order in which running days get added as days_per_week grows.
    let priority: [usize; 7] = [6, 3, 5, 1, 2, 4, 0]; // Sun, Thu, Sat, Tue, Wed, Fri, Mon
    let mut selected: Vec<usize> = priority
        .iter()
        .take(days_per_week as usize)
        .copied()
        .collect();
    selected.sort_unstable();

    let sunday_date = week_start + Duration::days(weekday_index(Weekday::Sun) as i64);
    let race_week = sunday_date >= race_date - Duration::days(6) && sunday_date <= race_date;

    let mut days = Vec::with_capacity(7);
    let weekday_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    // Figure out how much of target_km is "free" to spread across non-long days
    let saturday_km = if back_to_back {
        (long_run_km * 0.6).round()
    } else {
        0.0
    };
    let allocated_long = long_run_km + saturday_km;
    let remaining_km = (target_km - allocated_long).max(0.0);
    let non_long_running_days = selected
        .iter()
        .filter(|&&d| d != 6 && !(back_to_back && d == 5))
        .count()
        .max(1);
    let per_easy_day = remaining_km / non_long_running_days as f32;

    for d in 0..7 {
        let date = week_start + Duration::days(d as i64);

        if date > race_date {
            days.push(DayPlan {
                date,
                weekday: weekday_names[d].to_string(),
                kind: WorkoutKind::Rest,
                distance_km: None,
                label: "Recovery".to_string(),
                notes: "Post-race recovery. Rest, eat well, and let your legs come back before you think about the next run.".to_string(),
            });
            continue;
        }

        // Race day override
        if date == race_date {
            days.push(DayPlan {
                date,
                weekday: weekday_names[d].to_string(),
                kind: WorkoutKind::RaceDay,
                distance_km: Some(50.0),
                label: WorkoutKind::RaceDay.label().to_string(),
                notes: "This is it. Trust the training, start conservative, fuel early and often, and hike the climbs.".to_string(),
            });
            continue;
        }

        if !selected.contains(&d) {
            days.push(DayPlan {
                date,
                weekday: weekday_names[d].to_string(),
                kind: WorkoutKind::Rest,
                distance_km: None,
                label: "Rest".to_string(),
                notes: "Full rest or gentle stretching/mobility.".to_string(),
            });
            continue;
        }

        let (kind, distance, notes): (WorkoutKind, f32, &str) = match d {
            6 => {
                // Sunday - the long run, unless it's race week (handled by ShakeOut logic below)
                if race_week {
                    (
                        WorkoutKind::ShakeOut,
                        4.0,
                        "Short, easy shake-out jog. Keep it relaxed.",
                    )
                } else if back_to_back {
                    (
                        WorkoutKind::BackToBackLong,
                        long_run_km,
                        "Second long run of the weekend, on tired legs on purpose \u{2014} this is race-specificity work.",
                    )
                } else {
                    (
                        WorkoutKind::Long,
                        long_run_km,
                        "Long, slow run. Practice race-day fueling and gear.",
                    )
                }
            }
            5 if back_to_back => (
                WorkoutKind::MediumLong,
                saturday_km,
                "Moderate effort. Sets you up tired for tomorrow's long run.",
            ),
            3 => {
                // Thursday - the quality day
                if phase == "Base" {
                    (WorkoutKind::Hills, per_easy_day.max(6.0), "Rolling hills or hill repeats at a strong, controlled effort.")
                } else if phase == "Taper" {
                    (WorkoutKind::Easy, per_easy_day.max(5.0), "Easy effort, a few strides to stay sharp.")
                } else {
                    (WorkoutKind::Tempo, per_easy_day.max(6.0), "Tempo effort in the middle, easy warm-up/cool-down.")
                }
            }
            1 => (
                WorkoutKind::Easy,
                per_easy_day.max(5.0),
                "Conversational pace. This run is about time on feet, not speed.",
            ),
            2 => (
                WorkoutKind::Strides,
                per_easy_day.max(5.0),
                "Easy running with 4-6 x 20s relaxed strides at the end.",
            ),
            4 => (
                WorkoutKind::Easy,
                per_easy_day.max(5.0),
                "Easy trail or road run, whatever surface you'll race on if possible.",
            ),
            0 => (
                WorkoutKind::CrossTrain,
                0.0,
                "Optional: swim, bike, or yoga. Keep it low-impact.",
            ),
            _ => (WorkoutKind::Easy, per_easy_day.max(5.0), "Easy running."),
        };

        let distance_km = if matches!(kind, WorkoutKind::CrossTrain | WorkoutKind::Strength) {
            None
        } else {
            Some(distance)
        };

        days.push(DayPlan {
            date,
            weekday: weekday_names[d].to_string(),
            kind,
            distance_km,
            label: kind.label().to_string(),
            notes: notes.to_string(),
        });
    }

    days
}
