import Foundation

public enum NCPClient {
    private static func ffiStartPacketCapture(interface: String?, protoFilter: String?) {
        NCPKit.startPacketCapture(interface: interface, protoFilter: protoFilter)
    }

    private static func ffiPollPacketsJson(maxCount: UInt32) throws -> String {
        try NCPKit.pollPacketsJson(maxCount: maxCount)
    }

    private static func ffiStopPacketCapture() {
        NCPKit.stopPacketCapture()
    }

    private static func ffiStartTextListener(port: UInt16, keepAlive: Bool) {
        NCPKit.startTextListener(port: port, keepAlive: keepAlive)
    }

    private static func ffiPollTextMessagesJson(maxCount: UInt32) throws -> String {
        try NCPKit.pollTextMessagesJson(maxCount: maxCount)
    }

    private static func ffiStopTextListener() {
        NCPKit.stopTextListener()
    }
    public static func version() -> String {
        ncpVersion()
    }

    public static func systemSnapshot() throws -> NCPSystemSnapshot {
        let json = try systemSnapshotJson()
        let data = Data(json.utf8)
        return try JSONDecoder().decode(NCPSystemSnapshot.self, from: data)
    }

    public static func scanHost(
        host: String,
        range: String,
        timeout: String,
        udp: Bool,
        concurrency: UInt32
    ) throws -> String {
        try scanHostJson(host: host, range: range, timeout: timeout, udp: udp, concurrency: concurrency)
    }

    public static func httpRequest(
        url: String,
        method: String,
        body: String?,
        repeat: UInt32,
        follow: Bool,
        maxRedirects: UInt32
    ) throws -> NCPHttpResponse {
        let json = try httpRequestJson(
            url: url,
            method: method,
            body: body,
            repeat: `repeat`,
            follow: follow,
            maxRedirects: maxRedirects
        )
        let data = Data(json.utf8)
        return try JSONDecoder().decode(NCPHttpResponse.self, from: data)
    }

    public static func sendTextMessage(
        host: String,
        port: UInt16,
        message: String,
        repeat: UInt32,
        interval: String
    ) throws {
        _ = try sendText(host: host, port: port, message: message, repeat: `repeat`, interval: interval)
    }

    public static func startPacketCapture(interface: String? = nil, protoFilter: String? = nil) {
        ffiStartPacketCapture(interface: interface, protoFilter: protoFilter)
    }

    public static func pollPackets(maxCount: UInt32 = 100) throws -> [NCPPacketData] {
        let json = try ffiPollPacketsJson(maxCount: maxCount)
        let data = Data(json.utf8)
        return try JSONDecoder().decode([NCPPacketData].self, from: data)
    }

    public static func stopPacketCapture() {
        ffiStopPacketCapture()
    }

    public static func startTextListener(port: UInt16, keepAlive: Bool) {
        ffiStartTextListener(port: port, keepAlive: keepAlive)
    }

    public static func pollTextMessages(maxCount: UInt32 = 100) throws -> [NCPTextMessage] {
        let json = try ffiPollTextMessagesJson(maxCount: maxCount)
        let data = Data(json.utf8)
        return try JSONDecoder().decode([NCPTextMessage].self, from: data)
    }

    public static func stopTextListener() {
        ffiStopTextListener()
    }
}
