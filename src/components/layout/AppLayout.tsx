import { Layout, Menu, Switch, Typography, theme } from "antd";
import {
  DashboardOutlined,
  ToolOutlined,
  FileTextOutlined,
  SettingOutlined,
  SunOutlined,
  MoonOutlined,
} from "@ant-design/icons";
import { useNavigate, useLocation } from "react-router-dom";
import type { ReactNode } from "react";
import ResourceBar from "./ResourceBar";
import {
  fontSizes,
  spacing,
  surfaceGradientLight,
  glassFillLight,
  glassBlur,
  fontFamily,
} from "../../styles/tokens";

const { Sider, Content } = Layout;

interface Props {
  children: ReactNode;
  isDark: boolean;
  onToggleTheme: () => void;
}

const menuItems = [
  { key: "/", icon: <DashboardOutlined />, label: "仪表盘" },
  { key: "/services", icon: <ToolOutlined />, label: "服务管理" },
  { key: "/logs", icon: <FileTextOutlined />, label: "日志" },
  { key: "/settings", icon: <SettingOutlined />, label: "配置" },
];

/**
 * 主窗口壳层：方案 B — 雾蓝氛围 + 玻璃侧栏/顶栏，向托盘弹层视觉靠拢。
 */
export default function AppLayout({ children, isDark, onToggleTheme }: Props) {
  const navigate = useNavigate();
  const location = useLocation();
  const { token } = theme.useToken();

  const contentBg = isDark ? token.colorBgLayout : surfaceGradientLight;
  const siderBg = isDark ? undefined : glassFillLight;

  return (
    <Layout style={{ height: "100vh", fontFamily }}>
      <Sider
        width={200}
        theme={isDark ? "dark" : "light"}
        style={{
          borderRight: `1px solid ${token.colorBorderSecondary}`,
          background: siderBg,
          backdropFilter: isDark ? undefined : glassBlur,
          WebkitBackdropFilter: isDark ? undefined : glassBlur,
        }}
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
            background: "transparent",
          }}
        >
          <div
            style={{
              height: 48,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              borderBottom: `1px solid ${token.colorBorderSecondary}`,
              flexShrink: 0,
            }}
          >
            <Typography.Title level={5} style={{ margin: 0, fontFamily }}>
              ServicePilot
            </Typography.Title>
          </div>

          <Menu
            mode="inline"
            selectedKeys={[location.pathname]}
            items={menuItems}
            onClick={({ key }) => navigate(key)}
            style={{
              borderRight: 0,
              marginTop: spacing.sm,
              flex: 1,
              background: "transparent",
            }}
          />

          <div
            style={{
              height: 48,
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              borderTop: `1px solid ${token.colorBorderSecondary}`,
              padding: `0 ${spacing.lg}px`,
              flexShrink: 0,
            }}
          >
            <Switch
              checkedChildren={<MoonOutlined />}
              unCheckedChildren={<SunOutlined />}
              checked={isDark}
              onChange={onToggleTheme}
              size="small"
            />
            <Typography.Text
              type="secondary"
              style={{ fontSize: fontSizes.SM.size }}
            >
              v0.1.0
            </Typography.Text>
          </div>
        </div>
      </Sider>
      <Layout style={{ background: "transparent" }}>
        <ResourceBar isDark={isDark} />
        <Content
          style={{
            padding: spacing.xl,
            overflow: "auto",
            background: contentBg,
          }}
        >
          {children}
        </Content>
      </Layout>
    </Layout>
  );
}
