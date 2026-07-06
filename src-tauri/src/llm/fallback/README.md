# llm/fallback — module map

The offline-engine code lives in `src-tauri/src/llm/fallback/`.
This is a quick reference for which file does what.

```
llm/fallback/
├── mod.rs                     # public re-exports
├── manager.rs                 # FallbackManager — filesystem +
│                              # llama-server process lifecycle
├── registry.rs                # MODELS spec table + HF resolver +
│                              # speed_test_url + download cache
├── download.rs                # single-conn chunked stream + SHA-256
└── multi_download.rs          # v0.8: Range-aware parallel downloader
```

## Public API (`mod.rs`)

Re-exports the names used outside the module:

| Symbol | From | What it does |
|---|---|---|
| `FallbackManager` | manager.rs | start / stop / install / resolve a model |
| `find_spec(id)` | registry.rs | look up a `FallbackModelSpec` by id |
| `MODELS` | registry.rs | static `&[FallbackModelSpec]` for the picker |
| `resolve_spec(spec, dir)` | registry.rs | resolve via HF API + cache |
| `resolve_spec_with_speedtest(spec, dir)` | registry.rs | resolve + auto-reroute to fastest mirror |
| `resolve_spec_with_preferred(spec, dir, url)` | registry.rs | resolve pinned to a user-picked URL |
| `speed_test_url(url, label, kind)` | registry.rs | probe one URL, returns `SpeedTestResult` |
| `stream_to_file(...)` | download.rs | stream + verify (single-conn) |
| `sha256_file(path)` | download.rs | hash any file |
| `multi_download::download(...)` | multi_download.rs | Range-aware parallel downloader |

## Quick decisions

- **Manager vs Registry** — *Registry* owns the model catalogue
  and HF caching; *Manager* owns the running llama-server process.
  Commands either consult the registry (read-only) or drive the
  manager (mutating).
- **Download vs Multi-Download** — *download.rs* is the legacy
  single-connection streamer used by every binary download
  (llama-server.exe, GGUF). *multi_download.rs* slices the GGUF
  range-aware into N parts for v0.8+. The pick happens inside
  `llm_fallback::commands::run_install` based on whether the
  target server advertised `Accept-Ranges: bytes`.

## When you add a new model

1. Append an entry to `MODELS` in `registry.rs`.
2. Add a UI label in `views/ai/LocalModelPanel.vue`'s `fallbackModels`
  transform OR just rely on the API returning spec.
3. Mirror resolution is automatic; no code change needed.

## When you change the download format

- `download.rs` is invoked from `commands/llm_fallback.rs` for the
  llama-server stage (small ZIP, no Range needed).
- `multi_download.rs` is invoked from `run_install` for the GGUF
  stage (multi-GB, range-Aware).
- Both rely on the same `DownloadProgress` shape so the frontend
  progress UI works unchanged.
