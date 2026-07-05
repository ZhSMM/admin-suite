# Admin Suite LLM 集成 — 设计文档 (v2 含离线兜底)

> 版本：v0.6.0-rev2 草案  
> 状态：待评审  
> 变更重点：新增 **"本地小模型兜底"** —— 应用首次启动时若用户没配云端供应商，提示下载一个 ~1GB 的 GGUF 小模型 + 自动启动本地 llama-server，离线/在线都能用

---

## 0. 目标与边界（更新）

### 0.1 用户故事（新增第 4 条）

| 角色 | 故事 |
| --- | --- |
| 系统管理员 | 统一管理多个 LLM 供应商，按场景切换模型，控制谁能用云端 API |
| 普通开发者 | 断网环境下用本地 Ollama / 兜底模型干活；连上 Wi-Fi 一键切回云端 |
| 合规审计 | 谁、什么时候、调用了哪个模型、花了多少 token、多少钱 |
| **新装用户** | **首次启动没配过任何 LLM，应用主动提示"下载一个 1GB 的小模型让你断网也能用"，同意后自动完成** |

### 0.2 非目标（不变）

- 训练 / 微调
- 图像生成
- TTS / STT
- Agent 框架
- 进程内嵌推理（candle / mistral.rs）— 详见 §2 为什么不用

---

## 1. 总体架构（重写）

```
┌─────────────────────────────────────────────────┐
│  Vue 组件层                                       │
│  - AI 工具页（Chat / Translate / Explain …）       │
│  - 管理员页（Providers / Models / Usage）          │
│  - Settings → AI tab（含"下载兜底模型"）          │
│  - 首次启动引导（"是否下载离线模型？"对话框）     │
└─────────────────────────────────────────────────┘
                ▲             │
       invoke   │             │ emit("llm:chunk" / "llm:download_progress")
                │             ▼
┌─────────────────────────────────────────────────┐
│  Tauri Commands                                   │
│  llm_*（v0.6.0）                                  │
│  + llm_fallback_*（本版本新增）                    │
│    llm_fallback_status      查状态              │
│    llm_fallback_download    启动下载             │
│    llm_fallback_cancel      取消下载             │
│    llm_fallback_use         设为默认 provider    │
│    llm_fallback_delete      删除本地模型         │
└─────────────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────┐
│  Provider Adapter 适配层（v0.6.0）                  │
│  + FallbackAdapter（专门走本地 llama-server）      │
└─────────────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────┐
│  Fallback Engine 管理（src-tauri/src/llm/fallback/）│
│  ┌─────────────────────────────────────┐         │
│  │ ModelRegistry  内置候选模型清单     │         │
│  │ Downloader     多镜像下载 + 校验     │         │
│  │ ServerManager  启动 / 监控 llama-server │
│  │ HealthChecker  健康检查 + 自动重启  │         │
│  └─────────────────────────────────────┘         │
│                                                  │
│  本地存储：                                       │
│  <data_dir>/llm/bin/llama-server(.exe)           │
│  <data_dir>/llm/models/<id>.gguf                 │
│  <data_dir>/llm/fallback_state.json              │
└─────────────────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────────────┐
│  Storage / Crypto / Audit                         │
└─────────────────────────────────────────────────┘
```

---

## 2. 为什么选 llama.cpp server 而不是进程内嵌推理

| 方案 | 优点 | 缺点 | 选择 |
| --- | --- | --- | --- |
| **llama.cpp server** | 业界事实标准 / OpenAI 兼容 API 开箱即用 / 隔离性好 / 升级简单（换 binary 即可） | 多了个进程；端口要管；需要随平台下 binary | ✅ **本方案** |
| **进程内 candle / mistral.rs** | 启动更快 / 不占端口 | Rust 编译依赖多 / 与 model 版本耦合 / GGUF 支持一度滞后 / bundle 体积暴涨 | ❌ 太重 |
| **Ollama** | 用户认知度高 | 需要用户自己装；跨平台安装路径不一致；无法"零配置开箱即用" | ❌ 违背"自动下载"目标 |
| **llamafile** | 单文件可执行 / 极简 | 大模型 OOM 风险 / 多平台预编译版本维护成本 | ⚠️ 备选，v0.6.x 不引入 |
| **LM Studio** | GUI 友好 | 同 Ollama —— 要用户装 | ❌ |

