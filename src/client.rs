use anyhow::{bail, Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use serde::Deserialize;

use crate::models::{GpsPoint, RawActivity};

const LOGIN_URL: &str = "https://teamcnapi.coros.com/account/login";
const ACTIVITY_LIST_URL: &str = "https://teamcnapi.coros.com/activity/query";
const ACTIVITY_DOWNLOAD_URL: &str = "https://teamcnapi.coros.com/activity/detail/download";
const DASHBOARD_URL: &str = "https://teamcnapi.coros.com/dashboard/query";
const PAGE_SIZE: u32 = 20;
/// Only running-related sport types: outdoor(100), indoor(101), trail(102), track(103)
const RUNNING_MODE_LIST: &str = "100,101,102,103";

pub struct CorosClient {
    http: Client,
    access_token: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    data: Option<LoginData>,
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginData {
    access_token: String,
}

#[derive(Deserialize)]
struct ActivityListResponse {
    data: Option<ActivityListData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityListData {
    data_list: Option<Vec<ActivityItem>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityItem {
    label_id: Option<serde_json::Value>,
    sport_type: Option<u32>,
    start_time: Option<u64>,
    distance: Option<f64>,
    total_time: Option<f64>,
    avg_speed: Option<f64>,
    calorie: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct DashboardResponse {
    data: Option<DashboardData>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DashboardData {
    summary_info: Option<DashboardSummary>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DashboardSummary {
    pub stamina_level: Option<f64>,
    pub aerobic_endurance_score: Option<f64>,
    pub lactate_threshold_capacity_score: Option<f64>,
    pub anaerobic_endurance_score: Option<f64>,
    pub anaerobic_capacity_score: Option<f64>,
    pub rhr: Option<u32>,
    pub lthr: Option<u32>,
    pub ltsp: Option<f64>,
    pub recovery_pct: Option<f64>,
    pub run_score_list: Option<Vec<RaceScoreItem>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RaceScoreItem {
    #[serde(rename = "type")]
    pub race_type: Option<u32>,
    pub duration: Option<u64>,
    pub avg_pace: Option<f64>,
}

#[derive(Deserialize)]
struct DownloadResponse {
    data: Option<DownloadData>,
    message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadData {
    file_url: String,
}

impl CorosClient {
    pub async fn login(account: &str, password_md5: &str) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .context("Failed to build HTTP client")?;

        let body = serde_json::json!({
            "account": account,
            "accountType": 2,
            "pwd": password_md5,
        });

        let resp = http
            .post(LOGIN_URL)
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Origin", "https://t.coros.com")
            .header("Referer", "https://t.coros.com/")
            .json(&body)
            .send()
            .await
            .context("Failed to send login request")?;

        let login_resp: LoginResponse = resp.json().await.context("Failed to parse login response")?;

        let access_token = login_resp
            .data
            .map(|d| d.access_token)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                let msg = login_resp
                    .message
                    .unwrap_or_else(|| "unknown error".to_string());
                anyhow::anyhow!("Login failed: {msg}")
            })?;

        Ok(Self { http, access_token })
    }

    pub async fn fetch_all_running_activities(&self) -> Result<Vec<RawActivity>> {
        let mut all_activities = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{ACTIVITY_LIST_URL}?modeList={RUNNING_MODE_LIST}&pageNumber={page}&size={PAGE_SIZE}"
            );

            let resp = self
                .http
                .get(&url)
                .header("accesstoken", &self.access_token)
                .header(
                    "cookie",
                    format!(
                        "CPL-coros-region=2; CPL-coros-token={}",
                        self.access_token
                    ),
                )
                .send()
                .await
                .with_context(|| format!("Failed to fetch activity page {page}"))?;

            let list_resp: ActivityListResponse = resp
                .json()
                .await
                .with_context(|| format!("Failed to parse activity page {page}"))?;

            let items = match list_resp.data.and_then(|d| d.data_list) {
                Some(items) if !items.is_empty() => items,
                _ => break,
            };

            for item in &items {
                let label_id = match &item.label_id {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    _ => continue,
                };

                all_activities.push(RawActivity {
                    label_id,
                    sport_type: item.sport_type.unwrap_or(100),
                    start_time: item.start_time.unwrap_or(0),
                    distance: item.distance.unwrap_or(0.0),
                    elapsed_time: item.total_time.unwrap_or(0.0),
                    avg_speed: item.avg_speed.unwrap_or(0.0),
                    calorie: item.calorie.unwrap_or(0.0),
                });
            }

            if items.len() < PAGE_SIZE as usize {
                break;
            }
            page += 1;
        }

        if all_activities.is_empty() {
            bail!("No running activities found");
        }

        println!("Fetched {} running activities", all_activities.len());
        Ok(all_activities)
    }

    pub async fn fetch_dashboard(&self) -> Result<crate::models::DashboardInfo> {
        let resp = self
            .http
            .get(DASHBOARD_URL)
            .header("accesstoken", &self.access_token)
            .header(
                "cookie",
                format!("CPL-coros-region=2; CPL-coros-token={}", self.access_token),
            )
            .send()
            .await
            .context("Failed to fetch dashboard")?;

        let dashboard: DashboardResponse = resp
            .json()
            .await
            .context("Failed to parse dashboard response")?;

        let summary = dashboard
            .data
            .and_then(|d| d.summary_info)
            .unwrap_or_else(|| {
                println!("Warning: dashboard summary is empty");
                DashboardSummary {
                    stamina_level: None,
                    aerobic_endurance_score: None,
                    lactate_threshold_capacity_score: None,
                    anaerobic_endurance_score: None,
                    anaerobic_capacity_score: None,
                    rhr: None,
                    lthr: None,
                    ltsp: None,
                    recovery_pct: None,
                    run_score_list: None,
                }
            });

        let race_predictions: Vec<crate::models::RacePrediction> = summary
            .run_score_list
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                let race_type = r.race_type?;
                let duration_seconds = r.duration?;
                let name = match race_type {
                    5 => "5K",
                    4 => "10K",
                    2 => "Half Marathon",
                    1 => "Marathon",
                    _ => return None,
                };
                Some(crate::models::RacePrediction {
                    race: name.to_string(),
                    duration_seconds,
                    avg_pace: r
                        .avg_pace
                        .map(|p| crate::models::format_pace(p))
                        .unwrap_or_default(),
                })
            })
            .collect();

        let threshold_pace = summary
            .ltsp
            .filter(|&p| p > 0.0)
            .map(crate::models::format_pace);

        Ok(crate::models::DashboardInfo {
            running_level: summary.stamina_level,
            scores: crate::models::RunningScores {
                aerobic_endurance: summary.aerobic_endurance_score,
                lactate_threshold: summary.lactate_threshold_capacity_score,
                anaerobic_endurance: summary.anaerobic_endurance_score,
                anaerobic_capacity: summary.anaerobic_capacity_score,
            },
            resting_hr: summary.rhr,
            threshold_hr: summary.lthr,
            threshold_pace,
            recovery_pct: summary.recovery_pct,
            race_predictions,
        })
    }

    /// Download the GPX file for a single activity and parse it into GPS points.
    pub async fn fetch_activity_route(
        &self,
        label_id: &str,
        sport_type: u32,
    ) -> Result<Vec<GpsPoint>> {
        let resp = self
            .http
            .post(ACTIVITY_DOWNLOAD_URL)
            .header("accesstoken", &self.access_token)
            .header(
                "cookie",
                format!("CPL-coros-region=2; CPL-coros-token={}", self.access_token),
            )
            .query(&[
                ("labelId", label_id),
                ("sportType", &sport_type.to_string()),
                ("fileType", "1"), // GPX
            ])
            .send()
            .await
            .with_context(|| format!("Failed to request GPX download for {label_id}"))?;

        let dl: DownloadResponse = resp
            .json()
            .await
            .with_context(|| format!("Failed to parse download response for {label_id}"))?;

        let file_url = dl
            .data
            .map(|d| d.file_url)
            .filter(|u| !u.is_empty())
            .ok_or_else(|| {
                let msg = dl.message.unwrap_or_else(|| "unknown error".to_string());
                anyhow::anyhow!("GPX download failed for {label_id}: {msg}")
            })?;

        let gpx_bytes = self
            .http
            .get(&file_url)
            .send()
            .await
            .with_context(|| format!("Failed to download GPX file for {label_id}"))?
            .bytes()
            .await?;

        let points = parse_gpx_points(&gpx_bytes)?;
        Ok(points)
    }
}

/// Extract lat/lon track points from GPX XML, down-sampling to keep at most
/// ~200 points so the JSON stays compact.
fn parse_gpx_points(gpx_bytes: &[u8]) -> Result<Vec<GpsPoint>> {
    let mut reader = Reader::from_reader(gpx_bytes);
    reader.config_mut().trim_text(true);

    let mut points = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e))
                if e.name().as_ref() == b"trkpt" =>
            {
                let mut lat = None;
                let mut lon = None;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"lat" => {
                            lat = std::str::from_utf8(&attr.value)
                                .ok()
                                .and_then(|v| v.parse::<f64>().ok());
                        }
                        b"lon" => {
                            lon = std::str::from_utf8(&attr.value)
                                .ok()
                                .and_then(|v| v.parse::<f64>().ok());
                        }
                        _ => {}
                    }
                }
                if let (Some(lat), Some(lon)) = (lat, lon) {
                    points.push(GpsPoint { lat, lon });
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("Error parsing GPX: {e}"),
            _ => {}
        }
        buf.clear();
    }

    if points.len() <= 200 {
        return Ok(points);
    }

    let step = points.len() as f64 / 200.0;
    let mut sampled: Vec<GpsPoint> = (0..199)
        .map(|i| points[(i as f64 * step) as usize].clone())
        .collect();
    sampled.push(points.last().unwrap().clone());
    Ok(sampled)
}
