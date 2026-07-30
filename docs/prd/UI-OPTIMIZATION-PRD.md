# ServicePilot UI 优化增量 PRD

## 1. 优化目标

> **让运维监控信息一秒可读、异常状态不可忽视、操作意图肉眼可辨——在不改变任何现有功能逻辑的前提下，将 ServicePilot 的 UI 从「能用」提升到「好用」。**

---

## 2. 优化范围

### 第一批次：P0 + P1（共 9 项，第 1-2 周）

| 分级 | 编号 | 问题 | 理由 |
|------|------|------|------|
| P0 | 001 | ResourceBar CircleProgress 与文字挤压 | 顶部资源条是用户每秒扫读的核心信息入口，当前 24px 圆形进度环 + 百分比文字 + 数值文字三者叠挤，CPU/MEM 数值几乎不可读 |
| P0 | 002 | 服务列表操作列 6 个 icon button 无视觉差异 | 桌面应用最高频操作场景，停止/重启/访问/日志/编辑/删除全部灰色图标，误操作风险高，容错性差 |
| P0 | 003 | 异常服务行视觉提示仅 6% 透明度 | 监控类应用的致命缺陷：status=error/failed 的行只有 `rgba(255,77,79,0.06)` 背景，与正常行几乎无区别 |
| P1 | 004 | Sider 底部主题切换与版本号布局混乱 | `position: absolute; bottom: 16` 的浮层布局，Switch + 版本号挤在一起，视觉重心不稳 |
| P1 | 005 | Dashboard 右侧 3 卡堆叠与左侧 3 卡不等高 | 左侧 CPU/MEM/GPU 三张 span=6 卡片 vs 右侧嵌套 Row 中 3 张 size="small" 卡片，高度不齐，视觉割裂 |
| P1 | 006 | Services 工具栏筛选与操作未分层 | 分组筛选、状态筛选、搜索框、批量操作、添加按钮全部平铺在一个 Row 中，信息层级缺失 |
| P1 | 007 | 服务名称作为 `<a>` 缺少点击区域提示 | `<a onClick={...}>` 无 href、无 hover 样式增强，用户不知道可点击跳转日志 |
| P1 | 008 | Settings 编辑器暗色主题与亮色页面冲突 | CodeMirror 硬编码 `oneDark` 主题，亮色模式下编辑器是黑底，与页面白色 Card 冲突 |
| P1 | 009 | 全局颜色硬编码 30+ 处 | `#ff4d4f`、`#52c41a`、`#faad14`、`#f0f0f0`、`#1e1e1e` 等散落在 inline style 中，无法统一换肤 |

**P1 字号治理（010）**：与 009 一并在 Design Token 化阶段处理，不单独立项。

### 第二批次：P2（共 7 项，第 3 周）

| 编号 | 问题 | 说明 |
|------|------|------|
| 011 | 日志左侧服务列表无分组/搜索/状态筛选 | 当前仅按运行状态过滤，服务多时难定位 |
| 012 | ResourceBar 右侧「运行 N/M」Tag 视觉孤立 | 与左侧资源信息无视觉关联 |
| 013 | Dashboard 表格 CPU 单元格进度条与数字拥挤 | `Progress size="small"` 无 `format`，条+数字挤在 90px 列宽 |
| 014 | Loading / Empty / Error 三态设计不统一 | Dashboard 用 `Spin`、Services 用 `Empty`、Logs 用纯文字，风格不统一 |
| 015 | 图标按钮缺 aria-label，焦点环不明显 | 可访问性合规问题 |
| 016 | 侧边栏菜单激活态缺品牌一致性 | Ant Design 默认激活色，未与品牌 Primary 对齐 |

### 第三批次：P3（第 4 周+）

| 编号 | 问题 | 说明 |
|------|------|------|
| 017 | 缺键盘快捷键 | Cmd+R 重启、Cmd+L 看日志、Cmd+K 全局搜索 |

### 范围说明

- **本 PRD 聚焦第一批次（P0 + P1）**，第二/三批次作为后续迭代规划，需求池中列出但不展开验收标准。
- **增量优化，不重写**：保持现有功能逻辑、API 调用、路由结构不变，仅改 UI/UX 层。
- **技术栈不变**：React 18 + Ant Design v5.29.3 + Tauri v2，不做技术选型。

---

## 3. 用户故事

### US-1（对应 P0-001）
> 作为一名运维开发者，我希望顶部资源条能一眼看清 CPU 和内存使用率，以便我在服务异常时快速判断是否是资源瓶颈导致。