**结论**：用 **llama.cpp server** —— 跟我们的 OpenAI-Compatible adapter 完美复用，只需要在 Settings 加一个 FallbackAdapter 把请求转接到 `127.0.0.1:<random_port>/v1/...`。

---

## 3. 兜底模型候选清单（内置硬编码）

```rust
// src-tauri/src/llm/fallback/models.rs
pub struct FallbackModel {
    pub id: &'static str,             // 内部 id
    pub display_name: &'static str,   // UI 显示
    pub family: &'static str,
    pub parameter_count: &'static str, // "1.5B" / "3B"
    pub quantization: &'static str,   // "Q4_K_M"
    pub size_bytes: u64,              // 压缩后
    pub sha256: &'static str,         // 校验用
    pub context_window: u32,
    pub primary_url: &'static str,    // HuggingFace
    pub mirror_urls: &'static [&'static str],  // 失败时按顺序试
    pub gguf_filename: &'static str,
    pub min_ram_gb: u32,              // 推荐最小内存
    pub quality_tier: QualityTier,    // PowerSaver / Balanced / Quality
}

pub static FALLBACK_MODELS: &[FallbackModel] = &[
    FallbackModel {
        id: "qwen2.5-0.5b-instruct-q4km",
        display_name: "Qwen2.5 0.5B Instruct (Q4_K_M)",
        family: "qwen2.5",
        parameter_count: "0.5B",
        quantization: "Q4_K_M",
        size_bytes: 467 * 1024 * 1024,
        sha256: "…",  // release 时填
        context_window: 8192,
        primary_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
        mirror_urls: &[
            "https://hf-mirror.com/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
            "https://www.modelscope.cn/api/v1/models/Qwen/Qwen2.5-0.5B-Instruct-GGUF/repo/files/master/qwen2.5-0.5b-instruct-q4_k_m.gguf",
        ],
        gguf_filename: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
        min_ram_gb: 2,
        quality_tier: QualityTier::PowerSaver,
    },
    FallbackModel {
        id: "qwen2.5-1.5b-instruct-q4km",  // ★ 默认推荐
        display_name: "Qwen2.5 1.5B Instruct (Q4_K_M)",
        family: "qwen2.5",
        parameter_count: "1.5B",
        quantization: "Q4_K_M",
        size_bytes: 1_100 * 1024 * 1024,
        sha256: "…",
        context_window: 8192,
        primary_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        mirror_urls: &[
            "https://hf-mirror.com/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
            "https://www.modelscope.cn/api/v1/models/Qwen/Qwen2.5-1.5B-Instruct-GGUF/repo/files/master/qwen2.5-1.5b-instruct-q4_k_m.gguf",
        ],
        gguf_filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        min_ram_gb: 4,
        quality_tier: QualityTier::Balanced,
    },
    FallbackModel {
        id: "llama-3.2-3b-instruct-q4km",
        display_name: "Llama 3.2 3B Instruct (Q4_K_M)",
        family: "llama3.2",
        parameter_count: "3B",
        quantization: "Q4_K_M",
        size_bytes: 2_050 * 1024 * 1024,
        sha256: "…",
        context_window: 8192,
        primary_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        mirror_urls: &[
            "https://hf-mirror.com/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        ],
        gguf_filename: "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        min_ram_gb: 6,
        quality_tier: QualityTier::Quality,
    },
    FallbackModel {
        id: "qwen2.5-3b-instruct-q4km",
        display_name: "Qwen2.5 3B Instruct (Q4_K_M)",
        family: "qwen2.5",
        parameter_count: "3B",
        quantization: "Q4_K_M",
        size_bytes: 2_050 * 1024 * 1024,
        sha256: "…",
        context_window: 8192,
        primary_url: "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
        mirror_urls: &[
            "https://hf-mirror.com/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
        ],
        gguf_filename: "qwen2.5-3b-instruct-q4_k_m.gguf",
        min_ram_gb: 6,
        quality_tier: QualityTier::Quality,
    },
];
```

