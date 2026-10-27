import { App, Typography, theme } from "antd";
import type { ReactNode, MouseEvent } from "react";
import { isTauriEnv } from "../../api";
import type { ShortcutConfig } from "../../types";
import { normalizeShortcutUrl } from "../../utils/shortcutUrl";

/** 输入入口和可选链接内容；浏览器直接访问网页，桌面交由默认应用打开，失败显示说明。 */
export default function ShortcutLink({
  shortcut,
  children,
}: {
  shortcut: ShortcutConfig;
  children?: ReactNode;
}) {
  const { message } = App.useApp();
  const { token } = theme.useToken();
  let url: string | undefined;
  try {
    url = normalizeShortcutUrl(shortcut.url);
  } catch {
    // 配置中的非法地址仍显示名称；点击时给出明确错误，避免生成可执行链接。
  }

  /** 输入点击事件；保留浏览器正常链接行为，桌面打开失败时报告错误。 */
  const handleClick = async (event: MouseEvent<HTMLElement>) => {
    if (!url) {
      event.preventDefault();
      message.error("入口地址无效，请编辑后重试");
      return;
    }
    if (!isTauriEnv && !url.startsWith("file:")) return;
    event.preventDefault();
    if (!isTauriEnv) {
      message.info(
        "请在桌面应用中打开本地 HTML 页面，浏览器限制直接访问本地文件",
      );
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
    } catch (error) {
      message.error(`无法打开入口：${String(error)}`);
    }
  };

  return (
    <Typography.Link
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      onClick={handleClick}
      style={{ color: token.colorPrimary }}
    >
      {children ?? shortcut.name}
    </Typography.Link>
  );
}
