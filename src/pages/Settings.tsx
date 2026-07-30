import { useState, useEffect, useRef } from "react";
import { Button, App, Space, Card, Typography, Alert } from "antd";
import { SaveOutlined, ReloadOutlined } from "@ant-design/icons";
import { EditorView, basicSetup } from "codemirror";
import { yaml } from "@codemirror/lang-yaml";
import { oneDark } from "@codemirror/theme-one-dark";
import type { Extension } from "@codemirror/state";
import { api } from "../api";
import { useTheme } from "../contexts/ThemeContext";
import { borderRadius } from "../styles/tokens";

/**
 * 根据主题模式返回 CodeMirror extensions。
 * 暗色模式包含 oneDark 扩展，亮色模式使用默认浅色主题。
 */
function getEditorExtensions(dark: boolean): Extension[] {
  const exts: Extension[] = [basicSetup, yaml()];
  if (dark) {
    exts.push(oneDark);
  }
  return exts;
}

export default function Settings() {
  const { message } = App.useApp();
  const { isDark } = useTheme();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  /** 保存编辑器文档内容，用于主题切换时重建编辑器 */
  const docRef = useRef<string | null>(null);

  useEffect(() => {
    if (!editorRef.current) return;

    let cancelled = false;

    const initEditor = async () => {
      // 销毁旧编辑器实例
      if (viewRef.current) {
        viewRef.current.destroy();
      }

      const extensions = getEditorExtensions(isDark);
      let doc: string;

      if (docRef.current !== null) {
        // 主题切换：使用已保存的文档内容
        doc = docRef.current;
      } else {
        // 首次加载：从 API 获取配置
        try {
          const data = await api.getConfigRaw();
          if (cancelled) return;
          doc = data || exampleConfig;
        } catch (e) {
          if (cancelled) return;
          message.error(String(e));
          doc = exampleConfig;
        }
      }

      if (cancelled) return;

      viewRef.current = new EditorView({
        doc,
        extensions,
        parent: editorRef.current!,
      });
    };

    initEditor();

    // 清理：保存文档内容并销毁编辑器
    return () => {
      cancelled = true;
      if (viewRef.current) {
        docRef.current = viewRef.current.state.doc.toString();
        viewRef.current.destroy();
      }
    };
  }, [isDark]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSave = async () => {
    if (!viewRef.current) return;
    const content = viewRef.current.state.doc.toString();
    setSaving(true);
    setError(null);

    try {
      // 先校验
      await api.validateConfig(content);
      await api.saveConfigRaw(content);
      message.success("配置已保存，服务列表已更新");
    } catch (e) {
      const errMsg = String(e);
      setError(errMsg);
      message.error("保存失败：" + errMsg);
    }
    setSaving(false);
  };

  const handleReload = async () => {
    try {
      const data = await api.getConfigRaw();
      if (viewRef.current) {
        viewRef.current.dispatch({
          changes: {
            from: 0,
            to: viewRef.current.state.doc.length,
            insert: data || exampleConfig,
          },
        });
      }
      message.success("已重新加载");
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div>
      <Card
        title="配置文件编辑"
        extra={
          <Space>
            <Button icon={<ReloadOutlined />} onClick={handleReload}>
              重新加载
            </Button>
            <Button
              type="primary"
              icon={<SaveOutlined />}
              onClick={handleSave}
              loading={saving}
            >
              保存配置
            </Button>
          </Space>
        }
      >
        <Typography.Paragraph type="secondary" style={{ marginBottom: 12 }}>
          直接编辑 YAML 配置文件，支持语法高亮。修改后点击「保存配置」即可生效，正在运行的服务不受影响。
        </Typography.Paragraph>

        {error && (
          <Alert
            type="error"
            message="保存失败"
            description={error}
            showIcon
            closable
            style={{ marginBottom: 12 }}
            onClose={() => setError(null)}
          />
        )}

        <div
          ref={editorRef}
          style={{
            border: `1px solid ${
              isDark
                ? "var(--color-border)"
                : "var(--color-border-secondary)"
            }`,
            borderRadius: borderRadius.input,
            overflow: "hidden",
            minHeight: 450,
          }}
        />
      </Card>
    </div>
  );
}

const exampleConfig = `# 本地服务管理平台 - 服务配置文件
# 编辑后点击「保存配置」即可生效

services:
  - id: antibody_annotation
    name: Antibody Annotation
    command: bash /Users/liubo/github/antibody_annotation/deploy.sh
    url: http://localhost:3000
    work_dir: /Users/liubo/github/antibody_annotation
    env:
      PORT: "3000"
    group: Web服务
    description: 抗体标注 Web 服务
    stop_timeout: 10

  - id: ollama
    name: Ollama AI
    command: ollama serve
    group: AI模型
    description: Ollama 大模型推理服务（高资源消耗）
    stop_timeout: 15
`;
