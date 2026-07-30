import { Typography } from "antd";
import {
  Area,
  AreaChart,
  ResponsiveContainer,
  Tooltip,
  YAxis,
} from "recharts";
import type { ResourceHistoryPoint, SystemResource } from "../../types";
import { fontSizes, spacing, borderRadius } from "../../styles/tokens";

/** 图例单项：色块 + 标签 + 当前百分比 */
export interface LegendItem {
  key: string;
  label: string;
  color: string;
  value: number | null | undefined;
}

export type ResourceChartKind = "cpu" | "memory" | "gpu";

interface ResourceHistoryChartProps {
  /** 图表类型：CPU / 内存 / GPU */
  kind: ResourceChartKind;
  /** 启动后累计的历史采样 */
  history: ResourceHistoryPoint[];
  /** 当前资源快照（图例数值与副标题） */
  resource: SystemResource | null;
  /** 图表高度（px） */
  height?: number;
  /** 是否紧凑模式（托盘弹窗） */
  compact?: boolean;
}

/** 启动阶段左侧补零秒数，避免空数据等待；满窗后不再截断，完整保留历史 */
const BOOTSTRAP_ZERO_SECS = 50;
/** 槽位间隔（与后端约 1s 采样对齐） */
const SLOT_MS = 1000;

const COLORS = {
  user: "#1677ff",
  system: "#f5222d",
  idle: "#d9d9d9",
  used: "#1677ff",
  free: "#d9d9d9",
  gpuDevice: "#1677ff",
  gpuRenderer: "#f5222d",
  gpuTiler: "#13c2c2",
};

/** 单点图表数据结构 */
interface ChartPoint {
  ts: number;
  idx: number;
  user?: number;
  system?: number;
  idle?: number;
  used?: number;
  free?: number;
  device?: number;
  renderer?: number;
  tiler?: number;
}

/**
 * 构建图表数据：启动不足 50s 时左侧补 0；达到/超过 50s 后完整展示全部历史，不截断。
 *
 * 输入：原始 history、图表 kind
 * 输出：ChartPoint[]（从左到右：旧→新；新数据从右侧堆积）
 */
function buildChartData(
  history: ResourceHistoryPoint[],
  kind: ResourceChartKind
): ChartPoint[] {
  // 已有足够真实采样：直接映射全部历史，不再补零、也不截断
  if (history.length >= BOOTSTRAP_ZERO_SECS) {
    return history.map((p, i) => mapPoint(p, i, kind));
  }

  const anchorTs =
    history.length > 0 ? history[history.length - 1].ts : Date.now();
  const zerosNeeded = BOOTSTRAP_ZERO_SECS - history.length;

  const zeroSlots: ChartPoint[] = Array.from({ length: zerosNeeded }, (_, i) => {
    const ts = anchorTs - (BOOTSTRAP_ZERO_SECS - 1 - i) * SLOT_MS;
    return zeroPoint(ts, i, kind);
  });

  const realSlots = history.map((p, i) =>
    mapPoint(p, zerosNeeded + i, kind)
  );

  return [...zeroSlots, ...realSlots];
}

/** 构造全 0 占位点 */
function zeroPoint(ts: number, idx: number, kind: ResourceChartKind): ChartPoint {
  if (kind === "cpu") {
    return { ts, idx, user: 0, system: 0, idle: 100 };
  }
  if (kind === "memory") {
    return { ts, idx, used: 0, free: 100 };
  }
  return { ts, idx, device: 0, renderer: 0, tiler: 0 };
}

/** 将 ResourceHistoryPoint 映射为图表点 */
function mapPoint(
  p: ResourceHistoryPoint,
  idx: number,
  kind: ResourceChartKind
): ChartPoint {
  if (kind === "cpu") {
    const user = p.cpu_user_percent ?? p.cpu_percent;
    const system = p.cpu_system_percent ?? 0;
    const idle =
      p.cpu_idle_percent ?? Math.max(0, 100 - user - system);
    return { ts: p.ts, idx, user, system, idle };
  }
  if (kind === "memory") {
    const used = p.memory_percent;
    return { ts: p.ts, idx, used, free: Math.max(0, 100 - used) };
  }
  return {
    ts: p.ts,
    idx,
    device: p.gpu_percent ?? 0,
    renderer: p.gpu_renderer_percent ?? 0,
    tiler: p.gpu_tiler_percent ?? 0,
  };
}

/**
 * 资源利用率历史堆叠面积图（参考 macOS 活动监视器风格）。
 *
 * 启动不足 50s 时左侧补 0 防止空态；之后完整保留历史，从右侧堆积，不截断。
 */
