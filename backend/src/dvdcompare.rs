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

    // The site uses a POST form with `param` + `searchtype` fields.
    let html = reqwest::Client::new()
        .post("https://www.dvdcompare.net/comparisons/search.php")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .form(&[("param", query), ("searchtype", "text")])
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
    // Links use relative hrefs like `film.php?fid=123`
    let Ok(selector) = Selector::parse("a[href*='film.php?fid=']") else {
        return vec![];
    };

    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for el in document.select(&selector) {
        let href = el.attr("href").unwrap_or("");
        let fid = href.split("fid=").nth(1).unwrap_or("").to_string();
        if fid.is_empty() || seen.contains(&fid) {
            continue;
        }
        // Collapse whitespace (titles span multiple text nodes)
        let title = el
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if title.is_empty() {
            continue;
        }
        seen.insert(fid.clone());
        results.push(SearchResult {
            url: format!("https://www.dvdcompare.net/comparisons/film.php?fid={fid}"),
            compid: fid,
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
    let url = format!("https://www.dvdcompare.net/comparisons/film.php?fid={compid}");

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

    Ok(parse_disc(&html, compid, &url))
}

fn parse_disc(html: &str, compid: &str, url: &str) -> DiscResponse {
    let document = Html::parse_document(html);

    let title = Selector::parse("title")
        .ok()
        .and_then(|s| document.select(&s).next())
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    // Timecodes appear as (M:SS) or (H:MM:SS) in parentheses
    let timecode_re =
        regex::Regex::new(r"\((\d{1,2}:\d{2}(?::\d{2})?)\)").expect("valid regex");

    let Ok(desc_sel) = Selector::parse("div.description") else {
        return DiscResponse {
            compid: compid.to_string(),
            title,
            features: vec![],
            url: url.to_string(),
        };
    };

    // Strip HTML tags from a string by parsing it as a fragment
    let strip_tags = |s: &str| -> String {
        Html::parse_fragment(s)
            .root_element()
            .text()
            .collect::<String>()
    };

    let mut features: Vec<Feature> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    'outer: for desc in document.select(&desc_sel) {
        // inner_html() returns raw HTML; <br> elements separate lines.
        // html5ever normalises XHTML <br /> → <br> in the DOM.
        let inner = desc.inner_html();

        for raw_line in inner.split("<br>") {
            let text = strip_tags(raw_line);
            let text = text.trim();
            if text.is_empty() {
                continue;
            }

            let timecodes: Vec<String> = timecode_re
                .captures_iter(text)
                .map(|c| c[1].to_string())
                .collect();
            if timecodes.is_empty() {
                continue;
            }

            // Remove the timecode(s) from the string to isolate the title
            let feat_title = timecode_re
                .replace_all(text, "")
                .trim_matches(|c: char| {
                    matches!(c, '-' | '*' | '"' | '\'' | '(' | ')') || c.is_whitespace()
                })
                .trim_end_matches(':')
                .to_string();

            if feat_title.is_empty() || seen.contains(&feat_title) {
                continue;
            }
            seen.insert(feat_title.clone());
            features.push(Feature { title: feat_title, timecodes });

            if features.len() >= 100 {
                break 'outer;
            }
        }
    }

    DiscResponse { compid: compid.to_string(), title, features, url: url.to_string() }
}
