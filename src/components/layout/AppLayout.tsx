import { Layout, Menu, Switch, Typography, theme, Grid } from "antd";
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
const { useBreakpoint } = Grid;

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
 * 主窗口壳层：方案 B 雾蓝氛围；窄屏自动折叠侧栏以保内容区可用宽度。
 */
export default function AppLayout({ children, isDark, onToggleTheme }: Props) {
  const navigate = useNavigate();
  const location = useLocation();
  const { token } = theme.useToken();
  const screens = useBreakpoint();

  // < lg(992)：折叠为图标侧栏，避免与内容区抢宽
  const collapsed = !screens.lg;
  const contentBg = isDark ? token.colorBgLayout : surfaceGradientLight;
  const siderBg = isDark ? undefined : glassFillLight;
  const contentPad = screens.md ? spacing.xl : spacing.md;

  return (
    <Layout style={{ height: "100vh", minWidth: 0, fontFamily }}>
      <Sider
        width={200}
        collapsedWidth={64}
        collapsed={collapsed}
        theme={isDark ? "dark" : "light"}
        style={{
          borderRight: `1px solid ${token.colorBorderSecondary}`,
          background: siderBg,
          backdropFilter: isDark ? undefined : glassBlur,
          WebkitBackdropFilter: isDark ? undefined : glassBlur,
          flexShrink: 0,
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
              padding: `0 ${spacing.sm}px`,
            }}
          >
            <Typography.Title
              level={5}
              style={{
                margin: 0,
                fontFamily,
                fontSize: collapsed ? fontSizes.SM.size : undefined,
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {collapsed ? "SP" : "ServicePilot"}
            </Typography.Title>
          </div>

          <Menu
            mode="inline"
            inlineCollapsed={collapsed}
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
              justifyContent: collapsed ? "center" : "space-between",
              borderTop: `1px solid ${token.colorBorderSecondary}`,
              padding: collapsed ? 0 : `0 ${spacing.lg}px`,
              flexShrink: 0,
              gap: spacing.sm,
            }}
          >
            <Switch
              checkedChildren={<MoonOutlined />}
              unCheckedChildren={<SunOutlined />}
              checked={isDark}
              onChange={onToggleTheme}
              size="small"
            />
            {!collapsed && (
              <Typography.Text
                type="secondary"
                style={{ fontSize: fontSizes.SM.size }}
              >
                v0.1.0
              </Typography.Text>
            )}
          </div>
        </div>
      </Sider>
      <Layout style={{ background: "transparent", minWidth: 0 }}>
        <ResourceBar isDark={isDark} compact={!screens.md} />
        <Content
          style={{
            padding: contentPad,
            overflow: "auto",
            background: contentBg,
            minWidth: 0,
          }}
        >
          {children}
        </Content>
      </Layout>
    </Layout>
  );
}
