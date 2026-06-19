import SwiftUI
import NCPKit

// MARK: - Packet Data Model

struct PacketRow: Identifiable {
    let id = UUID()
    let timestamp: Date
    let source: String
    let destination: String
    let protocol_: String
    let length: Int
    let info: String
}

// MARK: - Packet Row View

struct PacketRowView: View {
    let packet: PacketRow
    let sourceWidth: CGFloat
    let destWidth: CGFloat

    var body: some View {
        HStack(spacing: 0) {
            Text(packet.timestamp, format: .dateTime.hour().minute().second().secondFraction(.milliseconds(0)))
                .frame(width: 80, alignment: .leading)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(AppTheme.textSecondary)
                .lineLimit(1)

            Text(packet.source)
                .frame(width: sourceWidth, alignment: .leading)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(AppTheme.textPrimary)
                .lineLimit(1)
                .truncationMode(.middle)

            Text(packet.destination)
                .frame(width: destWidth, alignment: .leading)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(AppTheme.textPrimary)
                .lineLimit(1)
                .truncationMode(.middle)

            Text(packet.protocol_)
                .frame(width: 55)
                .font(.caption.bold())
                .foregroundColor(protocolColor)
                .lineLimit(1)

            Text("\(packet.length)")
                .frame(width: 50, alignment: .trailing)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(AppTheme.textSecondary)
                .lineLimit(1)

            Text(packet.info)
                .frame(maxWidth: .infinity, alignment: .leading)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(AppTheme.textSecondary)
                .lineLimit(1)
                .truncationMode(.tail)
        }
        .frame(height: 22)
        .padding(.horizontal, 12)
        .padding(.vertical, 3)
    }

    private var protocolColor: Color {
        switch packet.protocol_ {
        case "TCP": .blue
        case "UDP": .green
        case "ICMP": .purple
        default: AppTheme.textSecondary
        }
    }
}

// MARK: - Capture Engine

private final class CaptureEngine {
    private var pollTimer: DispatchSourceTimer?
    private let queue = DispatchQueue(label: "com.netcatpp.capture", qos: .userInitiated)
    private var ffiReadOffset: UInt64 = 0

    private(set) var isRunning = false

    var onPacket: ((PacketRow) -> Void)?
    var onStatusChange: ((Bool) -> Void)?
    var onError: ((String) -> Void)?

    func start(interface: String) {
        guard !isRunning else { return }
        isRunning = true
        onStatusChange?(true)
        ffiReadOffset = 0

        let iface = interface.isEmpty ? "en0" : interface
        NCPClient.startPacketCapture(interface: iface, protoFilter: nil)

        let timer = DispatchSource.makeTimerSource(queue: queue)
        timer.schedule(deadline: .now() + 0.5, repeating: .milliseconds(400), leeway: .milliseconds(50))
        timer.setEventHandler { [weak self] in self?.tick() }
        timer.activate()
        pollTimer = timer
    }

    private func tick() {
        guard isRunning else { return }

        // Poll FFI (works if app is run with sudo)
        if let pkt = pollFFI() {
            DispatchQueue.main.async { [weak self] in self?.onPacket?(pkt) }
        }

        // Also poll /tmp/ncp-capture-output for user-run helper
        if let pkt = pollOutputFile() {
            DispatchQueue.main.async { [weak self] in self?.onPacket?(pkt) }
        }
    }

    private func pollFFI() -> PacketRow? {
        guard isRunning else { return nil }
        do {
            let pkts = try NCPClient.pollPackets(maxCount: 200)
            guard !pkts.isEmpty else { return nil }
            return PacketRow(
                timestamp: Date(timeIntervalSince1970: TimeInterval(pkts[0].timestamp_ms) / 1000),
                source: "\(pkts[0].source):\(pkts[0].source_port)",
                destination: "\(pkts[0].destination):\(pkts[0].dest_port)",
                protocol_: pkts[0].protocol_,
                length: pkts[0].length,
                info: pkts[0].flags
            )
        } catch {
            return nil
        }
    }

    private func pollOutputFile() -> PacketRow? {
        let url = URL(fileURLWithPath: "/tmp/ncp-capture-output")
        guard let fh = try? FileHandle(forReadingFrom: url) else { return nil }
        defer { try? fh.close() }
        guard let size = try? fh.seekToEnd(), size > ffiReadOffset else {
            try? fh.seek(toOffset: ffiReadOffset)
            return nil
        }
        try? fh.seek(toOffset: ffiReadOffset)
        let data = fh.readDataToEndOfFile()
        guard !data.isEmpty else { return nil }
        ffiReadOffset += UInt64(data.count)
        let text = String(data: data, encoding: .utf8) ?? ""
        for line in text.split(separator: "\n", omittingEmptySubsequences: true) {
            guard let j = String(line).data(using: .utf8),
                  let pkt = try? JSONDecoder().decode(NCPPacketData.self, from: j) else { continue }
            return PacketRow(
                timestamp: Date(timeIntervalSince1970: TimeInterval(pkt.timestamp_ms) / 1000),
                source: "\(pkt.source):\(pkt.source_port)",
                destination: "\(pkt.destination):\(pkt.dest_port)",
                protocol_: pkt.protocol_,
                length: pkt.length,
                info: pkt.flags
            )
        }
        return nil
    }

