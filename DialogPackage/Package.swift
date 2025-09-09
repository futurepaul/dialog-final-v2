// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "DialogPackage",
    platforms: [
        .iOS(.v16),
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "Dialog",
            targets: ["Dialog", "dialogFFI"]
        )
    ],
    targets: [
        .target(
            name: "Dialog",
            dependencies: ["dialogFFI"],
            path: "Sources/Dialog"
        ),
        .binaryTarget(
            name: "dialogFFI",
            path: "XCFrameworks/dialogFFI.xcframework"
        )
    ]
)