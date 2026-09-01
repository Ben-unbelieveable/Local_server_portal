import WidgetKit
import SwiftUI
import AppIntents

/// 主应用写入的资源/服务快照
struct WidgetSnapshot: Codable {
    var cpu: Double
    var memory: Double
    var gpu: Double?
    var lan: String
    var publicIp: String
    var running: Int
    var total: Int
    var services: [WidgetServiceItem]

    enum CodingKeys: String, CodingKey {
        case cpu, memory, gpu, lan, running, total, services
        case publicIp = "public_ip"
    }

    static let empty = WidgetSnapshot(
        cpu: 0, memory: 0, gpu: nil,
        lan: "—", publicIp: "—",
        running: 0, total: 0, services: []
    )
}

struct WidgetServiceItem: Codable {
    var id: String?
    var name: String
    var status: String
    var url: String?
}

struct SnapshotEntry: TimelineEntry {
    let date: Date
    let snapshot: WidgetSnapshot
}

enum SnapshotStore {
    static func load() -> WidgetSnapshot {
        let url = URL(fileURLWithPath: NSHomeDirectory())
            .appendingPathComponent("widget_snapshot.json")
        guard let data = try? Data(contentsOf: url),
              let snap = try? JSONDecoder().decode(WidgetSnapshot.self, from: data) else {
            return .empty
        }
        return snap
    }
}

struct SnapshotProvider: TimelineProvider {
    func placeholder(in context: Context) -> SnapshotEntry {
        SnapshotEntry(date: Date(), snapshot: .empty)
    }

    func getSnapshot(in context: Context, completion: @escaping (SnapshotEntry) -> Void) {
        completion(SnapshotEntry(date: Date(), snapshot: SnapshotStore.load()))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<SnapshotEntry>) -> Void) {
        let entry = SnapshotEntry(date: Date(), snapshot: SnapshotStore.load())
        let next = Calendar.current.date(byAdding: .second, value: 30, to: Date()) ?? Date()
        completion(Timeline(entries: [entry], policy: .after(next)))
    }
}

/// 资源卡固定色：CPU 蓝 / 内存青 / GPU 紫（与面板组件约定一致）
private enum ResourceTint {
    static let cpu = Color(red: 0.23, green: 0.51, blue: 0.96)
    static let memory = Color(red: 0.15, green: 0.72, blue: 0.78)
    static let gpu = Color(red: 0.56, green: 0.37, blue: 0.89)
}

/// 大号小组件：无标题；资源三卡；网络+运行计数一行；服务竖排。
/// WidgetKit 不支持 ScrollView / 悬停滚动，超出部分以「还有 N 个」收束。
struct ServicePilotWidgetView: View {
    var entry: SnapshotEntry

    /// 大号卡片可稳定放下的服务行数（含溢出提示时少一行）
    private let maxServiceRows = 7