    func stop() {
        isRunning = false
        onStatusChange?(false)
        pollTimer?.cancel()
        pollTimer = nil
        NCPClient.stopPacketCapture()
    }

    deinit { stop() }
}

// MARK: - Helper

private func findHelperBinary() -> String? {
    let exec = CommandLine.arguments.first ?? ""
    let execDir = URL(fileURLWithPath: exec).deletingLastPathComponent().path
    let cwd = FileManager.default.currentDirectoryPath
    let rootDir = URL(fileURLWithPath: cwd).deletingLastPathComponent().deletingLastPathComponent().path
    let candidates = [
        "\(Bundle.main.bundlePath)/Contents/MacOS/ncp-capture-helper",
        "\(Bundle.main.bundlePath)/Contents/Resources/ncp-capture-helper",
        "\(execDir)/ncp-capture-helper",
        "\(cwd)/ncp-capture-helper",
        "\(rootDir)/target/release/ncp-capture-helper",
    ]
    return candidates.first { FileManager.default.isExecutableFile(atPath: $0) }
}

private func sourceHelperPath() -> String? {
    let cwd = FileManager.default.currentDirectoryPath
    let rootDir = URL(fileURLWithPath: cwd).deletingLastPathComponent().deletingLastPathComponent().path
    let src = "\(rootDir)/crates/ncp-capture-helper/src"
    if FileManager.default.fileExists(atPath: src) { return rootDir }
    let alt = "\(Bundle.main.bundlePath)/../../crates/ncp-capture-helper/src"
    if FileManager.default.fileExists(atPath: alt) {
        return URL(fileURLWithPath: alt).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().path
    }
    return nil
}

// MARK: - Main View

struct PacketScannerView: View {
    @EnvironmentObject var vm: AppViewModel
    @State private var isCapturing = false
    @State private var packets: [PacketRow] = []
    @State private var filterProtocol = "All"
    @State private var interfaceName = ""
    @State private var showHelp = false

    private let engine = CaptureEngine()

    var body: some View {
        VStack(spacing: 12) {
            toolbar
            packetList
            statusBar
        }
        .padding(12)
        .background(AppTheme.background.ignoresSafeArea())
        .onAppear {
            engine.onPacket = { [self] in addPacket($0) }
            engine.onStatusChange = { [self] in isCapturing = $0 }
            engine.onError = { [self] in vm.statusMessage = $0 }
        }
        .sheet(isPresented: $showHelp) { helpSheet }
    }

    // MARK: - Toolbar

    private var toolbar: some View {
        HStack(spacing: 12) {
            Picker("Protocol", selection: $filterProtocol) {
                Text("All").tag("All")
                Text("TCP").tag("TCP")
                Text("UDP").tag("UDP")
                Text("ICMP").tag("ICMP")
            }
            .pickerStyle(.segmented)
            .frame(width: 200)

            VStack(alignment: .leading, spacing: 4) {
                Text("Interface")
                    .font(.caption2)
                    .foregroundColor(AppTheme.textSecondary)
                StyledTextField(text: $interfaceName, placeholder: "auto", width: 110, height: 28)
            }

            Spacer()

            Button {
                if isCapturing { stop() } else { start() }
            } label: {
                HStack(spacing: 6) {
                    Circle()
                        .fill(isCapturing ? AppTheme.danger : AppTheme.textTertiary)
                        .frame(width: 8, height: 8)
                    Text(isCapturing ? "Stop" : "Start Capture")
                }
            }
            .buttonStyle(.borderedProminent)
            .tint(isCapturing ? AppTheme.danger : AppTheme.accent)
        }
        .cardStyle()
    }

    // MARK: - Packet List