> **为什么选 Qwen2.5-1.5B 当默认**：1GB 大小 / 4GB RAM 即可跑 / 中文 + 英文都能用 / 工具调用（function calling）官方支持 / 社区维护活跃。  
> **0.5B 太弱**（简单问答都吃力），**3B 又占 RAM**。1.5B 是甜点。

---

## 4. llama-server 二进制管理

### 4.1 来源

- 官方 release：`https://github.com/ggerganov/llama.cpp/releases`
- Windows：`llama-<version>-bin-win-cuda-cu12.2-x64.zip`（含 `llama-server.exe`）
- macOS / Linux：略，v0.6.0 优先 Windows

### 4.2 下载与存储

- 路径：`<data_dir>/llm/bin/llama-server(.exe)`
- 不在 installer 里打包（体积 + 版本更新麻烦）
- 第一次启动检测：文件不存在 → 后台下载 + 解压
- 版本号写死到 `LLAMA_SERVER_VERSION = "b3900"`（按 release 时刻）
- SHA-256 硬编码校验

### 4.3 进程管理

```rust
// src-tauri/src/llm/fallback/server.rs
pub struct LlamaServerProcess {
    child: tokio::process::Child,
    base_url: String,  // http://127.0.0.1:<port>/v1
    port: u16,
    started_at: Instant,
    idle_timer: JoinHandle<()>,  // 5min 无请求自动退出
    health_handle: JoinHandle<()>,
}

impl LlamaServerProcess {
    pub async fn spawn(model_path: &Path) -> Result<Self>;
    pub async fn is_healthy(&self) -> bool;
    pub async fn ensure_running(&self) -> Result<()>;
    pub fn base_url(&self) -> &str;
    pub async fn shutdown(self);
}
```

**生命周期**：
- **懒启动**：第一次 chat 调用时 spawn
- **空闲退出**：5 分钟无调用自动 kill（省内存）
- **健康检查**：每 30 秒 GET `/health`，失败 → 自动重启（指数退避最多 3 次）
- **进程退出时清理**：Tauri `app.on_exit` hook 里 graceful shutdown

**端口**：从 `39100` 起递增探测空闲端口（写到 fallback_state.json，下次启动沿用）

**资源限制**：
- 启动参数：`-ngl 20`（默认 20 层 GPU offload，没显卡自动退到 CPU）
- 上下文长度：固定 4096（够用，1.5B 跑 8k 慢）
- 批大小：默认 512

---

## 5. 下载管理

### 5.1 状态机

```
NotDownloaded
    ↓ 用户点击 / 引导触发
Downloading(progress_pct, bytes_done, total_bytes, eta_seconds)
    ↓ 字节流读完
Verifying
    ↓ sha256 匹配 → Ready；不匹配 → HashMismatch
Ready
    ↓ 用户手动删除 / 卸载
NotDownloaded
```

任何状态出错 → `Error(message)`，重试时回到 `Downloading`。

### 5.2 下载流程（细节）

1. 前端 invoke `llm_fallback_download({ model_id: "qwen2.5-1.5b-instruct-q4km" })`
2. 后端查 `fallback_state.json`：
   - 状态是 `NotDownloaded` / `Error` / `HashMismatch` → 启动下载
   - 状态是 `Downloading` → 直接订阅进度事件（同一进程内多订阅）
   - 状态是 `Verifying` / `Ready` → 直接返回当前状态
3. 后端按顺序试 primary → mirror_urls[0] → mirror_urls[1] …，任一成功就停
4. 下载期间每 500ms emit `llm:download_progress`：
   ```json
   { "model_id": "...", "phase": "downloading", "bytes_done": 524288000, "total_bytes": 1153433600, "pct": 45.5, "speed_bps": 8388608, "eta_seconds": 75 }
   ```
