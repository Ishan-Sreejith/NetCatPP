import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var vm: AppViewModel
    @State private var selectedTab = 0
    
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                TabBarButton(title: "Dashboard", icon: "gauge.with.dots.needle.67percent", tag: 0, selected: $selectedTab)
                TabBarButton(title: "Scanner", icon: "antenna.radiowaves.left.and.right", tag: 1, selected: $selectedTab)
                TabBarButton(title: "HTTP", icon: "globe", tag: 2, selected: $selectedTab)
                TabBarButton(title: "Packets", icon: "network", tag: 3, selected: $selectedTab)
                TabBarButton(title: "Text", icon: "message", tag: 4, selected: $selectedTab)
                TabBarButton(title: "Settings", icon: "gearshape", tag: 5, selected: $selectedTab)
            }
            .frame(height: 50)
            .padding(.horizontal, 16)
            .background(AppTheme.secondaryBackground)
            
            ZStack {
                switch selectedTab {
                case 0: DashboardView()
                case 1: ScannerView()
                case 2: HTTPView()
                case 3: PacketScannerView()
                case 4: MessengerView()
                case 5: SettingsView()
                default: DashboardView()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(minWidth: 980, minHeight: 680)
        .background(AppTheme.background.ignoresSafeArea())
    }
}

struct TabBarButton: View {
    let title: String
    let icon: String
    let tag: Int
    @Binding var selected: Int
    
    var body: some View {
        Button {
            selected = tag
        } label: {
            VStack(spacing: 4) {
                Image(systemName: icon)
                    .font(.system(size: 18))
                Text(title)
                    .font(.caption2)
            }
            .foregroundColor(selected == tag ? AppTheme.accent : AppTheme.textSecondary)
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.plain)
    }
}