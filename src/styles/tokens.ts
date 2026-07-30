import type { ThemeConfig } from "antd";

// ============================================================
// Semantic Colors — 统一语义色板，全项目唯一颜色来源
// ============================================================

export interface SemanticColors {
  colorPrimary: string;
  colorSuccess: string;
  colorWarning: string;
  colorError: string;
  colorAccent: string;
}

export const semanticColors: SemanticColors = {
  colorPrimary: "#1677ff",
  colorSuccess: "#10b981",
  colorWarning: "#f59e0b",
  colorError: "#ef4444",
  colorAccent: "#a855f7",
};

// ============================================================
// Neutral / Structural Colors — 中性色（亮色模式默认值）
// ============================================================

export interface NeutralColors {
  colorBorder: string;
  colorBorderSecondary: string;
  colorText: string;
  colorTextSecondary: string;
  colorTextTertiary: string;
  colorBgContainer: string;
  colorBgLayout: string;
  colorBgDark: string;
  colorSiderDark: string;
}

export const neutralColors: NeutralColors = {
  colorBorder: "#d9d9d9",
  colorBorderSecondary: "#f0f0f0",
  colorText: "rgba(0, 0, 0, 0.88)",
  colorTextSecondary: "#8c8c8c",
  colorTextTertiary: "#bfbfbf",
  colorBgContainer: "#ffffff",
  colorBgLayout: "#f5f5f5",
  colorBgDark: "#141414",
  colorSiderDark: "#303030",
};

// ============================================================
// Font Sizes — 6 阶字号体系
// ============================================================

export interface FontSize {
  size: number;
  weight: number;
}

export interface FontSizeScale {
  XS: FontSize;
  SM: FontSize;
  Base: FontSize;
  MD: FontSize;
  LG: FontSize;
  XL: FontSize;
}

export const fontSizes: FontSizeScale = {
  XS: { size: 11, weight: 400 },
  SM: { size: 12, weight: 400 },
  Base: { size: 14, weight: 400 },
  MD: { size: 16, weight: 500 },
  LG: { size: 20, weight: 600 },
  XL: { size: 28, weight: 700 },
};

// ============================================================
// Spacing — 间距体系 (4 / 8 / 12 / 16 / 24 / 32)
// ============================================================

export interface SpacingScale {
  xs: number;
  sm: number;
  md: number;
  lg: number;
  xl: number;
  xxl: number;
}

export const spacing: SpacingScale = {
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 24,
  xxl: 32,
};

// ============================================================
// Border Radius — 圆角体系
// ============================================================

export interface RadiusScale {
  tag: number;
  input: number;
  card: number;
  button: number;
  pill: number;
}

export const borderRadius: RadiusScale = {
  tag: 4,
  input: 6,
  card: 10,
  button: 8,
  pill: 999,
};

// ============================================================
// Shadows — 阴影体系
// ============================================================

export interface ShadowScale {
  sm: string;
  md: string;
  lg: string;
}

export const shadows: ShadowScale = {
  sm: "0 1px 2px 0 rgba(0, 0, 0, 0.05)",
  md: "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1)",
  lg: "0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1)",
};

// ============================================================
// Threshold — 阈值配色函数
// ============================================================

export interface ThresholdConfig {
  warning: number;
  danger: number;
}

export type ThresholdLevel = "success" | "warning" | "danger";

export const defaultThresholds: ThresholdConfig = {
  warning: 50,
  danger: 80,
};

/**
 * 根据百分比和阈值配置返回对应的语义色。
 * @param percent 0-100 的百分比数值
 * @param thresholds 可选阈值配置，默认 { warning: 50, danger: 80 }
 * @returns 语义色 hex 字符串
 */
export function getThresholdColor(
  percent: number,
  thresholds?: ThresholdConfig
): string {
  const { warning, danger } = thresholds ?? defaultThresholds;
  if (percent > danger) return semanticColors.colorError;
  if (percent > warning) return semanticColors.colorWarning;
  return semanticColors.colorSuccess;
}

/**
 * 根据百分比和阈值配置返回对应的阈值级别。
 * @param percent 0-100 的百分比数值
 * @param thresholds 可选阈值配置，默认 { warning: 50, danger: 80 }
 * @returns ThresholdLevel 枚举值
 */
export function getThresholdLevel(
  percent: number,
  thresholds?: ThresholdConfig
): ThresholdLevel {
  const { warning, danger } = thresholds ?? defaultThresholds;
  if (percent > danger) return "danger";
  if (percent > warning) return "warning";
  return "success";
}

// ============================================================
// Ant Design Theme Token — 传入 ConfigProvider 的 token 对象
// ============================================================

export const antdThemeToken: ThemeConfig["token"] = {
  colorPrimary: semanticColors.colorPrimary,
  colorSuccess: semanticColors.colorSuccess,
  colorWarning: semanticColors.colorWarning,
  colorError: semanticColors.colorError,
  borderRadius: borderRadius.input,
  borderRadiusLG: borderRadius.card,
};

// ============================================================
// CSS Variables — 供 global.css :root 注册及 inline style 使用
// ============================================================

export const cssVariables: Record<string, string> = {
  "--color-primary": semanticColors.colorPrimary,
  "--color-success": semanticColors.colorSuccess,
  "--color-warning": semanticColors.colorWarning,
  "--color-error": semanticColors.colorError,
  "--color-accent": semanticColors.colorAccent,
  "--color-border": neutralColors.colorBorder,
  "--color-border-secondary": neutralColors.colorBorderSecondary,
  "--color-text-secondary": neutralColors.colorTextSecondary,
  "--color-text-tertiary": neutralColors.colorTextTertiary,
  "--color-bg-container": neutralColors.colorBgContainer,
  "--color-bg-layout": neutralColors.colorBgLayout,
  "--shadow-sm": shadows.sm,
  "--shadow-md": shadows.md,
  "--shadow-lg": shadows.lg,
};