### US-2（对应 P0-002）
> 作为一名运维开发者，我希望服务列表的操作按钮有颜色区分，以便我不需要看 Tooltip 就能快速找到「停止」或「访问」按钮，避免误操作。

### US-3（对应 P0-003）
> 作为一名运维开发者，我希望异常状态的服务行有明显的视觉标记（红色色条 + 闪烁标签），以便我在扫读服务列表时不会漏掉任何一个异常服务。

### US-4（对应 P1-006）
> 作为一名运维开发者，我希望服务管理的筛选条件和操作按钮分层展示，以便我能快速筛选服务列表，同时批量操作按钮不会与筛选控件混在一起。

### US-5（对应 P1-008）
> 作为一名运维开发者，我希望配置编辑器的主题能跟随应用的亮色/暗色模式，以便在亮色模式下不会被黑色编辑器刺眼。

### US-6（对应 P1-009）
> 作为一名前端开发者，我希望颜色和字号统一通过 Design Token 管理，以便后续维护时只改一处即可全局生效，且暗色模式适配更简单。

---

## 4. 需求池（按优先级）

### 第一批次：P0 + P1

| 编号 | 优先级 | 需求描述 | 涉及文件 | 验收标准 |
|------|--------|---------|---------|---------|
| 001 | P0 | ResourceBar 从 CircleProgress 改为 Pill 样式 | `src/components/layout/ResourceBar.tsx` | 1. CPU/MEM/GPU 各显示为横向 Pill（`[CPU 24% ▰▰▱▱▱]` 风格），宽度不超 120px/项；2. 垂直高度统一 32px；3. 阈值配色保留（>80% 红、>50% 黄、其余绿）；4. 右侧「运行 N/M」Tag 整合进资源条右侧，视觉连贯 |
| 002 | P0 | 服务列表操作列图标按钮按语义着色 | `src/pages/Services.tsx` | 1. 启动=Primary 蓝、停止=Danger 红、访问=Default 蓝、日志=Default 灰、编辑=Default 灰、删除=Danger 灰红；2. 重启按钮移入 hover 展开（Dropdown/Popover），默认不占位；3. 列宽从 280px 缩减至 ≤200px；4. 所有按钮保留 Tooltip |
| 003 | P0 | 异常服务行视觉提示强化 | `src/styles/global.css`、`src/pages/Services.tsx` | 1. 背景色从 6% 提至 4% + 3px 左侧红色色条（`box-shadow: inset 3px 0 0 #ef4444`）；2. 状态 Tag 增加 `pulse` 闪烁动画（1.5s 周期，opacity 1→0.4→1）；3. Dashboard 表格同步应用异常行高亮 |
| 004 | P1 | Sider 底部主题切换与版本号重排 | `src/components/layout/AppLayout.tsx` | 1. 底部区域使用 Flex 布局，Switch 居左、版本号居右，垂直居中对齐；2. 增加上方 1px 分割线；3. 高度统一 48px 与顶部 Logo 区对称 |
| 005 | P1 | Dashboard 卡片等高对齐 | `src/pages/Dashboard.tsx` | 1. 右侧 3 个统计卡改为与左侧同尺寸 Card（非 size="small"）；2. 左右 Row 内所有 Card 高度一致（使用 `display:flex; flex-direction:column` + Content 区域撑满）；3. 第 4 列（运行/停止/异常）改为单张卡片内 3 行 Statistic，消除嵌套 Row |
| 006 | P1 | Services 工具栏双层分层 | `src/pages/Services.tsx` | 1. 第一层（筛选行）：分组 Select + 状态 Select + 搜索 Input.Search，左对齐；2. 第二层（操作行）：批量启动/停止（仅选中时显示）+ 添加服务，右对齐；3. 两层之间 12px 间距，视觉上有分隔感 |
| 007 | P1 | 服务名称点击区域增强 | `src/pages/Services.tsx`、`src/pages/Dashboard.tsx` | 1. `<a>` 改为 `Typography.Link` 或添加 `style={{ cursor: 'pointer' }}` + hover 下划线；2. 增加 Tooltip 提示「点击查看日志」；3. Dashboard 表格行名同步处理 |
| 008 | P1 | Settings 编辑器主题跟随应用模式 | `src/pages/Settings.tsx` | 1. 亮色模式使用 CodeMirror 默认浅色主题（移除 `oneDark` 或替换为 light theme extension）；2. 暗色模式保留 `oneDark`；3. 编辑器容器 border 颜色随主题变化（亮色 `#d9d9d9`、暗色 `#303030`） |
| 009 | P1 | Design Token 体系建立 | `src/App.tsx`（ConfigProvider）、`src/styles/tokens.ts`（新建） | 1. 新建 `tokens.ts` 定义语义色、字号、间距、圆角、阴影 Token；2. `App.tsx` 的 ConfigProvider 注册 `colorPrimary`、`colorSuccess`、`colorWarning`、`colorError`、`borderRadius`、`fontSize` 等 Token；3. 替换全部 inline style 中的硬编码颜色（`#ff4d4f`→`token.colorError`、`#52c41a`→`token.colorSuccess`、`#faad14`→`token.colorWarning`、`#f0f0f0`→`token.colorBorderSecondary` 等）；4. 字号统一为 6 阶 type scale（XS 11 / SM 12 / Base 14 / MD 16 / LG 20 / XL 28） |

