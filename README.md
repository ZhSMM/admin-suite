# Admin Suite

基于 **Rust + Tauri + Vue 3** 的桌面端后台管理系统骨架。
覆盖四大需求：

1. **SQLite + Flyway 风格迁移**：所有 schema 改动通过 `src-tauri/migrations/V{n}__{desc}.sql` 版本化,启动时校验 checksum 后按序执行,失败回滚。
2. **RBAC 菜单/资源授权**：用户 → 角色 → 权限三级模型,菜单也通过 `role_menus` 显式授权,前后端双重校验。
3. **管理菜单优先**：登录后侧边栏默认展示 `系统 → 用户/角色/菜单/权限/主题/语言/审计`,后续可平滑接入业务菜单。
4. **主题 + 多语言可插拔**：内置三套主题 (Light / Dark / Ocean),中英文两套语言包;JSON 资源存放在 `resources` 表中,前端按需 import 即可,任何主题/语言都可被覆盖或新增。

---

## 目录结构

```
admin-suite/
├── src/                          # Vue 3 前端
│   ├── api/                      # Tauri invoke 包装 + 类型
│   ├── components/               # 通用组件 (Sidebar / Header / Switcher)
│   ├── i18n/                     # vue-i18n + 内置语言包
│   ├── layouts/                  # 默认布局
│   ├── router/                   # 路由 + 权限守卫
│   ├── stores/                   # Pinia: auth / theme / locale / menu
│   ├── themes/                   # 主题应用器 (CSS variables 注入)
│   ├── views/                    # 登录 / Dashboard / admin/*
│   └── main.ts
├── src-tauri/                    # Rust 后端
│   ├── migrations/               # Flyway 风格 SQL 迁移
│   │   ├── V1__init_schema.sql
│   │   └── V2__seed_acl.sql
│   └── src/
│       ├── auth/                 # argon2 密码 / 会话 / RBAC
│       ├── commands/             # Tauri command 实现
│       ├── db/                   # 连接池 + Flyway 风格迁移器
│       ├── models/               # 领域模型 (serde)
│       ├── error.rs
│       ├── lib.rs                # Tauri bootstrap + 命令注册
│       └── main.rs
├── examples/                     # 主题/语言包样例 JSON
│   ├── theme-sunset.json
│   └── locale-ja-JP.json
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

---

## 快速开始

### 前置依赖

| 工具 | 最低版本 | 备注 |
|---|---|---|
| Node.js | 18+ | 用于构建前端 |
| Rust | 1.77+ | `rustup toolchain install stable` |
| Tauri CLI | 跟随 npm | `npm i -D @tauri-apps/cli` |
| WebView2 (Win) / WebKit (mac/linux) | 系统自带 | Tauri 依赖 |

### 第一次启动

```bash
# 1. 安装前端依赖
npm install

# 2. 启动开发模式（Vite + Tauri 同时启动）
npm run tauri:dev
```

第一次运行时会:
1. 在 `~/.admin-suite/admin-suite.sqlite` 创建数据库;
2. 执行 `V1__init_schema.sql` 创建全部表;
3. 执行 `V2__seed_acl.sql` 写入权限 / 角色 / 菜单 / 默认主题 / 默认语言;
4. 写入内置超级管理员 (用户名 `admin` / 密码 `admin123`)。

### 后续启动

直接 `npm run tauri:dev` 即可,迁移系统会校验版本/checksum,无新脚本则秒进。

### 打包发布

```bash
npm run tauri:build
```

产物在 `src-tauri/target/release/bundle/`。

---

## Flyway 风格迁移

### 命名规则

| 前缀 | 含义 | 示例 |
|---|---|---|
| `V{n}__{desc}.sql` | 版本化,只执行一次 | `V1__init_schema.sql` |
| `R__{desc}.sql` | 可重复执行 (checksum 变化时重跑) | `R__views.sql` |

执行顺序:版本号按数字顺序 (1 < 2 < 10),然后按文件名字典序跑 repeatable。

### 升级流程

```bash
# 新增一个 V3__add_xxx.sql,启动应用即可自动应用
$EDITOR src-tauri/migrations/V3__add_orders.sql
npm run tauri:dev
```

启动日志示例:

```
[INFO] admin_suite_lib::db::migrate] applied 1 migration(s)
[INFO] admin_suite_lib::db::migrate]   + V3__add_orders.sql (3, 12 ms)
```

### 失败保护

- 每次迁移在**事务**内执行,失败整体回滚,后续迁移不会跑;
- 启动时校验已应用迁移的 **SHA-256 checksum**,文件被改过直接拒绝启动 (防止"我改了历史脚本没注意")。
- 在 SQLite 内建表 `flyway_schema_history` 持久化历史,与 Flyway 字段对齐,可直接拿 SQL 工具查看。

---

## RBAC

```
┌──────────┐         ┌──────────┐         ┌──────────────┐
│  users   │ N ─── N │  roles   │ N ─── N │ permissions  │
└──────────┘         └──────────┘         └──────────────┘
                       │     │
                       │     │ N ─── N
                       │     ▼
                       │   ┌──────┐
                       └──▶│ menus│ (role_menus)
                           └──────┘
