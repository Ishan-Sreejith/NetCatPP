import SwiftUI
import NCPKit

struct SettingsView: View {
    @EnvironmentObject var vm: AppViewModel
    
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Settings")
                    .font(.title.bold())
                    .foregroundColor(AppTheme.textPrimary)
                
                VStack(alignment: .leading, spacing: 12) {
                    Text("About")
                        .font(.headline)
                    
                    HStack {
                        Text("App Version")
                            .foregroundColor(AppTheme.textSecondary)
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(AppTheme.textPrimary)
                    }
                    
                    Divider()
                    
                    HStack {
                        Text("Core Version")
                            .foregroundColor(AppTheme.textSecondary)
                        Spacer()
                        Text(NCPClient.version())
                            .foregroundColor(AppTheme.accent)
                    }
                }
                .cardStyle()
                
                VStack(alignment: .leading, spacing: 12) {
                    Text("Runtime")
                        .font(.headline)
                    
                    HStack {
                        Text("Status")
                            .foregroundColor(AppTheme.textSecondary)
                        Spacer()
                        Text(vm.statusMessage)
                            .foregroundColor(AppTheme.textPrimary)
                    }
                    
                    Divider()
                    
                    HStack {
                        Text("Last Update")
                            .foregroundColor(AppTheme.textSecondary)
                        Spacer()
                        Text(Date(), format: .dateTime.hour().minute().second())
                            .foregroundColor(AppTheme.textPrimary)
                    }
                }
                .cardStyle()
                
                VStack(alignment: .leading, spacing: 12) {
                    Text("Quick Actions")
                        .font(.headline)
                    
                    Button {
                        Task { await vm.refreshDashboard() }
                    } label: {
                        Label("Refresh Dashboard", systemImage: "arrow.clockwise")
                    }
                    .buttonStyle(.bordered)
                    
                    Button {
                        vm.scannerRows = []
                    } label: {
                        Label("Clear Scan Results", systemImage: "trash")
                    }
                    .buttonStyle(.bordered)
                    
                    Button {
                        vm.httpResponse = ""
                    } label: {
                        Label("Clear HTTP Response", systemImage: "trash")
                    }
                    .buttonStyle(.bordered)
                }
                .cardStyle()
                
                Spacer()
            }
            .padding(16)
        }
        .background(AppTheme.background.ignoresSafeArea())
    }
}