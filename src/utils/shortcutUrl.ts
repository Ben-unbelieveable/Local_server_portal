/** 输入用户地址；返回规范化 URL，拒绝非网页协议和非本机 HTML 文件。 */
export function normalizeShortcutUrl(value: string): string {
  const input = value.trim();
  if (!/^(https?:\/\/|file:\/\/\/)/i.test(input)) {
    throw new Error("请输入完整的 http://、https:// 或 file:/// 地址");
  }
  let url: URL;
  try {
    url = new URL(input);
  } catch {
    throw new Error("请输入有效的访问地址");
  }
  if (url.protocol === "file:") {
    if (url.hostname || !/\.html?$/i.test(url.pathname)) {
      throw new Error("本地页面仅支持本机 .html 或 .htm 文件");
    }
  } else if (!["http:", "https:"].includes(url.protocol) || !url.hostname) {
    throw new Error("仅支持 HTTP(S) 网页或本地 HTML 页面");
  }
  return url.href;
}