    private var packetList: some View {
        GeometryReader { geo in
            VStack(spacing: 0) {
                header(geo: geo)
                if packets.isEmpty {
                    emptyState
                } else {
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(filteredPackets) { pkt in
                                PacketRowView(
                                    packet: pkt,
                                    sourceWidth: columnWidth(geo.size.width, ratio: 0.38),
                                    destWidth: columnWidth(geo.size.width, ratio: 0.38)
                                )
                                Divider().padding(.leading, 12)
                            }
                        }
                    }
                }
            }
        }
    }

    private func header(geo: GeometryProxy) -> some View {
        let sw = columnWidth(geo.size.width, ratio: 0.38)
        let dw = columnWidth(geo.size.width, ratio: 0.38)
        return HStack(spacing: 0) {
            Text("Time").frame(width: 80, alignment: .leading)
            Text("Source").frame(width: sw, alignment: .leading)
            Text("Dest").frame(width: dw, alignment: .leading)
            Text("Proto").frame(width: 55)
            Text("Len").frame(width: 50, alignment: .trailing)
            Text("Info").frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.caption.bold())
        .foregroundColor(AppTheme.textSecondary)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(AppTheme.secondaryBackground)
    }

    private func columnWidth(_ total: CGFloat, ratio: CGFloat) -> CGFloat {
        let fixed: CGFloat = 80 + 55 + 50 + 24
        let remaining = max(total - fixed, 200)
        return max(remaining * ratio, 100)
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            Spacer()
            Image(systemName: "network")
                .font(.system(size: 40))
                .foregroundColor(AppTheme.textTertiary)
            Text(isCapturing ? "Waiting for packets…\nNo packets yet — FFI capture needs root" : "Click Start Capture to monitor network packets")
                .font(.subheadline)
                .foregroundColor(AppTheme.textSecondary)
                .multilineTextAlignment(.center)
            if isCapturing {
                Button("Show Setup Instructions") { showHelp = true }
                    .buttonStyle(.link)
            }
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Status Bar

    private var statusBar: some View {
        HStack {
            Text("\(packets.count) packet\(packets.count == 1 ? "" : "s")")
                .font(.caption)
                .foregroundColor(AppTheme.textSecondary)
            Spacer()
            Button("Clear") { packets = [] }
                .buttonStyle(.link).controlSize(.small).disabled(packets.isEmpty)
            Button { showHelp = true } label: {
                Image(systemName: "questionmark.circle")
                    .foregroundColor(AppTheme.textSecondary)
            }
            .buttonStyle(.plain)
            .help("Packet capture setup instructions")
        }
    }

    // MARK: - Help Sheet

    private var helpSheet: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Packet Capture Setup")
                    .font(.headline)
                Spacer()
                Button("Close") { showHelp = false }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.escape)
            }

            Group {
                Text("1. Build the helper from source (one-time):")
                    .font(.subheadline.bold())

                let helperPath = sourceHelperPath()
                let buildCmd = "cargo build --release -p ncp-capture-helper"

                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(buildCmd)
                            .font(.system(.caption, design: .monospaced))
                            .textSelection(.enabled)
                            .padding(8)
                            .background(AppTheme.tertiaryBackground)
                            .cornerRadius(4)
                        if let path = helperPath {
                            Text("Run in: \(path)")
                                .font(.caption2)
                                .foregroundColor(AppTheme.textSecondary)
                        }
                    }
                    VStack(spacing: 6) {
                        Button("Copy") {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(buildCmd, forType: .string)
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        if let path = helperPath {
                            Button("Source") {
                                NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: path)
                            }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        }
                    }
                }
            }

            Divider()

            Group {
                Text("2. Run the helper with sudo (each session):")
                    .font(.subheadline.bold())

                let helper = findHelperBinary() ?? "ncp-capture-helper"
                let cmd = "sudo '\(helper)' --interface \(interfaceName.isEmpty ? "en0" : interfaceName) > /tmp/ncp-capture-output"

                HStack {
                    Text(cmd)
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                        .padding(10)
                        .background(AppTheme.tertiaryBackground)
                        .cornerRadius(6)
                        .overlay(
                            RoundedRectangle(cornerRadius: 6)
                                .stroke(AppTheme.border, lineWidth: 1)
                        )
                    Button("Copy") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(cmd, forType: .string)
                    }
                    .buttonStyle(.bordered)
                }

                Text("Output goes to /tmp/ncp-capture-output — the app reads from it automatically.")
                    .font(.caption)
                    .foregroundColor(AppTheme.textSecondary)
                Text("Press Ctrl+C in Terminal to stop when done.")
                    .font(.caption)
                    .foregroundColor(AppTheme.textSecondary)
            }

            Divider()

            Group {
                Text("Or just run the whole app with sudo (no helper needed):")
                    .font(.subheadline.bold())

                let app = Bundle.main.bundlePath.isEmpty ? "NetCat++.app" : Bundle.main.bundlePath
                let appCmd = "sudo '\(app)/Contents/MacOS/NCPDashApp'"
                Text(appCmd)
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                    .padding(8)
                    .background(AppTheme.tertiaryBackground)
                    .cornerRadius(4)
            }
        }
        .padding(20)
        .frame(width: 580)
        .background(AppTheme.background)
    }

    // MARK: - Actions

    private func start() {
        packets = []
        let iface = interfaceName.trimmingCharacters(in: .whitespacesAndNewlines)
        engine.start(interface: iface)
    }

    private func stop() {
        engine.stop()
    }

    private func addPacket(_ pkt: PacketRow) {
        packets.append(pkt)
        if packets.count > 1000 {
            packets.removeFirst(packets.count - 1000)
        }
    }

    private var filteredPackets: [PacketRow] {
        filterProtocol == "All"
            ? packets
            : packets.filter { $0.protocol_ == filterProtocol }
    }
}

#Preview {
    PacketScannerView()
        .environmentObject(AppViewModel())
}
