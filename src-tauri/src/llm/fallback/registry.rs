//! Dynamic model registry — discover + resolve GGUF models from
//! HuggingFace at install time, instead of hardcoding every URL.
//!
//! ## Architecture
//!
//! 1. `MODELS` is a small static list of well-known model families
//!    (Qwen2.5 1.5B, Llama-3.2 3B, Phi-3.5-mini, etc.). Each entry
//!    has an `hf_repo` and a `preferred_file_glob`.
//!
//! 2. When the user picks a model + clicks Install, we call
//!    `resolve_model(spec)` which:
//!      a. Checks `<data_dir>/llm/model_cache.json` for a cached
//!         resolution younger than 24 h. Hit → return cached.
//!      b. Miss: query `GET /api/models/{repo}` for the file list.
//!      c. Filter siblings by `preferred_file_glob`, pick the best
//!         match (prefer Q4_K_M, then Q4_0, then anything Q4, then
//!         smallest Q5/Q6, etc.).
//!      d. HEAD the resolved URL to confirm `Content-Length` + ETag
//!         (= SHA256 of the LFS blob).
//!      e. Append mirror URLs (hf-mirror.com, modelscope.cn).
//!      f. Persist to cache file. Return the ResolvedModel.
//!
//! 3. `discover_trending() -> Vec<TrendingModel>` returns top-N
//!    GGUF-Instruct models by HuggingFace downloads. Used by the
//!    "popular models" panel in the UI.
//!
//! ## Failure modes
//!
//! - HF API unreachable / rate-limited → fall back to hardcoded URLs
//!   (caller passes `fallback: ResolvedModel`).
//! - File glob doesn't match → fall back to first .gguf sibling.
//! - Cache corrupted → delete + re-resolve.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// What the user picks in the dropdown.
#[derive(Debug, Clone)]
pub struct FallbackModelSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub family: &'static str,
    pub parameter_count: &'static str,
    pub quantization: &'static str,
    pub min_ram_gb: u32,
    pub hf_repo: &'static str,
    pub preferred_file_glob: &'static str,
    pub size_estimate_bytes: u64,
}

/// What we resolve at install time — concrete URL + size + hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedModel {
    pub id: String,
    pub display_name: String,
    pub primary_url: String,
    pub mirrors: Vec<String>,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub resolved_at_unix_ms: i64,
}

/// Trending model summary returned by `discover_trending()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingModel {
    pub id: String,
    pub hf_repo: String,
    pub display_name: String,
    pub downloads: u64,
    pub likes: u32,
    pub family_guess: String,
    pub approx_size_bytes: u64,
}

/// On-disk cache for resolved models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionCache {
    pub entries: std::collections::HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub resolved: ResolvedModel,
    pub cached_at_unix_ms: i64,
}

impl CacheEntry {
    /// Construct a fresh entry using the current wall-clock time.
    pub fn fresh(resolved: ResolvedModel) -> Self {
        Self {
            resolved,
            cached_at_unix_ms: unix_ms(),
        }
    }
}

const CACHE_TTL_MS: i64 = 24 * 60 * 60 * 1000; // 24 hours
const HF_API_BASE: &str = "https://huggingface.co";

