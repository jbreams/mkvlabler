use std::rc::Rc;
use yew::prelude::*;

use crate::types::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppState {
    pub root_dir: String,
    pub tmdb_enabled: bool,
    pub files: Vec<VideoFile>,
    pub clusters: Vec<Vec<String>>,
    pub mappings: Mappings,
    pub selected_file: Option<String>,
    pub show_mapped: bool,
    pub sort_mode: SortMode,
    pub directory: String,
    pub active_tab: ActiveTab,
    pub tv_shows: Vec<ShowResult>,
    pub selected_show: Option<u64>,
    pub episodes: Vec<EpisodeResult>,
    pub movies: Vec<MovieResult>,
    pub selected_movie: Option<u64>,
    pub dvd_results: Vec<DvdSearchResult>,
    pub dvd_features: Vec<DvdFeature>,
    pub selected_dvd: Option<String>,
    pub status: String,
    pub scanning: bool,
}

impl AppState {
    pub fn cluster_index(&self, file_id: &str) -> Option<usize> {
        self.clusters
            .iter()
            .position(|cl| cl.iter().any(|id| id == file_id))
    }

    pub fn mapped_count(&self) -> usize {
        self.mappings.len()
    }
}

pub enum AppAction {
    SetRootDir { root: String, tmdb_enabled: bool },
    SetDirectory(String),
    SetScanning(bool),
    SetStatus(String),
    ScanComplete { files: Vec<VideoFile>, clusters: Vec<Vec<String>>, directory: String },
    SelectFile(Option<String>),
    SetMapping(String, Mapping),
    UnmapFile(String),
    ToggleShowMapped,
    SetSortMode(SortMode),
    SetActiveTab(ActiveTab),
    SetTvShows(Vec<ShowResult>),
    SelectShow(Option<u64>),
    SetEpisodes(Vec<EpisodeResult>),
    SetMovies(Vec<MovieResult>),
    SelectMovie(Option<u64>),
    SetDvdResults(Vec<DvdSearchResult>),
    SelectDvd(Option<String>),
    SetDvdFeatures(Vec<DvdFeature>),
    ClearAll,
}

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut s = (*self).clone();
        match action {
            AppAction::SetRootDir { root, tmdb_enabled } => {
                s.root_dir = root;
                s.tmdb_enabled = tmdb_enabled;
            }
            AppAction::SetDirectory(dir) => s.directory = dir,
            AppAction::SetScanning(b) => s.scanning = b,
            AppAction::SetStatus(msg) => s.status = msg,
            AppAction::ScanComplete { files, clusters, directory } => {
                s.files = files;
                s.clusters = clusters;
                s.directory = directory;
                s.mappings.clear();
                s.selected_file = None;
                s.scanning = false;
            }
            AppAction::SelectFile(id) => s.selected_file = id,
            AppAction::SetMapping(id, mapping) => {
                s.mappings.insert(id, mapping);
            }
            AppAction::UnmapFile(id) => {
                s.mappings.remove(&id);
            }
            AppAction::ToggleShowMapped => s.show_mapped = !s.show_mapped,
            AppAction::SetSortMode(mode) => s.sort_mode = mode,
            AppAction::SetActiveTab(tab) => s.active_tab = tab,
            AppAction::SetTvShows(shows) => {
                s.tv_shows = shows;
                s.selected_show = None;
                s.episodes.clear();
            }
            AppAction::SelectShow(id) => s.selected_show = id,
            AppAction::SetEpisodes(eps) => s.episodes = eps,
            AppAction::SetMovies(movies) => {
                s.movies = movies;
                s.selected_movie = None;
            }
            AppAction::SelectMovie(id) => s.selected_movie = id,
            AppAction::SetDvdResults(results) => {
                s.dvd_results = results;
                s.selected_dvd = None;
                s.dvd_features.clear();
            }
            AppAction::SelectDvd(id) => s.selected_dvd = id,
            AppAction::SetDvdFeatures(features) => s.dvd_features = features,
            AppAction::ClearAll => {
                s.files.clear();
                s.clusters.clear();
                s.mappings.clear();
                s.selected_file = None;
                s.status = String::new();
            }
        }
        Rc::new(s)
    }
}

pub type AppContext = UseReducerHandle<AppState>;
