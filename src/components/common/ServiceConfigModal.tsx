import {
  Modal,
  Form,
  Input,
  InputNumber,
  Row,
  Col,
  App,
} from "antd";
import { useEffect } from "react";
import { api } from "../../api";
import type { ServiceConfig, ServiceRuntime } from "../../types";

const { TextArea } = Input;

export interface ServiceConfigModalProps {
  /** 是否打开 */
  open: boolean;
  /** 编辑时传入已有服务；新增时为 null */
  editingService: ServiceRuntime | null;
  /** 关闭回调 */
  onCancel: () => void;
  /** 保存成功后回调（刷新列表等） */
  onSaved: () => void;
}

/**
 * 服务新增/编辑配置弹窗。
 *
 * 输入：open、editingService（null=新增）、onCancel、onSaved
 * 输出：Modal + Form；成功调用 addService / updateService 后触发 onSaved
 */
export default function ServiceConfigModal({
  open,
  editingService,
  onCancel,
  onSaved,
}: ServiceConfigModalProps) {
  const { message } = App.useApp();
  const [form] = Form.useForm();

  useEffect(() => {
    if (!open) return;
    if (editingService) {
      form.setFieldsValue({
        id: editingService.config.id,
        name: editingService.config.name,
        command: editingService.config.command,
        url: editingService.config.url || "",
        work_dir: editingService.config.work_dir || "",
        group: editingService.config.group || "",
        description: editingService.config.description || "",
        stop_timeout: editingService.config.stop_timeout,
      });
    } else {
      form.resetFields();
      form.setFieldsValue({ stop_timeout: 10 });
    }
  }, [open, editingService, form]);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const config: ServiceConfig = {
        id: editingService ? editingService.config.id : values.id,
        name: values.name,
        command: values.command,
        url: values.url || undefined,
        work_dir: values.work_dir || undefined,
        env: editingService?.config.env ?? {},
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
      onSaved();
    } catch (e) {
      if (e && typeof e === "object" && "errorFields" in e) return;
      message.error(String(e));
    }
  };

  return (
    <Modal
      title={editingService ? "编辑服务" : "添加服务"}
      open={open}
      onOk={handleSave}
      onCancel={onCancel}
      width={520}
      okText="保存"
      cancelText="取消"
      destroyOnClose
      // 托盘小窗内弹层需挂到当前窗口，避免定位异常
      getContainer={() => document.body}
      zIndex={2000}
    >
      <Form form={form} layout="vertical" size="small">
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
          <TextArea rows={2} placeholder="如：bash /path/to/deploy.sh" />
        </Form.Item>
        <Form.Item name="work_dir" label="工作目录">
          <Input placeholder="默认为命令所在目录" />
        </Form.Item>
        <Form.Item
          name="url"
          label="访问地址"
          tooltip="服务运行后可访问的 URL"
        >
          <Input placeholder="如：http://localhost:3000" />
        </Form.Item>
        <Row gutter={12}>
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
  );
}