/// Well-known model families. Order matters: most popular first.
pub static MODELS: &[FallbackModelSpec] = &[
    FallbackModelSpec {
        id: "qwen2.5-1.5b-instruct",
        display_name: "Qwen2.5 1.5B Instruct",
        family: "qwen2.5",
        parameter_count: "1.5B",
        quantization: "Q4_K_M",
        min_ram_gb: 4,
        hf_repo: "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
        preferred_file_glob: "*q4_k_m*.gguf",
        size_estimate_bytes: 1_100_000_000,
    },
    FallbackModelSpec {
        id: "qwen2.5-3b-instruct",
        display_name: "Qwen2.5 3B Instruct",
        family: "qwen2.5",
        parameter_count: "3B",
        quantization: "Q4_K_M",
        min_ram_gb: 6,
        hf_repo: "Qwen/Qwen2.5-3B-Instruct-GGUF",
        preferred_file_glob: "*q4_k_m*.gguf",
        size_estimate_bytes: 2_100_000_000,
    },
    FallbackModelSpec {
        id: "llama-3.2-3b-instruct",
        display_name: "Llama 3.2 3B Instruct",
        family: "llama3.2",
        parameter_count: "3B",
        quantization: "Q4_K_M",
        min_ram_gb: 6,
        hf_repo: "bartowski/Llama-3.2-3B-Instruct-GGUF",
        preferred_file_glob: "*Q4_K_M*.gguf",
        size_estimate_bytes: 2_050_000_000,
    },
    FallbackModelSpec {
        id: "llama-3.2-1b-instruct",
        display_name: "Llama 3.2 1B Instruct",
        family: "llama3.2",
        parameter_count: "1B",
        quantization: "Q4_K_M",
        min_ram_gb: 3,
        hf_repo: "bartowski/Llama-3.2-1B-Instruct-GGUF",
        preferred_file_glob: "*Q4_K_M*.gguf",
        size_estimate_bytes: 800_000_000,
    },
    FallbackModelSpec {
        id: "phi-3.5-mini-instruct",
        display_name: "Phi-3.5 mini Instruct",
        family: "phi3",
        parameter_count: "3.8B",
        quantization: "Q4_K_M",
        min_ram_gb: 6,
        hf_repo: "MaziyarPanahi/Phi-3.5-mini-instruct-GGUF",
        preferred_file_glob: "*Q4_K_M*.gguf",
        size_estimate_bytes: 2_300_000_000,
    },
    FallbackModelSpec {
        id: "smollm2-1.7b-instruct",
        display_name: "SmolLM2 1.7B Instruct",
        family: "smollm2",
        parameter_count: "1.7B",
        quantization: "Q4_K_M",
        min_ram_gb: 4,
        hf_repo: "bartowski/SmolLM2-1.7B-Instruct-GGUF",
        preferred_file_glob: "*Q4_K_M*.gguf",
        size_estimate_bytes: 1_100_000_000,
    },
];

pub fn find_spec(id: &str) -> Option<&'static FallbackModelSpec> {
    MODELS.iter().find(|m| m.id == id)
}

/// Look up a model's resolution by id in cache; checks freshness.
pub fn cache_get(cache: &ResolutionCache, id: &str, now_unix_ms: i64) -> Option<ResolvedModel> {
    let entry = cache.entries.get(id)?;
    if now_unix_ms - entry.cached_at_unix_ms < CACHE_TTL_MS {
        Some(entry.resolved.clone())
    } else {
        None
    }
}

pub fn cache_put(cache: &mut ResolutionCache, resolved: ResolvedModel) {
    cache.entries.insert(resolved.id.clone(), CacheEntry::fresh(resolved));
}

pub fn load_cache(dir: &Path) -> ResolutionCache {
    let path = dir.join("model_cache.json");
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => ResolutionCache::default(),
    }
}

pub fn save_cache(dir: &Path, cache: &ResolutionCache) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let body = serde_json::to_vec_pretty(cache).map_err(io::Error::other)?;
    let tmp = dir.join("model_cache.json.tmp");
    std::fs::write(&tmp, &body)?;
    std::fs::rename(&tmp, dir.join("model_cache.json"))?;
    Ok(())
}

/// Pick the best matching file from a sibling list given a glob.
/// Preference order: Q4_K_M > Q4_0 > Q4_K_S > any Q4 > Q5_K_M > Q5_0 > Q5_K_S >
/// Q6_K > Q8_0 > smallest gguf.
pub fn pick_best_gguf(siblings: &[HfSibling], glob: &str) -> Option<HfSibling> {
    let mut candidates: Vec<&HfSibling> = siblings
        .iter()
        .filter(|s| {
            s.rfilename.to_lowercase().ends_with(".gguf")
                && glob_matches(&s.rfilename.to_lowercase(), glob)
        })
        .collect();
    if candidates.is_empty() {
        // Fallback: any .gguf file, smallest first.
        candidates = siblings
            .iter()
            .filter(|s| s.rfilename.to_lowercase().ends_with(".gguf"))
            .collect();
    }
    if candidates.is_empty() {
        return None;
    }
    // Sort by preference tier (lower = better), then by size (smaller = better).
    candidates.sort_by_key(|s| (quant_preference(&s.rfilename), s.size.unwrap_or(u64::MAX)));
    candidates.first().map(|s| (*s).clone())
}

