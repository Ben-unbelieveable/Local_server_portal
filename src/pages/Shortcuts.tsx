import { useCallback, useEffect, useState } from "react";
import {
  Alert,
  App,
  Button,
  Card,
  Col,
  Empty,
  Form,
  Input,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Spin,
  Tag,
  Typography,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  ExportOutlined,
  FileOutlined,
  GlobalOutlined,
  PlusOutlined,
  ReloadOutlined,
} from "@ant-design/icons";
import { api, isTauriEnv } from "../api";
import ShortcutLink from "../components/common/ShortcutLink";
import { useTauriEvent } from "../hooks/useTauriEvent";
import { spacing } from "../styles/tokens";
import type { ShortcutConfig } from "../types";
import { normalizeShortcutUrl } from "../utils/shortcutUrl";

type ShortcutFields = Omit<ShortcutConfig, "id">;

/** 无输入；展示共享入口列表，并通过表单完成持久化增删改及分组、搜索。 */
export default function Shortcuts() {
  const { message } = App.useApp();
  const [shortcuts, setShortcuts] = useState<ShortcutConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [group, setGroup] = useState<string>();
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<ShortcutConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm<ShortcutFields>();

  /** 无输入；读取共享后端，失败保留列表并展示可重试的错误。 */
  const fetchShortcuts = useCallback(async () => {
    setLoading(true);
    try {
      setShortcuts(await api.getShortcuts());
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchShortcuts();
  }, [fetchShortcuts]);
  useTauriEvent("shortcuts-changed", fetchShortcuts);
  useTauriEvent("config-reloaded", fetchShortcuts);

  // 浏览器没有 Tauri 事件，通过本机桥与桌面侧的入口修改同步。
  useEffect(() => {
    if (isTauriEnv) return;
    const timer = setInterval(() => {
      api
        .getShortcuts()
        .then((data) => {
          setShortcuts(data);
          setError(null);
        })
        .catch(() => {});
    }, 2000);
    return () => clearInterval(timer);
  }, []);

  const groups = [...new Set(shortcuts.map((s) => s.group || ""))];
  const search = query.trim().toLocaleLowerCase();
  const filtered = shortcuts.filter(
    (s) =>
      (group === undefined || (s.group || "") === group) &&
      [s.name, s.url, s.group, s.description].some((value) =>
        value?.toLocaleLowerCase().includes(search),
      ),
  );

  /** 输入待编辑入口或 null；初始化弹窗，避免复用上次表单内容。 */
  const showEditor = (shortcut: ShortcutConfig | null) => {
    setEditing(shortcut);
    form.resetFields();
    if (shortcut)
      form.setFieldsValue({
        ...shortcut,
        group: shortcut.group ?? "",
        description: shortcut.description ?? "",
      });
    setModalOpen(true);
  };

  /** 输入表单字段；规范化地址并调用共享 CRUD，成功后关闭表单及刷新列表。 */
  const save = async (values: ShortcutFields) => {
    setSaving(true);
    try {
      const config: ShortcutConfig = {
        id: editing?.id ?? crypto.randomUUID(),
        name: values.name.trim(),
        url: normalizeShortcutUrl(values.url),
        group: values.group?.trim() || undefined,
        description: values.description?.trim() || undefined,
      };
      if (editing) await api.updateShortcut(editing.id, config);
      else await api.addShortcut(config);
      setModalOpen(false);
      message.success(editing ? "入口已更新" : "入口已添加");
      setGroup(undefined);
      setQuery("");
      await fetchShortcuts();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  /** 输入入口 ID；确认后仅删除配置，成功则刷新并重置可能失效的分组筛选。 */
  const remove = async (id: string) => {
    try {
      await api.removeShortcut(id);
      message.success("入口已删除");
      setGroup(undefined);
      await fetchShortcuts();
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div style={{ minWidth: 0 }}>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "space-between",
          gap: spacing.md,
          marginBottom: spacing.lg,
        }}
      >
        <div>
          <Typography.Title level={4} style={{ margin: "0 0 4px" }}>
            常用入口
          </Typography.Title>
          <Typography.Text type="secondary">
            收藏常用服务和页面，一键访问，无需配置启动命令。
          </Typography.Text>
        </div>
        <Space>
          <Button
            icon={<ReloadOutlined />}
            onClick={() => void fetchShortcuts()}
            loading={loading}
          >
            刷新
          </Button>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={() => showEditor(null)}
          >
            添加入口
          </Button>
        </Space>
      </div>

      {error && (
        <Alert
          type="error"
          showIcon
          message="无法读取常用入口"
          description={error}
          style={{ marginBottom: spacing.md }}
        />
      )}

      <Space wrap style={{ marginBottom: spacing.lg }}>
        <Input.Search
          aria-label="搜索入口"
          placeholder="搜索名称、地址、分组或说明"
          allowClear
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          style={{ width: 280, maxWidth: "100%" }}
        />
        <Select
          aria-label="筛选入口分组"
          placeholder="全部分组"
          allowClear
          value={group}
          onChange={setGroup}
          style={{ width: 160 }}
          options={groups.map((value) => ({ value, label: value || "未分组" }))}
        />
        <Typography.Text type="secondary">
          {filtered.length} / {shortcuts.length} 个入口
        </Typography.Text>
      </Space>

      <Spin spinning={loading}>
        {filtered.length ? (
          <Row gutter={[spacing.md, spacing.md]}>
            {filtered.map((shortcut) => (
              <Col
                key={shortcut.id}
                xs={24}
                md={12}
                xl={8}
                style={{ display: "flex" }}
              >
                <Card
                  style={{ width: "100%", display: "flex", flexDirection: "column" }}
                  styles={{ body: { flex: 1 } }}
                  title={
                    <Space style={{ maxWidth: "100%" }}>
                      {shortcut.url.startsWith("file:") ? (
                        <FileOutlined />
                      ) : (
                        <GlobalOutlined />
                      )}
                      <ShortcutLink shortcut={shortcut} />
                    </Space>
                  }
                  actions={[
                    <ShortcutLink key="visit" shortcut={shortcut}>
                      <ExportOutlined /> 访问
                    </ShortcutLink>,
                    <Button
                      key="edit"
                      type="text"
                      icon={<EditOutlined />}
                      onClick={() => showEditor(shortcut)}
                    >
                      编辑
                    </Button>,
                    <Popconfirm
                      key="delete"
                      title={`删除「${shortcut.name}」入口？`}
                      description="仅移除快捷入口。"
                      okText="删除"
                      cancelText="取消"
                      onConfirm={() => remove(shortcut.id)}
                    >
                      <Button danger type="text" icon={<DeleteOutlined />}>
                        删除
                      </Button>
                    </Popconfirm>,
                  ]}
                >
                  <Tag>{shortcut.group || "未分组"}</Tag>
                  {shortcut.url.startsWith("file:") && (
                    <Tag color="blue">本地 HTML</Tag>
                  )}
                  <Typography.Paragraph
                    type="secondary"
                    style={{ margin: "12px 0 8px", overflowWrap: "anywhere" }}
                    copyable={{ text: shortcut.url }}
                  >
                    {shortcut.url}
                  </Typography.Paragraph>
                  <Typography.Paragraph
                    style={{
                      marginBottom: 0,
                      whiteSpace: "pre-wrap",
                      overflowWrap: "anywhere",
                    }}
                  >
                    {shortcut.description || "暂无说明"}
                  </Typography.Paragraph>
                </Card>
              </Col>
            ))}
          </Row>
        ) : !loading && !error ? (
          <Empty
            description={
              shortcuts.length
                ? "没有匹配的入口"
                : "还没有常用入口，添加常用网站或本地页面吧"
            }
          >
            {shortcuts.length ? (
              <Button
                onClick={() => {
                  setQuery("");
                  setGroup(undefined);
                }}
              >
                清除筛选
              </Button>
            ) : (
              <Button type="primary" onClick={() => showEditor(null)}>
                添加第一个入口
              </Button>
            )}
          </Empty>
        ) : null}
      </Spin>

      <Modal
        forceRender
        title={editing ? "编辑入口" : "添加入口"}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={() => form.submit()}
        confirmLoading={saving}
        cancelButtonProps={{ disabled: saving }}
        closable={!saving}
        maskClosable={!saving}
        keyboard={!saving}
        okText="保存"
        cancelText="取消"
      >
        <Form form={form} layout="vertical" onFinish={save} disabled={saving}>
          <Form.Item
            name="name"
            label="名称"
            rules={[
              { required: true, whitespace: true, message: "请输入入口名称" },
            ]}
          >
            <Input placeholder="如：GitHub、本地控制台" />
          </Form.Item>
          <Form.Item
            name="url"
            label="访问地址"
            extra="支持互联网、本机/局域网 HTTP(S) 地址，或 file:/// 开头的本地 HTML 页面。"
            rules={[
              { required: true, message: "请输入访问地址" },
              {
                validator: async (_, value) => {
                  if (value) normalizeShortcutUrl(value);
                },
              },
            ]}
          >
            <Input placeholder="https://github.com 或 http://localhost:3000" />
          </Form.Item>
          <Form.Item name="group" label="分组">
            <Input placeholder="如：开发工具、本地服务（可选）" />
          </Form.Item>
          <Form.Item name="description" label="说明">
            <Input.TextArea rows={2} placeholder="页面或服务的用途（可选）" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
