import { Space, Typography, theme } from "antd";
import { useEffect, useState } from "react";
import type { SystemResource } from "../../types";
import type { ThresholdConfig } from "../../styles/tokens";
import { api } from "../../api";
import { useTauriEvent } from "../../hooks/useTauriEvent";
import type { ResourceUpdateEvent } from "../../types";
import {
  fontSizes,
  spacing,
  borderRadius,
  getThresholdColor,
  glassFillLight,
  glassBlur,
  semanticColors,
} from "../../styles/tokens";

interface ResourceBarProps {
  /** 是否暗色主题（由 AppLayout 传入） */
  isDark?: boolean;
  /** 窄屏紧凑：隐藏 MEM/GPU 附加文案，允许换行 */
  compact?: boolean;
}

/**
 * 顶部资源条：玻璃质感对齐托盘顶栏；窄屏下紧凑换行。
 */
export default function ResourceBar({
  isDark = false,
  compact = false,
}: ResourceBarProps) {
  const { token } = theme.useToken();
  const [resource, setResource] = useState<SystemResource | null>(null);
  const [runningCount, setRunningCount] = useState(0);
  const [totalCount, setTotalCount] = useState(0);

  const fetchResources = async () => {
    try {
      const [res, services] = await Promise.all([
        api.getSystemResources(),
        api.getServices(),
      ]);
      setResource(res);
      setTotalCount(services.length);
      setRunningCount(services.filter((s) => s.status === "running").length);
    } catch {
      // 忽略
    }
  };

  useEffect(() => {
    fetchResources();
    const interval = setInterval(fetchResources, 3000);
    return () => clearInterval(interval);
  }, []);

  useTauriEvent<ResourceUpdateEvent>("resource-update", (payload) => {
    setResource(payload.system);
  });

  useTauriEvent("service-status-changed", () => {
    fetchResources();
  });

  if (!resource) return null;

  /**
   * 渲染 Pill 进度条（CSS 自绘）
   * @param label 标签（CPU / MEM / GPU）
   * @param percent 0-100 百分比
   * @param thresholds 阈值配置
   * @param extra 附加文本
   */
  const renderPill = (
    label: string,
    percent: number,
    thresholds?: ThresholdConfig,
    extra?: string
  ) => {
    const color = getThresholdColor(percent, thresholds);
    const roundedPercent = Math.round(percent);
    return (
      <Space size={spacing.xs} align="center">
        <div
          style={{
            position: "relative",
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            height: 28,
            maxWidth: 120,
            minWidth: 80,
            padding: `0 ${spacing.md}px`,
            borderRadius: borderRadius.pill,
            background: isDark
              ? token.colorFillSecondary
              : "rgba(255,255,255,0.72)",
            border: `1px solid ${token.colorBorderSecondary}`,
            overflow: "hidden",
          }}
        >
          <div
            style={{
              position: "absolute",
              left: 0,
              top: 0,
              bottom: 0,
              width: `${roundedPercent}%`,
              background: color,
              opacity: 0.18,
              borderRadius: borderRadius.pill,
              transition: "width 0.3s ease",
            }}
          />
          <Typography.Text
            style={{
              position: "relative",
              fontSize: fontSizes.SM.size,
              fontWeight: fontSizes.MD.weight,
              color: token.colorText,
              whiteSpace: "nowrap",
            }}
          >
            {label} {roundedPercent}%
          </Typography.Text>
        </div>
        {extra && (
          <Typography.Text
            type="secondary"
            style={{ fontSize: fontSizes.SM.size }}
          >
            {extra}
          </Typography.Text>
        )}
      </Space>
    );
  };

  return (
    <div
      style={{
        minHeight: compact ? 40 : 44,
        display: "flex",
        alignItems: "center",
        flexWrap: "wrap",
        rowGap: spacing.xs,
        padding: compact
          ? `${spacing.xs}px ${spacing.md}px`
          : `0 ${spacing.xl}px`,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        background: isDark ? token.colorBgContainer : glassFillLight,
        backdropFilter: isDark ? undefined : glassBlur,
        WebkitBackdropFilter: isDark ? undefined : glassBlur,
        gap: compact ? spacing.md : spacing.xl,
      }}
    >
      <Space size={compact ? spacing.sm : spacing.lg} wrap>
        {renderPill("CPU", resource.cpu_percent, {
          warning: 50,
          danger: 80,
        })}
        {renderPill(
          "MEM",
          resource.memory_percent,
          { warning: 60, danger: 85 },
          compact
            ? undefined
            : `${resource.memory_used_gb.toFixed(1)} / ${resource.memory_total_gb.toFixed(0)} GB`
        )}
        {resource.gpu_percent != null &&
          renderPill(
            "GPU",
            resource.gpu_percent,
            { warning: 50, danger: 80 },
            compact
              ? undefined
              : resource.gpu_memory_used_mb != null
                ? `${resource.gpu_memory_used_mb.toFixed(0)} MB`
                : undefined
          )}
      </Space>
      <div style={{ flex: 1, minWidth: spacing.sm }} />
      <div
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: spacing.sm,
          height: 28,
          padding: `0 ${spacing.md}px`,
          borderRadius: borderRadius.pill,
          background: isDark
            ? token.colorFillSecondary
            : "rgba(255,255,255,0.72)",
          border: `1px solid ${token.colorBorderSecondary}`,
          fontSize: fontSizes.SM.size,
          color: token.colorTextSecondary,
          flexShrink: 0,
        }}
      >
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: semanticColors.colorSuccess,
            flexShrink: 0,
          }}
        />
        {compact
          ? `${runningCount}/${totalCount}`
          : `运行中 ${runningCount} / ${totalCount}`}
      </div>
    </div>
  );
}