### 第二批次：P2（后续迭代，列出不展开）

| 编号 | 优先级 | 需求描述 | 涉及文件 |
|------|--------|---------|---------|
| 011 | P2 | 日志左侧服务列表增加分组/搜索/状态筛选 | `src/pages/Logs.tsx` |
| 012 | P2 | ResourceBar 右侧「运行 N/M」整合为统计 Pill | `src/components/layout/ResourceBar.tsx` |
| 013 | P2 | Dashboard 表格 CPU 单元格优化布局 | `src/pages/Dashboard.tsx` |
| 014 | P2 | 统一 Loading / Empty / Error 三态组件 | 全局 |
| 015 | P2 | 图标按钮补 aria-label + 焦点环增强 | 全局 |
| 016 | P2 | 侧边栏菜单激活态品牌色对齐 | `src/components/layout/AppLayout.tsx` |

### 第三批次：P3

| 编号 | 优先级 | 需求描述 | 涉及文件 |
|------|--------|---------|---------|
| 017 | P3 | 键盘快捷键（Cmd+R 重启、Cmd+L 日志、Cmd+K 搜索） | 全局 |

---

## 5. Design Token 规范

### 5.1 语义色 Token

| Token 名称 | 值 | 用途 | 注册方式 |
|------------|-----|------|---------|
| `colorPrimary` | `#1677ff` | 主操作色、激活态 | ConfigProvider `token` |
| `colorSuccess` | `#10b981` | 运行中状态、正常指标 | ConfigProvider `token` |
| `colorWarning` | `#f59e0b` | 中间阈值告警 | ConfigProvider `token` |
| `colorError` | `#ef4444` | 异常/失败/危险操作 | ConfigProvider `token` |
| `colorAccent` | `#a855f7` | 品牌强调色（备用） | CSS 变量 `--sp-color-accent` |
| `colorBorderSecondary` | `#f0f0f0`（亮）/ `#303030`（暗） | 分割线、卡片边框 | Ant Design 内置 Token |
| `colorBgError` | `rgba(239,68,68,0.04)` | 异常行背景 | CSS 类 `.row-error` |
| `colorBgLog` | `#1e1e1e` | 日志终端背景 | CSS 变量 `--sp-bg-log` |

### 5.2 字号 Type Scale

| Token 名称 | 字号 / 字重 | 用途 | 注册方式 |
|------------|-------------|------|---------|
| `fontSizeXS` | 11px / 400 | 辅助说明、时间戳 | CSS 变量 `--sp-font-xs` |
| `fontSizeSM` | 12px / 400 | 标签、次要文字 | Ant Design `token.fontSizeSM` |
| `fontSizeBase` | 14px / 400 | 正文、表格 | Ant Design `token.fontSize`（默认） |
| `fontSizeMD` | 16px / 500 | 卡片标题、统计数值 | Ant Design `token.fontSizeHeading5` |
| `fontSizeLG` | 20px / 600 | 页面标题、统计大数 | Ant Design `token.fontSizeHeading4` |
| `fontSizeXL` | 28px / 700 | Dashboard 大标题 | Ant Design `token.fontSizeHeading2` |

### 5.3 间距

| Token | 值 | 用途 |
|-------|-----|------|
| `space.xs` | 4px | 紧凑间距（图标内部） |
| `space.sm` | 8px | 组件内间距 |
| `space.md` | 12px | 组件间间距 |
| `space.base` | 16px | 卡片内边距、区块间距 |
| `space.lg` | 24px | 页面内边距、大区块间距 |
| `space.xl` | 32px | 页面顶/底间距 |

### 5.4 圆角

| Token | 值 | 用途 | 注册方式 |
|-------|-----|------|---------|
| `borderRadiusTag` | 4px | Tag、Badge | CSS 变量 |
| `borderRadiusInput` | 6px | Input、Select | ConfigProvider `token.borderRadius` |
| `borderRadiusButton` | 8px | Button | ConfigProvider `token.borderRadius`（AntD 不区分 Button/Input，取 6px 统一） |
| `borderRadiusCard` | 10px | Card | ConfigProvider `token.borderRadiusLG` |
| `borderRadiusPill` | 999px | Pill、圆形元素 | CSS 变量 |

