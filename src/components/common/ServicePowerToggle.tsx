import type { CSSProperties, MouseEvent } from "react";
import type { ServiceStatus } from "../../types";
import {
  fontSizes,
  semanticColors,
  neutralColors,
} from "../../styles/tokens";

export interface ServicePowerToggleProps {
  /** 当前服务状态 */
  status: ServiceStatus;
  /** 点击启停：stopped→start，running→stop；过渡态忽略 */
  onToggle: () => void;
  /** 是否禁用点击（如无权限） */
  disabled?: boolean;
  /** 额外样式 */
  style?: CSSProperties;
}

/**
 * 胶囊式服务启停开关（托盘列表专用）。
 *
 * 状态与视觉：
 * - stopped / failed / error：红点在左，文案「已停止」/「异常」
 * - starting：绿点在左，文案「启动中」
 * - running：绿点在右，文案「运行中」在左
 * - stopping：红点在右，文案「关闭中」
 *
 * 输入：status、onToggle
 * 输出：可点击胶囊按钮；过渡态（starting/stopping）不可再点
 */
export default function ServicePowerToggle({
  status,
  onToggle,
  disabled = false,
  style,
}: ServicePowerToggleProps) {
  const isRunning = status === "running";
  const isStarting = status === "starting";
  const isStopping = status === "stopping";
  const isError = status === "failed" || status === "error";
  const isBusy = isStarting || isStopping;

  // 圆点靠右：运行中 或 关闭中（尚未回到左边）
  const thumbRight = isRunning || isStopping;
  // 圆点颜色：启动中/运行中为绿，其余为红
  const thumbGreen = isRunning || isStarting;

  const label = isStarting
    ? "启动中"
    : isStopping
      ? "关闭中"
      : isRunning
        ? "运行中"
        : isError
          ? "异常"
          : "已停止";

  const canClick =
    !disabled &&
    !isBusy &&
    (status === "stopped" ||
      status === "running" ||
      status === "failed" ||
      status === "error");

  const handleClick = (e: MouseEvent) => {
    e.stopPropagation();
    if (!canClick) return;
    onToggle();
  };

  const width = 88;
  const height = 26;
  const thumb = 18;
  const pad = 4;

  return (
    <button
      type="button"
      aria-label={label}
      aria-busy={isBusy}
      disabled={!canClick}
      onClick={handleClick}
      title={
        isBusy
          ? label
          : isRunning
            ? "点击停止"
            : "点击启动"
      }
      style={{
        position: "relative",
        width,
        height,
        padding: 0,
        border: `1px solid ${neutralColors.colorBorder}`,
        borderRadius: height / 2,
        background: "#fff",
        boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
        cursor: canClick ? "pointer" : "default",
        flexShrink: 0,
        outline: "none",
        opacity: disabled ? 0.5 : 1,
        ...style,
      }}
    >
      {/* 文案：运行中/关闭中在左侧；已停止/启动中/异常在右侧（给左边圆点留空） */}
      <span
        style={{
          position: "absolute",
          top: 0,
          bottom: 0,
          display: "flex",
          alignItems: "center",
          fontSize: fontSizes.XS.size,
          fontWeight: 500,
          color: neutralColors.colorText,
          whiteSpace: "nowrap",
          pointerEvents: "none",
          transition: "left 0.25s ease, right 0.25s ease",
          ...(thumbRight
            ? { left: pad + 4, right: thumb + pad + 6, justifyContent: "flex-start" }
            : { left: thumb + pad + 6, right: pad + 4, justifyContent: "flex-end" }),
        }}
      >
        {label}
      </span>

      {/* 滑动圆点 */}
      <span
        style={{
          position: "absolute",
          top: (height - thumb) / 2,
          left: thumbRight ? width - thumb - pad : pad,
          width: thumb,
          height: thumb,
          borderRadius: "50%",
          background: thumbGreen
            ? semanticColors.colorSuccess
            : semanticColors.colorError,
          boxShadow: "0 1px 2px rgba(0,0,0,0.15)",
          transition: "left 0.28s ease, background-color 0.2s ease",
          pointerEvents: "none",
        }}
      />
    </button>
  );
}
