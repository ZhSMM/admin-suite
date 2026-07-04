# Admin Suite — 开发指南

> 给后续接手者的“**这本书里没写**”式速查手册。不是 API 文档,是踩坑总结 + 怎么改。

---

## 0. 一图流(架构)

```
┌───────────────────── Vue 3 前端 (src/) ─────────────────────┐
│  views / stores(Pinia) / router(权限守卫) / i18n / themes  │
│         ↓ invoke()                                          │
│  Tauri Runtime (WebView2 on Windows)                       │
│         ↓                                                   │
│   ┌─────── Tauri commands (lib.rs) ────────┐               │
│   │  auth_login / users_list / roles_list / …              │
│   └────────────────┬───────────────────────┘               │
│                    ↓                                       │
│   ┌───────── commands/*.rs ──────────┐                     │
│   │  require_permission(token, code) │ ←─SessionStore      │
│   │  db.with_conn(|c| …)             │ ←─parking_lot Mutex │
│   │  db.with_tx(|tx| …)              │   (单连接,不可重入) │
│   └────────────────┬─────────────────┘                     │
│                    ↓                                       │
│   ┌────────── Db + migrate.rs (Flyway 风格) ──────────┐    │
│   │  V{n}__{desc}.sql + R__{desc}.sql                  │    │
│   │  flyway_schema_history (sha256 + 应用时间)          │    │
│   └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**关键约束:**
- 单进程单连接 SQLite,`parking_lot::Mutex` 守护。**不可重入** —— 详见 §6。
- 后端是**同步**的(不用 async)。Tauri 把命令扔到自己的 runtime 上执行。
- RBAC 三级:`user → role → permission`,菜单/资源都通过 `role_*` 关联表显式授权。
- 主题/语言存在 `resources` 表(JSON),前端按需 import 合并到 vue-i18n / CSS vars。

---

## 1. 准备开发环境

| 工具 | 版本 | 备注 |
|---|---|---|
| Node | 20.x | `E:\installation\node\` 在 PATH |
| Rust | stable | `E:\installation\rust\cargo\bin\` 在 PATH |
| Python | 3.x | 给 `gen-icons.py` 用的 |
| Visual Studio Build Tools | 2022,带 “C++ 桌面开发” | Tauri 链接到 WebView2 需要 MSVC |

```powershell
# PowerShell,启动 dev 时一次性设好
$env:Path = "E:\installation\node;E:\installation\rust\cargo\bin;" + $env:Path
cd "C:\Users\19114\.minimax-agent-cn\projects\admin-suite"
npm install
npm run tauri:dev      # 启动 Vite + Tauri,自动跑 V1..V6 迁移
```

首次启动会:
1. 创建 `%APPDATA%\com.admin-suite.app\` 目录(Windows 下 Tauri 默认 bundle id 映射的数据目录)。
2. 跑 Flyway 迁移,缺数据则种 V2 的默认超级管理员 `admin / admin123`。
3. 开 WebView2 窗口,自动登录 → Dashboard。

---

## 2. 目录结构(只列“会被改到的”)

```
admin-suite/
├── src/                              # 前端
│   ├── api/                          # Tauri invoke 包装,每个资源一个文件
│   ├── components/                   # 通用组件 (Sidebar / Header / Palette / …)
│   ├── i18n/{index.ts,locales/}      # 内置 zh-CN / en-US,可在 DB 资源里 override
│   ├── layouts/DefaultLayout.vue     # 登录后的整体布局(顶栏 + 侧栏 + 内容 + Palette)
│   ├── router/index.ts               # 路由表 + 权限守卫(beforeEach)
│   ├── stores/{auth,theme,locale,menu,palette}.ts  # Pinia
│   ├── themes/index.ts               # 主题应用器(CSS variables 注入)
│   ├── views/
│   │   ├── admin/                    # 系统管理(Users / Roles / Menus / Themes / Locales / Audit / Settings / Backups)
│   │   └── tools/                    # 11 个工具页面
│   └── main.ts
├── src-tauri/
│   ├── migrations/                   # Flyway 风格 SQL
│   │   ├── V1__init_schema.sql       # users / roles / permissions / menus / resources / audit_log
│   │   ├── V2__seed_acl.sql          # 默认权限 + 内置角色 + super-admin
│   │   ├── V3__seed_tools.sql        # 5 个 Tools + 菜单
│   │   ├── V4__seed_tools_more.sql   # 再加 6 个 Tools
│   │   ├── V5__add_menu_title_key.sql# menus.title_key 列
│   │   ├── V6__seed_string_crypto.sql# String Converter + Crypto
│   │   └── V7__seed_app_state.sql    # app_state 表 + settings/backup/permissions/menus
│   ├── src/
│   │   ├── auth/                     # argon2 密码 / SessionStore / require_permission
│   │   ├── commands/                 # 业务实现(auth/users/roles/permissions/menus/resources/audit/settings/backup/…)
│   │   ├── db/                       # Db (Mutex<Connection>) + migrate.rs
│   │   ├── models/                   # serde 序列化模型(给前端用)
│   │   ├── error.rs                  # AppError 枚举 → JSON 给前端
│   │   └── lib.rs                    # Tauri bootstrap + 命令注册 + 自动备份 + 应用 pending restore
│   └── tauri.conf.json
└── .github/workflows/
    ├── ci.yml                        # PR 检查:cargo check/test + vue-tsc + vite build
    └── release.yml                   # tag v* 触发,Windows runner 出 setup.exe + msi
