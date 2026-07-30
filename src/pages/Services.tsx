import {
  Table,
  Button,
  Space,
  Select,
  Tooltip,
  App,
  Popconfirm,
  Empty,
  Dropdown,
  theme,
  Input,
} from "antd";
import {
  ExportOutlined,
  PlusOutlined,
  ReloadOutlined,
  EditOutlined,
  DeleteOutlined,
  FileTextOutlined,
  MoreOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
} from "@ant-design/icons";
import { useEffect, useState, useCallback } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import { api } from "../api";
import { isTauriEnv } from "../api";
import { useTauriEvent } from "../hooks/useTauriEvent";
import type {
  ServiceRuntime,
  StatusChangeEvent,
  BatchResult,
} from "../types";
import {
  semanticColors,
  spacing,
  borderRadius,
} from "../styles/tokens";
import ServiceNameLink from "../components/common/ServiceNameLink";
import ServicePowerToggle from "../components/common/ServicePowerToggle";
import ServiceConfigModal from "../components/common/ServiceConfigModal";

/**
 * 服务管理页：方案 B — 列表语言向托盘靠拢（状态点 + 胶囊启停 + 白卡片表）。
 */
export default function Services() {
  const navigate = useNavigate();
  const { token } = theme.useToken();
  const [searchParams] = useSearchParams();
  const { message, modal } = App.useApp();
  const [services, setServices] = useState<ServiceRuntime[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingService, setEditingService] = useState<ServiceRuntime | null>(
    null
  );
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [filterGroup, setFilterGroup] = useState<string>("all");
  const [filterStatus, setFilterStatus] = useState<string>("all");
  const [searchText, setSearchText] = useState("");

  const fetchServices = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getServices();
      setServices(data);
    } catch (e) {
      message.error(String(e));
    }
    setLoading(false);
  }, [message]);

  useEffect(() => {
    fetchServices();
  }, [fetchServices]);

  useEffect(() => {
    const highlight = searchParams.get("highlight");
    if (highlight) {
      setTimeout(() => {
        const row = document.querySelector(`[data-row-key="${highlight}"]`);
        row?.scrollIntoView({ behavior: "smooth", block: "center" });
      }, 300);
    }
  }, [searchParams, services]);

  useTauriEvent<StatusChangeEvent>("service-status-changed", () => {
    fetchServices();
  });

  const openServiceUrl = (url: string) => {
    if (isTauriEnv) {
      import("@tauri-apps/plugin-shell")
        .then(({ open }) => {
          open(url);
        })
        .catch(() => window.open(url, "_blank"));
    } else {
      window.open(url, "_blank");
    }
  };

  const filteredServices = services.filter((s) => {
    if (filterGroup !== "all") {
      const g = s.config.group || "默认";
      if (g !== filterGroup) return false;
    }
    if (filterStatus !== "all") {
      if (filterStatus === "error") {
        if (s.status !== "error" && s.status !== "failed") return false;
      } else if (s.status !== filterStatus) {
        return false;
      }
    }
    if (
      searchText &&
      !s.config.name.toLowerCase().includes(searchText.toLowerCase())
    )
      return false;
    return true;
  });

  const groups = [
    "all",
    ...new Set(services.map((s) => s.config.group || "默认")),
  ];

  const handleAdd = () => {
    setEditingService(null);
    setModalOpen(true);
  };

  const handleEdit = (service: ServiceRuntime) => {
    setEditingService(service);
    setModalOpen(true);
  };

  const handleDelete = async (id: string, stopFirst: boolean) => {
    try {
      await api.removeService(id, stopFirst);
      message.success("服务已删除");
      fetchServices();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleStart = async (id: string) => {
    try {
      await api.startService(id);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleStop = async (id: string) => {
    try {
      await api.stopService(id);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleRestart = async (id: string) => {
    try {
      await api.restartService(id);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleBatchStart = async () => {
    try {
      const results = await api.batchStart(selectedRowKeys);
      showBatchResult(results);
      fetchServices();
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleBatchStop = async () => {
    try {
      const results = await api.batchStop(selectedRowKeys);
      showBatchResult(results);
      fetchServices();
    } catch (e) {
      message.error(String(e));
    }
  };

  const showBatchResult = (results: BatchResult[]) => {
    const success = results.filter((r) => r.success).length;
    const failed = results.filter((r) => !r.success).length;
    modal.info({
      title: "批量操作完成",
      content: `成功 ${success} 个，失败 ${failed} 个`,
    });
  };

  /** 状态点颜色（与托盘列表一致） */
  const statusDotColor = (status: ServiceRuntime["status"]) => {
    if (status === "running") return semanticColors.colorSuccess;
    if (status === "failed" || status === "error")
      return semanticColors.colorError;
    if (status === "starting" || status === "stopping")
      return semanticColors.colorWarning;
    return token.colorBorder;
  };

  const columns = [
    {
      title: "名称",
      dataIndex: ["config", "name"],
      key: "name",
      render: (name: string, record: ServiceRuntime) => (
        <Space size={spacing.sm}>
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: statusDotColor(record.status),
              display: "inline-block",
              flexShrink: 0,
            }}
          />
          <ServiceNameLink
            name={name}
            serviceId={record.config.id}
            onClick={(id) => navigate(`/logs/${id}`)}
          />
        </Space>
      ),
    },
    {
      title: "分组",
      key: "group",
      width: 100,
      render: (_: unknown, record: ServiceRuntime) =>
        record.config.group || "默认",
    },
    {
      title: "PID",
      key: "pid",
      width: 80,
      render: (_: unknown, record: ServiceRuntime) => record.pid || "-",
    },
    {
      title: "描述",
      key: "desc",
      ellipsis: true,
      render: (_: unknown, record: ServiceRuntime) =>
        record.config.description || "-",
    },
    {
      title: "启停",
      key: "power",
      width: 110,
      render: (_: unknown, record: ServiceRuntime) => (
        <ServicePowerToggle
          status={record.status}
          onToggle={() => {
            if (
              record.status === "stopped" ||
              record.status === "failed" ||
              record.status === "error"
            ) {
              handleStart(record.config.id);
            } else if (record.status === "running") {
              handleStop(record.config.id);
            }
          }}
        />
      ),
    },
    {
      title: "操作",
      key: "action",
      width: 180,
      render: (_: unknown, record: ServiceRuntime) => {
        const isRunning = record.status === "running";
        return (
          <Space size={2}>
            <Tooltip title="编辑">
              <Button
                size="small"
                type="text"
                icon={<EditOutlined />}
                style={{ color: semanticColors.colorPrimary }}
                onClick={() => handleEdit(record)}
              />
            </Tooltip>
            {isRunning && record.config.url && (
              <Tooltip title={`访问 ${record.config.url}`}>
                <Button
                  size="small"
                  type="text"
                  icon={<ExportOutlined />}
                  style={{ color: semanticColors.colorPrimary }}
                  onClick={() => openServiceUrl(record.config.url!)}
                />
              </Tooltip>
            )}
            {isRunning && (
              <Dropdown
                trigger={["hover"]}
                menu={{
                  items: [
                    {
                      key: "restart",
                      label: "重启",
                      icon: <ReloadOutlined />,
                    },
                  ],
                  onClick: () => handleRestart(record.config.id),
                }}
              >
                <Tooltip title="更多">
                  <Button size="small" type="text" icon={<MoreOutlined />} />
                </Tooltip>
              </Dropdown>
            )}
            <Tooltip title="日志">
              <Button
                size="small"
                type="text"
                icon={<FileTextOutlined />}
                style={{ color: token.colorTextSecondary }}
                onClick={() => navigate(`/logs/${record.config.id}`)}
              />
            </Tooltip>
            <Popconfirm
              title="确定删除此服务？"
              description={
                record.status === "running"
                  ? "该服务正在运行中，是否同时停止？"
                  : undefined
              }
              onConfirm={() =>
                handleDelete(record.config.id, record.status === "running")
              }
            >
              <Tooltip title="删除">
                <Button
                  size="small"
                  danger
                  type="text"
                  icon={<DeleteOutlined />}
                />
              </Tooltip>
            </Popconfirm>
          </Space>
        );
      },
    },
  ];

  return (
    <div>
      <div
        style={{
          marginBottom: spacing.md,
          display: "flex",
          gap: spacing.sm,
          alignItems: "center",
        }}
      >
        <Select
          value={filterGroup}
          onChange={setFilterGroup}
          style={{ width: 120 }}
          options={groups.map((g) => ({
            label: g === "all" ? "全部分组" : g,
            value: g,
          }))}
        />
        <Select
          value={filterStatus}
          onChange={setFilterStatus}
          style={{ width: 120 }}
          options={[
            { label: "全部状态", value: "all" },
            { label: "运行中", value: "running" },
            { label: "已停止", value: "stopped" },
            { label: "异常", value: "error" },
            { label: "启动失败", value: "failed" },
          ]}
        />
        <Input.Search
          placeholder="搜索服务名称..."
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          style={{ maxWidth: 300 }}
        />
      </div>

      <div
        style={{
          marginBottom: spacing.md,
          display: "flex",
          justifyContent: "flex-end",
        }}
      >
        <Space>
          {selectedRowKeys.length > 0 && (
            <>
              <Button icon={<PlayCircleOutlined />} onClick={handleBatchStart}>
                批量启动 ({selectedRowKeys.length})
              </Button>
              <Button
                danger
                icon={<PoweroffOutlined />}
                onClick={handleBatchStop}
              >
                批量停止 ({selectedRowKeys.length})
              </Button>
            </>
          )}
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
            添加服务
          </Button>
        </Space>
      </div>

      <div className="surface-card" style={{ borderRadius: borderRadius.card }}>
        <Table
          dataSource={filteredServices}
          columns={columns}
          rowKey={(r) => r.config.id}
          loading={loading}
          rowSelection={{
            selectedRowKeys,
            onChange: (keys) => setSelectedRowKeys(keys as string[]),
          }}
          pagination={false}
          size="middle"
          locale={{
            emptyText: (
              <Empty description="还没有服务">
                <Button
                  type="primary"
                  icon={<PlusOutlined />}
                  onClick={handleAdd}
                >
                  添加第一个服务
                </Button>
              </Empty>
            ),
          }}
          rowClassName={(record) => {
            if (record.status === "error" || record.status === "failed")
              return "row-error";
            return "";
          }}
          onRow={(record) =>
            ({
              "data-row-key": record.config.id,
            }) as React.HTMLAttributes<HTMLElement>
          }
        />
      </div>

      <ServiceConfigModal
        open={modalOpen}
        editingService={editingService}
        onCancel={() => setModalOpen(false)}
        onSaved={() => {
          setModalOpen(false);
          fetchServices();
        }}
      />
    </div>
  );
}
