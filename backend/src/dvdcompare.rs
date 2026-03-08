use axum::{extract::Query, Json};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::collections::HashSet;
use tracing::debug;

use crate::error::AppError;

#[derive(Debug, Snafu)]
pub enum DvdCompareError {
    #[snafu(display("HTTP request failed: {source}"))]
    Http { source: reqwest::Error },

    #[snafu(display("Failed to read response body: {source}"))]
    Body { source: reqwest::Error },
}

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
}

#[derive(Deserialize)]
pub struct DiscParams {
    compid: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub compid: String,
    pub title: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct Feature {
    pub title: String,
    pub timecodes: Vec<String>,
}

#[derive(Serialize)]
pub struct DiscResponse {
    pub compid: String,
    pub title: String,
    pub features: Vec<Feature>,
    pub url: String,
}

pub async fn search_handler(
    Query(params): Query<SearchParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let results =
        search_dvdcompare(&params.q).await.map_err(|e| AppError::internal(e))?;
    Ok(Json(serde_json::json!({ "results": results })))
}

pub async fn disc_handler(
    Query(params): Query<DiscParams>,
) -> Result<Json<DiscResponse>, AppError> {
    if params.compid.is_empty() {
        return Err(AppError::bad_request("Missing compid"));
    }
    let disc = fetch_disc(&params.compid).await.map_err(|e| AppError::internal(e))?;
    Ok(Json(disc))
}

async fn search_dvdcompare(query: &str) -> Result<Vec<SearchResult>, DvdCompareError> {
    debug!(query, "DVDCompare search");
    let url = format!(
        "https://www.dvdcompare.net/comparisons/search.php?title={}",
        urlencoding::encode(query)
    );

    let html = reqwest::Client::new()
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await
        .context(HttpSnafu)?
        .text()
        .await
        .context(BodySnafu)?;

    Ok(parse_search_results(&html))
}

fn parse_search_results(html: &str) -> Vec<SearchResult> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("a[href*='comparisons/film.php']") else {
        return vec![];
    };

    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for el in document.select(&selector) {
        let href = el.attr("href").unwrap_or("");
        let compid = href.split("compid=").nth(1).unwrap_or("").to_string();
        if compid.is_empty() || seen.contains(&compid) {
            continue;
        }
        let title = el.text().collect::<String>().trim().to_string();
        if title.is_empty() {
            continue;
        }
        seen.insert(compid.clone());
        results.push(SearchResult {
            url: format!("https://www.dvdcompare.net{href}"),
            compid,
            title,
        });
        if results.len() >= 20 {
            break;
        }
    }

    results
}

async fn fetch_disc(compid: &str) -> Result<DiscResponse, DvdCompareError> {
    debug!(compid, "DVDCompare disc fetch");
    let url = format!(
        "https://www.dvdcompare.net/comparisons/film.php?compid={compid}"
    );

    let html = reqwest::Client::new()
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await
        .context(HttpSnafu)?
        .text()
        .await
        .context(BodySnafu)?;

    Ok(parse_disc(&html, compid))
}

fn parse_disc(html: &str, compid: &str) -> DiscResponse {
    let base_url =
        format!("https://www.dvdcompare.net/comparisons/film.php?compid={compid}");

    let document = Html::parse_document(html);

    let title = Selector::parse("title")
        .ok()
        .and_then(|s| document.select(&s).next())
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let timecode_re =
        regex::Regex::new(r"\b\d{1,2}:\d{2}(?::\d{2})?\b").expect("valid regex");

    let Ok(row_sel) = Selector::parse("tr") else {
        return DiscResponse { compid: compid.to_string(), title, features: vec![], url: base_url };
    };
    let Ok(cell_sel) = Selector::parse("td, th") else {
        return DiscResponse { compid: compid.to_string(), title, features: vec![], url: base_url };
    };

    let mut features = Vec::new();

    for row in document.select(&row_sel) {
        let cells: Vec<String> = row
            .select(&cell_sel)
            .map(|c| c.text().collect::<String>().trim().to_string())
            .collect();

        let row_text = cells.join(" ");
        let timecodes: Vec<String> = timecode_re
            .find_iter(&row_text)
            .map(|m| m.as_str().to_string())
            .collect();

        if timecodes.is_empty() || cells.len() < 2 {
            continue;
        }

        if let Some(feat_title) = cells
            .iter()
            .find(|c| !c.is_empty() && !timecode_re.is_match(c) && c.len() > 2)
        {
            features.push(Feature { title: feat_title.clone(), timecodes });
        }

        if features.len() >= 50 {
            break;
        }
    }

    DiscResponse { compid: compid.to_string(), title, features, url: base_url }
}