```

---

## 3. 怎么加东西(常见任务)

### 3.1 加一个新 Tauri 命令(典型 CRUD 例子)

例:加一个 `settings_get(key) / settings_set(key, value)`。

**步骤:**

1. **数据先落 migration**(如果需要新表)
   - 新建 `src-tauri/migrations/V7__add_settings.sql`:
     ```sql
     CREATE TABLE IF NOT EXISTS settings (
       key   TEXT PRIMARY KEY,
       value TEXT NOT NULL,
       updated_at TEXT NOT NULL
     );
     ```
   - **不要在 V1..V6 里改。** 必须新建 V{n+1}。

2. **加 model**(`src-tauri/src/models/`)
   - 新建 `settings.rs`,定义 `pub struct Setting { key, value, updated_at }`,派生 `Serialize/Deserialize`,在 `models/mod.rs` 注册。

3. **加 command 实现**(`src-tauri/src/commands/settings.rs`)
   ```rust
   use crate::auth::rbac::require_permission;
   use crate::db::Db;
   use crate::auth::session::SessionStore;
   use crate::error::AppResult;
   
   pub fn get(db: &Db, _sessions: &SessionStore, _token: &str, key: &str) -> AppResult<String> {
       db.with_conn(|c| {
           let v: String = c.query_row(
               "SELECT value FROM settings WHERE key = ?", [key], |r| r.get(0))?;
           Ok(v)
       })
   }
   
   pub fn set(db: &Db, sessions: &SessionStore, token: &str,
              key: &str, value: &str) -> AppResult<()> {
       require_permission(sessions, token, "settings:write")?;
       db.with_conn(|c| {
           c.execute(
               "INSERT INTO settings(key, value, updated_at) VALUES(?,?,?)
                ON CONFLICT(key) DO UPDATE SET value=excluded.value,
                                                updated_at=excluded.updated_at",
               rusqlite::params![key, value, chrono::Utc::now().to_rfc3339()])?;
           Ok(())
       })
   }
   ```
   - 在 `commands/mod.rs` 注册 `pub mod settings;`。
   - **注意:** `require_permission` 必须在 `with_conn` 之前调用 —— 不要在闭包里调(见 §6 死锁)。

4. **在 `lib.rs` 注册 #[tauri::command]**
   ```rust
   fn settings_get(state: State<AppState>, token: String, key: String)
       -> Result<String, AppError> {
       settings_cmd::get(&state.db, &state.sessions, &token, &key).map_err(map_err)
   }
   fn settings_set(state: State<AppState>, token: String, key: String, value: String)
       -> Result<(), AppError> {
       settings_cmd::set(&state.db, &state.sessions, &token, &key, &value).map_err(map_err)
   }
   ```
   然后在 `tauri::generate_handler![…]` 列表里加上 `settings_get, settings_set`。

5. **前端 API 包装**(`src/api/settings.ts`)
   ```ts
   import { invoke } from '@tauri-apps/api/tauri'
   export const settingsApi = {
     get:   (token: string, key: string) =>
       invoke<string>('settings_get', { token, key }),
     set:   (token: string, key: string, value: string) =>
       invoke<void>('settings_set', { token, key, value }),
   }
   ```

6. **加权限 seed**(可选)
   - 编辑 `V2__seed_acl.sql`,加 `('settings:read', …), ('settings:write', …)`,然后给 super-admin 角色勾上。
   - **更安全的做法:** 新建 `V8__seed_settings_perms.sql`,不要改 V2。

7. **跑测试**
   ```powershell
   cd src-tauri; cargo test --lib
   ```
   至少加一个 happy-path 测试,见 §5。

### 3.2 加一个新菜单(管理员侧栏)

1. **菜单数据走 migration** —— 见 `V3__seed_tools.sql` 风格。在 `menus` 表 INSERT 一行。
   - 关键字段:`code`(唯一标识)、`parent_code`(空=顶级)、`path`(前端路由)、`permission_code`(空=纯展示)、`title` / `title_key`。
   - `title_key` 是 i18n key,例如 `'menu.my_thing'`。`SidebarItem.vue` 优先用 `t(title_key)`,找不到再 fallback 到原始 `title`。

2. **加路由**(`src/router/index.ts`)
   ```ts
   {
     path: 'system/my-thing',
     name: 'my-thing',
     component: () => import('@/views/admin/MyThing.vue'),
     meta: { requiresAuth: true, permission: 'mything:read', title: 'menu.my_thing' }
   }
   ```

3. **建组件** `src/views/admin/MyThing.vue`,然后在 i18n `zh-CN.ts` / `en-US.ts` 加 `menu.my_thing` 的翻译。

### 3.3 加一个新 Tool(实用工具页)

Tools 不需要 RBAC 单独权限,统一用 `tool:use`(见 V3 seed)。

1. **建组件** `src/views/tools/MyTool.vue`(看 `Hash.vue` 的结构,简单直接)。
2. **加路由** 同上,`path: 'tools/mytool'`,`permission: 'tool:use'`。
3. **migration 里加菜单:**
   ```sql
   INSERT INTO menus(id, code, parent_code, path, title, title_key,
                     permission_code, icon, sort) VALUES
     ('m_mytool', 'tool.mytool', 'group.tools', '/tools/mytool',
      'My Tool', 'tools.mytool.title', 'tool:use', 'Tool', 99);
   ```
4. **加 i18n key** 在 `src/i18n/locales/{en-US,zh-CN}.ts` 的 `tools` 命名空间下。

### 3.4 加一个新主题 / 新语言

**主题**(CSS variables 形式):

- 在 `src/themes/index.ts` 加一组 `{ name, vars: { '--el-color-primary': '…', … } }`。
- 进 Themes 页(管理员)可以在线选择;也可以在 V2 seed 里加默认主题。

**语言:**

- 在 `src/i18n/locales/` 下新建 `fr-FR.ts`(参考 `en-US.ts`),导出 messages 对象。
- 在 `src/i18n/index.ts` 把它加入 `bundledMessages`。
- 进 Locales 页 → Export → 选 source/target → 生成 JSON → 下载/导入到 DB。
- DB 资源会**覆盖** bundled(在 `useLocaleStore.load()` 里 merge,DB 优先)。

### 3.5 加一个新 Flyway 迁移

命名:`V{n}__{description}.sql`,序号单调递增。

- **不要改历史 V\* 文件**(已发布版本的用户库会卡 checksum)。
- `R__{description}.sql` 是 repeatable(每次 checksum 变了都重跑),只用于视图/函数/数据补丁。
- 启动时 `migrate::run_migrations(db, dir)` 自动跑;也提供 `migrate_cmd` 让前端手动触发。

### 3.6 加一个全局设置

走 `app_state` kv 表 + `commands/settings.rs` 校验白名单。详见 §10.1。

### 3.7 让新页面出现在 ⌘K 命令面板

只要新路由有 `meta.title` 就会被自动收录(见 §10.2)。不需要单独注册。

格式参考:
```sql
-- V7__add_settings.sql
CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- 同 migration 里可以塞多条
INSERT OR IGNORE INTO permissions(id, code, name, resource, action)
  VALUES ('p_settings', 'settings:read', 'Settings Read', 'settings', 'read');
