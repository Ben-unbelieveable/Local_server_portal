# ServicePilot UI 优化交付概览

## 📋 任务
基于 UI 审计报告，对本地 Tauri v2 桌面应用（React + Ant Design）进行第一批次 UI 优化（P0+P1 共 9 项），并替换应用图标 + 修复 QA 警告。

## ✅ 完成内容

### SOP 流程
产品经理(PRD) → 架构师(设计+任务分解) → 工程师(代码实现) → QA(回归验证)

### 9 项 PRD 需求（全部 PASS）

| 编号 | 优先级 | 需求 | 状态 |
|------|--------|------|------|
| 001 | P0 | ResourceBar CircleProgress → Pill 样式 | ✅ |
| 002 | P0 | 操作列图标语义着色 + 重启 Dropdown | ✅ |
| 003 | P0 | 异常行视觉强化（色条 + pulse 动画） | ✅ |
| 004 | P1 | Sider 底部 Flex 重排 | ✅ |
| 005 | P1 | Dashboard 卡片等高对齐 | ✅ |
| 006 | P1 | 工具栏双层分层 | ✅ |
| 007 | P1 | 服务名称点击区域增强 | ✅ |
| 008 | P1 | CodeMirror 主题跟随应用模式 | ✅ |
| 009 | P1 | Design Token 体系建立 | ✅ |

### 文件清单（4 新建 + 8 修改 = 12 文件）

**新建文件：**
- `src/styles/tokens.ts` — Design Token 定义（语义色/字号/间距/圆角/阴影/阈值函数）
- `src/contexts/ThemeContext.tsx` — 主题上下文（isDark + toggleTheme）
- `src/components/common/StatusTag.tsx` — 状态标签组件（封装 statusMap + pulse 动画）
- `src/components/common/ServiceNameLink.tsx` — 服务名称链接组件

**修改文件：**
- `src/App.tsx` — ConfigProvider 注册完整 Token + ThemeProvider 包裹
- `src/styles/global.css` — CSS 变量注册 + 异常行强化 + pulse 动画
- `src/pages/TrayPopup.tsx` — 硬编码颜色替换为 Token + 字号 Token 化
- `src/components/layout/ResourceBar.tsx` — CircleProgress → CSS 自绘 Pill
- `src/components/layout/AppLayout.tsx` — Sider 底部 Flex 重排
- `src/pages/Settings.tsx` — CodeMirror 主题跟随 isDark
- `src/pages/Services.tsx` — 操作列着色 + 工具栏分层 + 异常行 + 名称链接
- `src/pages/Dashboard.tsx` — 卡片等高 + 第4列改造 + 异常行 + 名称链接

### 验证结果
- `npx tsc --noEmit` ✅ 零报错
- `npx vite build` ✅ 构建成功（2.43s）
- QA 逐项代码审查 ✅ 9/9 PASS
- W2/W5/W6 警告已修复 ✅

## 🎨 图标替换 + 警告修复（增量）

### Icon 系列（15 个 PNG 全部替换）
- 使用 ImageGen 基于用户截图风格生成 1024×1024 主图
- sips 批量 resize 到 14 个衍生尺寸（32×32 ~ 310×310）
- 设计：蓝白渐变背景 + 三条 Pill 进度条（红/绿状态点）+ 心电波形

### 警告修复
| 警告 | 修复 | 文件 |
|------|------|------|
| W2 | antdThemeToken 补充 `borderRadiusLG: borderRadius.card`(10) | `src/styles/tokens.ts:194` |
| W5 | TrayPopup 字号 10/13 → `fontSizes.XS/SM.size` | `src/pages/TrayPopup.tsx:222,317` |
| W6 | StatusTag 字号 11 → `fontSizes.XS.size` + 新增 import | `src/components/common/StatusTag.tsx:4,48` |
| Alpha | `iconAsTemplate: true → false`（彩色图标不适合模板模式） | `src-tauri/tauri.conf.json:26` |

## ⚠️ 已知问题（剩余 WARNING）

| # | 描述 | 影响 | 建议 |
|---|------|------|------|
| W1 | pulse 动画用 box-shadow 扩散而非 opacity 闪烁 | 视觉效果不同但仍有脉冲提示 | 可选改为 opacity 动画 |
| W3 | main.tsx 外层 ConfigProvider 硬编码 | 无功能影响（内层覆盖） | 可选纳入 Token 化 |
| W4 | Logs.tsx 3 处硬编码颜色 + 1 处字号字面量 | 属 P2 后续批次 | 后续迭代处理 |

## 📐 关键设计决策

1. **Pill 进度条**：CSS 自绘（非 Ant Progress），精确控制 32px 紧凑布局
2. **重启按钮**：Dropdown trigger="hover"，触发器 MoreOutlined 图标
3. **异常行动画**：CSS @keyframes tag-pulse（1.5s infinite）
4. **CodeMirror 主题**：条件 extension + EditorView 重建（docRef 保存内容）
5. **Token 管理**：单文件 tokens.ts + ConfigProvider + CSS 变量
6. **卡片等高**：CSS flexbox（align-items:stretch + flex:1 + height:100%）
7. **ThemeContext**：解决 Settings 页面无法获取 isDark 的问题
8. **iconAsTemplate: false**：彩色图标不适合 macOS 模板模式（会丢失颜色 + RGB 无 Alpha 通道会导致实心方块）

## 🔗 交付文件

- `docs/prd/UI-OPTIMIZATION-PRD.md` — 增量 PRD
- `docs/design/UI-OPTIMIZATION-DESIGN.md` — 系统设计文档
- `docs/design/class-diagram.mermaid` — 类图
- `docs/design/sequence-diagram.mermaid` — 时序图
- 12 个源代码文件（4 新建 + 8 修改）
- 15 个 icon PNG 文件（全部替换）

## 📅 后续迭代

| 批次 | 范围 | 内容 |
|------|------|------|
| 第二批次 | P2（7 项） | 日志分组/搜索、三态统一、aria-label、菜单激活态 |
| 第三批次 | P3（1 项） | 键盘快捷键（Cmd+R/L/K） |
| 技术债 | W1/W3/W4 | pulse 动画类型、main.tsx Token 化、Logs Token 化 |
