import { createContext, useContext, type ReactNode } from "react";

/**
 * 主题上下文值
 */
export interface ThemeContextValue {
  /** 是否暗色模式 */
  isDark: boolean;
  /** 切换主题 */
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  isDark: false,
  toggleTheme: () => {},
});

interface ThemeProviderProps {
  isDark: boolean;
  toggleTheme: () => void;
  children: ReactNode;
}

/**
 * 主题 Provider 组件，包裹在应用根部以提供 isDark 和 toggleTheme。
 */
export function ThemeProvider({
  isDark,
  toggleTheme,
  children,
}: ThemeProviderProps) {
  return (
    <ThemeContext.Provider value={{ isDark, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

/**
 * 消费主题上下文的 Hook。
 * @returns { isDark, toggleTheme }
 */
export function useTheme(): ThemeContextValue {
  return useContext(ThemeContext);
}