```

- 校验顺序:`is_super_admin` → 通配符 `*:*` → 精确匹配 → 资源通配符 `user:*`。
- 前端路由 `meta.permission` 触发守卫拦截;后端每个 command 入口都强制 `require_permission(...)`。
- 菜单除受 `permission_code` 限制外,还必须在 `role_menus` 中显式授权 (超级管理员全部可见)。

### 内置角色

| code | 描述 |
|---|---|
| `super_admin` | 通配权限,内置不可删 |
| `admin` | 大部分管理权限,无删除 |
| `viewer` | 只读 |

### 角色权限分配

进入 `Roles` → `Assign permissions` / `Assign menus`,勾选后保存。

---

## 主题与多语言

### 内置

- 主题:`light`、`dark`、`ocean` (见 `V2__seed_acl.sql` 中的 `resources` 表初始化)。
- 语言:`en-US`、`zh-CN` (同时内置到前端的 `src/i18n/locales/` 作为离线默认)。

### 自定义主题

上传一个 JSON 文件,格式:

```json
{
  "id": "sunset",
  "label": "Sunset",
  "isDark": false,
  "tokens": {
    "--bg-primary": "#fff7ed",
    "--bg-secondary": "#ffedd5",
    "--primary-color": "#f97316",
    "--text-primary": "#7c2d12"
  }
}
```

进入 `Themes` → `Import theme` 选择文件 → `Activate` 应用。

### 自定义语言

```json
{
  "id": "ja-JP",
  "label": "日本語",
  "messages": {
    "common.ok": "OK",
    "common.cancel": "キャンセル",
    "auth.login": "ログイン",
    "menu.users": "ユーザー管理"
  }
}
```

进入 `Languages` → `Import language pack` → `Activate`。已加载的语言包键会**覆盖**前端默认内置 (`vue-i18n merge`),所以你只需上传差异部分。

### 抽象

主题和语言共用一张表 `resources (resource_type='theme'|'locale')`,前端通过 `useThemeStore` / `useLocaleStore` 拉取并切换。后端 `commands::resources` 是统一的注册表入口,新增类型只需要在该 enum 上扩展。

---

## 关键 Tauri 命令清单

| 命令 | 权限 | 用途 |
|---|---|---|
| `auth_login` | - | 登录,返回 token + 用户 + 权限 + 可见菜单 |
| `auth_logout` | - | 注销 |
| `auth_me` | - | 当前用户 |
| `users_list/get/create/update/delete` | `user:*` | 用户 CRUD |
| `roles_list/get/create/update/delete` | `role:*` | 角色 CRUD |
| `roles_assign_menus` | `role:write` | 给角色授权菜单 |
| `permissions_list` | `permission:read` | 权限字典 |
| `menus_tree/create/update/delete` | `menu:*` | 菜单 CRUD |
| `resources_list` | - | 主题/语言列表 |
| `resources_import_theme/_locale` | `theme/locale:manage` | 上传 JSON |
| `resources_activate` | `theme/locale:manage` | 切换激活 |
| `resources_update` | `theme/locale:manage` | 编辑内容 |
| `resources_delete` | `theme/locale:manage` | 删除 |
| `audit_list` | `audit:read` | 审计日志 |
| `migrate_run` / `migrate_status` | `permission:manage` | 查看/手动触发迁移 |
| `app_info` | - | 应用信息 (数据目录/默认账号) |

---

## 后续可扩展点

1. **功能菜单**:在 `menus` 表插入业务菜单 (例如 `m_orders`),在 `router/index.ts` 加对应路由,无需改动后端命令注册。
2. **业务模块**:把 `commands/` 拆成 `commands::orders`、`commands::inventory` 等子模块,沿用同一 RBAC 校验模式。
3. **多窗口**:Tauri 支持创建多 `WebviewWindow`,可在 Rust 端 `tauri::WebviewWindowBuilder` 加业务窗口。
4. **插件化**:把 `resources` 表当插件仓库,新增 `resource_type='plugin'` + 自定义 JSON schema,前端按 `type` 渲染。
5. **审计写入**:在每个 command 尾部调用 `commands::audit::write(...)`,或包成宏统一注入。

---

## 许可证

MIT. 商业使用请保留版权声明。