import { Button, Empty, Space, Typography, Tooltip } from "antd";
import {
  ExportOutlined,
  ReloadOutlined,
  CloseOutlined,
  EditOutlined,
  PlusOutlined,
  PoweroffOutlined,
} from "@ant-design/icons";
import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { api, isTauriEnv } from "../api";
import { useTauriEvent } from "../hooks/useTauriEvent";
import type {
  ServiceRuntime,
  SystemResource,
  ResourceUpdateEvent,
  StatusChangeEvent,
  ResourceHistoryPoint,
  NetworkInfo,
} from "../types";
import {
  semanticColors,
  neutralColors,
  fontSizes,
  spacing,
  borderRadius,
} from "../styles/tokens";
import ServicePowerToggle from "../components/common/ServicePowerToggle";
import ServiceConfigModal from "../components/common/ServiceConfigModal";
import ResourceHistoryChart from "../components/common/ResourceHistoryChart";
import NetworkIpPills from "../components/common/NetworkIpPills";

export default function TrayPopup() {
  const [services, setServices] = useState<ServiceRuntime[]>([]);
  const [resource, setResource] = useState<SystemResource | null>(null);
  const [history, setHistory] = useState<ResourceHistoryPoint[]>([]);
  const [networkInfo, setNetworkInfo] = useState<NetworkInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingService, setEditingService] = useState<ServiceRuntime | null>(
    null
  );

  const fetchData = useCallback(async () => {
    try {
      const [svc, res, hist, net] = await Promise.all([
        api.getServices(),
        api.getSystemResources(),
        api.getResourceHistory(),
        api.getNetworkInfo(),
      ]);
      setServices(svc);
      setResource(res);
      setHistory(hist);
      setNetworkInfo(net);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    document.documentElement.classList.add("tray-popup-window");
    return () => {
      document.documentElement.classList.remove("tray-popup-window");
    };
  }, []);

  useEffect(() => {
    fetchData();
    const netInterval = setInterval(() => {
      api.getNetworkInfo().then(setNetworkInfo).catch(() => {});
    }, 60000);
    return () => clearInterval(netInterval);
  }, [fetchData]);

  // 浏览器打开托盘路由时同样走本机桥轮询
  useEffect(() => {
    if (isTauriEnv) return;
    const timer = setInterval(fetchData, 2000);
    return () => clearInterval(timer);
  }, [fetchData]);

  useTauriEvent<ResourceUpdateEvent>("resource-update", (payload) => {
    setResource(payload.system);
    if (payload.history) {
      setHistory(payload.history);
    }
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

  /**
   * 打开主窗口：经 Rust command，避免托盘 Webview ACL 无法操作 main 窗口。
   */
  const handleOpenMain = async () => {
    try {
      await invoke("show_main_window");
    } catch {
      /* ignore */
    }
  };

  /**
   * 彻底退出应用（停止托管服务 + 结束进程，含托盘）。
   * 与顶栏「关闭」仅隐藏弹窗不同。
   */
  const handleQuitApp = async () => {
    try {
      await api.quitApp();
    } catch {
      /* ignore */
    }
  };

  const handleAdd = () => {
    setEditingService(null);
    setModalOpen(true);
  };

  const handleEdit = (svc: ServiceRuntime) => {
    setEditingService(svc);
    setModalOpen(true);
  };

  const getShortName = (name: string) => {
    if (!name) return "";
    return name.length > 14 ? name.slice(0, 14) + "..." : name;
  };

  const runningCount = services.filter((s) => s.status === "running").length;

  return (
    <div className="tray-popup-shell">
      {/* 顶部标题栏（可拖拽） */}
      <div
        style={
          {
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: `${fontSizes.SM.size}px ${fontSizes.MD.size}px`,
            borderBottom: `1px solid ${neutralColors.colorBorderSecondary}`,
            background: "rgba(255,255,255,0.6)",
            backdropFilter: "blur(8px)",
            WebkitAppRegion: "drag",
          } as React.CSSProperties
        }
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
            title="关闭面板"
          />
        </Space>
      </div>

      {/* 内容区 */}
      <div style={{ flex: 1, overflowY: "auto", padding: fontSizes.MD.size }}>
        {/* 资源历史曲线：CPU / 内存 / GPU 单行三列 */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr 1fr",
            gap: spacing.sm,
            marginBottom: spacing.md,
          }}
        >
          <ResourceHistoryChart
            kind="cpu"
            history={history}
            resource={resource}
            height={64}
            compact
          />
          <ResourceHistoryChart
            kind="memory"
            history={history}
            resource={resource}
            height={64}
            compact
          />
          <ResourceHistoryChart
            kind="gpu"
            history={history}
            resource={resource}
            height={64}
            compact
          />
        </div>

        {/* 局域网 / 公网 IP */}
        <div
          style={{
            display: "flex",
            justifyContent: "center",
            marginBottom: spacing.md,
          }}
        >
          <NetworkIpPills
            networkInfo={networkInfo}
            compact
            variant="card"
          />
        </div>

        {/* 服务列表 */}
        <div
          style={{
            background: "#fff",
            borderRadius: borderRadius.card,
            border: "1px solid rgba(0,0,0,0.06)",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              padding: `${spacing.sm}px ${spacing.md}px`,
              borderBottom: `1px solid ${neutralColors.colorBorderSecondary}`,
              fontWeight: 600,
              fontSize: fontSizes.SM.size,
            }}
          >
            服务列表 ({services.length})
          </div>
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
                return (
                  <div
                    key={svc.config.id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      padding: `${spacing.sm}px ${spacing.sm}px`,
                      borderBottom: `1px solid ${neutralColors.colorBgLayout}`,
                      gap: spacing.xs,
                    }}
                  >
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
                    <Tooltip title={svc.config.name}>
                      <span
                        style={{
                          flex: 1,
                          minWidth: 0,
                          fontSize: fontSizes.SM.size,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {getShortName(svc.config.name)}
                      </span>
                    </Tooltip>
                    <Tooltip title="编辑">
                      <Button
                        size="small"
                        type="text"
                        icon={<EditOutlined />}
                        onClick={() => handleEdit(svc)}
                        style={{ color: semanticColors.colorPrimary }}
                      />
                    </Tooltip>
                    <ServicePowerToggle
                      status={svc.status}
                      onToggle={() => {
                        if (
                          svc.status === "stopped" ||
                          svc.status === "failed" ||
                          svc.status === "error"
                        ) {
                          handleStart(svc.config.id);
                        } else if (svc.status === "running") {
                          handleStop(svc.config.id);
                        }
                      }}
                    />
                    <Tooltip
                      title={
                        svc.config.url ? `访问 ${svc.config.url}` : "无 URL"
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
                  </div>
                );
              })}
            </div>
          )}

          {/* 列表下方：添加管理服务 */}
          <div
            style={{
              display: "flex",
              justifyContent: "center",
              padding: `${spacing.md}px ${spacing.sm}px`,
              borderTop: `1px solid ${neutralColors.colorBorderSecondary}`,
            }}
          >
            <Button
              type="dashed"
              icon={<PlusOutlined />}
              onClick={handleAdd}
              style={{ width: "100%", maxWidth: 260 }}
            >
              添加管理服务
            </Button>
          </div>
        </div>
      </div>

      {/* 底部：运行计数 + 退出应用（彻底结束托盘） */}
      <div
        style={{
          padding: `${spacing.sm}px ${spacing.md}px`,
          borderTop: `1px solid ${neutralColors.colorBorderSecondary}`,
          background: "rgba(255,255,255,0.75)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: spacing.sm,
        }}
      >
        <Typography.Text
          type="secondary"
          style={{ fontSize: fontSizes.XS.size }}
        >
          {runningCount} / {services.length} 运行中
        </Typography.Text>
        <Button
          danger
          type="primary"
          size="small"
          icon={<PoweroffOutlined />}
          onClick={handleQuitApp}
          style={{
            borderRadius: borderRadius.button,
            fontWeight: 500,
          }}
        >
          退出应用
        </Button>
      </div>

      <ServiceConfigModal
        open={modalOpen}
        editingService={editingService}
        onCancel={() => setModalOpen(false)}
        onSaved={() => {
          setModalOpen(false);
          fetchData();
        }}
      />
    </div>
  );
}
