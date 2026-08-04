import { Space, Tooltip, message, theme } from "antd";
import { CopyOutlined, GlobalOutlined, WifiOutlined } from "@ant-design/icons";
import { useCallback, type ReactNode } from "react";
import type { NetworkInfo } from "../../types";
import { fontSizes, spacing, borderRadius, neutralColors } from "../../styles/tokens";

interface NetworkIpPillsProps {
  /** 网络信息；为 null 时不渲染 */
  networkInfo: NetworkInfo | null;
  /** 窄屏/托盘：仅显示 IP 值，不重复标签前缀 */
  compact?: boolean;
  /** 是否暗色主题（主窗 ResourceBar） */
  isDark?: boolean;
  /** bar=顶栏玻璃风；card=托盘白卡片风 */
  variant?: "bar" | "card";
}

/**
 * 局域网 / 公网 IP 展示 Pill，支持点击复制。
 */
export default function NetworkIpPills({
  networkInfo,
  compact = false,
  isDark = false,
  variant = "bar",
}: NetworkIpPillsProps) {
  const { token } = theme.useToken();

  const copyText = useCallback(async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      message.success(`已复制${label}`);
    } catch {
      message.error("复制失败");
    }
  }, []);

  if (!networkInfo) return null;

  const lanDisplay =
    networkInfo.lan_ips.length === 0 ? "—" : networkInfo.lan_ips.join(", ");
  const publicDisplay =
    networkInfo.public_ip ?? (networkInfo.public_ip_error ? "不可用" : "…");

  const pillBg =
    variant === "card"
      ? neutralColors.colorBgLayout
      : isDark
        ? token.colorFillSecondary
        : "rgba(255,255,255,0.72)";
  const pillBorder =
    variant === "card"
      ? neutralColors.colorBorderSecondary
      : token.colorBorderSecondary;

  const renderIpPill = (
    icon: ReactNode,
    label: string,
    value: string,
    copyValue?: string,
    tooltip?: string
  ) => (
    <Tooltip title={tooltip ?? (copyValue ? `点击复制 ${copyValue}` : value)}>
      <div
        role={copyValue ? "button" : undefined}
        tabIndex={copyValue ? 0 : undefined}
        onClick={() => copyValue && copyText(copyValue, label)}
        onKeyDown={(e) => {
          if (copyValue && (e.key === "Enter" || e.key === " ")) {
            copyText(copyValue, label);
          }
        }}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: spacing.xs,
          height: 28,
          padding: `0 ${spacing.md}px`,
          borderRadius: borderRadius.pill,
          background: pillBg,
          border: `1px solid ${pillBorder}`,
          fontSize: fontSizes.SM.size,
          color: token.colorTextSecondary,
          flexShrink: 0,
          cursor: copyValue ? "pointer" : "default",
          maxWidth: compact ? 150 : 240,
          minWidth: 0,
        }}
      >
        {icon}
        <span
          style={{
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {compact ? value : `${label} ${value}`}
        </span>
        {copyValue && (
          <CopyOutlined style={{ fontSize: fontSizes.XS.size, opacity: 0.6 }} />
        )}
      </div>
    </Tooltip>
  );

  return (
    <Space size={spacing.sm} wrap>
      {renderIpPill(
        <WifiOutlined />,
        "局域网",
        compact ? lanDisplay.split(",")[0] : lanDisplay,
        networkInfo.lan_ips[0],
        networkInfo.lan_ips.length > 1
          ? `全部局域网 IP：${networkInfo.lan_ips.join(", ")}`
          : undefined
      )}
      {renderIpPill(
        <GlobalOutlined />,
        "公网",
        publicDisplay,
        networkInfo.public_ip ?? undefined,
        networkInfo.public_ip_error
      )}
    </Space>
  );
}
