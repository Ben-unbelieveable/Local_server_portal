import {
  Card,
  Col,
  Row,
  Statistic,
  Table,
  Tag,
  Progress,
  Empty,
  Button,
  Typography,
  Tooltip,
  Spin,
  Divider,
} from "antd";
import {
  PlusOutlined,
  ExportOutlined,
} from "@ant-design/icons";
import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { api, isTauriEnv } from "../api";
import { useTauriEvent } from "../hooks/useTauriEvent";
import type {
  ServiceRuntime,
  SystemResource,
  ResourceUpdateEvent,
  StatusChangeEvent,
  ServiceResource,
  ResourceHistoryPoint,
} from "../types";
import {
  semanticColors,
  fontSizes,
  getThresholdColor,
  borderRadius,
} from "../styles/tokens";
import StatusTag from "../components/common/StatusTag";
import ServiceNameLink from "../components/common/ServiceNameLink";
import ResourceHistoryChart from "../components/common/ResourceHistoryChart";

export default function Dashboard() {
  const navigate = useNavigate();
  const [services, setServices] = useState<ServiceRuntime[]>([]);
  const [resource, setResource] = useState<SystemResource | null>(null);
  const [history, setHistory] = useState<ResourceHistoryPoint[]>([]);
  const [svcResources, setSvcResources] = useState<ServiceResource[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const [svc, res, svcRes, hist] = await Promise.all([
        api.getServices(),
        api.getSystemResources(),
        api.getServiceResources(),
        api.getResourceHistory(),
      ]);
      setServices(svc);
      setResource(res);
      setSvcResources(svcRes);
      setHistory(hist);
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
    setSvcResources(payload.services);
    if (payload.history) {
      setHistory(payload.history);
    }
  });

  useTauriEvent<StatusChangeEvent>("service-status-changed", () => {
    fetchData();
  });

  const runningCount = services.filter((s) => s.status === "running").length;
  const stoppedCount = services.filter((s) => s.status === "stopped").length;
  const errorCount = services.filter(
    (s) => s.status === "failed" || s.status === "error"
  ).length;

  // Top 5 by memory
  const top5 = [...svcResources]
    .sort((a, b) => b.memory_mb - a.memory_mb)
    .slice(0, 5);

  // 打开服务 URL
  const openServiceUrl = (url: string) => {
    if (isTauriEnv) {
      import("@tauri-apps/plugin-shell")
        .then(({ open }) => {
          open(url);
        })
        .catch(() => {
          window.open(url, "_blank");
        });
    } else {
      window.open(url, "_blank");
    }
  };

  const columns = [
    {
      title: "服务名称",
      dataIndex: ["config", "name"],
      key: "name",
      render: (name: string, record: ServiceRuntime) => (
        <ServiceNameLink
          name={name}
          serviceId={record.config.id}
          onClick={(id) => navigate(`/logs/${id}`)}
        />
      ),
    },
    {
      title: "状态",
      dataIndex: "status",
      key: "status",
      width: 100,
      render: (_: unknown, record: ServiceRuntime) => (
        <StatusTag status={record.status} />
      ),
    },
    {
      title: "CPU",
      key: "cpu",
      width: 90,
      render: (_: unknown, record: ServiceRuntime) => {
        if (record.status !== "running") return "-";
        const r = svcResources.find((x) => x.service_id === record.config.id);
        return (
          <Progress
            percent={r ? Math.round(r.cpu_percent) : 0}
            size="small"
            strokeColor={getThresholdColor(r?.cpu_percent ?? 0)}
          />
        );
      },
    },
    {
      title: "内存",
      key: "memory",
      width: 90,
      render: (_: unknown, record: ServiceRuntime) => {
        if (record.status !== "running") return "-";
        const r = svcResources.find((x) => x.service_id === record.config.id);
        return r ? `${r.memory_mb.toFixed(0)} MB` : "-";
      },
    },
    {
      title: "运行时长",
      key: "uptime",
      width: 90,
      render: (_: unknown, record: ServiceRuntime) => {
        if (record.status !== "running") return "-";
        const h = Math.floor(record.uptime_secs / 3600);
        const m = Math.floor((record.uptime_secs % 3600) / 60);
        const s = record.uptime_secs % 60;
        return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
      },
    },
    {
      title: "操作",
      key: "action",
      width: 80,
      render: (_: unknown, record: ServiceRuntime) => {
        if (record.status === "running" && record.config.url) {
          return (
            <Tooltip title={`访问 ${record.config.url}`}>
              <Button
                size="small"
                type="link"
                icon={<ExportOutlined />}
                onClick={(e) => {
                  e.stopPropagation();
                  openServiceUrl(record.config.url!);
                }}
              >
                访问
              </Button>
            </Tooltip>
          );
        }
        return "-";
      },
    },
  ];

  if (loading) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: "60vh",
        }}
      >
        <Spin size="large" tip="加载中..." />
      </div>
    );
  }

  if (services.length === 0) {
    return (
      <Empty description="还没有服务" style={{ marginTop: 80 }}>
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => navigate("/services")}
        >
          添加第一个服务
        </Button>
      </Empty>
    );
  }

  return (
    <div>
      {/* 资源历史曲线：CPU / 内存 / GPU + 服务计数 */}
      <Row gutter={[16, 16]} style={{ display: "flex", alignItems: "stretch" }}>
        <Col span={6} style={{ display: "flex" }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <ResourceHistoryChart
              kind="cpu"
              history={history}
              resource={resource}
              height={110}
            />
          </div>
        </Col>
        <Col span={6} style={{ display: "flex" }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <ResourceHistoryChart
              kind="memory"
              history={history}
              resource={resource}
              height={110}
            />
          </div>
        </Col>
        <Col span={6} style={{ display: "flex" }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <ResourceHistoryChart
              kind="gpu"
              history={history}
              resource={resource}
              height={110}
            />
          </div>
        </Col>
        <Col span={6} style={{ display: "flex" }}>
          <Card
            style={{
              flex: 1,
              height: "100%",
              borderRadius: borderRadius.card,
              border: "1px solid rgba(0,0,0,0.06)",
              boxShadow: "none",
            }}
            styles={{ body: { padding: 16 } }}
          >
            <Statistic
              title="运行中"
              value={runningCount}
              valueStyle={{
                color: semanticColors.colorSuccess,
                fontSize: fontSizes.LG.size,
              }}
            />
            <Divider style={{ margin: "12px 0" }} />
            <Statistic
              title="已停止"
              value={stoppedCount}
              valueStyle={{ fontSize: fontSizes.LG.size }}
            />
            <Divider style={{ margin: "12px 0" }} />
            <Statistic
              title="异常"
              value={errorCount}
              valueStyle={{
                color: errorCount > 0 ? semanticColors.colorError : undefined,
                fontSize: fontSizes.LG.size,
              }}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col span={16}>
          <Card
            title="服务列表"
            style={{
              borderRadius: borderRadius.card,
              border: "1px solid rgba(0,0,0,0.06)",
              boxShadow: "none",
            }}
          >
            <Table
              dataSource={services}
              columns={columns}
              rowKey={(r) => r.config.id}
              pagination={false}
              size="small"
              rowClassName={(record) => {
                if (record.status === "error" || record.status === "failed")
                  return "row-error";
                return "";
              }}
              onRow={(record) => ({
                style: { cursor: "pointer" },
                onClick: () =>
                  navigate(`/services?highlight=${record.config.id}`),
              })}
            />
          </Card>
        </Col>
        <Col span={8}>
          <Card
            title="内存占用 Top 5"
            style={{
              borderRadius: borderRadius.card,
              border: "1px solid rgba(0,0,0,0.06)",
              boxShadow: "none",
            }}
          >
            {top5.length === 0 ? (
              <Empty
                description="暂无运行中的服务"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            ) : (
              top5.map((item, index) => {
                const svc = services.find(
                  (s) => s.config.id === item.service_id
                );
                return (
                  <div
                    key={item.service_id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      marginBottom: 8,
                      gap: 8,
                    }}
                  >
                    <Tag color={index < 3 ? "gold" : "default"}>{index + 1}</Tag>
                    <span style={{ flex: 1 }}>
                      {svc?.config.name || item.service_id}
                    </span>
                    <Progress
                      percent={Math.round(
                        (item.memory_mb /
                          (resource?.memory_total_gb ?? 1) /
                          1024) *
                          100
                      )}
                      size="small"
                      style={{ width: 100 }}
                      showInfo={false}
                    />
                    <Typography.Text
                      type="secondary"
                      style={{ fontSize: fontSizes.SM.size, width: 60 }}
                    >
                      {item.memory_mb.toFixed(0)} MB
                    </Typography.Text>
                  </div>
                );
              })
            )}
          </Card>
        </Col>
      </Row>
    </div>
  );
}
