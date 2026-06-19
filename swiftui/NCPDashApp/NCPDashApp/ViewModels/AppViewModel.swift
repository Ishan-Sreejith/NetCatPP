import Foundation
import SwiftUI
import NCPKit

@MainActor
final class AppViewModel: ObservableObject {
    @Published var dashboard = DashboardSnapshot.empty
    @Published var statusMessage: String = "Ready"
    @Published var isRefreshing: Bool = false
    @Published var scannerHost: String = "127.0.0.1"
    @Published var scannerRange: String = "1-1024"
    @Published var scannerRows: [ScanResultRow] = []
    @Published var httpURL: String = "https://example.com"
    @Published var httpMethod: HTTPMethod = .GET
    @Published var httpBody: String = ""
    @Published var httpResponse: String = ""
    @Published var textHost: String = "127.0.0.1"
    @Published var textPort: String = "9000"
    @Published var textMessage: String = "hello from swiftui"
    @Published var textListenerPort: String = "9000"
    @Published var textListenerMessages: [NCPTextMessage] = []
    @Published var isTextListening: Bool = false


    private var timer: Timer?
    private var textTimer: Timer?

    init() {
        startPolling()
    }

    deinit {
        timer?.invalidate()
        textTimer?.invalidate()
    }

    func startPolling() {
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { await self?.refreshDashboard() }
        }
    }

    func refreshDashboard() async {
        if isRefreshing { return }
        isRefreshing = true
        defer { isRefreshing = false }
        do {
            let snapshot = try await Task.detached {
                try NCPClient.systemSnapshot()
            }.value
            if snapshot.top_processes.isEmpty {
                let retry = try await Task.detached {
                    try NCPClient.systemSnapshot()
                }.value
                if !retry.top_processes.isEmpty {
                    dashboard = DashboardSnapshot(
                        cpuUsage: retry.cpu_usage,
                        memoryUsedGB: retry.memory_used_gb,
                        memoryTotalGB: retry.memory_total_gb,
                        topProcesses: retry.top_processes,
                        memoryPressure: retry.memory_pressure,
                        gpuPressure: retry.gpu_pressure
                    )
                    statusMessage = "Updated \(Date().formatted(date: .omitted, time: .standard))"
                    return
                }
            }
            dashboard = DashboardSnapshot(
                cpuUsage: snapshot.cpu_usage,
                memoryUsedGB: snapshot.memory_used_gb,
                memoryTotalGB: snapshot.memory_total_gb,
                topProcesses: snapshot.top_processes,
                memoryPressure: snapshot.memory_pressure,
                gpuPressure: snapshot.gpu_pressure
            )
            statusMessage = "Updated \(Date().formatted(date: .omitted, time: .standard))"
        } catch {
            statusMessage = "Snapshot failed: \(error.localizedDescription)"
        }
    }

    func runScan() async {
        statusMessage = "Running scan..."
        do {
            let host = scannerHost
            let range = scannerRange
            let json = try await Task.detached {
                try NCPClient.scanHost(host: host, range: range, timeout: "400ms", udp: false, concurrency: 256)
            }.value
            let data = Data(json.utf8)
            let decoded = try JSONSerialization.jsonObject(with: data) as? [String: Any]
            let ports = decoded?["open_ports"] as? [[String: Any]] ?? []
            scannerRows = ports.map {
                ScanResultRow(
                    port: $0["port"] as? Int ?? 0,
                    service: $0["service"] as? String ?? "-",
                    status: $0["status"] as? String ?? "unknown"
                )
            }.sorted(by: { $0.port < $1.port })
            statusMessage = "Scan complete: \(scannerRows.count) open"
        } catch {
            statusMessage = "Scan failed: \(error.localizedDescription)"
        }
    }

    func runHttp() async {
        statusMessage = "Running HTTP request..."
        do {
            let url = httpURL
            let method = httpMethod.rawValue
            let body = httpBody.isEmpty ? nil : httpBody
            let response = try await Task.detached {
                try NCPClient.httpRequest(
                    url: url,
                    method: method,
                    body: body,
                    repeat: 1,
                    follow: true,
                    maxRedirects: 8
                )
            }.value
            let latencyStr = String(format: "%.2f", response.avg_latency_ms)
            httpResponse = "Status: \(response.status)\nURL: \(response.final_url)\nAvg: \(latencyStr)ms\n\n\(response.body)"
            statusMessage = "HTTP done"
        } catch {
            statusMessage = "HTTP failed: \(error.localizedDescription)"
        }
    }

    func sendText() async {
        guard let port = UInt16(textPort) else {
            statusMessage = "Invalid port"
            return
        }
        do {
            let host = textHost
            let message = textMessage
            try await Task.detached {
                try NCPClient.sendTextMessage(host: host, port: port, message: message, repeat: 1, interval: "0ms")
            }.value
            statusMessage = "Text sent"
        } catch {
            statusMessage = "Send failed: \(error.localizedDescription)"
        }
    }

    func startTextListener() {
        guard let port = UInt16(textListenerPort) else {
            statusMessage = "Invalid listen port"
            return
        }
        if isTextListening { return }
        isTextListening = true
        textListenerMessages = []
        NCPClient.startTextListener(port: port, keepAlive: true)
        statusMessage = "Listening on :\(port)"
        textTimer?.invalidate()
        textTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            Task { await self?.pollTextMessages() }
        }
    }

    func stopTextListener() {
        isTextListening = false
        textTimer?.invalidate()
        textTimer = nil
        NCPClient.stopTextListener()
        statusMessage = "Listener stopped"
    }

    func pollTextMessages() async {
        guard isTextListening else { return }
        do {
            let incoming = try await Task.detached {
                try NCPClient.pollTextMessages(maxCount: 50)
            }.value
            if !incoming.isEmpty {
                textListenerMessages.append(contentsOf: incoming)
                if textListenerMessages.count > 200 {
                    textListenerMessages.removeFirst(textListenerMessages.count - 200)
                }
            }
        } catch {
            statusMessage = "Listen failed: \(error.localizedDescription)"
        }
    }
}
