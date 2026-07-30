import { useEffect, useRef } from "react";

// 检测是否在 Tauri 环境中
const isTauriEnv = typeof window !== "undefined" && "__TAURI__" in window;

export function useTauriEvent<T>(eventName: string, handler: (payload: T) => void) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!isTauriEnv) return; // 浏览器预览模式下跳过

    let cancelled = false;
    let unlistenFn: (() => void) | undefined;

    const setup = async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        if (cancelled) return;
        const unlisten = await listen<T>(eventName, (event) => {
          handlerRef.current(event.payload);
        });
        if (cancelled) {
          unlisten();
        } else {
          unlistenFn = unlisten;
        }
      } catch {
        // Tauri event 不可用（如浏览器预览）
      }
    };

    setup();

    return () => {
      cancelled = true;
      unlistenFn?.();
    };
  }, [eventName]);
}
