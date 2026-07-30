import { useState, useEffect, useRef } from "react";
import {
  Button,
  App,
  Space,
  Card,
  Typography,
  Alert,
  Switch,
  Divider,
} from "antd";
import { SaveOutlined, ReloadOutlined } from "@ant-design/icons";
import { EditorView, basicSetup } from "codemirror";
import { yaml } from "@codemirror/lang-yaml";
import { oneDark } from "@codemirror/theme-one-dark";
import type { Extension } from "@codemirror/state";
import { api, isTauriEnv } from "../api";
import { useTheme } from "../contexts/ThemeContext";
import { borderRadius, spacing } from "../styles/tokens";

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
  const [quitWhenCloseMain, setQuitWhenCloseMain] = useState(false);
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const [prefsLoading, setPrefsLoading] = useState(false);
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  /** 保存编辑器文档内容，用于主题切换时重建编辑器 */
  const docRef = useRef<string | null>(null);

  /** 加载关窗策略与开机启动状态 */
  useEffect(() => {
    let cancelled = false;
    const loadPrefs = async () => {
      setPrefsLoading(true);
      try {
        const prefs = await api.getAppPreferences();
        if (cancelled) return;
        setQuitWhenCloseMain(!!prefs.quit_when_close_main);

        if (isTauriEnv) {
          const { isEnabled } = await import("@tauri-apps/plugin-autostart");
          const enabled = await isEnabled();
          if (!cancelled) setLaunchAtLogin(enabled);
        } else {
          setLaunchAtLogin(!!prefs.launch_at_login);
        }
      } catch (e) {
        if (!cancelled) console.error(e);
      } finally {
        if (!cancelled) setPrefsLoading(false);
      }
    };
    loadPrefs();
    return () => {
      cancelled = true;
    };
  }, []);

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

  /**
   * 切换：关闭主窗口时是否同步退出托盘
   * @param checked 是否同步退出
   */
  const handleQuitWhenCloseChange = async (checked: boolean) => {
    setQuitWhenCloseMain(checked);
    try {
      await api.setQuitWhenCloseMain(checked);
      message.success(
        checked
          ? "已开启：关闭主窗口将退出托盘"
          : "已关闭：关闭主窗口仅隐藏，托盘继续运行"
      );
    } catch (e) {
      setQuitWhenCloseMain(!checked);
      message.error(String(e));
    }
  };

  /**
   * 切换开机启动（OS 登录项 + 偏好镜像）
   * @param checked 是否开机启动
   */
  const handleLaunchAtLoginChange = async (checked: boolean) => {
    if (!isTauriEnv) {
      message.warning("开机启动仅在桌面应用内可用");
      return;
    }
    setLaunchAtLogin(checked);
    try {
      const { enable, disable } = await import("@tauri-apps/plugin-autostart");
      if (checked) {
        await enable();
      } else {
        await disable();
      }
      await api.setLaunchAtLoginPref(checked);
      message.success(checked ? "已开启开机启动" : "已关闭开机启动");
    } catch (e) {
      setLaunchAtLogin(!checked);
      message.error("切换开机启动失败：" + String(e));
    }
  };

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
      <Card title="应用行为" style={{ marginBottom: spacing.md }} loading={prefsLoading}>
        <Typography.Paragraph type="secondary" style={{ marginBottom: spacing.md }}>
          本工具以托盘为主：关闭主窗口默认不退出托盘；在托盘内点「退出应用」才会彻底结束。
        </Typography.Paragraph>

        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "flex-start",
            gap: spacing.md,
            marginBottom: spacing.md,
          }}
        >
          <div style={{ flex: 1 }}>
            <Typography.Text strong>关闭主窗口时同步退出托盘</Typography.Text>
            <br />
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              关闭 Dock 中激活的应用窗口时，是否一并退出托盘与后台进程。默认关闭（仅隐藏主窗）。
            </Typography.Text>
          </div>
          <Switch
            checked={quitWhenCloseMain}
            onChange={handleQuitWhenCloseChange}
          />
        </div>

        <Divider style={{ margin: `${spacing.sm}px 0` }} />

        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "flex-start",
            gap: spacing.md,
          }}
        >
          <div style={{ flex: 1 }}>
            <Typography.Text strong>开机启动</Typography.Text>
            <br />
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              登录 macOS 后自动启动（LaunchAgent）。可在系统设置 → 通用 → 登录项中核对。
            </Typography.Text>
          </div>
          <Switch
            checked={launchAtLogin}
            onChange={handleLaunchAtLoginChange}
            disabled={!isTauriEnv}
          />
        </div>
      </Card>

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
