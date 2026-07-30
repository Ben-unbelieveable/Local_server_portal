import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import type { CSSProperties } from "react";
import { useParams } from "react-router-dom";
import { Select, Input, Switch, Button, Space, Tag, Typography, Empty, Row, Col } from "antd";
import { SearchOutlined, ClearOutlined } from "@ant-design/icons";
import { List } from "react-window";
import type { ListImperativeAPI } from "react-window";
import { api } from "../api";
import type { ServiceRuntime, LogEntry } from "../types";

const ROW_HEIGHT = 22;

export default function Logs() {
  const { serviceId } = useParams<{ serviceId?: string }>();
  const [services, setServices] = useState<ServiceRuntime[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(serviceId || null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [searchKeyword, setSearchKeyword] = useState("");
  const [filterStream, setFilterStream] = useState<string>("all");
  const [autoScroll, setAutoScroll] = useState(true);
  const [listHeight, setListHeight] = useState(500);
  const listRef = useRef<ListImperativeAPI>(null);

  useEffect(() => {
    api.getServices().then(setServices).catch(console.error);
  }, []);

  useEffect(() => {
    if (serviceId) setSelectedId(serviceId);
  }, [serviceId]);

  const fetchLogs = useCallback(async () => {
    if (!selectedId) return;
    try {
      const data = searchKeyword
        ? await api.searchLogs(selectedId, searchKeyword)
        : await api.getRecentLogs(selectedId, 500);
      setLogs(data);
    } catch { /* ignore */ }
  }, [selectedId, searchKeyword]);

  useEffect(() => {
    fetchLogs();
    const interval = setInterval(fetchLogs, 2000);
    return () => clearInterval(interval);
  }, [fetchLogs]);

  // 过滤 + 搜索匹配
  const filteredLogs = useMemo(() => {
    let result = logs;
    if (filterStream !== "all") {
      result = result.filter((l) => l.stream === filterStream);
    }
    if (searchKeyword) {
      result = result.filter((l) => l.line.toLowerCase().includes(searchKeyword.toLowerCase()));
    }
    return result;
  }, [logs, filterStream, searchKeyword]);

  // 自动滚动到底部
  useEffect(() => {
    if (autoScroll && listRef.current && filteredLogs.length > 0) {
      listRef.current.scrollToRow({ index: filteredLogs.length - 1 });
    }
  }, [filteredLogs.length, autoScroll]);

  // 测量容器高度
  useEffect(() => {
    const updateHeight = () => {
      const el = document.getElementById("log-container");
      if (el) setListHeight(el.clientHeight);
    };
    updateHeight();
    window.addEventListener("resize", updateHeight);
    return () => window.removeEventListener("resize", updateHeight);
  }, []);

  const runningServices = services.filter(
    (s) => s.status === "running" || s.status === "error" || s.status === "failed"
  );

  const handleClear = () => {
    setLogs([]);
    setSearchKeyword("");
  };

  // 虚拟列表行渲染 — 匹配 react-window v2 rowComponent 签名
  const LogRow = useCallback(
    ({ index, style }: { index: number; style: CSSProperties }) => {
      const log = filteredLogs[index];
      if (!log) return null;
      return (
        <div
          style={{
            ...style,
            color: log.stream === "stderr" ? "#f48771" : "#d4d4d4",
            whiteSpace: "pre",
            fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
            fontSize: 13,
            lineHeight: `${ROW_HEIGHT}px`,
            paddingLeft: 12,
            paddingRight: 12,
          }}
        >
          <span style={{ color: "#888", marginRight: 8 }}>[{log.timestamp}]</span>
          <span
            style={{
              color: log.stream === "stderr" ? "#f48771" : "#6a9955",
              marginRight: 8,
            }}
          >
            [{log.stream}]
          </span>
          {log.line}
        </div>
      );
    },
    [filteredLogs]
  );

  return (
    <div style={{ display: "flex", gap: 16, height: "calc(100vh - 140px)" }}>
      {/* 左侧服务列表 */}
      <div
        style={{
          width: 200,
          flexShrink: 0,
          borderRight: "1px solid #f0f0f0",
          overflow: "auto",
        }}
      >
        <Typography.Text type="secondary" style={{ padding: "8px 0", display: "block" }}>
          服务列表
        </Typography.Text>
        {runningServices.length === 0 ? (
          <Empty description="无运行中服务" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          runningServices.map((s) => (
            <div
              key={s.config.id}
              onClick={() => setSelectedId(s.config.id)}
              style={{
                padding: "8px 12px",
                cursor: "pointer",
                borderRadius: 4,
                background: selectedId === s.config.id ? "#e6f4ff" : "transparent",
                marginBottom: 4,
              }}
            >
              <Tag color={s.status === "running" ? "green" : "red"} style={{ marginRight: 4 }}>
                {s.status === "running" ? "●" : "✕"}
              </Tag>
              {s.config.name}
            </div>
          ))
        )}
      </div>

      {/* 右侧日志内容 */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
        {!selectedId ? (
          <Empty description="请选择一个服务查看日志" style={{ marginTop: 80 }} />
        ) : (
          <>
            {/* 工具栏 */}
            <Row gutter={8} style={{ marginBottom: 12 }}>
              <Col flex="auto">
                <Input
                  prefix={<SearchOutlined />}
                  placeholder="搜索日志..."
                  value={searchKeyword}
                  onChange={(e) => setSearchKeyword(e.target.value)}
                  allowClear
                />
              </Col>
              <Col>
                <Select
                  value={filterStream}
                  onChange={setFilterStream}
                  style={{ width: 100 }}
                  options={[
                    { label: "全部", value: "all" },
                    { label: "stdout", value: "stdout" },
                    { label: "stderr", value: "stderr" },
                  ]}
                />
              </Col>
              <Col>
                <Space>
                  <Switch
                    checked={autoScroll}
                    onChange={setAutoScroll}
                    checkedChildren="自动滚动"
                    unCheckedChildren="暂停"
                  />
                  <Button icon={<ClearOutlined />} onClick={handleClear} size="small">
                    清除
                  </Button>
                </Space>
              </Col>
            </Row>

            {searchKeyword && (
              <Typography.Text type="secondary" style={{ marginBottom: 8 }}>
                找到 {filteredLogs.length} 条匹配
              </Typography.Text>
            )}

            {/* 虚拟滚动日志 */}
            <div
              id="log-container"
              style={{
                flex: 1,
                background: "#1e1e1e",
                borderRadius: 6,
                overflow: "hidden",
              }}
            >
              {filteredLogs.length === 0 ? (
                <div
                  style={{
                    color: "#888",
                    textAlign: "center",
                    marginTop: 40,
                    fontFamily: "monospace",
                  }}
                >
                  暂无日志输出
                </div>
              ) : (
                <List
                  listRef={listRef}
                  rowCount={filteredLogs.length}
                  rowHeight={ROW_HEIGHT}
                  rowComponent={LogRow}
                  rowProps={{} as any}
                  defaultHeight={listHeight}
                  overscanCount={50}
                  style={{ width: "100%" }}
                />
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