5. 写文件用 `tokio::fs::File` + 8MB buffer
6. 完成后：`tokio::task::spawn_blocking` 算 SHA-256（CPU bound）
7. 校验通过 → 写 `Ready` 到 state.json
8. 校验失败 → 删文件 → 状态 `HashMismatch`，下次调用自动重下

### 5.3 断点续传（v0.6.1，v0.6.0 不做）

v0.6.0 失败就从头下。简单稳健。v0.6.1 加 HTTP Range。

### 5.4 取消

`llm_fallback_cancel({ model_id })` → 把内部 `tokio::sync::Notify` 触发，正在下载的 task 收到信号后 abort，删除半成品文件，状态回到 `NotDownloaded`。

---

## 6. 首次启动引导

### 6.1 触发时机

应用启动完成（bootstrap done）后，逻辑如下：

```
1. 检查 llm_providers 表是否为空
2. 检查 llm_fallback_state.json：fallback.enabled + fallback.model_id 已设
3. 检查对应模型文件是否存在
4. 都不满足 → 弹模态引导：
   "想下载一个 ~1GB 的小模型让你断网也能用 AI 工具吗？
    推荐：Qwen2.5-1.5B Instruct（约 1.1GB）
    [稍后] [开始下载]"
```

用户点 "稍后" → 关闭引导，下次启动还问。
用户点 "开始下载" → 写入 `fallback_state.json`（enabled=true, model_id=qwen2.5-1.5b-instruct-q4km），进入 §5 下载流程，进度条显示在 Settings → AI tab。

### 6.2 不强制

- 用户可以永久跳过（在 Settings → AI → "不再提醒"复选框）
- 用户已经配了云端供应商 + 不需要本地兜底 → 引导自动跳过
- 引导弹窗本身有 [×] 关闭按钮

---

## 7. FallbackAdapter 设计

```rust
// src-tauri/src/llm/providers/fallback.rs
pub struct FallbackAdapter {
    server: Arc<LlamaServerProcess>,
}

#[async_trait]
impl LlmProvider for FallbackAdapter {
    fn kind(&self) -> ProviderKind { ProviderKind::OpenAiCompat }  // 复用 OpenAI 兼容层

    async fn chat(&self, _ctx: &ProviderContext, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        // ctx.base_url 在 create_context 时被设成本地 server URL
        // 直接转给 reqwest OpenAI 客户端
        openai_compat::chat(_ctx, req).await
    }

    async fn chat_stream(&self, ctx, req, on_chunk) -> Result<ChatResponse, LlmError> {
        // 同 chat，走 OpenAI 流式
        openai_compat::chat_stream(ctx, req, on_chunk).await
    }
}
```

**关键**：FallbackAdapter 本质上把"本地 server URL"塞进 OpenAICompat，所有协议解析都复用现成代码——零增量复杂度。

### 7.1 自动注册

启动时如果检测到 `fallback_state.json.enabled = true` + 模型文件存在，就在 `llm_providers` 表里自动写入/更新一行：

```sql
INSERT OR REPLACE INTO llm_providers (id, code, name, kind, base_url, auth_type, api_key_enc, enabled)
VALUES ('p_fallback', '__fallback__', 'Local Fallback', 'openai_compat',
        'http://127.0.0.1:<port>/v1', 'none', NULL, 1);
```

模型同步注册：

```sql
INSERT OR REPLACE INTO llm_models (id, provider_id, code, display_name, capabilities, context_window)
VALUES ('m_fallback_<model_id>', 'p_fallback', '<model_id>', '<display_name>',
        '["chat","stream"]', <context_window>);
```

`base_url` 里的 port 写当前 server 监听端口，启动时更新。

### 7.2 路由策略

`decide_route()` 函数（v0.6.0 已设计）扩展：