```

**checksum 校验逻辑在 `src-tauri/src/db/migrate.rs`** —— SHA-256 已迁移 vs 文件实际,不一致就拒绝启动。

---

## 4. RBAC 流程(读懂这一节就懂了一半)

### 前端

`router.beforeEach`(`src/router/index.ts`):
1. 公开路由(`meta.publicRoute`) → 直接放行。
2. 未登录 → 跳 `/login?redirect=…`。
3. 路由声明了 `meta.permission` 但当前用户没有 → 跳 `/dashboard`(静默拒绝)。
4. 进入路由前确保 `localeStore.hydrate()`(主题/语言)。

`auth.ts` store:
- 登录成功后把 `permissions[]` 同步写到 `localStorage[admin-suite.permissions]`。
- **理由:** 页面刷新时 `me` 调用是异步的,守卫跑得比它快,所以用本地缓存保底。
- `hasPermission(code)` 的判断优先级:
  1. `is_super_admin` → `true`
  2. `'*:*'` 在列表里 → `true`(V2 给 admin 角色 seed 过)
  3. 精确匹配 `code`
  4. 通配 `resource:*`(例如 `user:*`)

### 后端

每个 `#[tauri::command]` 都接受 `state: State<AppState>` + `token: String`:

```rust
pub fn list(db: &Db, sessions: &SessionStore, token: &str, q: UserListQuery)
    -> AppResult<UserListPage>
{
    let me = require_permission(sessions, token, "user:read")?;
    // ... 业务逻辑 ...
}
```

