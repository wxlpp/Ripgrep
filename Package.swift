// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "RipgrepKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "RipgrepKitCore", targets: ["RipgrepKitCore"]),
        .library(name: "RipgrepKitTool", targets: ["RipgrepKitTool"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser", from: "1.5.0"),
    ],
    targets: [
        .binaryTarget(
            name: "RipgrepCore",
            path: "Frameworks/RipgrepCore.xcframework"
        ),
        .target(name: "RipgrepKitFFI",
                dependencies: ["RipgrepCore"],
                sources: ["RipgrepCore.swift"],
                swiftSettings: [.swiftLanguageMode(.v5)]),
        .target(name: "RipgrepKitCore", dependencies: ["RipgrepKitFFI"]),
        .target(name: "RipgrepKitTool", dependencies: [
            "RipgrepKitCore",
            .product(name: "ArgumentParser", package: "swift-argument-parser"),
        ]),
        .testTarget(
            name: "RipgrepKitCoreTests",
            dependencies: ["RipgrepKitCore"],
            resources: [.copy("Fixtures")]
        ),
        .testTarget(
            name: "RipgrepKitToolTests",
            dependencies: ["RipgrepKitTool"],
            resources: [.copy("Fixtures")]
        ),
    ],
    swiftLanguageModes: [.v6]
)
