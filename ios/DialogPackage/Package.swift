// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "DialogPackage",
    platforms: [ .iOS(.v18), .macOS(.v14) ],
    products: [
        .library(name: "Dialog", targets: ["Dialog"]) 
    ],
    targets: [
        .target(
            name: "Dialog",
            dependencies: ["dialogFFI"],
            path: "Sources/Dialog"
        ),
        .binaryTarget(
            name: "dialogFFI",
            path: "XCFrameworks/dialogFFI_v18.xcframework"
        )
    ]
)

