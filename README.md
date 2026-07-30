# 本地服务管理平台 (Local Service Manager)

> 跨平台（macOS + Windows）本地开发服务管理桌面应用，一站式管理 antibody_annotation、argo_portal、model_graph_web、ollama 等本地服务。

## 项目概述

本地服务管理平台是一个基于 **Tauri v2** 的桌面应用，用于管理本地开发环境中的多个微服务。提供可视化的服务启停控制、系统资源监控（CPU/内存/GPU）、实时日志查看和 YAML 配置编辑等功能。

### 解决的痛点

- **服务分散难管理**：多个本地服务需要逐个终端启动，状态不可见
- **资源占用不明**：不知道哪个服务消耗了多少 CPU/GPU/内存
- **日志分散难排查**：服务日志分散在各终端窗口，缺乏统一搜索和过滤
- **手动操作繁琐**：没有批量启停能力，开发环境切换效率低

## 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 桌面框架 | Tauri | v2.x |
| 后端语言 | Rust | edition 2021 |
| 前端框架 | React + TypeScript | 18.x / 5.x |
| UI 组件库 | Ant Design | 5.29 |
| 路由 | React Router DOM | 6.30 |
| 虚拟滚动 | react-window | 2.3 |
| 代码编辑器 | CodeMirror 6 | 6.x |
| 系统监控 | sysinfo (Rust crate) | 0.31 |
| 序列化 | serde + serde_yaml + serde_json | - |
| 异步运行时 | Tokio | 1.x |
| 构建工具 | Vite | 5.x |
| 包管理器 | pnpm | 10.x |

## 项目结构

```
local-service-manager/
├── index.html                     # 入口 HTML
├── package.json                   # 前端依赖和脚本
├── tsconfig.json                  # TypeScript 配置
├── tsconfig.node.json             # Node TypeScript 配置
├── vite.config.ts                 # Vite 构建配置
├── src/                           # 前端源码 (React + TypeScript)
│   ├── main.tsx                   # 应用入口
│   ├── App.tsx                    # 根组件 (路由 + 主题)
│   ├── vite-env.d.ts              # Vite 类型声明
│   ├── api/
│   │   └── index.ts               # API 封装 (Tauri IPC / Mock)
│   ├── types/
│   │   └── index.ts               # TypeScript 类型定义
│   ├── hooks/
│   │   ├── useSystemTray.ts       # 系统托盘 Hook
│   │   └── useTauriEvent.ts       # Tauri 事件监听 Hook
│   ├── pages/
│   │   ├── Dashboard.tsx          # 仪表盘页面
│   │   ├── Services.tsx           # 服务管理页面
│   │   ├── Logs.tsx               # 日志查看页面
│   │   └── Settings.tsx           # 配置编辑页面
│   ├── components/
│   │   └── layout/
│   │       ├── AppLayout.tsx      # 应用布局 (侧边栏 + 顶栏)
│   │       └── ResourceBar.tsx    # 资源监控栏
│   └── styles/
│       └── global.css             # 全局样式
├── src-tauri/                     # 后端源码 (Rust)
│   ├── Cargo.toml                 # Rust 依赖配置
│   ├── build.rs                   # Tauri 构建脚本
│   ├── tauri.conf.json            # Tauri 应用配置
│   ├── .gitignore
│   ├── icons/
│   │   └── icon.png               # 应用图标
│   ├── capabilities/
│   │   └── default.json           # 权限配置
│   └── src/
│       ├── main.rs                # Rust 入口
│       ├── lib.rs                 # 插件注册 + 事件系统 + IPC 路由
│       ├── commands/
│       │   └── mod.rs             # Tauri Commands (IPC 接口)
│       ├── models/
│       │   └── mod.rs             # 数据模型定义
│       └── services/
│           ├── mod.rs             # 服务模块导出
│           ├── service_manager.rs # 服务生命周期管理
│           ├── resource_monitor.rs # 系统/进程资源监控
│           ├── log_manager.rs     # 日志采集和管理
│           └── config_manager.rs  # 配置文件读写
└── dist/                          # 构建产物
```

## 功能特性

### 服务管理

| 功能 | 说明 |
|------|------|
| 服务 CRUD | 添加、编辑、删除服务配置 |
| 生命周期控制 | 启动、停止、重启单个服务 |
| 批量操作 | 一键启动/停止全部服务 |
| 状态展示 | 实时显示 stopped / starting / running / stopping / failed / error |
| 快捷访问 | 配置了 URL 的服务支持一键在浏览器中打开 |
| 异常恢复 | 进程异常退出自动检测并推送通知 |

### 系统托盘