    var body: some View {
        let snap = entry.snapshot
        VStack(alignment: .leading, spacing: 10) {
            metricsRow(snap)
            summaryRow(snap)
            Divider().opacity(0.4)
            serviceSection(snap)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .containerBackground(.background, for: .widget)
    }

    // MARK: 资源三卡

    private func metricsRow(_ snap: WidgetSnapshot) -> some View {
        HStack(alignment: .top, spacing: 8) {
            resourceCard(
                title: "CPU",
                percent: snap.cpu,
                tint: ResourceTint.cpu,
                available: true
            )
            resourceCard(
                title: "内存",
                percent: snap.memory,
                tint: ResourceTint.memory,
                available: true
            )
            resourceCard(
                title: "GPU",
                percent: snap.gpu ?? 0,
                tint: ResourceTint.gpu,
                available: snap.gpu != nil
            )
        }
    }

    /// 输入：标题、占用百分比、固定色、是否有数据；输出：白底指标卡。
    private func resourceCard(
        title: String,
        percent: Double,
        tint: Color,
        available: Bool
    ) -> some View {
        let label = available ? "\(Int(percent.rounded()))%" : "—"
        let progress = available ? min(max(percent / 100.0, 0), 1) : 0
        return VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(label)
                .font(.title2.weight(.semibold).monospacedDigit())
                .foregroundStyle(available ? tint : Color.secondary)
                .minimumScaleFactor(0.7)
                .lineLimit(1)
            ProgressView(value: progress)
                .progressViewStyle(.linear)
                .tint(available ? tint : Color.secondary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(Color.primary.opacity(0.05))
        )
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(title) \(label)")
    }

    // MARK: 局域网 / 公网 / 运行中 一行

    private func summaryRow(_ snap: WidgetSnapshot) -> some View {
        HStack(spacing: 0) {
            summaryCell(title: "局域网", value: snap.lan)
            summaryCell(title: "公网", value: snap.publicIp)
            summaryCell(title: "运行中", value: "\(snap.running)/\(snap.total)")
        }
        .padding(.vertical, 6)
        .padding(.horizontal, 4)
        .background(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(Color.primary.opacity(0.05))
        )
    }

    private func summaryCell(title: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.caption.weight(.medium).monospacedDigit())
                .foregroundStyle(.primary)
                .lineLimit(1)
                .minimumScaleFactor(0.7)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 8)
        .accessibilityLabel("\(title) \(value)")
    }

    // MARK: 服务竖排

    private func serviceSection(_ snap: WidgetSnapshot) -> some View {
        let overflow = snap.total > maxServiceRows
        let limit = overflow ? maxServiceRows - 1 : maxServiceRows
        let shown = Array(snap.services.prefix(limit))
        let rest = max(snap.total - shown.count, 0)

        return VStack(alignment: .leading, spacing: 0) {
            if snap.services.isEmpty {
                Text("暂无服务")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.top, 4)
            } else {
                ForEach(Array(shown.enumerated()), id: \.offset) { _, svc in
                    serviceRow(svc)
                }
                if overflow && rest > 0 {
                    Text("还有 \(rest) 个服务")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .padding(.top, 6)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func serviceRow(_ svc: WidgetServiceItem) -> some View {
        let running = svc.status == "running"
        let starting = svc.status == "starting"
        let canStop = running || starting
        let visit = running ? visitURL(svc.url) : nil
        return HStack(spacing: 6) {
            Circle()
                .fill(statusTint(svc.status))
                .frame(width: 7, height: 7)
            Text(svc.name)
                .font(.caption)
                .foregroundStyle(.primary)
                .lineLimit(1)
            Spacer(minLength: 4)
            if let id = svc.id, !id.isEmpty {
                Button(
                    intent: ToggleServiceIntent(
                        serviceId: id,
                        action: canStop ? "stop" : "start"
                    )
                ) {
                    Image(systemName: canStop ? "stop.circle.fill" : "play.circle.fill")
                        .font(.body)
                        .foregroundStyle(canStop ? Color.red : Color.green)
                        .symbolRenderingMode(.hierarchical)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(canStop ? "停止 \(svc.name)" : "启动 \(svc.name)")
            }
            if let visit {
                Link(destination: visit) {
                    Image(systemName: "arrow.up.right.circle.fill")
                        .font(.body)
                        .foregroundStyle(Color.blue)
                        .symbolRenderingMode(.hierarchical)
                }
                .accessibilityLabel("访问 \(svc.name)")
            }
        }
        .padding(.vertical, 3)
        .invalidatableContent()
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(svc.name) \(statusLabel(svc.status))")
    }

    /// 输入：配置中的 url；输出：可打开的 http(s) URL，缺 scheme 时补 http。
    private func visitURL(_ raw: String?) -> URL? {
        guard let raw, !raw.isEmpty else { return nil }
        if let url = URL(string: raw), url.scheme != nil {
            return url
        }
        return URL(string: "http://\(raw)")
    }

    private func statusTint(_ status: String) -> Color {
        switch status {
        case "running": return .green
        case "starting", "stopping": return .orange
        case "failed", "error": return .red
        default: return .secondary.opacity(0.55)
        }
    }

    private func statusLabel(_ status: String) -> String {
        switch status {
        case "running": return "运行中"
        case "starting": return "启动中"
        case "stopping": return "停止中"
        case "failed", "error": return "异常"
        default: return "已停止"
        }
    }
}

@main
struct MyWidget: Widget {
    let kind = "LocalServiceManagerWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: SnapshotProvider()) { entry in
            ServicePilotWidgetView(entry: entry)
        }
        .configurationDisplayName("ServicePilot")
        .description("查看本机资源、网络与服务状态")
        .supportedFamilies([.systemLarge])
    }
}

/// 将起停请求写入扩展容器，供主应用拾取执行。
enum WidgetActionBridge {
    /// 输入：服务 id、start/stop；输出：写入 `widget_action.json`，失败静默忽略。
    static func enqueue(serviceId: String, action: String) {
        let url = URL(fileURLWithPath: NSHomeDirectory())
            .appendingPathComponent("widget_action.json")
        let payload: [String: String] = [
            "id": serviceId,
            "action": action,
        ]
        guard let data = try? JSONSerialization.data(withJSONObject: payload) else { return }
        try? data.write(to: url, options: .atomic)
    }
}

/// 小组件起停按钮：不打开主窗口，由主应用在本机执行 start/stop。
struct ToggleServiceIntent: AppIntent {
    static var title: LocalizedStringResource = "起停服务"
    static var description = IntentDescription("从桌面小组件启动或停止本地服务")
    static var openAppWhenRun = false
    static var isDiscoverable = false

    @Parameter(title: "服务")
    var serviceId: String

    @Parameter(title: "动作")
    var action: String

    init() {
        serviceId = ""
        action = "start"
    }

    init(serviceId: String, action: String) {
        self.serviceId = serviceId
        self.action = action
    }

    /// 输入：intent 参数；输出：写入待处理动作后立即返回。
    func perform() async throws -> some IntentResult {
        WidgetActionBridge.enqueue(serviceId: serviceId, action: action)
        return .result()
    }
}
