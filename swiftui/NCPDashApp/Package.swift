// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NCPDashApp",
    platforms: [
        .macOS(.v13),
        .iOS(.v16)
    ],
    products: [
        .executable(name: "NCPDashApp", targets: ["NCPDashApp"])
    ],
    dependencies: [
        .package(path: "../NCPKit")
    ],
    targets: [
        .executableTarget(
            name: "NCPDashApp",
            dependencies: ["NCPKit"],
            path: "NCPDashApp"
        )
    ]
)