- **动态菜单**：显示当前系统资源摘要和所有服务状态
- **服务切换**：右键菜单直接启动/停止单个服务
- **批量控制**：一键启动/停止全部服务
- **通知推送**：服务异常退出时发送系统通知

### 资源监控

| 指标 | 数据来源 | 说明 |
|------|----------|------|
| CPU 使用率 | sysinfo | 系统总体 + 单服务进程级别 |
| 内存使用 | sysinfo | 系统总体 (GB) + 单服务 (MB) |
| GPU 使用率 | nvidia-smi / ioreg / wmic | NVIDIA / Apple Silicon / 通用 |
| GPU 显存 | nvidia-smi / ioreg | 已用/总量 |

- **自动采集**：每 2 秒采集一次，通过 Tauri 事件推送到前端
- **跨平台 GPU 检测**：
  - Linux/Windows NVIDIA：调用 `nvidia-smi`
  - macOS Apple Silicon：解析 `ioreg` MTLDevice 输出
  - Windows 通用：调用 `wmic` 查询

### 日志查看

- **虚拟滚动**：基于 react-window v2，支持万级日志流畅滚动
- **实时刷新**：每 2 秒自动拉取最新日志
- **自动滚动**：可暂停/恢复自动滚动到底部
- **搜索过滤**：关键字搜索 + stdout/stderr 分流
- **终端风格 UI**：深色背景 + 等宽字体 + 颜色编码

### 配置编辑

- **CodeMirror 6 编辑器**：YAML 语法高亮、自动缩进、深色主题
- **保存前校验**：YAML 格式验证 + 配置完整性检查
- **实时预览**：编辑即预览，保存即生效

### Mock 模式

- **浏览器预览**：自动检测 Tauri 环境，无 Tauri 时自动降级为 Mock 模式
- **无需 Rust 编译**：`pnpm dev` 即可在浏览器中独立预览前端
- **开发友好**：前端开发无需等待 Rust 编译

## 快速开始

### 环境要求

| 工具 | 最低版本 | 说明 |
|------|----------|------|
| Node.js | 18+ | 前端运行时 |
| pnpm | 10+ | 包管理器 |
| Rust | 1.70+ | Tauri 后端编译 |
| Git Bash | - | Windows 平台必需（提供 shell 环境） |

### 安装依赖

```bash
# 安装前端依赖
pnpm install
```

### 开发模式

```bash
# 完整 Tauri 桌面应用开发（含 Rust 后端编译）
pnpm tauri dev

# 仅前端浏览器预览（Mock 模式，无需 Rust）
pnpm dev
```

### 构建

```bash
# 构建生产版本
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录。

## 配置说明

### 配置文件位置

| 平台 | 路径 |
|------|------|
| macOS | `~/.local-service-manager/config.yaml` |
| Windows | `%USERPROFILE%\.local-service-manager\config.yaml` |
| Linux | `~/.local-service-manager/config.yaml` |

### 配置格式

```yaml
services:
  - id: "antibody_annotation"
    name: "抗体注释服务"
    command: "python main.py"
    url: "http://localhost:8001"
    work_dir: "/path/to/project"
    env:
      CUDA_VISIBLE_DEVICES: "0"
    group: "AI服务"
    description: "抗体序列注释和结构预测服务"
    stop_timeout: 10

  - id: "argo_portal"
    name: "Argo 门户"
    command: "npm run start"
    url: "http://localhost:3000"
    work_dir: "/path/to/argo_portal"
    group: "Web服务"
    stop_timeout: 5
```

### 字段说明

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| id | string | ✅ | - | 唯一标识符 |
| name | string | ✅ | - | 显示名称 |
| command | string | ✅ | - | 启动命令 |
| url | string | ❌ | - | 服务访问地址（设置后可在浏览器中打开） |
| work_dir | string | ❌ | - | 工作目录 |
| env | object | ❌ | {} | 环境变量 |
| group | string | ❌ | - | 分组标签 |
| description | string | ❌ | - | 描述信息 |
| stop_timeout | number | ❌ | 10 | 停止超时（秒） |

## 跨平台说明

| 特性 | macOS | Windows | Linux |
|------|-------|---------|-------|
| 服务管理 | ✅ | ✅ | ✅ |
| CPU/内存监控 | ✅ sysinfo | ✅ sysinfo | ✅ sysinfo |
| NVIDIA GPU 监控 | ✅ nvidia-smi | ✅ nvidia-smi | ✅ nvidia-smi |
| Apple Silicon GPU 监控 | ✅ ioreg | N/A | N/A |
| 系统托盘 | ✅ | ✅ | ✅ |
| 通知推送 | ✅ | ✅ | ✅ |
| Shell 命令执行 | ✅ | ⚠️ 需 Git Bash | ✅ |

> **Windows 注意**：服务启动依赖 shell 环境，Windows 下需要安装 Git Bash 并在配置中指定 shell 路径。

## 架构说明

### 通信模型

```
┌─────────────────────────────────────────────────┐
│                    Frontend                       │
│              (React + TypeScript)                 │
│  ┌─────────┐ ┌──────────┐ ┌───────────────────┐ │
│  │Dashboard│ │ Services │ │ Logs / Settings    │ │
│  └────┬────┘ └────┬─────┘ └────────┬──────────┘ │
│       └───────────┼───────────────┘              │
│                   │                               │
│              api/index.ts                         │
│         (Tauri invoke / Mock)                     │
└───────────────────┼──────────────────────────────┘
                    │  IPC (invoke + events)