`require_permission` 内部:
- 拿 token 查 SessionStore → 拿 AuthenticatedUser。
- `is_super_admin == true` → 直接放行。
- 否则检查 `permission_codes` 是否包含该 code(`*:*` / `resource:*` 通配)。
- 都没命中 → 返回 `AppError::PermissionDenied`。

---

## 5. 测试

### 后端 Rust

```powershell
cd src-tauri
cargo test --lib                     # 全部
cargo test --lib users::              # 单模块
cargo test --lib users::tests::users_list_does_not_deadlock  # 单测试
```

测试模板(参考 `commands/users.rs` 末尾的 `mod tests`):
- 用 `Db::open(temp_path)` 起一个临时 sqlite。
- 调用 `migrate::run_migrations(&db, &dir)` bootstrap schema。
- seed 测试数据。
- 跑被测函数,assert 结果。

**回归测试铁律:** 任何可能死锁的路径(比如 `with_conn` 内调用 `with_conn`),测试里**手动建线程跑**,别让测试 runner 自己挂。

### 前端

```powershell
npm run build      # = vue-tsc -b && vite build
```

- 没有 vitest 单元测试(整套逻辑偏 UI)。改逻辑时人工跑 `tauri:dev` 走一遍。
- TypeScript 严格模式,`vue-tsc` 会卡住所有 `any` 漏写。

---

## 6. ⚠️ 必读踩坑

### 6.1 parking_lot::Mutex 不可重入 —— users::list 死锁

**症状:** 某些命令"点了没反应",前端转圈后超时。release build 不抛错,直接 hang。

**根因:**
```rust
db.with_conn(|c| {                       // ← 拿到锁
    // ... 查询 users ...
    for u in users {
        let role_ids = load_role_ids(db, &u.id)?;  // ← 又调 db.with_conn
        // parking_lot Mutex 不是 std::sync.ReentrantMutex,这里就死锁
    }
})
```

**修法模式(`commands/users.rs` 是范例):**
```rust
// 顶层 wrapper(给"不在 with_conn 里"的调用方)
pub fn load_role_ids(db: &Db, user_id: &str) -> AppResult<Vec<String>> {
    db.with_conn(|c| load_role_ids_in(c, user_id))
}

// 内部 worker(给"已经在 with_conn 里"的调用方)
fn load_role_ids_in(c: &mut rusqlite::Connection, user_id: &str)
    -> AppResult<Vec<String>>
{
    let mut stmt = c.prepare("SELECT role_id FROM user_roles WHERE user_id = ?")?;
    let rows = stmt.query_map([user_id], |r| r.get::<_, String>(0))?
                   .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// 调用方
db.with_conn(|c| {
    let users = /* select users */;
    drop(stmt);  // ← 重要:丢 stmt 释放它对 c 的不可变借用
    for u in users {
        let ids  = load_role_ids_in(c, &u.id)?;   // 复用 c
        let codes = load_role_codes_in(c, &u.id)?;
    }
    Ok(())
})
```

**两条铁律:**
1. `with_conn` 闭包里**绝不再调** `with_conn` / `with_tx`。
2. 复用 `c` 之前先 `drop(stmt)`,否则 borrow checker 会给你 E0502 不可变借用冲突。

### 6.2 修改已发布的 V\* 迁移 = 致命

发布过的版本里,用户的 DB 已经记录了 V3 的 checksum。你改 V3 文件 → checksum 变 → 启动时拒绝。

- **必须**新建 V{n+1} 文件做改动。
- 例外:`R__*.sql`(repeatable)可以改,会自动重跑。

### 6.3 前端 i18n 全量覆盖(2026-07-04 重构)

