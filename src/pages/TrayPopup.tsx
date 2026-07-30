import { Button, Card, Progress, Tooltip, Space, Empty, Typography } from "antd";
import {
  PlayCircleOutlined,
  PoweroffOutlined,
  ExportOutlined,
  ReloadOutlined,
  CloseOutlined,
} from "@ant-design/icons";
import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { api } from "../api";
import { useTauriEvent } from "../hooks/useTauriEvent";
import type {
  ServiceRuntime,
  SystemResource,
  ResourceUpdateEvent,
  StatusChangeEvent,
} from "../types";
import {
  semanticColors,
  neutralColors,
  fontSizes,
  spacing,
  borderRadius,
  getThresholdColor,
} from "../styles/tokens";
import StatusTag from "../components/common/StatusTag";

// macOS 平台检测（用于 GPU 利用率提示文案）
const isMac =
  typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

export default function TrayPopup() {
  const [services, setServices] = useState<ServiceRuntime[]>([]);
  const [resource, setResource] = useState<SystemResource | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const [svc, res] = await Promise.all([
        api.getServices(),
        api.getSystemResources(),
      ]);
      setServices(svc);
      setResource(res);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useTauriEvent<ResourceUpdateEvent>("resource-update", (payload) => {
    setResource(payload.system);
  });

  useTauriEvent<StatusChangeEvent>("service-status-changed", () => {
    fetchData();
  });

  const handleStart = async (id: string) => {
    try {
      await api.startService(id);
    } catch {
      /* ignore */
    }
  };
  const handleStop = async (id: string) => {
    try {
      await api.stopService(id);
    } catch {
      /* ignore */
    }
  };
  const handleVisit = async (url: string) => {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch {
      window.open(url, "_blank");
    }
  };
  const handleClose = async () => {
    try {
      await invoke("hide_tray_popup");
    } catch {
      /* ignore */
    }
  };
  const handleRefresh = () => {
    fetchData();
  };
  const handleOpenMain = async () => {
    try {
      const { Window } = await import("@tauri-apps/api/window");
      const mainWindow = await Window.getByLabel("main");
      if (mainWindow) {
        await mainWindow.show();
        await mainWindow.setFocus();
      }
    } catch {
      /* ignore */
    }
  };

  // 截断过长的服务名称
  const getShortName = (name: string) => {
    if (!name) return "";
    return name.length > 14 ? name.slice(0, 14) + "..." : name;
  };

  const runningCount = services.filter((s) => s.status === "running").length;

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        background:
          "linear-gradient(180deg, #e6f4ff 0%, #f0f7ff 50%, #ffffff 100%)",
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
        fontFamily:
          "-apple-system, BlinkMacSystemFont, 'PingFang SC', sans-serif",
      }}
    >
      {/* 顶部标题栏（可拖拽） */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          padding: `${fontSizes.SM.size}px ${fontSizes.MD.size}px`,
          borderBottom: `1px solid ${neutralColors.colorBorderSecondary}`,
          background: "rgba(255,255,255,0.6)",
          backdropFilter: "blur(8px)",
          WebkitAppRegion: "drag",
        } as React.CSSProperties}
      >
        <Typography.Text strong style={{ fontSize: fontSizes.Base.size }}>
          本地服务管理
        </Typography.Text>
        <Space
          size={fontSizes.XS.size}
          style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
        >
          <Button
            size="small"
            type="text"
            icon={<ReloadOutlined />}
            onClick={handleRefresh}
          />
          <Button
            size="small"
            type="text"
            icon={<ExportOutlined />}
            onClick={handleOpenMain}
            title="打开主窗口"
          />
          <Button
            size="small"
            type="text"
            icon={<CloseOutlined />}
            onClick={handleClose}
          />
        </Space>
      </div>

      {/* 内容区 */}
      <div style={{ flex: 1, overflowY: "auto", padding: fontSizes.MD.size }}>
        {/* 资源卡片区 */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: spacing.sm,
            marginBottom: spacing.md,
          }}
        >
          <Card size="small" style={{ borderRadius: borderRadius.card }}>
            <Typography.Text type="secondary" style={{ fontSize: fontSizes.XS.size }}>
              CPU
            </Typography.Text>
            <div style={{ fontSize: fontSizes.LG.size, fontWeight: fontSizes.LG.weight }}>
              {resource ? Math.round(resource.cpu_percent) : "-"}%
            </div>
            {resource && (
              <Progress
                percent={Math.round(resource.cpu_percent)}
                size="small"
                showInfo={false}
                strokeColor={getThresholdColor(resource.cpu_percent, {
                  warning: 50,
                  danger: 80,
                })}
              />
            )}
          </Card>
          <Card size="small" style={{ borderRadius: borderRadius.card }}>
            <Typography.Text type="secondary" style={{ fontSize: fontSizes.XS.size }}>
              内存
            </Typography.Text>
            <div style={{ fontSize: fontSizes.LG.size, fontWeight: fontSizes.LG.weight }}>
              {resource ? Math.round(resource.memory_percent) : "-"}%
            </div>
            {resource && (
              <Progress
                percent={Math.round(resource.memory_percent)}
                size="small"
                showInfo={false}
                strokeColor={getThresholdColor(resource.memory_percent, {
                  warning: 60,
                  danger: 85,
                })}
              />
            )}
            <Typography.Text type="secondary" style={{ fontSize: fontSizes.XS.size }}>
              {resource
                ? `${resource.memory_used_gb.toFixed(1)} / ${resource.memory_total_gb.toFixed(0)} GB`
                : ""}
            </Typography.Text>
          </Card>
          <Card
            size="small"
            style={{ borderRadius: borderRadius.card, gridColumn: "1 / span 2" }}
          >
            <Typography.Text type="secondary" style={{ fontSize: fontSizes.XS.size }}>
              GPU {resource?.gpu_name ? `· ${resource.gpu_name}` : ""}
            </Typography.Text>
            <div style={{ fontSize: fontSizes.MD.size, fontWeight: fontSizes.MD.weight }}>
              {resource?.gpu_percent != null ? (
                `${Math.round(resource.gpu_percent)}%`
              ) : (
                <Tooltip
                  title={
                    isMac
                      ? "macOS 平台限制，无法读取 GPU 利用率"
                      : "GPU 利用率不可用"
                  }
                >
                  <span style={{ fontSize: fontSizes.SM.size, color: neutralColors.colorTextSecondary }}>
                    不可用
                  </span>
                </Tooltip>
              )}
            </div>
            {resource?.gpu_percent != null && (
              <Progress
                percent={Math.round(resource.gpu_percent)}
                size="small"
                showInfo={false}
                strokeColor={getThresholdColor(resource.gpu_percent, {
                  warning: 50,
                  danger: 80,
                })}
              />
            )}
          </Card>
        </div>

        {/* 服务列表 */}
        <Card
          size="small"
          title={<span>服务列表 ({services.length})</span>}
          style={{ borderRadius: borderRadius.card }}
          bodyStyle={{ padding: spacing.xs }}
        >
          {services.length === 0 ? (
            <Empty
              description={loading ? "加载中..." : "暂无服务"}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              style={{ padding: "20px 0" }}
            />
          ) : (
            <div>
              {services.map((svc) => {
                const isRunning = svc.status === "running";
                const isStartable =
                  svc.status === "stopped" ||
                  svc.status === "failed" ||
                  svc.status === "error";
                return (
                  <div
                    key={svc.config.id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      padding: `${spacing.sm}px ${spacing.sm}px`,
                      borderBottom: `1px solid ${neutralColors.colorBgLayout}`,
                      gap: spacing.sm,
                    }}
                  >
                    {/* 状态点 */}
                    <span
                      style={{
                        width: spacing.xs + 4,
                        height: spacing.xs + 4,
                        borderRadius: "50%",
                        flexShrink: 0,
                        background: isRunning
                          ? semanticColors.colorSuccess
                          : svc.status === "failed" || svc.status === "error"
                            ? semanticColors.colorError
                            : neutralColors.colorBorder,
                      }}
                    />
                    {/* 简称 + tooltip 全称 */}
                    <Tooltip title={svc.config.name}>
                      <span
                        style={{
                          flex: 1,
                          fontSize: fontSizes.SM.size,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {getShortName(svc.config.name)}
                      </span>
                    </Tooltip>
                    {/* 状态标签 */}
                    <StatusTag status={svc.status} size="small" />
                    {/* 操作按钮 */}
                    <Space size={2}>
                      <Tooltip title="启动">
                        <Button
                          size="small"
                          type="text"
                          icon={<PlayCircleOutlined />}
                          disabled={!isStartable}
                          onClick={() => handleStart(svc.config.id)}
                          style={{
                            color: isStartable
                              ? semanticColors.colorSuccess
                              : undefined,
                          }}
                        />
                      </Tooltip>
                      <Tooltip title="停止">
                        <Button
                          size="small"
                          type="text"
                          icon={<PoweroffOutlined />}
                          disabled={!isRunning}
                          onClick={() => handleStop(svc.config.id)}
                          style={{
                            color: isRunning
                              ? semanticColors.colorError
                              : undefined,
                          }}
                        />
                      </Tooltip>
                      <Tooltip
                        title={
                          svc.config.url
                            ? `访问 ${svc.config.url}`
                            : "无 URL"
                        }
                      >
                        <Button
                          size="small"
                          type="text"
                          icon={<ExportOutlined />}
                          disabled={!isRunning || !svc.config.url}
                          onClick={() =>
                            svc.config.url && handleVisit(svc.config.url)
                          }
                          style={{
                            color:
                              isRunning && svc.config.url
                                ? semanticColors.colorPrimary
                                : undefined,
                          }}
                        />
                      </Tooltip>
                    </Space>
                  </div>
                );
              })}
            </div>
          )}
        </Card>
      </div>

      {/* 底部状态栏 */}
      <div
        style={{
          padding: `${spacing.xs + 2}px ${spacing.md}px`,
          borderTop: `1px solid ${neutralColors.colorBorderSecondary}`,
          background: "rgba(255,255,255,0.6)",
          fontSize: fontSizes.XS.size,
          color: neutralColors.colorTextSecondary,
          display: "flex",
          justifyContent: "space-between",
        }}
      >
        <span>
          {runningCount} / {services.length} 运行中
        </span>
        <span>v0.1.0</span>
      </div>
    </div>
  );
}
