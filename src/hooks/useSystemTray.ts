import { useEffect } from "react";

const isTauri = typeof window !== "undefined" && "__TAURI__" in window;
const isPopup =
  typeof window !== "undefined" && window.location.hash === "#/tray-popup";

/**
 * 系统托盘 Hook。
 *
 * 托盘图标的创建和点击事件处理已在 Rust 端完成
 *（lib.rs setup 中通过 on_tray_icon_event 挂载到 tauri.conf.json 自动创建的托盘）。
 *
 * 此 Hook 保留为 no-op，仅用于兼容 App.tsx 中的调用，避免前端重新创建
 * 无图标的托盘导致 macOS 菜单栏不显示管理图标。
 */
export function useSystemTray() {
  useEffect(() => {
    // no-op: 托盘由 Rust 端管理
    // 引用以保持 isTauri/isPopup 判定可被未来扩展复用，且避免 lint 未使用告警
    void isTauri;
    void isPopup;
  }, []);
}
