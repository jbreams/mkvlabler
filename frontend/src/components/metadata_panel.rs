use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::{
    state::{AppAction, AppContext},
    types::{ActiveTab, Mapping, MappingKind},
};

#[function_component(MetadataPanel)]
pub fn metadata_panel() -> Html {
    let ctx = use_context::<AppContext>().expect("AppContext missing");
    let search_ref = use_node_ref();
    let season_ref = use_node_ref();

    // ── Search ────────────────────────────────────────────────────────────────

    let on_search = {
        let ctx = ctx.clone();
        let search_ref = search_ref.clone();
        Callback::from(move |_| {
            let query = search_ref
                .cast::<HtmlInputElement>()
                .map(|el| el.value())
                .unwrap_or_default();
            if query.is_empty() {
                return;
            }

            // TVmaze search
            {
                let ctx2 = ctx.clone();
                let query = query.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    ctx2.dispatch(AppAction::SetStatus("Searching...".to_string()));
                    match crate::api::search_tvmaze(&query).await {
                        Ok(shows) => {
                            ctx2.dispatch(AppAction::SetTvShows(shows));
                            ctx2.dispatch(AppAction::SetStatus(String::new()));
                        }
                        Err(e) => ctx2.dispatch(AppAction::SetStatus(format!("TVmaze error: {e}"))),
                    }
                });
            }

            // DVDCompare search (runs in parallel)
            {
                let ctx2 = ctx.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match crate::api::search_dvdcompare(&query).await {
                        Ok(results) => ctx2.dispatch(AppAction::SetDvdResults(results)),
                        Err(e) => ctx2.dispatch(AppAction::SetStatus(format!("DVDCompare error: {e}"))),
                    }
                });
            }
        })
    };

    let on_load_eps = {
        let ctx = ctx.clone();
        let season_ref = season_ref.clone();
        Callback::from(move |_| {
            let Some(show_id) = ctx.selected_show else { return };
            let season = season_ref
                .cast::<HtmlInputElement>()
                .and_then(|el| el.value().parse::<u64>().ok());
            let ctx2 = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                ctx2.dispatch(AppAction::SetStatus("Loading episodes...".to_string()));
                match crate::api::fetch_episodes(show_id, season).await {
                    Ok(eps) => {
                        ctx2.dispatch(AppAction::SetEpisodes(eps));
                        ctx2.dispatch(AppAction::SetActiveTab(ActiveTab::Episodes));
                        ctx2.dispatch(AppAction::SetStatus(String::new()));
                    }
                    Err(e) => ctx2.dispatch(AppAction::SetStatus(format!("Episodes error: {e}"))),
                }
            });
        })
    };

    // ── Tab switching ─────────────────────────────────────────────────────────

    let make_tab = |tab: ActiveTab, label: &'static str| {
        let ctx = ctx.clone();
        let is_active = ctx.active_tab == tab;
        let cb = Callback::from(move |_| ctx.dispatch(AppAction::SetActiveTab(tab.clone())));
        html! {
            <div class={format!("tab{}", if is_active { " active" } else { "" })} onclick={cb}>
                { label }
            </div>
        }
    };

    // ── Tab content ───────────────────────────────────────────────────────────

    let tab_content = match ctx.active_tab {
        ActiveTab::Shows => render_shows(&ctx),
        ActiveTab::Episodes => render_episodes(&ctx),
        ActiveTab::Features => render_features(&ctx),
        ActiveTab::Dvd => render_dvd(&ctx),
    };

    html! {
        <div class="panel" style="border-right:none">
            <div class="search-section">
                <div class="search-row">
                    <input ref={search_ref} type="text" placeholder="Search show/movie title..." />
                    <button onclick={on_search}>{"Search"}</button>
                </div>
                <div class="season-row">
                    <label>{"Season:"}</label>
                    <input ref={season_ref} type="number" placeholder="1" min="1" />
                    <button class="small" onclick={on_load_eps}>{"Load eps"}</button>
                </div>
            </div>

            <div class="tab-bar">
                { make_tab(ActiveTab::Shows, "Shows") }
                { make_tab(ActiveTab::Episodes, "Episodes") }
                { make_tab(ActiveTab::Features, "Features") }
                { make_tab(ActiveTab::Dvd, "DVDCompare") }
            </div>

            <div class="metadata-results">
                { tab_content }
            </div>
        </div>
    }
}

