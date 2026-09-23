use anyhow::{ensure, Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, time::Duration};

const SEARCH_URL: &str = "https://ievent.life/API/Event/searchV2";
const PAGE_LIMIT: usize = 10;

#[derive(Deserialize)]
struct SearchResponse {
    status: String,
    data: SearchData,
}

#[derive(Deserialize)]
struct SearchData {
    total: usize,
    events: Vec<RemoteEvent>,
}

#[derive(Deserialize)]
struct RemoteEvent {
    event_id: u64,
    title: String,
    #[serde(default)]
    locations: Vec<RemoteLocation>,
}

#[derive(Deserialize)]
struct RemoteLocation {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSuggestion {
    pub title: String,
    pub venue: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSuggestions {
    pub items: Vec<EventSuggestion>,
    pub total: usize,
    pub truncated: bool,
}

fn checked_date(date: &str) -> Result<()> {
    let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").context("请先填写有效的收藏日期")?;
    ensure!(
        parsed.format("%Y-%m-%d").to_string() == date,
        "请先填写有效的收藏日期"
    );
    Ok(())
}

pub async fn search(date: &str) -> Result<EventSuggestions> {
    checked_date(date)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut total = 0;
    for page in 1..=PAGE_LIMIT {
        let response: SearchResponse = client
            .get(SEARCH_URL)
            .query(&[
                ("startDate", date.to_string()),
                ("endDate", date.to_string()),
                ("page", page.to_string()),
                ("sort", "desc".to_string()),
            ])
            .send()
            .await
            .context("无法连接推活日记")?
            .error_for_status()
            .context("推活日记检索失败")?
            .json()
            .await
            .context("无法读取推活日记的活动结果")?;
        ensure!(response.status == "success", "推活日记检索失败");
        total = response.data.total;
        let count = response.data.events.len();
        for event in response.data.events {
            if seen.insert(event.event_id) && !event.title.trim().is_empty() {
                items.push(EventSuggestion {
                    title: event.title.trim().to_string(),
                    venue: event
                        .locations
                        .first()
                        .map(|l| l.name.trim())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
        if count == 0 || page * 20 >= total || items.len() >= total {
            break;
        }
    }
    let truncated = items.len() < total;
    Ok(EventSuggestions {
        items,
        total,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_complete_calendar_dates_and_site_response() {
        assert!(checked_date("2026-07-18").is_ok());
        assert!(checked_date("2026-02-30").is_err());
        assert!(checked_date("2026-7-18").is_err());
        let response: SearchResponse = serde_json::from_str(r#"{
            "status":"success","data":{"total":1,"events":[
                {"event_id":10061,"title":"心跳闪耀","date":"2026-07-19","locations":[{"name":"所有人 Livehouse"}]}
            ]}}
        "#).unwrap();
        assert_eq!(response.data.events[0].title, "心跳闪耀");
        assert_eq!(
            response.data.events[0].locations[0].name,
            "所有人 Livehouse"
        );
    }
}
