import SwiftUI

struct ScannerView: View {
    @EnvironmentObject var vm: AppViewModel
    @State private var isScanning = false
    
    var body: some View {
        VStack(spacing: 16) {
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Host")
                        .font(.caption2)
                        .foregroundColor(AppTheme.textSecondary)
                    StyledTextField(text: $vm.scannerHost, placeholder: "127.0.0.1")
                        .frame(width: 140)
                }
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("Port Range")
                        .font(.caption2)
                        .foregroundColor(AppTheme.textSecondary)
                    StyledTextField(text: $vm.scannerRange, placeholder: "1-1024")
                        .frame(width: 100)
                }
                
                Spacer()
                
                Button {
                    Task {
                        isScanning = true
                        await vm.runScan()
                        isScanning = false
                    }
                } label: {
                    HStack(spacing: 6) {
                        if isScanning {
                            ProgressView()
                                .scaleEffect(0.7)
                        }
                        Text(isScanning ? "Scanning..." : "Scan")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(isScanning)
            }
            .cardStyle()
            
            if vm.scannerRows.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 40))
                        .foregroundColor(AppTheme.textTertiary)
                    Text("Enter a host and port range, then click Scan")
                        .font(.subheadline)
                        .foregroundColor(AppTheme.textSecondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Results")
                            .font(.headline)
                        Spacer()
                        Text("\(vm.scannerRows.count) open ports")
                            .font(.caption)
                            .foregroundColor(AppTheme.textSecondary)
                    }
                    
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(vm.scannerRows) { row in
                                HStack(spacing: 12) {
                                    Image(systemName: row.status == "open" ? "lock.open.fill" : "lock.fill")
                                        .foregroundColor(row.status == "open" ? AppTheme.success : AppTheme.textTertiary)
                                        .frame(width: 20)
                                    
                                    Text("\(row.port)")
                                        .font(.system(.body, design: .monospaced))
                                        .foregroundColor(AppTheme.textPrimary)
                                        .frame(width: 60, alignment: .leading)
                                    
                                    Text(row.service)
                                        .foregroundColor(AppTheme.textSecondary)
                                    
                                    Spacer()
                                    
                                    Text(row.status.uppercased())
                                        .font(.caption2.bold())
                                        .foregroundColor(row.status == "open" ? AppTheme.success : AppTheme.textTertiary)
                                        .padding(.horizontal, 8)
                                        .padding(.vertical, 2)
                                        .background(
                                            (row.status == "open" ? AppTheme.success : AppTheme.textTertiary).opacity(0.1)
                                        )
                                        .cornerRadius(4)
                                }
                                .padding(.horizontal, 16)
                                .padding(.vertical, 8)
                                .background(AppTheme.secondaryBackground.opacity(0.3))
                                Divider()
                            }
                        }
                    }
                }
                .cardStyle()
            }
        }
        .padding(16)
        .background(AppTheme.background.ignoresSafeArea())
    }
}