### 5.5 阴影

| Token | 值 | 用途 |
|-------|-----|------|
| `shadow.sm` | `0 1px 2px rgba(0,0,0,0.06)` | 悬浮 Tag、Tooltip |
| `shadow.md` | `0 2px 8px rgba(0,0,0,0.08)` | Card 默认 |
| `shadow.lg` | `0 6px 16px rgba(0,0,0,0.12)` | Modal、Popover |

### 5.6 注册策略

- **ConfigProvider 注册**：`colorPrimary`、`colorSuccess`、`colorWarning`、`colorError`、`borderRadius`、`borderRadiusLG`、`fontSize`、`fontSizeSM`、`fontSizeHeading2/4/5` — 这些通过 Ant Design 的 token 系统自动渗透到所有组件。
- **CSS 变量注册**：`--sp-color-accent`、`--sp-font-xs`、`--sp-bg-log`、`--sp-radius-pill`、间距和阴影 Token — 在 `global.css` 的 `:root` 中定义，供 inline style 和 CSS 类引用。
- **暗色模式适配**：ConfigProvider 已有 `theme.darkAlgorithm`，语义色 Token 自动适配。CSS 变量需在 `:root[data-theme="dark"]` 下覆盖。

---

## 6. 待确认问题

| # | 问题 | 涉及方 | 说明 |
|---|------|--------|------|
| 1 | ResourceBar Pill 中的进度条是否使用 CSS 自绘还是用 Ant Design `Progress` 的 `strokeColor` + 自定义 `trailColor`？ | 前端 | CSS 自绘更轻量但需自行处理动画；Ant Design Progress 更一致但需覆盖默认样式 |
| 2 | 操作列重启按钮移入 hover 展开后，是否需要保留键盘可访问性？ | 前端 | Dropdown 默认支持键盘导航，但需确认焦点管理是否符合预期 |
| 3 | 异常行 Tag 闪烁动画是否会影响性能？服务数量 >50 时需验证 | 前端 | 可考虑用 `will-change: opacity` 或限制动画到仅 `error` 状态（不含 `failed`） |
| 4 | Settings 编辑器亮色主题使用哪个 CodeMirror extension？ | 前端 | `@codemirror/view` 的 `EditorView.theme()` 自定义，还是用第三方 light theme 包 |
| 5 | Design Token 的 `tokens.ts` 文件组织方式：单文件还是按类别拆分？ | 架构师 | 建议单文件 + 按类别分组 export，避免过度拆分 |
| 6 | Dashboard 第 4 列改为单张卡片后，三个 Statistic 的纵向排列是否需要增加分隔线？ | 前端/设计 | 可用 `Divider` 或纯间距区分 |
| 7 | 是否需要将 Token 同时输出为 Tailwind CSS config？当前项目是否使用 Tailwind？ | 架构师 | 审计报告提到技术栈含 Tailwind CSS，但实际代码中未见 Tailwind 使用，需确认 |

---

## 附录：重设计核心要点速览

### ResourceBar Pill 样式示意

```
┌──────────────────────────────────────────────────────────────┐
│ [CPU 24% ▰▰▱▱▱]  [MEM 61% ▰▰▰▱▱]  [GPU 8% ▰▱▱▱▱]   运行 3/5 │
└──────────────────────────────────────────────────────────────┘
  ← 每项 ≤120px →                              ← Tag 整合 →
```

### 服务列表操作列语义着色

| 操作 | 图标 | 颜色 | 可见性 |
|------|------|------|--------|
| 启动 | CaretRight | Primary 蓝 | 默认显示（非运行态） |
| 停止 | PauseCircle | Danger 红 | 默认显示（运行态） |
| 访问 | Export | Default 蓝 | 默认显示（有 URL 时） |
| 日志 | FileText | Default 灰 | 默认显示 |
| 编辑 | Edit | Default 灰 | 默认显示 |
| 删除 | Delete | Danger 灰红 | 默认显示 |
| 重启 | Reload | Default 灰 | **hover 展开** |

### 异常行视觉强化

```
┌─┬─────────────────────────────────────────────────┐
│▌│ ● 异常  Ollama AI    PID: 12345    ...  [操作]   │  ← 3px 红色色条 + 4% 红底 + Tag 闪烁
└─┴─────────────────────────────────────────────────┘
```

---

*文档版本：v1.0 | 创建日期：2025-07-30 | 作者：Alice（产品经理）*