```
if (no provider enabled) {
    if (fallback enabled + model ready + server up) {
        use fallback
    } else {
        return Error("No LLM configured. Go to Settings → AI to set one up, or download the fallback model.")
    }
} else if (settings.local_first = true) {
    if (any local provider available) {
        use local provider
    } else if (fallback ready) {
        use fallback
    } else if (any cloud provider available && network_up) {
        use cloud (with confirm dialog)
    } else {
        return Error(...)
    }
} else {
    use user-selected provider
    if (it's cloud && network down) {
        if (fallback ready) {
            auto-fallback with notice toast
        } else {
            return Error("Network unavailable and no offline model.")
        }
    }
}
```

---

## 8. 数据库扩展（V10 更新）

新增一张表记录 fallback 状态（不放在 SQLite 主库也行，但放一起方便审计）：

```sql
CREATE TABLE llm_fallback_config (
    id              INTEGER PRIMARY KEY DEFAULT 1,
    enabled         INTEGER NOT NULL DEFAULT 0,   -- 用户是否启用
    model_id        TEXT,                          -- 选中的 FALLBACK_MODELS.id
    notify_on_start INTEGER NOT NULL DEFAULT 1,   -- 启动时是否弹引导
    updated_at      TEXT NOT NULL,
    CHECK (id = 1)  -- 单行
);
```

实际下载进度 / server 健康状态写在 `<data_dir>/llm/fallback_state.json`：

```json
{
  "enabled": true,
  "model_id": "qwen2.5-1.5b-instruct-q4km",
  "phase": "Ready",
  "model_path": "C:\\Users\\...\\admin-suite\\llm\\models\\qwen2.5-1.5b-instruct-q4_k_m.gguf",
  "llama_server_path": "C:\\Users\\...\\admin-suite\\llm\\bin\\llama-server.exe",
  "llama_server_version": "b3900",
  "port": 39100,
  "last_started_at": "2026-07-05T08:42:00Z",
  "last_health_at": "2026-07-05T08:55:12Z",
  "last_error": null,
  "total_downloads": 1,
  "total_runtime_seconds": 1800
}
```

为什么 JSON 而不是 DB：fallback 状态读写频繁 + 偶尔 race，文件 + Mutex 更简单，且崩溃时也不会丢 DB 一致性。

---

## 9. 新增 Tauri 命令（fallback 部分）

| 命令 | 入参 | 权限 | 说明 |
| --- | --- | --- | --- |
| `llm_fallback_list_models` | — | `llm:use` | 返回 `FALLBACK_MODELS` 静态列表（含元数据，**不含**下载 URL / SHA，要拿得调专门的命令） |
| `llm_fallback_status` | `{model_id?}` | `llm:use` | 当前状态（state.json） |
| `llm_fallback_download` | `{model_id}` | `llm:manage`（第一次需） | 启动下载，返回订阅用的 request_id |
| `llm_fallback_cancel` | `{model_id}` | `llm:manage` | 取消进行中 |
| `llm_fallback_delete` | `{model_id}` | `llm:manage` | 删 GGUF 文件 |
| `llm_fallback_set_default` | `{model_id, enabled}` | `llm:manage` | 写 fallback_config 表 |
| `llm_fallback_dismiss_startup_prompt` | — | `llm:manage` | 把 notify_on_start 置 0 |

事件通道：
- `llm:download_progress` — 下载进度
- `llm:download_complete` — 完成
- `llm:download_error` — 失败
- `llm:server_status` — server 健康变化

---

## 10. 前端：Settings → AI tab 增强

新增 Section："Offline Fallback"

```
┌─────────────────────────────────────────────────────┐
│ Offline Fallback                                     │
│                                                      │
│  ○ Qwen2.5 0.5B  (467 MB)   极省内存,基础对话      │
│  ● Qwen2.5 1.5B  (1.1 GB)   推荐 / 默认             │
│  ○ Llama 3.2 3B  (2.0 GB)   更好质量,需 6GB 内存    │
│  ○ Qwen2.5 3B    (2.0 GB)   中文更强,需 6GB 内存    │
│                                                      │
│  Status: ✓ Ready (downloaded 2 days ago, 5h runtime) │
│                                                      │
│  [重新下载]  [更换模型]  [删除模型]                  │
│                                                      │
│  □ 启动时提示下载（无供应商时）                      │
│  ☑ 自动作为离线兜底                                  │
└─────────────────────────────────────────────────────┘
```

