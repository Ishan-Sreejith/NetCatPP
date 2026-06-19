import Foundation
import NCPKit

enum HTTPMethod: String, CaseIterable, Codable {
    case GET
    case POST
    case PUT
    case PATCH
    case DELETE
}

struct DashboardSnapshot {
    var cpuUsage: Double
    var memoryUsedGB: Double
    var memoryTotalGB: Double
    var topProcesses: [NCPProcess]
    var memoryPressure: Double?
    var gpuPressure: Double?

    static let empty = DashboardSnapshot(cpuUsage: 0, memoryUsedGB: 0, memoryTotalGB: 0, topProcesses: [], memoryPressure: nil, gpuPressure: nil)
}

struct ScanResultRow: Identifiable {
    let id = UUID()
    let port: Int
    let service: String
    let status: String
}