fn glob_matches(name: &str, glob: &str) -> bool {
    // Very small glob: `*foo*.gguf` style only. We don't need full glob syntax.
    let g = glob.to_lowercase();
    if !g.contains('*') {
        return name == g;
    }
    // Split on `*` and check all parts appear in order.
    let parts: Vec<&str> = g.split('*').filter(|p| !p.is_empty()).collect();
    let mut idx = 0;
    for part in parts {
        if let Some(pos) = name[idx..].find(part) {
            idx += pos + part.len();
        } else {
            return false;
        }
    }
    true
}

fn quant_preference(name: &str) -> u32 {
    let n = name.to_lowercase();
    if n.contains("q4_k_m") { 0 }
    else if n.contains("q4_k_s") { 1 }
    else if n.contains("q4_0") { 2 }
    else if n.contains("q4_1") { 3 }
    else if n.contains("q4") { 4 }
    else if n.contains("q5_k_m") { 5 }
    else if n.contains("q5_0") { 6 }
    else if n.contains("q5_1") { 7 }
    else if n.contains("q6_k") { 8 }
    else if n.contains("q8_0") { 9 }
    else if n.contains("q3") { 20 }
    else if n.contains("q2") { 21 }
    else { 15 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSibling {
    pub rfilename: String,
    #[serde(default)]
    pub size: Option<u64>,
}

/// Fetch model metadata + sibling list from HuggingFace.
pub async fn fetch_hf_model(repo: &str) -> Result<HfModelResponse, String> {
    let url = format!("{}/api/models/{}", HF_API_BASE, repo);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .get(&url)
        .header("User-Agent", "admin-suite/0.6")
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GET {url}: HTTP {}", resp.status()));
    }
    resp.json::<HfModelResponse>()
        .await
        .map_err(|e| format!("parse {url}: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModelResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u32,
    #[serde(default)]
    pub siblings: Vec<HfSibling>,
}

/// HEAD a URL to confirm Content-Length + extract SHA256 from ETag.
/// Returns (size_bytes, sha256_hex_or_none).
pub async fn head_url_meta(url: &str) -> Result<(u64, Option<String>), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .head(url)
        .header("User-Agent", "admin-suite/0.6")
        .send()
        .await
        .map_err(|e| format!("HEAD {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HEAD {url}: HTTP {}", resp.status()));
    }
    let size = resp.content_length().unwrap_or(0);
    let sha = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"').to_string());
    Ok((size, sha))
}

/// Build the standard mirror list (primary URL comes from HF).
pub fn build_mirrors(repo: &str, primary_filename: &str) -> Vec<String> {
    let mut out = Vec::new();
    let hf_mirror_base = "https://hf-mirror.com";
    let modelscope_base = "https://www.modelscope.cn";
    // For HF repos, mirrors are well-known. Modelscope has its own path scheme.
    out.push(format!("{}/{}/resolve/main/{}", hf_mirror_base, repo, primary_filename));
    out.push(format!(
        "{}/models/{}/resolve/main/{}",
        modelscope_base, repo, primary_filename
    ));
    out
}

/// Resolve a static spec into a concrete URL + size + sha256.
/// Uses cache when fresh; queries HF + HEAD otherwise.
pub async fn resolve_spec(
    spec: &FallbackModelSpec,
    cache_dir: &Path,
) -> Result<ResolvedModel, String> {
    let mut cache = load_cache(cache_dir);
    let now = unix_ms();
    if let Some(cached) = cache_get(&cache, spec.id, now) {
        return Ok(cached);
    }

    // Live query.
    let hf = fetch_hf_model(spec.hf_repo).await?;
    let best = pick_best_gguf(&hf.siblings, spec.preferred_file_glob)
        .ok_or_else(|| format!("no .gguf file found in {}", spec.hf_repo))?;

    let primary_url = format!(
        "{}/{}/resolve/main/{}",
        HF_API_BASE, spec.hf_repo, best.rfilename
    );
    let (size, sha) = match head_url_meta(&primary_url).await {
        Ok((s, h)) => (s, h),
        Err(_) => (
            best.size.unwrap_or(spec.size_estimate_bytes),
            None,
        ),
    };

    let resolved = ResolvedModel {
        id: spec.id.to_string(),
        display_name: spec.display_name.to_string(),
        primary_url,
        mirrors: build_mirrors(spec.hf_repo, &best.rfilename),
        size_bytes: size,
        sha256: sha,
        resolved_at_unix_ms: now,
    };
    cache_put(&mut cache, resolved.clone());
    let _ = save_cache(cache_dir, &cache);
    Ok(resolved)
}

/// Resolve a HuggingFace repo from a trending entry (used by "popular
/// models" → "use this" flow). Returns a synthetic spec; caller persists
/// it as the chosen model.
pub async fn resolve_repo(
    hf_repo: &str,
    glob: &str,
    display_name: &str,
    size_estimate_bytes: u64,
    cache_dir: &Path,
) -> Result<ResolvedModel, String> {
    let hf = fetch_hf_model(hf_repo).await?;
    let best = pick_best_gguf(&hf.siblings, glob)
        .ok_or_else(|| format!("no .gguf file found in {hf_repo}"))?;
    let primary_url = format!("{}/{}/resolve/main/{}", HF_API_BASE, hf_repo, best.rfilename);
    let (size, sha) = match head_url_meta(&primary_url).await {
        Ok((s, h)) => (s, h),
        Err(_) => (best.size.unwrap_or(size_estimate_bytes), None),
    };
    let resolved = ResolvedModel {
        id: format!("hf:{}", hf_repo.replace('/', ":")),
        display_name: display_name.to_string(),
        primary_url,
        mirrors: build_mirrors(hf_repo, &best.rfilename),
        size_bytes: size,
        sha256: sha,
        resolved_at_unix_ms: unix_ms(),
    };
    let mut cache = load_cache(cache_dir);
    cache_put(&mut cache, resolved.clone());
    let _ = save_cache(cache_dir, &cache);
    Ok(resolved)
}

/// Discover top-N trending GGUF-Instruct models from HuggingFace.
pub async fn discover_trending(limit: usize) -> Result<Vec<TrendingModel>, String> {
    let url = format!(
        "{}/api/models?search=Instruct+GGUF&filter=text-generation&sort=downloads&direction=-1&limit={}",
        HF_API_BASE, limit
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .get(&url)
        .header("User-Agent", "admin-suite/0.6")
        .send()
        .await
        .map_err(|e| format!("discover: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("discover HTTP {}", resp.status()));
    }
    let items: Vec<HfSearchItem> = resp
        .json()
        .await
        .map_err(|e| format!("discover parse: {e}"))?;
    let mut out: Vec<TrendingModel> = items
        .into_iter()
        .filter(|m| m.id.contains("GGUF") || m.id.contains("gguf"))
        .map(|m| {
            let id = m.id.clone();
            let repo = m.id.clone();
            let display_name = m
                .id
                .split('/')
                .next_back()
                .unwrap_or(&m.id)
                .to_string();
            TrendingModel {
                approx_size_bytes: estimate_size_from_id(&m.id),
                family_guess: guess_family(&m.id),
                id: id.clone(),
                hf_repo: repo,
                display_name,
                downloads: m.downloads,
                likes: m.likes,
            }
        })
        .collect();
    out.truncate(limit);
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HfSearchItem {
    id: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u32,
}

fn estimate_size_from_id(id: &str) -> u64 {
    let n = id.to_lowercase();
    if n.contains("0.5b") {
        500_000_000
    } else if n.contains("1b") || n.contains("1.1b") {
        900_000_000
    } else if n.contains("1.5b") {
        1_100_000_000
    } else if n.contains("3b") || n.contains("3.8b") {
        2_300_000_000
    } else if n.contains("7b") {
        4_500_000_000
    } else if n.contains("8b") {
        5_000_000_000
    } else if n.contains("14b") {
        9_000_000_000
    } else if n.contains("32b") || n.contains("30b") {
        20_000_000_000
    } else {
        3_000_000_000
    }
}

fn guess_family(id: &str) -> String {
    let n = id.to_lowercase();
    for fam in [
        "qwen", "llama", "phi", "mistral", "gemma", "smollm", "deepseek",
        "codestral", "lfm", "yi",
    ] {
        if n.contains(fam) {
            return fam.to_string();
        }
    }
    "unknown".to_string()
}

pub fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("llm").join("model_cache.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sib(name: &str, size: Option<u64>) -> HfSibling {
        HfSibling { rfilename: name.to_string(), size }
    }

    #[test]
    fn pick_best_prefers_q4_k_m() {
        let siblings = vec![
            sib("model-q8_0.gguf", Some(5_000_000_000)),
            sib("model-q4_k_m.gguf", Some(1_100_000_000)),
            sib("model-q5_0.gguf", Some(2_500_000_000)),
        ];
        let best = pick_best_gguf(&siblings, "*q4_k_m*.gguf").unwrap();
        assert!(best.rfilename.contains("q4_k_m"));
    }

    #[test]
    fn pick_best_falls_back_to_any_gguf() {
        let siblings = vec![
            sib("model.safetensors", None),
            sib("model-fp16.gguf", Some(2_000_000_000)),
            sib("README.md", None),
        ];
        let best = pick_best_gguf(&siblings, "*q4_k_m*.gguf").unwrap();
        assert!(best.rfilename.ends_with(".gguf"));
    }

    #[test]
    fn pick_best_returns_none_when_empty() {
        let siblings = vec![sib("README.md", None), sib("config.json", None)];
        assert!(pick_best_gguf(&siblings, "*q4*.gguf").is_none());
    }

    #[test]
    fn quant_preference_orders_q4_first() {
        assert!(quant_preference("model-q4_k_m.gguf") < quant_preference("model-q8_0.gguf"));
        assert!(quant_preference("model-q4_k_m.gguf") < quant_preference("model-q5_0.gguf"));
    }

    #[test]
    fn glob_simple_star_matches() {
        assert!(glob_matches("qwen2.5-1.5b-instruct-q4_k_m.gguf", "*q4_k_m*.gguf"));
        assert!(!glob_matches("qwen2.5-1.5b-instruct-q8_0.gguf", "*q4_k_m*.gguf"));
    }

    #[test]
    fn size_estimate_recognizes_families() {
        assert_eq!(estimate_size_from_id("Qwen/Qwen2.5-1.5B-Instruct-GGUF"), 1_100_000_000);
        assert_eq!(estimate_size_from_id("bartowski/Llama-3.2-3B-Instruct-GGUF"), 2_300_000_000);
    }

    #[test]
    fn cache_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "admin-suite-registry-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut cache = ResolutionCache::default();
        let r = ResolvedModel {
            id: "x".into(),
            display_name: "X".into(),
            primary_url: "https://example.com/x.gguf".into(),
            mirrors: vec![],
            size_bytes: 12345,
            sha256: Some("deadbeef".into()),
            resolved_at_unix_ms: 1,
        };
        // Insert with a fixed timestamp so we can test TTL expiry without
        // depending on the real wall-clock.
        cache.entries.insert(
            r.id.clone(),
            CacheEntry {
                resolved: r.clone(),
                cached_at_unix_ms: 1,
            },
        );
        save_cache(&dir, &cache).unwrap();
        let loaded = load_cache(&dir);
        assert_eq!(loaded.entries.get("x").unwrap().resolved.size_bytes, 12345);
        // Fresh: now - cached_at < TTL → Some.
        let hit = cache_get(&loaded, "x", 100).unwrap();
        assert_eq!(hit.size_bytes, 12345);
        // Stale: now - cached_at >= TTL → None.
        let miss = cache_get(&loaded, "x", 1 + CACHE_TTL_MS);
        assert!(miss.is_none(), "expected stale entry to expire");
    }
}