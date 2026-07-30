# ServicePilot 桌面应用 · UI 优化建议

> 评审时间：2026-07-30
> 评审对象：Tauri v2 + React 18 + Ant Design v5.29.3 本地服务管理平台
> 评审方式：源码静态审查 + 本地 dev server 实景截图（1280×800 / 1440×900）

## 一、整体评价

✅ **做得好的地方**：整体布局清晰、Mock mode 降级方案完备、TrayPopup 与主窗口分离得当、日志虚拟列表接入稳健。

⚠️ **需要改进**：信息密度不均、组件一致性偏弱、颜色与字号硬编码过多、可访问性不足、错误态视觉提示弱。

📊 **问题分级**：P0 × 3 · P1 × 6 · P2 × 8 · P3 × 1（共 18 项）

## 二、核心问题清单

### P0 · 阻断级（本周内必须修复）

| # | 问题 | 位置 | 影响 |
|---|------|------|------|
| 001 | 顶部资源条 CircleProgress 与文字相互挤压 | `ResourceBar.tsx:69-90` | 关键数值（CPU/MEM）难扫读 |
| 002 | 「操作」列 6 个 icon button 视觉无差异 | `Services.tsx:294-368` | 鼠标操作容错性差，桌面应用核心痛点 |
| 003 | 异常服务行视觉提示仅 6% 透明度 | `global.css:18-20` | 监控类应用最致命：发现不了异常 |

### P1 · 高优级（第二周）

- **004** Sider 底部主题切换与版本号布局混乱
- **005** Dashboard 右侧 3 卡堆叠与左侧 3 卡不等高
- **006** Services 工具栏筛选与操作未分层
- **007** 服务名称作为 `<a>` 缺少点击区域提示
- **008** Settings 编辑器暗色主题与亮色页面冲突
- **009** 全局颜色硬编码 30+ 处，缺 Design Token 抽象
- **010** 字号 11/12/13/14 混用，无 type scale

### P2 · 体验改进（第三周）

- **011** 日志左侧服务列表无分组/搜索/状态筛选
- **012** ResourceBar 右侧「运行 N/M」Tag 视觉孤立
- **013** Dashboard 表格 CPU 单元格进度条与数字拥挤
- **014** Loading / Empty / Error 三态设计不统一
- **015** 图标按钮缺 aria-label，焦点环不明显
- **016** 侧边栏菜单激活态缺品牌一致性

### P3 · 锦上添花

- **017** 缺键盘快捷键（Cmd+R 重启、Cmd+L 看日志）

## 三、重设计核心要点

### 3.1 ResourceBar · 从 CircleProgress 改为 Pill

```
Before:  ⚙ CPU  ⭕24%   ⚙ MEM  ⭕39%   12.5/32 GB   ⚙ GPU  ⭕18%
After:   [CPU 24% ▰▰▱▱▱] [MEM 39% ▰▰▰▰▱] [GPU 18% ▰▱▱▱▱] 12.5/32 GB | 运行 2/5
```

收益：横向宽度减少 40%，垂直高度统一 32px，扫读速度提升。

### 3.2 服务列表 · 操作列重构

```
Before:  [⏹] [↻] [↗] [📄] [✎] [🗑]   ← 6 个无差别图标
After:   [⏹ 红] [↗ 蓝] [📄 灰] [✎ 灰] [🗑 灰红]   ← 按操作语义上色
```

移除「重启」按钮（hover 展开或长按停止）。删除按钮用 danger 浅红背景。

### 3.3 异常行视觉强化

```css
/* Before */
.row-error { background: rgba(255, 77, 79, 0.06); }

/* After */
.row-error {
  background: rgba(239, 68, 68, 0.04);
  border-left: 3px solid #ef4444;
}
.row-error .ant-tag { animation: pulse-error 1.5s ease-in-out infinite; }
```

### 3.4 Toolbar 双层结构

```
┌─ 第一层 · 筛选 ─────────────────────────────────┐
│ [全部分组▾] [全部状态▾] [🔍 搜索服务...]         │
└──────────────────────────────────────────────────┘
┌─ 第二层 · 操作 ─────────────────────────────────┐
│                          [批量启动] [批量停止] [+ 添加服务] │
└──────────────────────────────────────────────────┘
```

## 四、推荐 Design Tokens

### 4.1 语义色

| Token | Hex | 用途 |
|-------|-----|------|
| Primary | `#1677ff` | 操作、链接、焦点、激活态 |
| Success | `#10b981` | 运行中、健康指标 OK |
| Warning | `#f59e0b` | 启动中、内存 60%+ |
| Danger | `#ef4444` | 启动失败、删除 |
| Accent | `#a855f7` | GPU / AI 相关强调 |

> 在 ConfigProvider 的 token 中注册 `colorSuccess`、`colorWarning`、`colorError`、`colorInfoSecondary`，用 `theme.useToken()` 在组件内取色，告别硬编码。

### 4.2 字号系统（6 阶）

| Token | Size / Weight | 用途 |
|-------|---------------|------|
| `fontSizeXS` | 11 / 400 | 标签、辅助说明 |
| `fontSizeSM` | 12 / 400 | 表格行、表单字段 |
| `fontSizeBase` | 14 / 400 | 正文、按钮、菜单 |
| `fontSizeMD` | 16 / 500 | Card 标题、Pill 数值 |
| `fontSizeLG` | 20 / 600 | Stat 数值、Section 标题 |
| `fontSizeXL` | 28 / 700 | 页面主标题 |

### 4.3 间距 / 圆角 / 阴影

- **间距**：4 / 8 / 12 / 16 / 24 / 32（仅这 6 个值）
- **圆角**：Tag=4、Input=6、Card=10、Button=8、徽标=999
- **阴影**：sm=悬浮、md=卡片、lg=弹窗

## 五、落地 Roadmap

| 阶段 | 时间 | 重点 |
|------|------|------|
| **P1** | 第 1 周 | 信息层级急救（修 3 个 P0 + 3 个 P1） |
| **P2** | 第 2 周 | Design Token 化 + 组件一致性 |
| **P3** | 第 3 周 | 体验增强（快捷键、动画、可访问性） |

## 六、建议保留的亮点

1. **Mock mode 降级** — 浏览器可直接预览，无需 Rust 后端
2. **TrayPopup 独立窗口** — hash 路由区分主窗口与托盘弹窗，避免布局污染
3. **react-window 虚拟列表** — 日志页高频刷新场景扛得住
4. **Ant Design 主题统一** — ConfigProvider 已就位，token 化基础完备

---

📎 **完整可视化报告（含 Before/After 重设计 mockup）**：
`/Users/liubo/Workbuddy/Local_server_portal/.workbuddy/artifacts/ui-audit.html`

🖼 **原始截图**：`docs/audit/*.png` 与 `.workbuddy/artifacts/*.png`