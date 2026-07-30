import { Tag } from "antd";
import type { CSSProperties } from "react";
import type { ServiceStatus } from "../../types";
import { fontSizes } from "../../styles/tokens";

/**
 * 状态标签映射：color 为 Ant Design Tag 预设色，text 为显示文本
 */
const statusMap: Record<
  ServiceStatus,
  { color: string; text: string }
> = {
  running: { color: "green", text: "运行中" },
  stopped: { color: "default", text: "已停止" },
  starting: { color: "orange", text: "启动中" },
  stopping: { color: "orange", text: "停止中" },
  failed: { color: "red", text: "启动失败" },
  error: { color: "red", text: "异常" },
};

export interface StatusTagProps {
  /** 服务状态 */
  status: ServiceStatus;
  /** 异常状态是否启用 pulse 动画，默认 true */
  pulse?: boolean;
  /** 标签尺寸 */
  size?: "small" | "default";
  /** 额外样式 */
  style?: CSSProperties;
}

/**
 * 状态标签组件 — 封装 statusMap + 异常状态 pulse 动画。
 * Services / Dashboard / TrayPopup 统一使用此组件。
 */
export default function StatusTag({
  status,
  pulse = true,
  size = "default",
  style,
}: StatusTagProps) {
  const config = statusMap[status] ?? statusMap.stopped;
  const isError = status === "error" || status === "failed";

  const className = isError && pulse ? "tag-error-pulse" : undefined;
  const sizeStyle: CSSProperties =
    size === "small"
      ? { fontSize: fontSizes.XS.size, padding: "0 6px", lineHeight: "20px" }
      : {};

  return (
    <Tag
      color={config.color}
      className={className}
      style={{ marginRight: 0, ...sizeStyle, ...style }}
    >
      {config.text}
    </Tag>
  );
}
