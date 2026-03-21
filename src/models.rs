use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub date: String,
    pub distance_km: f64,
    pub duration_seconds: u64,
    pub pace_per_km: String,
    pub sport_type: String,
    pub calories: u32,
    #[serde(skip_serializing)]
    pub label_id: String,
    #[serde(skip_serializing)]
    pub raw_sport_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsPoint {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRoute {
    pub tag: String,
    pub date: String,
    pub distance_km: f64,
    pub duration_seconds: u64,
    pub pace_per_km: String,
    pub sport_type: String,
    pub points: Vec<GpsPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub total_distance_km: f64,
    pub total_runs: u32,
    pub avg_pace: String,
    pub total_duration_seconds: u64,
    pub longest_run_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapEntry {
    pub date: String,
    pub distance_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacePrediction {
    pub race: String,
    pub duration_seconds: u64,
    pub avg_pace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningScores {
    pub aerobic_endurance: Option<f64>,
    pub lactate_threshold: Option<f64>,
    pub anaerobic_endurance: Option<f64>,
    pub anaerobic_capacity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardInfo {
    pub running_level: Option<f64>,
    pub scores: RunningScores,
    pub resting_hr: Option<u32>,
    pub threshold_hr: Option<u32>,
    pub threshold_pace: Option<String>,
    pub recovery_pct: Option<f64>,
    pub race_predictions: Vec<RacePrediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalBest {
    pub distance: String,
    pub time: String,
    pub pace: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningOutput {
    pub summary: RunSummary,
    pub heatmap: Vec<HeatmapEntry>,
    pub activities: Vec<Activity>,
    pub dashboard: Option<DashboardInfo>,
    pub personal_bests: Vec<PersonalBest>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub highlight_routes: Vec<HighlightRoute>,
    pub last_synced: String,
}

pub fn format_pace(seconds_per_km: f64) -> String {
    if seconds_per_km <= 0.0 || seconds_per_km.is_infinite() || seconds_per_km.is_nan() {
        return "N/A".to_string();
    }
    let minutes = (seconds_per_km / 60.0).floor() as u32;
    let secs = (seconds_per_km % 60.0).round() as u32;
    format!("{minutes}'{secs:02}\"")
}

fn sport_type_name(sport_type: u32) -> &'static str {
    match sport_type {
        100 => "outdoor_run",
        101 => "indoor_run",
        102 => "trail_run",
        103 => "track_run",
        _ => "run",
    }
}

pub fn build_output(
    raw_activities: &[RawActivity],
    dashboard: Option<DashboardInfo>,
) -> RunningOutput {
    let mut activities: Vec<Activity> = raw_activities
        .iter()
        .map(|raw| {
            let distance_km = raw.distance / 1000.0;

            // avg_speed from COROS is pace in seconds/km based on moving time.
            // Use it to derive actual moving duration (excluding pauses at red lights, etc.).
            let (duration_seconds, pace_seconds_per_km) = if raw.avg_speed > 0.0 && distance_km > 0.0 {
                let moving_secs = (raw.avg_speed * distance_km).round() as u64;
                (moving_secs, raw.avg_speed)
            } else {
                let elapsed = raw.elapsed_time as u64;
                let pace = if distance_km > 0.0 { elapsed as f64 / distance_km } else { 0.0 };
                (elapsed, pace)
            };

            Activity {
                date: format_timestamp(raw.start_time),
                distance_km: (distance_km * 100.0).round() / 100.0,
                duration_seconds,
                pace_per_km: format_pace(pace_seconds_per_km),
                sport_type: sport_type_name(raw.sport_type).to_string(),
                calories: (raw.calorie / 1000.0).round() as u32,
                label_id: raw.label_id.clone(),
                raw_sport_type: raw.sport_type,
            }
        })
        .collect();

    activities.sort_by(|a, b| b.date.cmp(&a.date));

    let summary = compute_summary(&activities);
    let heatmap = build_heatmap(&activities);
    let personal_bests = compute_personal_bests(&activities);
    let last_synced = chrono::Utc::now().to_rfc3339();

    RunningOutput {
        summary,
        heatmap,
        activities,
        dashboard,
        personal_bests,
        highlight_routes: Vec::new(),
        last_synced,
    }
}

fn compute_personal_bests(activities: &[Activity]) -> Vec<PersonalBest> {
    let targets = [
        ("1K", 1.0, 0.1),
        ("3K", 3.0, 0.2),
        ("5K", 5.0, 0.3),
        ("10K", 10.0, 0.5),
        ("Half Marathon", 21.0975, 1.0),
        ("Marathon", 42.195, 2.0),
    ];

    let mut bests = Vec::new();
    for &(name, target_km, tolerance) in &targets {
        let best = activities
            .iter()
            .filter(|a| (a.distance_km - target_km).abs() <= tolerance)
            .min_by_key(|a| a.duration_seconds);

        if let Some(a) = best {
            let hours = a.duration_seconds / 3600;
            let mins = (a.duration_seconds % 3600) / 60;
            let secs = a.duration_seconds % 60;
            let time_str = if hours > 0 {
                format!("{}:{:02}:{:02}", hours, mins, secs)
            } else {
                format!("{}:{:02}", mins, secs)
            };

            bests.push(PersonalBest {
                distance: name.to_string(),
                time: time_str,
                pace: a.pace_per_km.clone(),
                date: a.date.clone(),
            });
        }
    }
    bests
}

fn compute_summary(activities: &[Activity]) -> RunSummary {
    let total_runs = activities.len() as u32;
    let total_distance_km: f64 = activities.iter().map(|a| a.distance_km).sum();
    let total_duration_seconds: u64 = activities.iter().map(|a| a.duration_seconds).sum();
    let longest_run_km = activities
        .iter()
        .map(|a| a.distance_km)
        .fold(0.0_f64, f64::max);

    let avg_pace = if total_distance_km > 0.0 {
        format_pace(total_duration_seconds as f64 / total_distance_km)
    } else {
        "N/A".to_string()
    };

    RunSummary {
        total_distance_km: (total_distance_km * 100.0).round() / 100.0,
        total_runs,
        avg_pace,
        total_duration_seconds,
        longest_run_km: (longest_run_km * 100.0).round() / 100.0,
    }
}

fn build_heatmap(activities: &[Activity]) -> Vec<HeatmapEntry> {
    let today = chrono::Utc::now().date_naive();
    let one_year_ago = today - chrono::Duration::days(365);

    let mut date_map: BTreeMap<String, f64> = BTreeMap::new();
    for activity in activities {
        if let Ok(date) = NaiveDate::parse_from_str(&activity.date, "%Y-%m-%d") {
            if date >= one_year_ago && date <= today {
                *date_map.entry(activity.date.clone()).or_insert(0.0) += activity.distance_km;
            }
        }
    }

    date_map
        .into_iter()
        .map(|(date, distance_km)| HeatmapEntry {
            date,
            distance_km: (distance_km * 100.0).round() / 100.0,
        })
        .collect()
}

fn format_timestamp(timestamp_secs: u64) -> String {
    chrono::DateTime::from_timestamp(timestamp_secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Picks a small set of highlight activities worth fetching route maps for.
/// Returns `(tag, activity_index)` pairs, deduplicated by label_id.
pub fn select_highlights(activities: &[Activity]) -> Vec<(String, usize)> {
    use std::collections::HashSet;

    let outdoor: Vec<(usize, &Activity)> = activities
        .iter()
        .enumerate()
        .filter(|(_, a)| a.sport_type == "outdoor_run" || a.sport_type == "trail_run")
        .collect();

    if outdoor.is_empty() {
        return Vec::new();
    }

    let mut picks: Vec<(String, usize)> = Vec::new();
    let mut seen = HashSet::new();

    let mut try_add = |tag: &str, idx: usize| {
        if seen.insert(activities[idx].label_id.clone()) {
            picks.push((tag.to_string(), idx));
        }
    };

    // Longest distance
    if let Some(&(idx, _)) = outdoor.iter().max_by(|a, b| {
        a.1.distance_km
            .partial_cmp(&b.1.distance_km)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        try_add("longest", idx);
    }

    // Fastest pace (only runs >= 3 km to avoid warm-up jogs)
    if let Some(&(idx, _)) = outdoor
        .iter()
        .filter(|(_, a)| a.distance_km >= 3.0)
        .min_by_key(|(_, a)| {
            ((a.duration_seconds as f64 / a.distance_km) * 1000.0) as u64
        })
    {
        try_add("fastest", idx);
    }

    // PB for standard race distances
    let pb_targets = [
        ("pb_5k", 5.0, 0.3),
        ("pb_10k", 10.0, 0.5),
        ("pb_half", 21.0975, 1.0),
    ];
    for (tag, target_km, tolerance) in pb_targets {
        if let Some(&(idx, _)) = outdoor
            .iter()
            .filter(|(_, a)| (a.distance_km - target_km).abs() <= tolerance)
            .min_by_key(|(_, a)| a.duration_seconds)
        {
            try_add(tag, idx);
        }
    }

    // Most recent outdoor run
    if let Some(&(idx, _)) = outdoor.first() {
        try_add("recent", idx);
    }

    picks
}

/// Raw activity data as returned by the COROS API.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RawActivity {
    pub label_id: String,
    pub sport_type: u32,
    pub start_time: u64,
    pub distance: f64,
    /// Wall-clock elapsed time (includes pauses).
    pub elapsed_time: f64,
    /// Average pace in seconds/km (based on moving time, excludes pauses).
    pub avg_speed: f64,
    pub calorie: f64,
}
