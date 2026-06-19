import SwiftUI

struct MessengerView: View {
    @EnvironmentObject var vm: AppViewModel
    @State private var isSending = false
    
    var body: some View {
        VStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 12) {
                Text("Direct Text Sender")
                    .font(.title2.bold())
                    .foregroundColor(AppTheme.textPrimary)
                
                Text("Send raw text messages to TCP servers")
                    .font(.subheadline)
                    .foregroundColor(AppTheme.textSecondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .cardStyle()
            
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Host")
                        .font(.caption2)
                        .foregroundColor(AppTheme.textSecondary)
                    StyledTextField(text: $vm.textHost, placeholder: "127.0.0.1")
                }
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("Port")
                        .font(.caption2)
                        .foregroundColor(AppTheme.textSecondary)
                    StyledTextField(text: $vm.textPort, placeholder: "9000", width: 80, height: 30)
                }
                
                Spacer()
            }
            .cardStyle()
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Message")
                    .font(.caption2)
                    .foregroundColor(AppTheme.textSecondary)

                TextEditor(text: $vm.textMessage)
                    .font(.system(.body, design: .monospaced))
                    .scrollContentBackground(.hidden)
                    .background(AppTheme.tertiaryBackground)
                    .cornerRadius(8)
                    .frame(minHeight: 120)
            }
            .cardStyle()

            HStack {
                Button {
                    Task {
                        isSending = true
                        await vm.sendText()
                        isSending = false
                    }
                } label: {
                    HStack(spacing: 6) {
                        if isSending {
                            ProgressView()
                                .scaleEffect(0.7)
                        }
                        Text(isSending ? "Sending..." : "Send Message")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(isSending)

                Spacer()

                Text(vm.statusMessage)
                    .font(.caption)
                    .foregroundColor(AppTheme.textSecondary)
            }

            VStack(alignment: .leading, spacing: 12) {
                Text("Listener")
                    .font(.headline)
                    .foregroundColor(AppTheme.textPrimary)

                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Port")
                            .font(.caption2)
                            .foregroundColor(AppTheme.textSecondary)
                        StyledTextField(text: $vm.textListenerPort, placeholder: "9000", width: 80, height: 30)
                    }

                    Button {
                        if vm.isTextListening {
                            vm.stopTextListener()
                        } else {
                            vm.startTextListener()
                        }
                    } label: {
                        Text(vm.isTextListening ? "Stop Listening" : "Start Listening")
                    }
                    .buttonStyle(.bordered)

                    Spacer()
                }

                if vm.textListenerMessages.isEmpty {
                    Text(vm.isTextListening ? "Listening..." : "No messages yet")
                        .font(.caption)
                        .foregroundColor(AppTheme.textSecondary)
                } else {
                    ScrollView {
                        LazyVStack(spacing: 8) {
                            ForEach(vm.textListenerMessages) { msg in
                                VStack(alignment: .leading, spacing: 6) {
                                    HStack {
                                        Text(msg.peer)
                                            .font(.caption.bold())
                                            .foregroundColor(AppTheme.textPrimary)
                                        Spacer()
                                        Text(Date(timeIntervalSince1970: TimeInterval(msg.timestamp_ms) / 1000), format: .dateTime.hour().minute().second())
                                            .font(.caption2)
                                            .foregroundColor(AppTheme.textSecondary)
                                    }
                                    Text(msg.message)
                                        .font(.system(.caption, design: .monospaced))
                                        .foregroundColor(AppTheme.textPrimary)
                                }
                                .padding(10)
                                .background(AppTheme.secondaryBackground.opacity(0.5))
                                .cornerRadius(8)
                            }
                        }
                    }
                    .frame(minHeight: 120)
                }
            }
            .cardStyle()

            Spacer()
        }
        .padding(16)
        .background(AppTheme.background.ignoresSafeArea())
    }
}
