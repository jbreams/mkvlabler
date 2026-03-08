use gloo_net::http::Request;
use serde::de::DeserializeOwned;

use crate::types::*;

async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    Request::get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<T>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn scan_dir(dir: &str) -> Result<ScanResponse, String> {
    let url = format!("/api/scan?dir={}", urlencoding::encode(dir));
    get_json(&url).await
}

pub async fn search_tvmaze(query: &str) -> Result<Vec<ShowResult>, String> {
    #[derive(serde::Deserialize)]
    struct Resp {
        results: Vec<ShowResult>,
    }
    let url = format!("/api/tvmaze/search?q={}", urlencoding::encode(query));
    let r: Resp = get_json(&url).await?;
    Ok(r.results)
}

pub async fn fetch_episodes(show_id: u64, season: Option<u64>) -> Result<Vec<EpisodeResult>, String> {
    #[derive(serde::Deserialize)]
    struct Resp {
        episodes: Vec<EpisodeResult>,
    }
    let mut url = format!("/api/tvmaze/episodes?id={show_id}");
    if let Some(s) = season {
        url.push_str(&format!("&season={s}"));
    }
    let r: Resp = get_json(&url).await?;
    Ok(r.episodes)
}

pub async fn search_dvdcompare(query: &str) -> Result<Vec<DvdSearchResult>, String> {
    #[derive(serde::Deserialize)]
    struct Resp {
        results: Vec<DvdSearchResult>,
    }
    let url = format!("/api/dvdcompare/search?q={}", urlencoding::encode(query));
    let r: Resp = get_json(&url).await?;
    Ok(r.results)
}

pub async fn fetch_dvd_disc(compid: &str) -> Result<Vec<DvdFeature>, String> {
    #[derive(serde::Deserialize)]
    struct Resp {
        features: Vec<DvdFeature>,
    }
    let url = format!("/api/dvdcompare/disc?compid={}", urlencoding::encode(compid));
    let r: Resp = get_json(&url).await?;
    Ok(r.features)
}

pub async fn apply_renames(mappings: Vec<RenameMapping>) -> Result<Vec<RenameResult>, String> {
    #[derive(serde::Deserialize)]
    struct Resp {
        results: Vec<RenameResult>,
    }
    let body = serde_json::json!({ "mappings": mappings });
    let r: Resp = Request::post("/api/rename")
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(r.results)
}

pub fn preview_url(path: &str, start: f64, duration: f64) -> String {
    format!(
        "/api/preview?path={}&start={start}&duration={duration}",
        urlencoding::encode(path)
    )
}