fn render_shows(ctx: &AppContext) -> Html {
    if ctx.tv_shows.is_empty() {
        return html! {
            <div class="empty-state">
                {"Search for a show or movie"}<br/>{"to see episodes and features."}
            </div>
        };
    }

    ctx.tv_shows
        .iter()
        .map(|show| {
            let ctx = ctx.clone();
            let id = show.id;
            let is_active = ctx.selected_show == Some(id);
            let cb = Callback::from(move |_| ctx.dispatch(AppAction::SelectShow(Some(id))));
            html! {
                <div class={format!("result-item{}", if is_active { " active" } else { "" })} onclick={cb}>
                    <div class="result-title">{ &show.name }</div>
                    <div class="result-sub">
                        { format!("{} · {}", show.year, show.source) }
                    </div>
                    { if !show.summary.is_empty() {
                        html! { <div class="result-sub">{ &show.summary }</div> }
                    } else { html! {} }}
                </div>
            }
        })
        .collect()
}

fn render_episodes(ctx: &AppContext) -> Html {
    if ctx.episodes.is_empty() {
        return html! {
            <div class="empty-state">
                {"Select a show and click"}<br/>{"\"Load eps\" to fetch episodes."}
            </div>
        };
    }

    ctx.episodes
        .iter()
        .map(|ep| {
            let ctx = ctx.clone();
            let label = ep.label.clone();
            let filename_stem = ep.filename_stem.clone();
            let already_used = ctx.mappings.values().any(|m| m.label == label);

            let cb = {
                let ctx = ctx.clone();
                Callback::from(move |_| {
                    if let Some(ref id) = ctx.selected_file {
                        ctx.dispatch(AppAction::SetMapping(
                            id.clone(),
                            Mapping {
                                new_name: filename_stem.clone(),
                                label: label.clone(),
                                kind: MappingKind::Episode,
                            },
                        ));
                    }
                })
            };

            html! {
                <div
                    class={format!("episode-item{}", if already_used { " used" } else { "" })}
                    onclick={cb}
                >
                    <span class="ep-code">
                        { format!("S{:02}E{:02}", ep.season, ep.episode) }
                    </span>
                    <span class="ep-title">{ &ep.title }</span>
                    { if let Some(rt) = ep.runtime {
                        html! { <span class="ep-runtime">{ format!("{rt}m") }</span> }
                    } else { html! {} }}
                </div>
            }
        })
        .collect()
}

fn render_features(ctx: &AppContext) -> Html {
    if ctx.dvd_features.is_empty() {
        return html! {
            <div class="empty-state">
                {"Select a DVDCompare result"}<br/>{"to see disc features."}
            </div>
        };
    }

    ctx.dvd_features
        .iter()
        .map(|feat| {
            let ctx = ctx.clone();
            let title = feat.title.clone();
            let timecodes = feat.timecodes.join(", ");

            let cb = {
                let ctx = ctx.clone();
                let title = title.clone();
                Callback::from(move |_| {
                    if let Some(ref id) = ctx.selected_file {
                        ctx.dispatch(AppAction::SetMapping(
                            id.clone(),
                            Mapping {
                                new_name: title.clone(),
                                label: title.clone(),
                                kind: MappingKind::Feature,
                            },
                        ));
                    }
                })
            };

            html! {
                <div class="episode-item" onclick={cb}>
                    <span class="ep-title">{ &title }</span>
                    <span class="ep-runtime">{ timecodes }</span>
                </div>
            }
        })
        .collect()
}

fn render_dvd(ctx: &AppContext) -> Html {
    if ctx.dvd_results.is_empty() {
        return html! {
            <div class="empty-state">
                {"Search to find DVDCompare"}<br/>{"disc comparisons."}
            </div>
        };
    }

    ctx.dvd_results
        .iter()
        .map(|disc| {
            let ctx = ctx.clone();
            let compid = disc.compid.clone();
            let is_active = ctx.selected_dvd.as_deref() == Some(&compid);

            let cb = Callback::from(move |_| {
                let ctx2 = ctx.clone();
                let compid2 = compid.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    ctx2.dispatch(AppAction::SelectDvd(Some(compid2.clone())));
                    ctx2.dispatch(AppAction::SetActiveTab(ActiveTab::Features));
                    match crate::api::fetch_dvd_disc(&compid2).await {
                        Ok(features) => ctx2.dispatch(AppAction::SetDvdFeatures(features)),
                        Err(e) => ctx2.dispatch(AppAction::SetStatus(format!("DVDCompare error: {e}"))),
                    }
                });
            });

            html! {
                <div
                    class={format!("result-item{}", if is_active { " active" } else { "" })}
                    onclick={cb}
                >
                    <div class="result-title">{ &disc.title }</div>
                    <div class="result-sub">{ format!("compid={}", disc.compid) }</div>
                </div>
            }
        })
        .collect()
}
