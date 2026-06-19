import SwiftUI
import AppKit

struct HTTPView: View {
    @EnvironmentObject var vm: AppViewModel
    @State private var isLoading = false
    
    var body: some View {
        VStack(spacing: 16) {
            HStack(spacing: 12) {
                Picker("Method", selection: $vm.httpMethod) {
                    ForEach(HTTPMethod.allCases, id: \.self) { method in
                        Text(method.rawValue).tag(method)
                    }
                }
                .pickerStyle(.segmented)
                .frame(width: 180)
                
                StyledTextField(text: $vm.httpURL, placeholder: "https://api.example.com")
                    .frame(height: 30)
                
                Button {
                    Task {
                        isLoading = true
                        await vm.runHttp()
                        isLoading = false
                    }
                } label: {
                    HStack(spacing: 6) {
                        if isLoading {
                            ProgressView()
                                .scaleEffect(0.7)
                        }
                        Text(isLoading ? "Loading..." : "Send")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(isLoading)
            }
            .cardStyle()
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Request Body (optional)")
                    .font(.caption2)
                    .foregroundColor(AppTheme.textSecondary)
                if vm.httpMethod == .GET || vm.httpMethod == .DELETE {
                    Text("Body disabled for \(vm.httpMethod.rawValue)")
                        .font(.caption2)
                        .foregroundColor(AppTheme.textTertiary)
                }

                TextEditor(text: $vm.httpBody)
                    .font(.system(.body, design: .monospaced))
                    .scrollContentBackground(.hidden)
                    .background(AppTheme.tertiaryBackground)
                    .cornerRadius(8)
                    .frame(minHeight: 80)
                    .disabled(vm.httpMethod == .GET || vm.httpMethod == .DELETE)
            }
            .cardStyle()
            
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Response")
                        .font(.headline)
                    Spacer()
                    Button {
                        #if os(macOS)
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(vm.httpResponse, forType: .string)
                        #endif
                    } label: {
                        Label("Copy", systemImage: "doc.on.doc")
                    }
                    .buttonStyle(.link)
                    .controlSize(.small)
                }
                
                ScrollView {
                    Text(vm.httpResponse.isEmpty ? "Response will appear here..." : vm.httpResponse)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(vm.httpResponse.isEmpty ? AppTheme.textSecondary : AppTheme.textPrimary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled)
                }
            }
            .cardStyle()
        }
        .padding(16)
        .background(AppTheme.background.ignoresSafeArea())
    }
}
