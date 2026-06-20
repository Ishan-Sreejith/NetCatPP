# NCPDashApp

SwiftUI frontend for NetCat++. Built with NCPKit (Rust FFI via UniFFI).

### Build

```bash
# From project root: build Rust dylib first
bash scripts/build_swift_bridge.sh

# Then build this app
cd swiftui/NCPDashApp
swift build
swift run
```
