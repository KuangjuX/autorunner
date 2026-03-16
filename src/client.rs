use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::models::RawActivity;

const LOGIN_URL: &str = "https://teamcnapi.coros.com/account/login";
const ACTIVITY_LIST_URL: &str = "https://teamcnapi.coros.com/activity/query";
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
                    duration: item.total_time.unwrap_or(0.0),
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
}
