import { Routes, Route } from "react-router-dom";
import { ConfigProvider, theme, App as AntdApp } from "antd";
import { useState, useEffect } from "react";
import AppLayout from "./components/layout/AppLayout";
import Dashboard from "./pages/Dashboard";
import Services from "./pages/Services";
import Logs from "./pages/Logs";
import Settings from "./pages/Settings";
import TrayPopup from "./pages/TrayPopup";
import { useSystemTray } from "./hooks/useSystemTray";
import { antdThemeToken } from "./styles/tokens";
import { ThemeProvider } from "./contexts/ThemeContext";

// 检测是否为托盘弹窗窗口（通过 hash 路由区分）
const isTrayPopup =
  typeof window !== "undefined" && window.location.hash === "#/tray-popup";

export default function App() {
  const [isDark, setIsDark] = useState(() => {
    const saved = localStorage.getItem("theme");
    if (saved) return saved === "dark";
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  });

  useEffect(() => {
    localStorage.setItem("theme", isDark ? "dark" : "light");
  }, [isDark]);

  const toggleTheme = () => setIsDark((prev) => !prev);

  // 初始化系统托盘（弹窗窗口中为 no-op）
  useSystemTray();

  // 托盘弹窗窗口：直接渲染弹窗页面，不经过 AppLayout
  if (isTrayPopup) {
    return (
      <ConfigProvider
        theme={{
          algorithm: theme.defaultAlgorithm,
          token: antdThemeToken,
        }}
      >
        <AntdApp>
          <TrayPopup />
        </AntdApp>
      </ConfigProvider>
    );
  }

  return (
    <ConfigProvider
      theme={{
        algorithm: isDark ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: {
          ...antdThemeToken,
          ...(isDark
            ? {}
            : {
                colorBgLayout: "#eef5fc",
                colorBgContainer: "#ffffff",
              }),
        },
      }}
    >
      <ThemeProvider isDark={isDark} toggleTheme={toggleTheme}>
        <AppLayout isDark={isDark} onToggleTheme={toggleTheme}>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/services" element={<Services />} />
            <Route path="/logs" element={<Logs />} />
            <Route path="/logs/:serviceId" element={<Logs />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </AppLayout>
      </ThemeProvider>
    </ConfigProvider>
  );
}
