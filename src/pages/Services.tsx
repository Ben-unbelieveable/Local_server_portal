import {
  Table,
  Button,
  Space,
  Modal,
  Form,
  Input,
  InputNumber,
  Select,
  Tooltip,
  App,
  Popconfirm,
  Typography,
  Empty,
  Row,
  Col,
  Dropdown,
  theme,
} from "antd";
import {
  ExportOutlined,
  PlusOutlined,
  PauseCircleOutlined,
  ReloadOutlined,
  EditOutlined,
  DeleteOutlined,
  FileTextOutlined,
  CaretRightOutlined,
  MoreOutlined,
} from "@ant-design/icons";
import { useEffect, useState, useCallback } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import { api } from "../api";
import { isTauriEnv } from "../api";
import { useTauriEvent } from "../hooks/useTauriEvent";
import type {
  ServiceRuntime,
  ServiceConfig,
  ServiceStatus,
  StatusChangeEvent,
  BatchResult,
} from "../types";
import { semanticColors, fontSizes, spacing } from "../styles/tokens";
import StatusTag from "../components/common/StatusTag";
import ServiceNameLink from "../components/common/ServiceNameLink";

const { TextArea } = Input;

export default function Services() {
  const navigate = useNavigate();
  const { token } = theme.useToken();
  const [searchParams] = useSearchParams();
  const { message, modal } = App.useApp();
  const [services, setServices] = useState<ServiceRuntime[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingService, setEditingService] = useState<ServiceRuntime | null>(null);
  const [form] = Form.useForm();
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

  // 高亮跳转
  useEffect(() => {
    const highlight = searchParams.get("highlight");
    if (highlight) {
      // 简单滚动定位
      setTimeout(() => {
        const row = document.querySelector(`[data-row-key="${highlight}"]`);
        row?.scrollIntoView({ behavior: "smooth", block: "center" });
      }, 300);
    }
  }, [searchParams, services]);

  useTauriEvent<StatusChangeEvent>("service-status-changed", () => {
    fetchServices();
  });

  // 打开服务 URL
  const openServiceUrl = (url: string) => {
    if (isTauriEnv) {
      import("@tauri-apps/plugin-shell").then(({ open }) => {
        open(url);
      }).catch(() => window.open(url, "_blank"));
    } else {
      window.open(url, "_blank");
    }
  };

  // 过滤
  const filteredServices = services.filter((s) => {
    if (filterGroup !== "all") {
      const g = s.config.group || "默认";
      if (g !== filterGroup) return false;
    }
    if (filterStatus !== "all") {
      // 「异常」与仪表盘口径保持一致：命中 error 或 failed（启动失败）
      if (filterStatus === "error") {
        if (s.status !== "error" && s.status !== "failed") return false;
      } else if (s.status !== filterStatus) {
        return false;
      }
    }
    if (searchText && !s.config.name.toLowerCase().includes(searchText.toLowerCase())) return false;
    return true;
  });

  const groups = ["all", ...new Set(services.map((s) => s.config.group || "默认"))];

  const handleAdd = () => {
    setEditingService(null);
    form.resetFields();
    form.setFieldsValue({ stop_timeout: 10 });
    setModalOpen(true);
  };

  const handleEdit = (service: ServiceRuntime) => {
    setEditingService(service);
    form.setFieldsValue({
      id: service.config.id,
      name: service.config.name,
      command: service.config.command,
      url: service.config.url || "",
      work_dir: service.config.work_dir || "",
      group: service.config.group || "",
      description: service.config.description || "",
      stop_timeout: service.config.stop_timeout,
    });
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const config: ServiceConfig = {
        id: editingService ? editingService.config.id : values.id,
        name: values.name,
        command: values.command,
        url: values.url || undefined,
        work_dir: values.work_dir || undefined,
        env: {},
        group: values.group || undefined,
        description: values.description || undefined,
        stop_timeout: values.stop_timeout || 10,
      };

      if (editingService) {
        await api.updateService(editingService.config.id, config);
        message.success("服务已更新");
        if (editingService.status === "running") {
          message.info("部分修改将在下次启动时生效");
        }
      } else {
        await api.addService(config);
        message.success("服务已添加");
      }
      setModalOpen(false);
      fetchServices();
    } catch (e) {
      if (e && typeof e === "object" && "errorFields" in e) return;
      message.error(String(e));
    }
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

  const columns = [
    {
      title: "名称",
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
      title: "分组",
      key: "group",
      width: 100,
      render: (_: unknown, record: ServiceRuntime) => record.config.group || "默认",
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
      title: "操作",
      key: "action",
      width: 200,
      render: (_: unknown, record: ServiceRuntime) => {
        const isRunning = record.status === "running";
        const isTransition =
          record.status === "starting" || record.status === "stopping";
        return (
          <Space size="small">
            {isRunning ? (
              <>
                {/* 停止 — Danger 红 */}
                <Tooltip title="停止">
                  <Button
                    size="small"
                    danger
                    icon={<PauseCircleOutlined />}
                    onClick={() => handleStop(record.config.id)}
                  />
                </Tooltip>
                {/* 访问 — Default 蓝 */}
                {record.config.url && (
                  <Tooltip title={`访问 ${record.config.url}`}>
                    <Button
                      size="small"
                      type="default"
                      icon={<ExportOutlined />}
                      style={{ color: semanticColors.colorPrimary }}
                      onClick={() => openServiceUrl(record.config.url!)}
                    />
                  </Tooltip>
                )}
                {/* 更多操作 — Dropdown hover 触发 */}
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
                  <Tooltip title="更多操作">
                    <Button size="small" icon={<MoreOutlined />} />
                  </Tooltip>
                </Dropdown>
              </>
            ) : isTransition ? (
              <Button size="small" loading disabled>
                处理中
              </Button>
            ) : (
              /* 启动 — Primary 蓝 */
              <Tooltip title="启动">
                <Button
                  size="small"
                  type="primary"
                  icon={<CaretRightOutlined />}
                  onClick={() => handleStart(record.config.id)}
                />
              </Tooltip>
            )}
            {/* 日志 — Default 灰 */}
            <Tooltip title="日志">
              <Button
                size="small"
                type="default"
                icon={<FileTextOutlined />}
                style={{ color: token.colorTextSecondary }}
                onClick={() => navigate(`/logs/${record.config.id}`)}
              />
            </Tooltip>
            {/* 编辑 — Default 灰 */}
            <Tooltip title="编辑">
              <Button
                size="small"
                type="default"
                icon={<EditOutlined />}
                style={{ color: token.colorTextSecondary }}
                onClick={() => handleEdit(record)}
              />
            </Tooltip>
            {/* 删除 — Danger 灰红 */}
            <Popconfirm
              title="确定删除此服务？"
              description={
                record.status === "running"
                  ? "该服务正在运行中，是否同时停止？"
                  : undefined
              }
              onConfirm={() => handleDelete(record.config.id, record.status === "running")}
            >
              <Tooltip title="删除">
                <Button size="small" danger type="text" icon={<DeleteOutlined />} />
              </Tooltip>
            </Popconfirm>
          </Space>
        );
      },
    },
  ];

  return (
    <div>
      {/* 工具栏 — 双层分层 */}
      {/* 第一层：筛选行 */}
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
          options={groups.map((g) => ({ label: g === "all" ? "全部分组" : g, value: g }))}
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

      {/* 第二层：操作行 */}
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
              <Button onClick={handleBatchStart}>
                <CaretRightOutlined /> 批量启动 ({selectedRowKeys.length})
              </Button>
              <Button onClick={handleBatchStop}>
                <PauseCircleOutlined /> 批量停止 ({selectedRowKeys.length})
              </Button>
            </>
          )}
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
            添加服务
          </Button>
        </Space>
      </div>

      {/* 表格 */}
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
              <Button type="primary" icon={<PlusOutlined />} onClick={handleAdd}>
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
        onRow={(record) => ({
          "data-row-key": record.config.id,
        } as React.HTMLAttributes<HTMLElement>)}
      />

      {/* 添加/编辑弹窗 */}
      <Modal
        title={editingService ? "编辑服务" : "添加服务"}
        open={modalOpen}
        onOk={handleSave}
        onCancel={() => setModalOpen(false)}
        width={600}
        okText="保存"
        cancelText="取消"
      >
        <Form form={form} layout="vertical">
          {!editingService && (
            <Form.Item
              name="id"
              label="服务 ID"
              rules={[
                { required: true, message: "请输入服务 ID" },
                {
                  pattern: /^[a-zA-Z0-9_-]+$/,
                  message: "仅支持字母、数字、下划线和连字符",
                },
              ]}
            >
              <Input placeholder="如：antibody_annotation" />
            </Form.Item>
          )}
          <Form.Item
            name="name"
            label="服务名称"
            rules={[{ required: true, message: "请输入服务名称" }]}
          >
            <Input placeholder="如：Antibody Annotation" />
          </Form.Item>
          <Form.Item
            name="command"
            label="启动命令"
            rules={[{ required: true, message: "请输入启动命令" }]}
          >
            <TextArea
              rows={2}
              placeholder="如：bash /path/to/deploy.sh"
            />
          </Form.Item>
          <Form.Item name="work_dir" label="工作目录">
            <Input placeholder="默认为命令所在目录" />
          </Form.Item>
          <Form.Item
            name="url"
            label="访问地址"
            tooltip="服务运行后可访问的 URL，如 http://localhost:3000。设置后将在仪表盘和服务管理中显示「访问」按钮"
          >
            <Input placeholder="如：http://localhost:3000" />
          </Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="group" label="分组">
                <Input placeholder="如：Web服务" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="stop_timeout" label="停止超时（秒）">
                <InputNumber min={1} max={60} style={{ width: "100%" }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item name="description" label="描述">
            <TextArea rows={2} placeholder="服务描述（可选）" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