┌───────────────────┼──────────────────────────────┐
│              Tauri Core                           │
│  ┌────────────────┼────────────────────────────┐ │
│  │         commands/mod.rs                      │ │
│  │    (17 Tauri Commands - IPC 接口)            │ │
│  └────┬───────┬──────┬────────┬────────────────┘ │
│       │       │      │        │                    │
│  ┌────▼──┐┌───▼───┐┌▼──────┐┌▼──────────┐      │
│  │Service││Config ││Log    ││Resource    │      │
│  │Manager││Manager││Manager││Monitor     │      │
│  └───────┘└───────┘└───────┘└────────────┘      │
│                   Rust Backend                    │
└──────────────────────────────────────────────────┘
```

### 核心 Rust 模块

| 模块 | 职责 |
|------|------|
| `service_manager` | 服务生命周期管理：启动（子进程）、停止（SIGTERM → SIGKILL）、重启、状态追踪、批量操作 |
| `resource_monitor` | 系统资源采集：CPU/内存通过 sysinfo，GPU 通过 nvidia-smi/ioreg/wmic 命令行工具 |
| `log_manager` | 日志管理：内存环形缓冲区（默认 500 条），支持按 service_id 和时间范围检索 |
| `config_manager` | 配置管理：YAML 文件读写、默认配置生成、格式校验 |

### 前端页面

| 页面 | 路由 | 功能 |
|------|------|------|
| Dashboard | `/` | 系统资源总览 + 服务状态卡片 + 快速操作 |
| Services | `/services` | 服务列表、CRUD、启停控制、批量操作 |
| Logs | `/logs` / `/logs/:serviceId` | 虚拟滚动日志查看器、搜索过滤 |
| Settings | `/settings` | CodeMirror YAML 配置编辑器 |

### Tauri Commands (IPC 接口)

共 17 个 IPC 接口，覆盖服务管理、资源监控、日志查询和配置操作：

- `get_services` / `add_service` / `update_service` / `remove_service`
- `start_service` / `stop_service` / `restart_service`
- `batch_start` / `batch_stop`
- `get_system_resources` / `get_service_resources`
- `get_recent_logs` / `search_logs` / `get_history_logs`
- `get_config_raw` / `save_config_raw` / `validate_config`
- `shutdown_all_services`

### Tauri Events (事件推送)

| 事件 | 方向 | 说明 |
|------|------|------|
| `resource-update` | Rust → JS | 每 2 秒推送系统 + 服务资源快照 |
| `status-change` | Rust → JS | 服务状态变更通知 |
| `show-notification` | Rust → JS | 触发系统通知 |
| `service-process-started` | JS → Rust | 前端通知后端进程已启动 |
| `service-process-stopped` | JS → Rust | 前端通知后端进程已停止 |
| `service-process-error` | JS → Rust | 前端通知后端进程异常 |

## 开发说明

### Mock 模式原理

`api/index.ts` 中通过检测 `window.__TAURI_INTERNALS__` 判断运行环境：

- **Tauri 环境**：使用 `@tauri-apps/api` 的 `invoke()` 调用 Rust 后端
- **浏览器环境**：使用内存模拟数据（Mock），包括预设的 4 个示例服务、模拟资源数据、模拟日志等

### 添加新功能

1. **纯前端功能**：在 `src/` 下添加组件/页面，通过 `api/index.ts` 调用接口
2. **需要后端支持的功能**：
   - 在 `src-tauri/src/models/mod.rs` 定义数据结构
   - 在 `src-tauri/src/services/` 实现业务逻辑
   - 在 `src-tauri/src/commands/mod.rs` 注册 Tauri Command
   - 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册
   - 在 `src/types/index.ts` 同步类型定义
   - 在 `src/api/index.ts` 添加前端调用方法

### 主题切换

支持亮色/暗色主题，使用 Ant Design 的 `ConfigProvider` + `theme.Algorithm`，偏好存储在 `localStorage` 中，默认跟随系统设置。

---

> 📋 本文档随项目代码一起维护，如有更新请同步修改。
