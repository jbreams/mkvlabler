mod api;
mod components;
mod state;
mod types;

use web_sys::HtmlInputElement;
use yew::prelude::*;

use components::{CenterPanel, FileListPanel, MetadataPanel};
use state::{AppAction, AppContext, AppState};

#[function_component(App)]
fn app() -> Html {
    let state = use_reducer(AppState::default);

    // Expose state handle via context so all child components can read + dispatch.
    let ctx = state.clone();

    let dir_ref = use_node_ref();

    let on_scan = {
        let ctx = ctx.clone();
        let dir_ref = dir_ref.clone();
        Callback::from(move |_| {
            let dir = dir_ref
                .cast::<HtmlInputElement>()
                .map(|el| el.value())
                .unwrap_or_default();
            if dir.is_empty() {
                return;
            }
            let ctx2 = ctx.clone();
            ctx.dispatch(AppAction::SetDirectory(dir.clone()));
            ctx.dispatch(AppAction::SetScanning(true));
            ctx.dispatch(AppAction::SetStatus("scanning...".to_string()));

            wasm_bindgen_futures::spawn_local(async move {
                match api::scan_dir(&dir).await {
                    Ok(resp) => {
                        let count = resp.files.len();
                        ctx2.dispatch(AppAction::ScanComplete {
                            files: resp.files,
                            clusters: resp.clusters,
                            directory: resp.directory.clone(),
                        });
                        ctx2.dispatch(AppAction::SetStatus(format!("loaded {count} files")));

                        // Pre-fill the metadata search box with the directory name
                        // (handled via state; metadata panel reads from context)
                    }
                    Err(e) => {
                        ctx2.dispatch(AppAction::SetScanning(false));
                        ctx2.dispatch(AppAction::SetStatus(format!("Scan failed: {e}")));
                    }
                }
            });
        })
    };

    let file_count = state.files.len();
    let mapped_count = state.mapped_count();
    let cluster_count = state.clusters.len();
    let show_stats = file_count > 0;

    html! {
        <ContextProvider<AppContext> context={state}>
            <header>
                <h1>{"mkvlabel"}</h1>
                <div class="dir-row">
                    <input
                        ref={dir_ref}
                        type="text"
                        placeholder="/path/to/your/video/files"
                    />
                    <button onclick={on_scan} disabled={ctx.scanning}>
                        { if ctx.scanning { "scanning..." } else { "Scan" } }
                    </button>
                </div>
                <div style="font-family:var(--mono);font-size:10px;color:var(--muted)">
                    { &ctx.status }
                </div>
            </header>

            { if show_stats {
                html! {
                    <div class="stats-bar">
                        <div>{"Files: "}<span>{ file_count }</span></div>
                        <div>{"Mapped: "}<span>{ mapped_count }</span></div>
                        <div>{"Remaining: "}<span>{ file_count - mapped_count }</span></div>
                        <div>{"Clusters: "}<span>{ cluster_count }</span></div>
                    </div>
                }
            } else { html! {} }}

            <div class="main">
                <FileListPanel />
                <CenterPanel />
                <MetadataPanel />
            </div>
        </ContextProvider<AppContext>>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
