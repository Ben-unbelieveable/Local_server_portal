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
import { fontSizes, spacing } from "../../styles/tokens";

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

export default function AppLayout({ children, isDark, onToggleTheme }: Props) {
  const navigate = useNavigate();
  const location = useLocation();
  const { token } = theme.useToken();

  return (
    <Layout style={{ height: "100vh" }}>
      <Sider
        width={200}
        theme={isDark ? "dark" : "light"}
        style={{
          borderRight: `1px solid ${token.colorBorderSecondary}`,
        }}
      >
        {/* Flex 列布局：Logo + Menu(flex:1) + Footer */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
          }}
        >
          {/* Logo */}
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
            <Typography.Title level={5} style={{ margin: 0 }}>
              ⚡ ServicePilot
            </Typography.Title>
          </div>

          {/* Menu — flex:1 撑满中间空间 */}
          <Menu
            mode="inline"
            selectedKeys={[location.pathname]}
            items={menuItems}
            onClick={({ key }) => navigate(key)}
            style={{ borderRight: 0, marginTop: spacing.sm, flex: 1 }}
          />

          {/* Footer — Switch 左 + 版本号 右 */}
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
      <Layout>
        <ResourceBar />
        <Content
          style={{
            padding: spacing.xl,
            overflow: "auto",
            background: token.colorBgLayout,
          }}
        >
          {children}
        </Content>
      </Layout>
    </Layout>
  );
}
