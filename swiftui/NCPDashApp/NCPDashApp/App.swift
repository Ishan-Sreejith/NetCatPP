import SwiftUI
import AppKit
import NCPKit

@main
struct NCPDashApp: App {
    #if os(macOS)
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    #else
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    #endif
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appDelegate.viewModel)
        }
        #if os(macOS)
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1100, height: 750)
        .commands {
            CommandGroup(replacing: .appInfo) {
                Button("About NetCat++") {
                    NSApplication.shared.orderFrontStandardAboutPanel(
                        options: [
                            .applicationName: "NetCat++",
                            .applicationVersion: "1.0.0",
                            .version: "1",
                            .credits: NSAttributedString(
                                string: "A modern cross-platform networking toolkit",
                                attributes: [.foregroundColor: NSColor.secondaryLabelColor]
                            )
                        ]
                    )
                }
            }
            
            CommandGroup(after: .appSettings) {
                Button("Preferences...") {
                    NotificationCenter.default.post(name: .showSettings, object: nil)
                }
                .keyboardShortcut(",", modifiers: .command)
            }
        }
        #endif
        
        #if os(macOS)
        Settings {
            SettingsView()
        }
        #endif
    }
}

#if os(macOS)
typealias AppDefaultDelegate = NSObject & NSApplicationDelegate
#else
typealias AppDefaultDelegate = NSObject & UIApplicationDelegate
#endif

@MainActor final class AppDelegate: AppDefaultDelegate, ObservableObject {
    let viewModel = AppViewModel()
    
    #if os(macOS)
    private var statusItem: NSStatusItem?
    private var popover: NSPopover?
    private var eventMonitor: Any?
    #endif
    
    #if os(macOS)
    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApplication.shared.setActivationPolicy(.regular)
        setupMenuBar()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            NSApplication.shared.activate(ignoringOtherApps: true)
            NSApplication.shared.windows.forEach { window in
                window.makeKeyAndOrderFront(nil)
            }
        }
    }
    #else
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey : Any]? = nil) -> Bool {
        return true
    }
    #endif
    
    #if os(macOS)
    private func setupMenuBar() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        
        if let button = statusItem?.button {
            button.image = NSImage(systemSymbolName: "network", accessibilityDescription: "NetCat++")
            button.action = #selector(togglePopover)
            button.target = self
        }
        
        popover = NSPopover()
        popover?.contentSize = NSSize(width: 320, height: 400)
        popover?.behavior = .transient
        popover?.contentViewController = NSHostingController(rootView: PopoverView().environmentObject(viewModel))
        
        eventMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseDown, .rightMouseDown]) { [weak self] _ in
            if let popover = self?.popover, popover.isShown {
                popover.performClose(nil)
            }
        }
    }
    
    @objc private func togglePopover() {
        guard let button = statusItem?.button, let popover = popover else { return }
        
        if popover.isShown {
            popover.performClose(nil)
        } else {
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
            popover.contentViewController?.view.window?.makeKey()
        }
    }
    #endif
    
    #if os(macOS)
    func applicationWillTerminate(_ notification: Notification) {
        if let eventMonitor = eventMonitor {
            NSEvent.removeMonitor(eventMonitor)
        }
    }
    #endif
}

#if os(macOS)
struct PopoverView: View {
    @EnvironmentObject var vm: AppViewModel
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "network")
                    .font(.title2.weight(.semibold))
                Text("NetCat++")
                    .font(.title2.weight(.semibold))
            }
            
            Divider()
            
            HStack(spacing: 16) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("CPU")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    Text(String(format: "%.0f%%", vm.dashboard.cpuUsage))
                        .font(.title3.weight(.medium))
                }
                
                VStack(alignment: .leading, spacing: 4) {
                    Text("Memory")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                    Text(String(format: "%.1fGB", vm.dashboard.memoryUsedGB))
                        .font(.title3.weight(.medium))
                }
            }
            
            Divider()
            
            HStack(spacing: 8) {
                Button("Scan") {
                    Task { await vm.runScan() }
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                
                Button("HTTP") {
                    Task { await vm.runHttp() }
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                
                Button("Text") {
                    Task { await vm.sendText() }
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
            }
            
            Divider()
            
            Text(vm.statusMessage)
                .font(.caption)
                .foregroundColor(.secondary)
                .lineLimit(2)
            
            Spacer()
            
            HStack {
                Button("Open Dashboard") {
                    NSApplication.shared.activate(ignoringOtherApps: true)
                }
                .buttonStyle(.link)
                .controlSize(.small)
                
                Spacer()
                
                Text("v1.0.0")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding()
        .frame(width: 280, height: 360)
    }
}
#endif

extension Notification.Name {
    static let showSettings = Notification.Name("showSettings")
}