进度条在下半部分：

```
下载中: ███████████░░░░░░░░░  62%   745 MB / 1.1 GB
        8.2 MB/s  剩余约 50 秒
[取消]
```

---

## 11. 新增依赖

### 11.1 Rust

```toml
[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["stream", "json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
tokio-stream = "0.1"
sha2 = "0.10"   # 已有
zip = { version = "0.6", default-features = false, features = ["deflate"] }
rand = "0.8"    # 已有
```

> 注意：现有 `rusqlite` 是同步的，`tokio` 是新引入的异步 runtime。要么全 tokio，要么把 fallback engine 的 async 部分用 `std::thread` + `mpsc`。我建议 **引入 tokio**——因为 reqwest 的流式 API 是 async 的，强行同步会阻塞 UI。

### 11.2 前端

无新依赖。复用 `crypto.randomUUID()` 做 request_id。

---

## 12. 测试矩阵

### 12.1 后端

| 测试 | 覆盖 |
| --- | --- |
| `fallback::models::parse_state_json_round_trip` | state.json 序列化 |
| `fallback::models::state_machine_transitions_valid` | 状态机合法转换 |
| `fallback::models::state_machine_rejects_invalid` | 非法转换拒绝 |
| `fallback::server::port_allocator_returns_free` | 端口探测（用 mock netstat） |
| `fallback::server::idle_kill_after_5min` | 5min 空闲退出（用快进 timer） |
| `fallback::server::auto_restart_on_crash` | crash 后指数退避重启 |
| `download::resume_from_partial_bytes`（v0.6.1） | 断点续传 |
| `download::sha256_verifies_match` | 校验通过 |
| `download::sha256_verifies_mismatch_deletes` | 校验失败 → 删除 |
| `download::mirror_fallback_on_primary_fail` | 镜像 fallback |
| `route::decide_no_provider_no_fallback_errors` | 路由决策：都没有 |
| `route::decide_no_provider_with_fallback_uses_it` | 路由决策：用兜底 |
| `route::decide_cloud_down_auto_switches_to_fallback` | 云端断 → 自动兜底 |
| `route::decide_local_first_prefers_local` | 本地优先策略 |
| `migration::v10_creates_fallback_config_table` | V10 schema |

### 12.2 前端

- Settings AI tab: 切换模型触发下载流程，进度条正确更新
- 取消下载 → UI 状态正确回退
- 启动引导：3 种场景下都正确触发/不触发
- DownloadProgressBar: 进度数字与后端事件一致

---

## 13. 风险与缓解

| 风险 | 缓解 |
| --- | --- |
| 下载 1GB 失败率高（HuggingFace 国内慢） | 3 个镜像按顺序试；UI 显示当前试的镜像；失败后自动切下一个 |
| 用户磁盘空间不够 | 下载前检查 `available_space >= size_bytes * 1.2`，不够直接拒绝 |
| 杀毒软件误杀 llama-server.exe | 文档说明 + 添加 Windows Defender 排除项的说明链接 |
| llama-server 启动慢（首次 5-10s） | 启动时显示 "正在启动本地引擎…" 友好提示；后台预热可选 |
| 模型质量差（用户期望 GPT-4 级回答） | UI 显式标注 "这是离线小模型，回答质量有限" |
| 用户卸载时留下 ~1GB 模型 | 在卸载说明里写清；提供"删除本地模型"按钮 |
| 端口被占 | 端口探测从 39100 起递增，最多试 10 次 |
| 同时启多个 llama-server 实例 | 单例锁：进程内有 Mutex，第二次 spawn 直接复用已有 |
| NVIDIA / AMD / Intel GPU 检测 | 启动参数 `-ngl 20` 自动协商，CPU fallback 也能跑 |
| 下载期间断网 | 写到一半的文件保存，下次启动续传（v0.6.1） |

