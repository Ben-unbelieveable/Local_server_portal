import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

// 编译实际的共享地址校验函数，使用 Node 自带测试器，无需额外测试依赖。
const source = await readFile(
  new URL("../src/utils/shortcutUrl.ts", import.meta.url),
  "utf8",
);
const { outputText } = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2020,
  },
});
const { normalizeShortcutUrl } = await import(
  `data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`
);

test("保留互联网、本地服务、IPv6、查询参数及片段，规范化空格和中文路径", () => {
  for (const input of [
    " https://github.com ",
    "http://localhost:3000/a?tab=1#logs",
    "http://192.168.1.10:8080",
    "http://[::1]:3000/",
    "file:///Users/demo/本地 页面/index.html",
    "file:///C:/Sites/index.HTM#section",
  ]) {
    assert.equal(normalizeShortcutUrl(input), new URL(input.trim()).href);
  }
});

test("拒绝缺失协议、损坏地址、脚本、可执行文件和远程文件共享", () => {
  for (const input of [
    "",
    "localhost:3000",
    "https://",
    "https:github.com",
    "javascript:alert(1)",
    "data:text/html,hi",
    "ftp://example.com",
    "file:///tmp/script.sh",
    "file:///tmp/app.html.exe",
    "file://server/share/index.html",
    "file:///tmp/index%2ehtml",
  ]) {
    assert.throws(() => normalizeShortcutUrl(input), Error, input);
  }
});

test("桌面 shell 权限覆盖有效入口，并拒绝非页面文件和可执行协议", async () => {
  const config = JSON.parse(
    await readFile(
      new URL("../src-tauri/tauri.conf.json", import.meta.url),
      "utf8",
    ),
  );
  const scope = new RegExp(config.plugins.shell.open.replace("(?i)", ""), "i");
  for (const input of [
    "https://github.com",
    "http://localhost:3000",
    "http://[::1]:3000/",
    "file:///tmp/page%20one.HTML#test",
    "file:///C:/Sites/index.htm",
  ]) {
    assert.match(normalizeShortcutUrl(input), scope);
  }
  for (const input of [
    "javascript:alert(1)",
    "file:///tmp/script.sh",
    "file:///tmp/app.html.exe",
    "file:///tmp/app.html/run.sh",
  ]) {
    assert.doesNotMatch(input, scope);
  }
});