**铁律:** `.vue` 文件里**不允许出现可见的硬编码字符串**。所有用户可见文本(标签、按钮、提示、占位符、表格列头、ElMessage 文案、占位符示例文字)都必须走 `t('xxx.yyy')`。

#### Key 命名规范

`<scope>.<page>.<element>` 三段式,scope 固定为以下之一:

| scope | 用途 | 例 |
|---|---|---|
| `app` | 应用级别(标题、tagline、版本) | `app.name`, `app.tagline` |
| `common` | **2+ 页面共用**的通用词(按钮、状态、动作) | `common.ok`, `common.delete`, `common.dark` |
| `auth` | 登录 / 登出 / 账号 | `auth.login`, `auth.passwordRequired` |
| `menu` | 侧边栏菜单的 `title_key` | `menu.users`, `menu.tools.base` |
| `notfound` | 404 页 | `notfound.title`, `notfound.back` |
| `dashboard` | 仪表盘 | `dashboard.welcome`, `dashboard.dataDir` |
| `users` / `roles` / `perms` / `menus` / `themes` / `locales` / `audit` | 对应的管理页面 | `users.create`, `roles.transfer.available` |
| `tools.<name>` | 每个独立工具 | `tools.base.title`, `tools.gen.version.v4` |

**单一来源原则:** 同样的英文/中文词如果在 2+ 页面出现,放 `common.*`,不要复制到各页面的 namespace 下。

#### Bundle 文件

`src/i18n/locales/zh-CN.ts` 和 `en-US.ts` 维护 bundled 默认值。两者**必须同步**(同样的 key 集合),否则 `Locales` 页导出"补齐缺失的 key"会生成残缺文件。

#### 数据库覆盖

`resources` 表存 DB 里的语言包,通过 `useLocaleStore.hydrate()` 加载到 i18n,**覆盖** bundled(同 key 用 DB 的,DB 没有的 fallback 回 bundled)。改 bundled 文件**不会**影响已经存到 DB 的语言包 —— 那份要走 "导出 → 重新导入" 流程更新。

#### 工具:审计遗漏

`scripts/audit-i18n.py` 扫描 `src/` 下所有 `.vue`,找出含 CJK 字符或疑似 UI 字符串的行。CI 流程里建议加:
```powershell
python scripts/audit-i18n.py | Tee-Object -FilePath i18n-audit.log
```
如果输出非空(除了 sample data / Element Plus internal layout 字符串),就别提 PR。

#### 模板写法

```vue
<!-- 标签 -->
<el-button>{{ t('common.save') }}</el-button>

<!-- 动态 label -->
<el-table-column :label="t('users.columns.username')" />

<!-- placeholder 用 binding,不是字符串 -->
<el-input :placeholder="t('auth.password')" />

<!-- 枚举翻译:用 computed 而非硬编码数组 -->
const modes = computed(() => [
  { label: t('tools.encode.mode.url'), value: 'url' },
  ...
])

<!-- 表格里循环枚举,label 是 i18n key,通过函数解析 -->
function algoLabel(k: string) { return t(`tools.hash.algo.${k}`) }
<el-tag>{{ algoLabel(row.algo) }}</el-tag>

<!-- 验证消息(注意要传函数,让 i18n 切换时动态求值) -->
const rules = {
  username: [{ required: true, message: () => t('users.validation.usernameRequired'), trigger: 'blur' }]
}
```

#### 注意点

- **ElMessageBox.confirm 的 title 第二个参数**目前传 `''` —— 这是 Element Plus 的 API 限制,要标题就传个 key:
  ```ts
  ElMessageBox.confirm(t('common.confirmDelete'), t('common.confirm'), { type: 'warning' })
  ```
- **Element Plus layout 字符串**(`'total, prev, pager, next'`)是组件内部 schema,不是 UI 文案,**不用**翻译。
- **示例/sample 数据**(如 StringConverter 的 `'你好,world! …'`)是给用户的演示输入,不是 UI 字符串,可以保留。
- 新加页面时:`zh-CN.ts` 和 `en-US.ts` 的同一 scope 同步加,跑一次 `audit-i18n.py` 确认 0 命中。

### 6.4 Tauri 1.6 + allowlist

- `tauri.conf.json` 用最小 allowlist `{"all": false}`,**别**用 `api-all` —— 1.6 的 `generate_context!` 会拒绝。
- 需要新能力时在 `allowlist` 下显式开。

### 6.5 Windows + ssh-agent

