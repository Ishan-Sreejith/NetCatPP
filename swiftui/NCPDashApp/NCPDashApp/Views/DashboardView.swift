import SwiftUI
import NCPKit

struct DashboardView: View {
    @EnvironmentObject var vm: AppViewModel
    @State private var showAllProcesses = false
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                HStack {
                    Text("Dashboard")
                        .font(.title.bold())
                        .foregroundColor(AppTheme.textPrimary)
                    Spacer()
                    Button {
                        Task { await vm.refreshDashboard() }
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                    .buttonStyle(.bordered)
                    .disabled(vm.isRefreshing)
                }
                
                HStack(spacing: 12) {
                    MetricCard(title: "CPU", value: String(format: "%.1f%%", vm.dashboard.cpuUsage), tint: cpuColor)
                    MetricCard(title: "Memory", value: String(format: "%.1f / %.1f GB", vm.dashboard.memoryUsedGB, vm.dashboard.memoryTotalGB), tint: memoryColor)
                }

                if let memPressure = vm.dashboard.memoryPressure, let gpuPressure = vm.dashboard.gpuPressure {
                    HStack(spacing: 12) {
                        MetricCard(title: "Mem Pressure", value: String(format: "%.0f%%", memPressure), tint: memoryColor)
                        MetricCard(title: "iGPU Pressure", value: String(format: "%.0f%%", gpuPressure), tint: cpuColor)
                    }
                } else if let memPressure = vm.dashboard.memoryPressure {
                    HStack(spacing: 12) {
                        MetricCard(title: "Mem Pressure", value: String(format: "%.0f%%", memPressure), tint: memoryColor)
                    }
                }
                
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Text("Top Processes")
                            .font(.headline)
                            .foregroundColor(AppTheme.textPrimary)
                        Spacer()
                        Button(showAllProcesses ? "Show Less" : "Show All (\(vm.dashboard.topProcesses.count))") {
                            showAllProcesses.toggle()
                        }
                        .buttonStyle(.link)
                    }
                    
                    if vm.isRefreshing && vm.dashboard.topProcesses.isEmpty {
                        HStack {
                            Spacer()
                            VStack(spacing: 8) {
                                ProgressView()
                                    .scaleEffect(0.8)
                                Text("Loading process data...")
                                    .font(.caption)
                                    .foregroundColor(AppTheme.textSecondary)
                            }
                            .padding(.vertical, 20)
                            Spacer()
                        }
                    } else if vm.dashboard.topProcesses.isEmpty {
                        HStack {
                            Spacer()
                            Text("No process data yet")
                                .font(.caption)
                                .foregroundColor(AppTheme.textSecondary)
                                .padding(.vertical, 20)
                            Spacer()
                        }
                    } else {
                        ForEach(Array(processesToShow), id: \.pid) { p in
                            HStack(spacing: 12) {
                                Image(systemName: "app.badge")
                                    .foregroundColor(AppTheme.accent)
                                    .frame(width: 20)
                                Text(p.name)
                                    .foregroundColor(AppTheme.textPrimary)
                                    .lineLimit(1)
                                    .frame(minWidth: 150, alignment: .leading)
                                Spacer()
                                HStack(spacing: 16) {
                                    Text(String(format: "%.1f%%", p.cpu))
                                        .foregroundColor(cpuColor)
                                        .frame(width: 50, alignment: .trailing)
                                    Text("\(p.memory_mb) MB")
                                        .foregroundColor(AppTheme.textSecondary)
                                        .frame(width: 70, alignment: .trailing)
                                    Text("PID: \(p.pid)")
                                        .foregroundColor(AppTheme.textTertiary)
                                        .frame(width: 60, alignment: .trailing)
                                }
                                .font(.caption)
                            }
                            .padding(.vertical, 6)
                            .padding(.horizontal, 8)
                            .background(AppTheme.secondaryBackground.opacity(0.5))
                            .cornerRadius(6)
                        }
                    }
                }
                .cardStyle()
            }
            .padding(16)
        }
        .background(AppTheme.background.ignoresSafeArea())
    }
    
    var processesToShow: [NCPProcess] {
        Array(vm.dashboard.topProcesses.prefix(showAllProcesses ? 30 : 10))
    }
    
    var cpuColor: Color {
        if vm.dashboard.cpuUsage > 80 { return AppTheme.danger }
        if vm.dashboard.cpuUsage > 50 { return AppTheme.warning }
        return AppTheme.accent
    }
    
    var memoryColor: Color {
        let usedPercent = vm.dashboard.memoryTotalGB > 0 ? vm.dashboard.memoryUsedGB / vm.dashboard.memoryTotalGB : 0
        if usedPercent > 0.9 { return AppTheme.danger }
        if usedPercent > 0.7 { return AppTheme.warning }
        return AppTheme.accent
    }
}