export default function ResourceHistoryChart({
  kind,
  history,
  resource,
  height = 96,
  compact = false,
}: ResourceHistoryChartProps) {
  const chartData = buildChartData(history, kind);

  const title =
    kind === "cpu" ? "CPU" : kind === "memory" ? "内存" : "GPU";

  const subtitle =
    kind === "gpu"
      ? resource?.gpu_name ?? ""
      : kind === "cpu"
        ? resource?.gpu_name
          ? resource.gpu_name
          : ""
        : resource
          ? `${resource.memory_used_gb.toFixed(1)} / ${resource.memory_total_gb.toFixed(0)} GB`
          : "";

  const extraRight =
    kind === "gpu" && resource?.gpu_core_count != null
      ? `${resource.gpu_core_count} 核`
      : null;

  const legends: LegendItem[] =
    kind === "cpu"
      ? [
          {
            key: "user",
            label: "用户",
            color: COLORS.user,
            value: resource?.cpu_user_percent ?? resource?.cpu_percent,
          },
          {
            key: "system",
            label: "系统",
            color: COLORS.system,
            value: resource?.cpu_system_percent ?? 0,
          },
          {
            key: "idle",
            label: "闲置",
            color: COLORS.idle,
            value:
              resource?.cpu_idle_percent ??
              (resource ? Math.max(0, 100 - resource.cpu_percent) : null),
          },
        ]
      : kind === "memory"
        ? [
            {
              key: "used",
              label: "已用",
              color: COLORS.used,
              value: resource?.memory_percent,
            },
            {
              key: "free",
              label: "可用",
              color: COLORS.free,
              value: resource
                ? Math.max(0, 100 - resource.memory_percent)
                : null,
            },
          ]
        : [
            {
              key: "device",
              label: "利用率",
              color: COLORS.gpuDevice,
              value: resource?.gpu_percent,
            },
            {
              key: "renderer",
              label: "渲染利用率",
              color: COLORS.gpuRenderer,
              value: resource?.gpu_renderer_percent,
            },
            {
              key: "tiler",
              label: "Tiler 利用率",
              color: COLORS.gpuTiler,
              value: resource?.gpu_tiler_percent,
            },
          ];

  const pad = compact ? spacing.sm : spacing.md;

  return (
    <div
      style={{
        background: "#fff",
        borderRadius: borderRadius.card,
        border: "1px solid rgba(0,0,0,0.06)",
        padding: pad,
        height: "100%",
        display: "flex",
        flexDirection: "column",
        boxSizing: "border-box",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
          marginBottom: spacing.xs,
          gap: spacing.sm,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "baseline",
            gap: spacing.sm,
            minWidth: 0,
          }}
        >
          <Typography.Text
            strong
            style={{
              fontSize: compact ? fontSizes.SM.size : fontSizes.Base.size,
            }}
          >
            {title}
          </Typography.Text>
          {subtitle && (
            <Typography.Text
              type="secondary"
              style={{
                fontSize: fontSizes.XS.size,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {subtitle}
            </Typography.Text>
          )}
        </div>
        {extraRight && (
          <Typography.Text
            type="secondary"
            style={{ fontSize: fontSizes.XS.size, flexShrink: 0 }}
          >
            {extraRight}
          </Typography.Text>
        )}
      </div>

      <div
        style={{
          height,
          background: "rgba(0,0,0,0.03)",
          borderRadius: 6,
          marginBottom: spacing.sm,
        }}
      >
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart
            data={chartData}
            margin={{ top: 4, right: 4, left: 4, bottom: 0 }}
          >
            <YAxis domain={[0, 100]} hide />
            <Tooltip
              contentStyle={{ fontSize: 11 }}
              formatter={(value: number, name: string) => [
                `${Math.round(value)}%`,
                name === "user"
                  ? "用户"
                  : name === "system"
                    ? "系统"
                    : name === "idle"
                      ? "闲置"
                      : name === "used"
                        ? "已用"
                        : name === "free"
                          ? "可用"
                          : name === "device"
                            ? "利用率"
                            : name === "renderer"
                              ? "渲染"
                              : name === "tiler"
                                ? "Tiler"
                                : name,
              ]}
              labelFormatter={() => ""}
            />
            {kind === "cpu" && (
              <>
                <Area
                  type="monotone"
                  dataKey="user"
                  stackId="cpu"
                  stroke={COLORS.user}
                  fill={COLORS.user}
                  fillOpacity={0.85}
                  isAnimationActive={false}
                />
                <Area
                  type="monotone"
                  dataKey="system"
                  stackId="cpu"
                  stroke={COLORS.system}
                  fill={COLORS.system}
                  fillOpacity={0.85}
                  isAnimationActive={false}
                />
              </>
            )}
            {kind === "memory" && (
              <Area
                type="monotone"
                dataKey="used"
                stroke={COLORS.used}
                fill={COLORS.used}
                fillOpacity={0.85}
                isAnimationActive={false}
              />
            )}
            {kind === "gpu" && (
              <>
                <Area
                  type="monotone"
                  dataKey="device"
                  stroke={COLORS.gpuDevice}
                  fill={COLORS.gpuDevice}
                  fillOpacity={0.7}
                  isAnimationActive={false}
                />
                <Area
                  type="monotone"
                  dataKey="renderer"
                  stroke={COLORS.gpuRenderer}
                  fill={COLORS.gpuRenderer}
                  fillOpacity={0.45}
                  isAnimationActive={false}
                />
                <Area
                  type="monotone"
                  dataKey="tiler"
                  stroke={COLORS.gpuTiler}
                  fill={COLORS.gpuTiler}
                  fillOpacity={0.4}
                  isAnimationActive={false}
                />
              </>
            )}
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        {legends.map((item) => (
          <div
            key={item.key}
            style={{
              display: "flex",
              alignItems: "center",
              gap: spacing.sm,
              fontSize: fontSizes.XS.size,
            }}
          >
            <span
              style={{
                width: 10,
                height: 10,
                borderRadius: 2,
                background: item.color,
                flexShrink: 0,
              }}
            />
            <span style={{ flex: 1, color: "rgba(0,0,0,0.65)" }}>
              {item.label}
            </span>
            <span
              style={{ fontWeight: 500, fontVariantNumeric: "tabular-nums" }}
            >
              {item.value != null ? `${Math.round(item.value)}%` : "—"}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