- ssh-agent 在 Windows 需要 admin 启动,而且有用户会话绑定。
- 项目里 `git push` 走 `~/.ssh/config` 的 `IdentityFile` 直连更省事,见 `~/.ssh/config` 的 github.com 块。
- 没 key:`ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N ""`,然后把公钥贴到 GitHub Settings → SSH keys。

---

## 7. 发布流程(打 tag → GitHub Action → 安装包)

### 7.1 前置 checklist

- [ ] `cd src-tauri && cargo test --lib` 全过
- [ ] `npm run build`(`vue-tsc` + `vite build`)全过
- [ ] 所有新功能、新菜单、新权限都在 migration 里有 seed 数据
- [ ] `package.json` 和 `src-tauri/tauri.conf.json` 的 `version` 都 bump

### 7.2 触发

```powershell
git add -A
git commit -m "feat: <描述>"
git push origin main
git tag -a v0.5.2 -m "v0.5.2 - <一句话总结>"
git push origin v0.5.2
```

`.github/workflows/release.yml` 在 `v*` tag push 时:
1. `windows-latest` runner
2. `actions/checkout@v4` → `setup-node@v4` → `dtolnay/rust-toolchain@stable`
3. `npm ci`
4. `tauri-action@v0`(脚本里 `releaseDraft: false`)
5. 产出 `Admin.Suite_{version}_x64-setup.exe` + `Admin.Suite_{version}_x64_en-US.msi`
6. 自动创建 GitHub Release,资产挂在 tag 下面

### 7.3 监控(避免误以为 release 失败)

实战经验:整套流水线**大约 10 分钟**,但 Windows runner 排队可能拉到 1h+。

用 cron 自检:
```powershell
mavis cron self poll-vXYZ --every 2m `
  --prompt "Run: python C:\Users\19114\.minimax-agent-cn\projects\admin-suite\poll-release.py"
```

`poll-release.py` 检查:
- `workflows/runs` 最新 status
- `/releases/tags/{tag}` 是否 published + 资产是否齐全
- 全齐 → exit 0,删 cron(`mavis cron delete mavis poll-vXYZ`),发用户链接
- 没齐 → exit 1,继续轮询

### 7.4 手动触发(纯构建不发版)

`release.yml` 也接受 `workflow_dispatch`,会在 Actions 里直接跑,但**不会**自动建 Release —— 产物在 run 的 artifacts 区下载。

---

## 8. 常用命令速查

```powershell
# —— 前端 ——
npm run dev                  # vite dev server(无 Tauri,纯前端)
npm run build                # vue-tsc + vite build(产物在 dist/)
npm run preview              # 本地预览 dist

# —— 后端 ——
cd src-tauri
cargo check                  # 类型检查,快
cargo test --lib             # 全测试
cargo clippy --all-targets   # lint(可选)

# —— 全栈开发 ——
npm run tauri:dev            # vite + Tauri,完整开发模式
npm run tauri:build          # 本地出安装包(target/release/bundle/)

# —— 资源重置 ——
# 删 %APPDATA%\com.admin-suite.app\ 下整个 app 数据目录,下次启动会重新跑迁移 + seed

