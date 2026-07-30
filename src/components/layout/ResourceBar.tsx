import { Space, Tag, Typography, theme } from "antd";
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
} from "../../styles/tokens";

export default function ResourceBar() {
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
            height: 32,
            maxWidth: 120,
            minWidth: 80,
            padding: `0 ${spacing.md}px`,
            borderRadius: borderRadius.pill,
            background: token.colorFillSecondary,
            overflow: "hidden",
          }}
        >
          {/* 填充层 */}
          <div
            style={{
              position: "absolute",
              left: 0,
              top: 0,
              bottom: 0,
              width: `${roundedPercent}%`,
              background: color,
              opacity: 0.15,
              borderRadius: borderRadius.pill,
              transition: "width 0.3s ease",
            }}
          />
          {/* 文字层 */}
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
        height: 32,
        display: "flex",
        alignItems: "center",
        padding: `0 ${spacing.xl}px`,
        borderBottom: `1px solid ${token.colorBorderSecondary}`,
        background: "inherit",
        gap: spacing.xl,
      }}
    >
      <Space size={spacing.lg}>
        {renderPill("CPU", resource.cpu_percent, {
          warning: 50,
          danger: 80,
        })}
        {renderPill(
          "MEM",
          resource.memory_percent,
          { warning: 60, danger: 85 },
          `${resource.memory_used_gb.toFixed(1)} / ${resource.memory_total_gb.toFixed(0)} GB`
        )}
        {resource.gpu_percent != null &&
          renderPill(
            "GPU",
            resource.gpu_percent,
            { warning: 50, danger: 80 },
            resource.gpu_memory_used_mb != null
              ? `${resource.gpu_memory_used_mb.toFixed(0)} MB`
              : undefined
          )}
      </Space>
      <div style={{ flex: 1 }} />
      <Tag color="green">
        运行中 {runningCount} / {totalCount}
      </Tag>
    </div>
  );
}
