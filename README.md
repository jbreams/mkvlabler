# mkvlabel

A local web tool for bulk-labeling and renaming video files. Point it at a directory of MKV/MP4/AVI files, look up episode or feature names from TVmaze or DVDCompare, map files to names, and apply the renames — all from a browser tab.

Built with Rust ([Axum](https://github.com/tokio-rs/axum) backend, [Yew](https://yew.rs) WASM frontend).

## Features

- **Directory scan** — recursively finds `.mkv`, `.mp4`, `.avi`, `.m2ts`, `.ts` files and probes each with ffprobe in parallel (one worker per CPU core)
- **Stream info** — shows duration, file size, video/audio/subtitle stream details per file
- **Duration clustering** — groups files with similar runtimes together so episodes that belong to the same season are visually linked
- **In-browser preview** — seeks to any offset and streams a short clip directly in the page via ffmpeg (fragmented MP4 over HTTP); supports stream-copy for H.264 sources
- **TVmaze integration** — search for a TV show and load its full episode list; click an episode to map a file to it
- **DVDCompare integration** — scrape disc feature listings from dvdcompare.net and map files to bonus features, extras, etc.
- **Special feature types** — quick-assign buttons for Behind the Scenes, Deleted Scenes, Featurette, Trailer, and more
- **Bulk rename** — review all pending mappings before committing; renames are applied atomically per file with conflict detection

## Requirements

| Tool | Notes |
|------|-------|
| [Rust](https://rustup.rs) 1.75+ | Stable toolchain |
| `wasm32-unknown-unknown` target | `rustup target add wasm32-unknown-unknown` |
| [Trunk](https://trunkrs.dev) | `cargo install trunk` — builds and serves the WASM frontend |
| ffmpeg + ffprobe | Auto-downloaded by [ffmpeg-sidecar](https://github.com/nathanbabcock/ffmpeg-sidecar) on first run if not found on `$PATH` |

## Development

Run the backend and frontend in separate terminals:

```sh
# Terminal 1 — Axum backend (default port 7432)
cargo run -p mkvlabel -- --dir /path/to/your/videos

# Terminal 2 — Yew frontend with hot reload (default port 8080)
cd frontend
trunk serve
```

Open `http://localhost:8080`. The frontend proxies all `/api/*` requests to the backend automatically via `Trunk.toml`.

### CLI options

```
Usage: mkvlabel [OPTIONS]

Options:
      --port <PORT>  Port to listen on [default: 7432]
      --dir <DIR>    Default directory to scan [default: .]
  -h, --help         Print help
```

### Log verbosity

Logging uses `RUST_LOG` via [`tracing-subscriber`](https://docs.rs/tracing-subscriber):

```sh
# Default — debug for mkvlabel, warn for deps
cargo run -p mkvlabel

# Verbose — includes tower-http request traces
RUST_LOG=debug cargo run -p mkvlabel

# Quiet
RUST_LOG=warn cargo run -p mkvlabel
```

## Production build

```sh
# 1. Build the WASM frontend
cd frontend && trunk build --release && cd ..

# 2. Build the backend
cargo build -p mkvlabel --release
```

The backend serves only API routes; to ship as a single binary embed the `frontend/dist/` output using [`rust-embed`](https://github.com/pyrossh/rust-embed) or [`tower-http`'s `ServeDir`](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html).

## API reference

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/scan?dir=<path>` | Walk directory, probe all files in parallel, return file list and duration clusters |
| `GET` | `/api/preview?path=<path>&start=<sec>&duration=<sec>` | Stream a short clip as fragmented MP4 |
| `GET` | `/api/preview/stop?path=<path>` | Kill the running ffmpeg preview process for a file |
| `GET` | `/api/tvmaze/search?q=<query>` | Search TVmaze for shows |
| `GET` | `/api/tvmaze/episodes?id=<show_id>&season=<n>` | Fetch episode list |
| `GET` | `/api/dvdcompare/search?q=<query>` | Search DVDCompare |
| `GET` | `/api/dvdcompare/disc?compid=<id>` | Fetch disc feature list |
| `POST` | `/api/rename` | Apply batch renames `{ mappings: [{ old_path, new_name }] }` |