# —— DB 调试 ——
# 安装 SQLiteStudio / DB Browser for SQLite,打开 app data 目录下的 *.sqlite
```

---

## 9. 风格约定

- **Rust:** `rustfmt` 默认风格;clippy 警告及时处理(项目现在 zero warning)。
- **TS:** `vue-tsc` 严格模式,无 `any`,无 `@ts-ignore`。导入顺序:`vue` → 第三方 → `@/api` → `@/stores` → 相对。
- **Vue:** `<script setup lang="ts">` + Composition API,不用 Options API。
- **i18n:** 文案加 key,不准在 template 里写死中英文字符串(除了调试日志)。
- **错误:** 后端走 `AppError` 枚举(NotFound / Conflict / PermissionDenied / BadRequest / Unauthorized / Internal),`map_err` 转成 JSON 给前端。前端 catch 时按 `e.code` 判断。
- **日志:** 关键路径写 `println!("[module] …")`(开发期),audit 操作走 `commands::audit::log(…)`。

---

## 10. 全局设置 (Settings) 和 命令面板 (Command Palette)

### 10.1 Settings 架构

**`app_state` kv 表** 存所有全局设置,值全部是 TEXT,后端按 key 做白名单 + 范围校验。

新增一个设置的步骤:
1. 在 `src-tauri/migrations/V{n+1}__add_xxx_settings.sql` 加 INSERT OR IGNORE 一行默认。
2. 在 `src-tauri/src/commands/settings.rs::validate` 加一个分支:解析 + 范围检查 + 错误信息。
3. 在 `src/views/admin/Settings.vue` 的 `form` / `apply` / `save` 三个函数各加一行同步。
4. 在 `src/i18n/locales/{zh-CN,en-US}.ts` 加 `settings.*` 命名空间文案。

如果新设置需要在**后端其他模块**读取(比如 `session.timeout_minutes`),用 `commands::settings::get_or(db, "session.timeout_minutes", "480")`。`get_or` 永远不抛错(找不到 key 返回默认值),适合 bootstrap 阶段用。

### 10.2 命令面板 (⌘K)

**位置:** `src/components/CommandPalette.vue`,挂载在 `DefaultLayout.vue` 的全局 root。

**工作原理:** 启动时 `router.getRoutes()` 枚举所有带 `meta.title` 的路由,转成 `PaletteItem`,按 i18n key + path 做 includes 模糊匹配。`Enter` 跳路由,`↑/↓` 切换候选,`Esc` 关闭。

**新增可搜索项**有两条路径:
- 路由表里有 `meta.title` 自动出现,不用改 Palette。
- 路由之外的(比如只想在 Palette 里出现的快捷操作)在 `CommandPalette.vue` 的 `items` computed 末尾追加。

**禁用开关:** `settings.ui.command_palette = false` 时,只关闭 Ctrl+K 监听 —— 不删组件、不破坏其他逻辑。

### 10.3 数据库备份 / 还原

**位置:** `src-tauri/src/commands/backup.rs`,前端 `src/views/admin/Backups.vue`。

**核心机制:**
- `VACUUM INTO '<path>'` 拿一致快照(不阻塞读写)。
- 备份文件存 `<data_dir>/backups/admin-suite-YYYYMMDD-HHmmss.sqlite`,元数据来自 `flyway_schema_history.installed_on`(拿不到就 fallback 到 mtime)。
- 还原不直接改运行中的库(Windows 上文件被锁),而是写一个 `<data_dir>/.restore_pending` flag 文件,要求重启应用。`bootstrap` 启动时检查 flag,先 swap 库再开 connection。
- swap 前把当前库备份成 `pre-restore-<时间>.sqlite`,留一个回滚锚点。
- 路径合法性:任何 backup 操作都做 `canonicalize().starts_with(canonical_backups_dir)` 检查,拒绝 path traversal。

**自动备份:** bootstrap 里 `backup_cmd::maybe_auto_backup(db, data_dir)` 由 `backup.auto_on_start`(默认 true)开关控制。trim 按 `backup.keep_count` 保留最新的 N 个,超了删最旧。

**新增一个 backup 钩子(比如云上传):** 在 `backup.rs::create_backup` 末尾追加,不要在 `create` wrapper 里 —— wrapper 还要负责 trim。

### 10.4 内置语言包刷新

`resources` 表里存了 `en-US` / `zh-CN` 两行内置 locale,前端 `useLocaleStore.hydrate()` 从这里读它们的 messages。

**坑:** V2 seed 时只有 ~150 keys。后续给 bundle 加的新 key(比如 `settings.*`, `backups.*`, `palette.*`,各种 validation message)只更新了 `src/i18n/locales/*.ts` 文件,DB 那两行没人同步 —— 用户激活内置 locale 时新 key 走 en-US fallback,Locales 导出"补齐缺失 key"会给新 key 填空字符串。

**修法:** V8 migration 用 UPDATE 把内置 locale 的 `content` 列刷成当前 .ts bundle 的 JSON。文件由 `scripts/gen-v8-locale-refresh.py` 从 `src/i18n/locales/{en-US,zh-CN}.ts` 生成,所以:

1. 加新 key → 改 .ts
2. 跑 `python scripts/gen-v8-locale-refresh.py` → 生成 `V8__refresh_builtin_locales.sql`(覆盖)
3. 提 PR,review 生成的 SQL(diff 应该只动 content 列)
4. CI 的 `full_migration_suite_applies_clean` 测试会 assert `content.messages` 至少 400 keys —— V8 跑过后断言会失败,如果脚本忘了跑

**为什么 UPDATE 不是 REPLACE:** 保留 row id 避免破坏 `role_menus` / `active` 指针。`built_in=1` 标志也不变,这样"不能删内置 locale"的逻辑继续生效。用户想自定义就 export + import 成不同 code(比如 `zh-CN-company`)。

### 10.5 多语言生成

- `scripts/gen-zh-tw.py` — 用 opencc s2tw 把 `zh-CN.ts` 转繁体 + 输出 `dist/zh-TW.json`(Locales 页面"导入"格式)
- `scripts/gen-v8-locale-refresh.py` — 见 §10.4

跑之前 `pip install opencc-python-reimplemented`(项目没装,因为用户不一定会下繁中包)。

### 10.6 审计日志查询增强

`commands/audit::list` 的 `AuditQuery` 现在支持以下过滤:

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `actor_id` | i64 | 操作者用户 id |
| `action` | `&str` | 模糊匹配(`LIKE %action%`),大小写不敏感 |
| `resource` | `&str` | 模糊匹配 `target` 列(资源类型或路径) |
| `payload_search` | `&str` | 在 `payload` 列(`details`)上做 `LIKE %x%` |
| `from` / `to` | `i64` | unix 秒,闭区间 |
| `page` / `page_size` | i64 | 默认 0/50 |

后端测试在 `commands/audit::tests::filter_by_actor / filter_by_action_like / filter_by_resource_and_payload / filter_by_time_window / filters_combine_with_and` —— 加新过滤字段时按同一模式加一条单测。

前端 `views/admin/Audit.vue` 有 filter card:`action` / `actor` / `resource` / `payload` 四个文本框 + 起始 / 结束时间选择器 + 1h / 24h / 7d 快捷按钮 + Reset。每次输入触发 `scheduleReload`(250ms debounce)。i18n key 都在 `audit.filter.*` 命名空间。

### 10.7 最近使用 / 收藏 (Recent / Favorites)

纯前端 localStorage,Pinia store 是 `useRecentStore`(在 `src/stores/recent.ts`):

| 数据 | key | 上限 |
| --- | --- | --- |
| 每个 tool 的 recent 列表 | `admin-suite.recent.<tool-id>` | 10 |
| 全局 favorites | `admin-suite.favorites` | 50 |

写入时机:`src/composables/useToolRecorder.ts` 在 tool 页 mount 时保存一次,route 切换时(debounce 1s)再保存一次。tool 自身提供 `sanitize(snapshot)` 回调,负责抹掉 secret(Hash 工具只存 algorithm + truncated head;Crypto 只存 algorithm;Generate 不存密码学状态)。

UI:
- `RecentDrawer.vue` —— HeaderBar 右上角的 Clock 图标打开,两个 tab(Recent / Favorites)。点记录会派发 `CustomEvent('admin-suite:restore-snapshot')` 然后 `router.push`;tool 页在 `onMounted` 里监听这个 event 并把 snapshot 写回本地状态。
- `PinButton.vue` —— HeaderBar 工具名旁边的星标,点击 toggle 当前 tool 的 favorite。
- `CommandPalette.vue` —— query 空时优先列出 favorites,然后按 route 注册顺序补齐(最多 12 项)。

**新增一个 tool 的录制:** 在 tool 页 `<script setup>` 顶部 `const { snapshot, restore, pinned, togglePin } = useToolRecorder({ id: 'my-tool', t, sanitize: (s) => s })`,然后给按钮和输入框加 `v-model` 绑到 snapshot 即可。Route 注册时保证 `meta.title` + `meta.icon` 就能出现在 Palette。

### 10.8 前端测试 (Vitest)

`npm test` 跑 `vitest run`,用例在 `src/**/__tests__/*.spec.ts`。

- `vitest.config.ts` —— `happy-dom` env,`cache: false`,`cacheDir: '.vitest-cache'`,`@` alias 指向 `src/`。
- `test/setup.ts` —— 公共 stub(目前只 reset vue-i18n singleton 状态)。
- 当前覆盖:`useLocaleStore`(4 个 case,关键是 `apply()` 顺序)、`useRecentStore`(10 个 case)。

**坑:vue-tsc 增量缓存会让 vitest 加载陈旧 .js。** `npm run build` 会把 `*.tsbuildinfo` 写进仓根目录,下次 `npm test` 偶尔会用 `tsconfig.tsbuildinfo` 里缓存的产物。如果遇到"改了源文件但用例仍然按旧行为跑"的现象,删 `tsconfig.tsbuildinfo` + `node_modules/.vite/` 再试。`vitest.config.ts` 里已经 `cache: false` 但 `tsconfig.tsbuildinfo` 是 tsc 自己管的,vitest 管不了。

---

有疑问先翻这一份 —— 90% 的"为什么这么写"都能在这里找到答案。如果发现新坑,补到对应小节。