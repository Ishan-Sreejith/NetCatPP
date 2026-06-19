// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NCPKit",
    platforms: [
        .macOS(.v13),
        .iOS(.v16)
    ],
    products: [
        .library(name: "NCPKit", targets: ["NCPKit"])
    ],
    targets: [
        .target(
            name: "ncpffiFFI",
            dependencies: [],
            path: "Sources/ncpffiFFI"
        ),
        .binaryTarget(
            name: "ncpffiBinary",
            path: "Sources/NCPKit.xcframework"
        ),
        .target(
            name: "NCPKit",
            dependencies: ["ncpffiFFI", "ncpffiBinary"],
            path: "Sources/NCPKit"
        ),
        .testTarget(
            name: "NCPKitTests",
            dependencies: ["NCPKit"],
            path: "Tests/NCPKitTests"
        )
    ]
)