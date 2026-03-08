use yew::prelude::*;

use crate::{
    api::preview_url,
    state::{AppAction, AppContext},
    types::{Mapping, MappingKind},
};

const SPECIAL_TYPES: &[&str] = &[
    "Behind the Scenes",
    "Deleted Scenes",
    "Featurette",
    "Interview",
    "Scene",
    "Short",
    "Trailer",
    "Teaser",
    "Theme",
];

#[function_component(CenterPanel)]
pub fn center_panel() -> Html {
    let ctx = use_context::<AppContext>().expect("AppContext missing");
    let video_ref = use_node_ref();

    // Persist start/duration across file selections — these live at component
    // level so they survive re-renders caused by selecting a different file.
    let preview_start = use_state(|| 30.0_f64);
    let preview_dur = use_state(|| 12.0_f64);

    let Some(ref selected_id) = ctx.selected_file.clone() else {
        return html! {
            <div class="center">
                <div class="selected-file-info">
                    <div class="empty-state" style="padding:20px 0">
                        {"← Select a file to map it"}
                    </div>
                </div>
                <div class="mapping-area"></div>
            </div>
        };
    };

    let file = ctx.files.iter().find(|f| &f.id == selected_id).cloned();

    let Some(file) = file else {
        return html! { <div class="center"></div> };
    };

    let mapping = ctx.mappings.get(&file.id).cloned();

    // ── Preview callbacks ─────────────────────────────────────────────────────

    let on_start_input = {
        let preview_start = preview_start.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<f64>() {
                preview_start.set(v.max(0.0));
            }
        })
    };

    let on_dur_input = {
        let preview_dur = preview_dur.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<f64>() {
                preview_dur.set(v.max(1.0));
            }
        })
    };

    let on_skip_back = {
        let preview_start = preview_start.clone();
        let video_ref = video_ref.clone();
        let path = file.path.clone();
        let dur = *preview_dur;
        Callback::from(move |_| {
            let new_start = (*preview_start - 10.0).max(0.0);
            preview_start.set(new_start);
            if let Some(video) = video_ref.cast::<web_sys::HtmlVideoElement>() {
                video.set_src(&preview_url(&path, new_start, dur));
                let _ = video.load();
                let _ = video.play();
            }
        })
    };

    let on_skip_fwd = {
        let preview_start = preview_start.clone();
        let video_ref = video_ref.clone();
        let path = file.path.clone();
        let dur = *preview_dur;
        Callback::from(move |_| {
            let new_start = *preview_start + 10.0;
            preview_start.set(new_start);
            if let Some(video) = video_ref.cast::<web_sys::HtmlVideoElement>() {
                video.set_src(&preview_url(&path, new_start, dur));
                let _ = video.load();
                let _ = video.play();
            }
        })
    };

    let on_preview = {
        let path = file.path.clone();
        let video_ref = video_ref.clone();
        let start = *preview_start;
        let dur = *preview_dur;
        Callback::from(move |_| {
            let src = preview_url(&path, start, dur);
            if let Some(video) = video_ref.cast::<web_sys::HtmlVideoElement>() {
                video.set_src(&src);
                let _ = video.load();
                let _ = video.play();
            }
        })
    };

    let on_stop = {
        let path = file.path.clone();
        let video_ref = video_ref.clone();
        Callback::from(move |_| {
            if let Some(video) = video_ref.cast::<web_sys::HtmlVideoElement>() {
                video.pause().ok();
                video.set_src("");
            }
            let path = path.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = format!("/api/preview/stop?path={}", urlencoding::encode(&path));
                let _ = gloo_net::http::Request::get(&url).send().await;
            });
        })
    };

    let on_unmap = {
        let ctx = ctx.clone();
        let id = file.id.clone();
        Callback::from(move |_| ctx.dispatch(AppAction::UnmapFile(id.clone())))
    };

    // ── Quick-assign special feature ─────────────────────────────────────────

    let special_buttons: Html = SPECIAL_TYPES
        .iter()
        .map(|&kind| {
            let ctx = ctx.clone();
            let id = file.id.clone();
            let stem = file.stem.clone();
            let kind_str = kind.to_string();
            let cb = Callback::from(move |_| {
                let new_name = format!("{} - {}", stem, kind_str);
                ctx.dispatch(AppAction::SetMapping(
                    id.clone(),
                    Mapping {
                        new_name: new_name.clone(),
                        label: new_name,
                        kind: MappingKind::Special,
                    },
                ));
            });
            html! { <button class="small assign-btn" onclick={cb}>{ kind }</button> }
        })
        .collect();

    // ── Cluster siblings ──────────────────────────────────────────────────────

    let cluster_html = if let Some(idx) = ctx.cluster_index(&file.id) {
        let siblings: Vec<_> = ctx.clusters[idx]
            .iter()
            .filter(|id| *id != &file.id)
            .filter_map(|id| ctx.files.iter().find(|f| &f.id == id))
            .collect();

        if siblings.is_empty() {
            html! {}
        } else {
            let buttons: Html = siblings
                .iter()
                .map(|sibling| {
                    let ctx = ctx.clone();
                    let sid = sibling.id.clone();
                    let cb = Callback::from(move |_| {
                        ctx.dispatch(AppAction::SelectFile(Some(sid.clone())))
                    });
                    html! {
                        <button class="small assign-btn" onclick={cb}>
                            { &sibling.filename }
                        </button>
                    }
                })
                .collect();

            html! {
                <div class="quick-assign">
                    <h4>{ format!("Cluster {idx} — similar duration ({} files)", siblings.len() + 1) }</h4>
                    <div class="assign-grid">{ buttons }</div>
                </div>
            }
        }
    } else {
        html! {}
    };

    // ── Pending renames ───────────────────────────────────────────────────────

    let rename_panel = if ctx.mappings.is_empty() {
        html! {}
    } else {
        let items: Html = ctx
            .mappings
            .iter()
            .map(|(path, m)| {
                let filename = path.rsplit('/').next().unwrap_or(path);
                html! {
                    <div class="rename-item">
                        <span class="rename-old">{ filename }</span>
                        <span style="color:var(--muted)">{"→"}</span>
                        <span class="rename-new">{ &m.new_name }</span>
                    </div>
                }
            })
            .collect();

        html! {
            <div class="rename-panel">
                <div class="panel-header" style="font-size:10px">
                    { format!("Pending Renames ({})", ctx.mappings.len()) }
                </div>
                { items }
            </div>
        }
    };

    // ── Audio track info ──────────────────────────────────────────────────────

    let audio_info = if file.audio_streams.is_empty() {
        html! {}
    } else {
        let tracks: Vec<String> = file
            .audio_streams
            .iter()
            .map(|a| {
                format!(
                    "{} {} {}ch",
                    if a.language.is_empty() { "?" } else { &a.language },
                    a.codec,
                    a.channels
                )
            })
            .collect();
        html! {
            <div style="margin-top:6px;font-family:var(--mono);font-size:10px;color:var(--muted)">
                { format!("Audio: {}", tracks.join(" · ")) }
            </div>
        }
    };

    html! {
        <div class="center">
            // ── File info ────────────────────────────────────────────────────
            <div class="selected-file-info">
                <h3>{ &file.filename }</h3>
                <div class="props-grid">
                    <div>{"Duration: "}<strong>{ &file.duration_fmt }</strong></div>
                    <div>{"Size: "}<strong>{ &file.size_fmt }</strong></div>
                    <div>{"Video: "}<strong>{ format!("{} stream(s)", file.video_streams) }</strong></div>
                    <div>{"Audio: "}<strong>{ format!("{} track(s)", file.audio_streams.len()) }</strong></div>
                    <div>{"Subs: "}<strong>{ format!("{} track(s)", file.subtitle_streams.len()) }</strong></div>
                    <div>{"Title: "}<strong>{ if file.title.is_empty() { "—" } else { &file.title } }</strong></div>
                </div>
                { audio_info }
            </div>

            // ── Video preview ────────────────────────────────────────────────
            <div class="preview-player">
                <video ref={video_ref} controls=true playsinline=true />
            </div>
            <div class="preview-controls">
                <button class="small" onclick={on_skip_back} title="-10s">{"◀◀"}</button>
                <label>{"Start:"}</label>
                <input
                    type="number"
                    value={preview_start.to_string()}
                    oninput={on_start_input}
                    min="0"
                    step="10"
                    style="width:60px"
                />
                <label>{"sec"}</label>
                <button class="small" onclick={on_skip_fwd} title="+10s">{"▶▶"}</button>
                <label style="margin-left:8px">{"Dur:"}</label>
                <input
                    type="number"
                    value={preview_dur.to_string()}
                    oninput={on_dur_input}
                    min="1"
                    max="120"
                    step="5"
                    style="width:60px"
                />
                <label>{"sec"}</label>
                <button class="small" onclick={on_preview}>{"▶ Preview"}</button>
                <button class="small danger" onclick={on_stop}>{"■ Stop"}</button>
            </div>

            // ── Mapping workspace ────────────────────────────────────────────
            <div class="mapping-area">
                { if let Some(m) = &mapping {
                    html! {
                        <div class="current-mapping">
                            <label>{"Current Mapping"}</label>
                            <div class="mapped-to">{ &m.label }</div>
                            <div style="font-family:var(--mono);font-size:10px;color:var(--muted);margin-top:4px">
                                { format!("New filename: {}", m.new_name) }
                            </div>
                            <div style="margin-top:8px">
                                <button class="small danger" onclick={on_unmap}>{"Remove mapping"}</button>
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }}

                <div class="quick-assign">
                    <h4>{"Quick assign special feature type"}</h4>
                    <div class="assign-grid">{ special_buttons }</div>
                </div>

                { cluster_html }
            </div>

            { rename_panel }
        </div>
    }
}
