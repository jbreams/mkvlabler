use std::collections::HashMap;
use yew::prelude::*;

use crate::{
    state::{AppAction, AppContext},
    types::{RenameMapping, SortMode, VideoFile},
};

#[function_component(FileListPanel)]
pub fn file_list_panel() -> Html {
    let ctx = use_context::<AppContext>().expect("AppContext missing");

    let on_toggle_mapped = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.dispatch(AppAction::ToggleShowMapped))
    };

    let on_sort_dur = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.dispatch(AppAction::SetSortMode(SortMode::Duration)))
    };

    let on_sort_size = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.dispatch(AppAction::SetSortMode(SortMode::Size)))
    };

    let on_clear = {
        let ctx = ctx.clone();
        Callback::from(move |_| ctx.dispatch(AppAction::ClearAll))
    };

    let on_apply = {
        let ctx = ctx.clone();
        Callback::from(move |_| {
            let mappings: Vec<RenameMapping> = ctx
                .mappings
                .iter()
                .map(|(path, m)| RenameMapping {
                    old_path: path.clone(),
                    new_name: m.new_name.clone(),
                })
                .collect();

            let ctx_clone = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::apply_renames(mappings).await {
                    Ok(results) => {
                        let ok = results.iter().filter(|r| r.success).count();
                        let fail = results.iter().filter(|r| !r.success).count();
                        ctx_clone.dispatch(AppAction::SetStatus(format!(
                            "Renamed {ok} files{}",
                            if fail > 0 { format!(", {fail} failed") } else { String::new() }
                        )));
                        let dir = ctx_clone.directory.clone();
                        if !dir.is_empty() {
                            match crate::api::scan_dir(&dir).await {
                                Ok(resp) => ctx_clone.dispatch(AppAction::ScanComplete {
                                    files: resp.files,
                                    clusters: resp.clusters,
                                    directory: resp.directory,
                                }),
                                Err(e) => ctx_clone.dispatch(AppAction::SetStatus(format!("Rescan failed: {e}"))),
                            }
                        }
                    }
                    Err(e) => ctx_clone.dispatch(AppAction::SetStatus(format!("Rename failed: {e}"))),
                }
            });
        })
    };

    let has_mappings = !ctx.mappings.is_empty();
    let has_files = !ctx.files.is_empty();

    html! {
        <div class="panel">
            <div class="panel-header">
                {"Files"}
                <div style="display:flex;gap:6px">
                    <button class="small" onclick={on_toggle_mapped}>
                        {if ctx.show_mapped { "hide mapped" } else { "show mapped" }}
                    </button>
                    <button class="small" onclick={on_sort_dur}>{"sort dur"}</button>
                    <button class="small" onclick={on_sort_size}>{"sort size"}</button>
                </div>
            </div>

            <div class="panel-body">
                { render_file_list(&ctx) }
            </div>

            <div class="panel-footer">
                <button
                    class="primary"
                    onclick={on_apply.clone()}
                    disabled={!has_mappings}
                >
                    {"Apply Renames"}
                </button>
                <button
                    class="danger small"
                    onclick={on_clear}
                    disabled={!has_files}
                >
                    {"Clear"}
                </button>
            </div>
        </div>
    }
}

fn render_file_list(ctx: &AppContext) -> Html {
    if ctx.files.is_empty() {
        return html! {
            <div class="empty-state">
                {"Enter a directory and click Scan"}<br/>
                {"to load your video files."}
            </div>
        };
    }

    // Group by parent directory
    let mut by_dir: HashMap<String, Vec<&VideoFile>> = HashMap::new();
    for f in &ctx.files {
        if !ctx.show_mapped && ctx.mappings.contains_key(&f.id) {
            continue;
        }
        by_dir.entry(f.parent.clone()).or_default().push(f);
    }

    if by_dir.is_empty() {
        return html! { <div class="empty-state">{"All files mapped."}</div> };
    }

    let mut dirs: Vec<String> = by_dir.keys().cloned().collect();
    dirs.sort();

    let sort_mode = ctx.sort_mode.clone();
    let cluster_map: HashMap<String, usize> = ctx
        .clusters
        .iter()
        .enumerate()
        .flat_map(|(i, cl)| cl.iter().map(move |id| (id.clone(), i)))
        .collect();

    html! {
        <>
        { for dirs.iter().map(|dir| {
            let mut files = by_dir[dir].clone();
            match sort_mode {
                SortMode::Duration => files.sort_by(|a, b| a.duration.partial_cmp(&b.duration).unwrap_or(std::cmp::Ordering::Equal)),
                SortMode::Size     => files.sort_by_key(|f| f.size),
                SortMode::Path     => files.sort_by(|a, b| a.filename.cmp(&b.filename)),
            }

            let short_dir = dir.rsplit('/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/");

            html! {
                <div class="file-group">
                    <div class="file-group-header">
                        <span>{ &short_dir }</span>
                        <span style="color:var(--muted)">{ format!("{} files", files.len()) }</span>
                    </div>
                    { for files.iter().map(|f| render_file_item(f, ctx, &cluster_map)) }
                </div>
            }
        })}
        </>
    }
}

fn render_file_item(
    f: &VideoFile,
    ctx: &AppContext,
    cluster_map: &HashMap<String, usize>,
) -> Html {
    let id = f.id.clone();
    let is_selected = ctx.selected_file.as_deref() == Some(&id);
    let mapped = ctx.mappings.get(&id);
    let cluster_idx = cluster_map.get(&id);

    let on_select = {
        let ctx = ctx.clone();
        let id = id.clone();
        Callback::from(move |_| ctx.dispatch(AppAction::SelectFile(Some(id.clone()))))
    };


    let on_unmap = mapped.map(|_| {
        let ctx = ctx.clone();
        let id = id.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            ctx.dispatch(AppAction::UnmapFile(id.clone()));
        })
    });

    let item_class = format!(
        "file-item{}{}",
        if is_selected { " selected" } else { "" },
        if mapped.is_some() { " mapped" } else { "" },
    );

    html! {
        <div class={item_class} onclick={on_select}>
            <div class="file-info">
                <div class="file-name">{ &f.filename }</div>
                <div class="file-meta">
                    { &f.duration_fmt }
                    {" · "}
                    { &f.size_fmt }
                    { if let Some(idx) = cluster_idx {
                        html! { <span class="cluster-badge">{ format!("C{idx}") }</span> }
                    } else {
                        html! {}
                    }}
                </div>
                { if let Some(m) = mapped {
                    html! { <div class="file-mapped-label">{ format!("→ {}", m.label) }</div> }
                } else {
                    html! {}
                }}
            </div>
            { if let Some(cb) = on_unmap {
                html! {
                    <div class="file-actions">
                        <button class="small danger" onclick={cb} title="Remove mapping">{"✕"}</button>
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
