import Foundation

public struct NCPSystemSnapshot: Codable {
    public let cpu_usage: Double
    public let memory_used_gb: Double
    public let memory_total_gb: Double
    public let memory_pressure: Double?
    public let gpu_pressure: Double?
    public let top_processes: [NCPProcess]
}

public struct NCPProcess: Codable {
    public let pid: String
    public let name: String
    public let cpu: Double
    public let memory_mb: UInt64
}

public struct NCPHttpResponse: Codable {
    public let status: UInt16
    public let final_url: String
    public let headers: [[String]]
    public let body: String
    public let avg_latency_ms: Double
    public let total_ms_last_request: UInt64
}

public struct NCPPacketData: Codable, Identifiable {
    public var id: String { "\(timestamp_ms)-\(source)" }
    public let timestamp_ms: UInt64
    public let source: String
    public let source_port: UInt16
    public let destination: String
    public let dest_port: UInt16
    public let protocol_: String
    public let flags: String
    public let length: Int

    enum CodingKeys: String, CodingKey {
        case timestamp_ms, source, source_port, destination, dest_port
        case protocol_ = "protocol"
        case flags, length
    }
}

public struct NCPTextMessage: Codable, Identifiable {
    public var id: String { "\(timestamp_ms)-\(peer)" }
    public let timestamp_ms: UInt64
    public let peer: String
    public let message: String
}