---

## 14. 实施计划（更新）

### v0.6.0 — **基础 + 兜底一起做**（约 5 天）

| Day | 工作 |
| --- | --- |
| 1 | 引入 tokio + reqwest；V10 migration（含 llm_fallback_config）；fallback state machine + JSON 持久化；FALLBACK_MODELS 静态表 |
| 2 | Downloader：HTTP 下载 + SHA-256 校验 + 多镜像 fallback + 进度事件 + 取消 |
| 3 | LlamaServerProcess：spawn / port 探测 / health check / idle kill / auto-restart；下载 llama-server.exe |
| 4 | FallbackAdapter + 自动注册 provider/model 行 + decide_route 扩展；Tauri 命令 + 事件 |
| 5 | 前端：Settings → AI tab 完整实现 + 启动引导对话框 + 进度条组件 + audit-i18n + cargo test + vitest |

### v0.6.1（不变）：Google 适配器 + Translate/Explain/Summarize + Crash Explain 联动 + Locales AI Backfill

### v0.6.2（不变）：Usage 面板 + 角色预算 + CSV 导出

### v0.6.3（不变）：熔断器 + Failover 链 + 本地优先策略

### v0.6.4（不变）：剩余 AI 工具 + 离线队列 + **断点续传**

---

## 15. 验收标准（更新）

### 新增

- [ ] 在没配任何供应商、连网的环境下首次启动，弹出下载引导，点同意后 1.1GB 模型在 <5 分钟下载完
- [ ] 拔网线后，AI Chat 仍能跑通，延迟 <2s/token
- [ ] 镜像切换：mock HuggingFace 失败 → 自动切 hf-mirror.com
- [ ] SHA-256 校验：手动改坏 GGUF 文件 → UI 显示 "文件已损坏，请重新下载"
- [ ] 端口被占：启一个占用 39100 的进程 → fallback server 用 39101
- [ ] 5 分钟无调用 → server 自动退出，内存释放（任务管理器验证）
- [ ] 手动 kill server 进程 → 30s 内自动重启
- [ ] DB 文件里搜不到明文 API key
- [ ] 卸载 app 后 `<data_dir>/llm/` 残留 → 提供 "删除本地模型" 按钮或在卸载脚本里清

### 不变
- cargo test --lib 全绿，+15 个新测试
- vitest 全绿
- npm run build / cargo build --release 零 warning
- audit-i18n.py 零命中

---

## 16. 需要你确认的关键决策

| # | 决策点 | 默认建议 |
| --- | --- | --- |
| 1 | 默认推荐模型 | **Qwen2.5-1.5B-Instruct-Q4_K_M**（1.1GB，甜点） |
| 2 | 模型候选清单数量 | 4 个（0.5B / 1.5B / 3B Llama / 3B Qwen），用户可换 |
| 3 | llama-server 引入方式 | **首次启动后台下载**（不进 installer） |
| 4 | 启动引导 | **首次启动 + 没供应商 + 没装过兜底 → 弹模态**，用户可永久跳过 |
| 5 | 端口策略 | 39100 起递增，最多试 10 |
| 6 | idle kill | 5 分钟无调用自动退出 |
| 7 | 镜像顺序 | HF → hf-mirror → ModelScope |
| 8 | 断点续传 | **v0.6.0 不做**，v0.6.4 加 |
| 9 | GPU 支持 | 自动协商 `-ngl 20`，无 GPU 退 CPU |
| 10 | 是否要小模型可调节 temperature | **v0.6.0 固定 0.7**，v0.6.1 暴露 |
| 11 | 单实例 vs 多实例 | **单实例**（一次只跑一个 llama-server） |
| 12 | 下载磁盘检查 | **>= 模型大小 × 1.2**，不够拒绝并提示 |
| 13 | 自动删除过期模型 | **v0.6.0 不做**（让用户手动管） |

---

> 评审签字：__________  
> 进入开发：__________