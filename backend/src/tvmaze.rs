use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tracing::debug;

use crate::error::AppError;

#[derive(Debug, Snafu)]
pub enum TvmazeError {
    #[snafu(display("HTTP request failed: {source}"))]
    Http { source: reqwest::Error },

    #[snafu(display("Failed to deserialize response: {source}"))]
    Deserialize { source: reqwest::Error },
}

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
}

#[derive(Deserialize)]
pub struct EpisodesParams {
    id: u64,
    season: Option<u64>,
}

#[derive(Serialize)]
pub struct ShowResult {
    pub id: u64,
    pub name: String,
    pub year: String,
    pub summary: String,
    pub source: &'static str,
}

#[derive(Serialize)]
pub struct EpisodeResult {
    pub season: u64,
    pub episode: u64,
    pub title: String,
    pub runtime: Option<u64>,
    pub airdate: String,
    pub label: String,
    pub filename_stem: String,
}

pub async fn search_handler(
    Query(params): Query<SearchParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let results = search_shows(&params.q).await.map_err(|e| AppError::internal(e))?;
    Ok(Json(serde_json::json!({ "results": results })))
}

pub async fn episodes_handler(
    Query(params): Query<EpisodesParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let episodes =
        fetch_episodes(params.id, params.season).await.map_err(|e| AppError::internal(e))?;
    Ok(Json(serde_json::json!({ "episodes": episodes })))
}

async fn search_shows(query: &str) -> Result<Vec<ShowResult>, TvmazeError> {
    debug!(query, "TVmaze show search");
    let url = format!(
        "https://api.tvmaze.com/search/shows?q={}",
        urlencoding::encode(query)
    );

    let resp: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "mkvlabel/1.0")
        .send()
        .await
        .context(HttpSnafu)?
        .json()
        .await
        .context(DeserializeSnafu)?;

    let shows = resp
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .take(5)
        .filter_map(|r| {
            let show = r["show"].as_object()?;
            Some(ShowResult {
                id: show["id"].as_u64()?,
                name: show["name"].as_str().unwrap_or("").to_string(),
                year: show["premiered"].as_str().unwrap_or("").chars().take(4).collect(),
                summary: strip_html(show["summary"].as_str().unwrap_or(""))
                    .chars()
                    .take(100)
                    .collect(),
                source: "tvmaze",
            })
        })
        .collect();

    Ok(shows)
}

async fn fetch_episodes(
    show_id: u64,
    season: Option<u64>,
) -> Result<Vec<EpisodeResult>, TvmazeError> {
    debug!(show_id, ?season, "TVmaze episode fetch");
    let url = format!("https://api.tvmaze.com/shows/{show_id}/episodes");

    let resp: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "mkvlabel/1.0")
        .send()
        .await
        .context(HttpSnafu)?
        .json()
        .await
        .context(DeserializeSnafu)?;

    let episodes = resp
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|ep| {
            let s = ep["season"].as_u64()?;
            let e = ep["number"].as_u64()?;
            if let Some(filter) = season {
                if s != filter {
                    return None;
                }
            }
            let title = ep["name"].as_str().unwrap_or("").to_string();
            let label = format!("S{s:02}E{e:02} - {title}");
            Some(EpisodeResult {
                season: s,
                episode: e,
                title,
                runtime: ep["runtime"].as_u64(),
                airdate: ep["airdate"].as_str().unwrap_or("").to_string(),
                filename_stem: label.clone(),
                label,
            })
        })
        .collect();

    Ok(episodes)
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}
