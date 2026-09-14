// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "OhMyGrep",
    platforms: [.iOS(.v16), .macOS(.v13)],
    products: [
        .library(name: "OhMyGrep", targets: ["OhMyGrep"]),
        .library(name: "OhMyGrepTool", targets: ["OhMyGrepTool"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser", from: "1.5.0"),
    ],
    targets: [
        .binaryTarget(
            name: "OhMyGrepCore",
            path: "Frameworks/OhMyGrepCore.xcframework"
        ),
        .target(name: "OhMyGrepFFI",
                dependencies: ["OhMyGrepCore"],
                sources: ["OhMyGrepCore.swift"],
                swiftSettings: [.swiftLanguageMode(.v5)]),
        .target(name: "OhMyGrep", dependencies: ["OhMyGrepFFI"]),
        .target(name: "OhMyGrepTool", dependencies: [
            "OhMyGrep",
            .product(name: "ArgumentParser", package: "swift-argument-parser"),
        ]),
        .testTarget(
            name: "OhMyGrepTests",
            dependencies: ["OhMyGrep"],
            resources: [.copy("Fixtures")]
        ),
        .testTarget(
            name: "OhMyGrepToolTests",
            dependencies: ["OhMyGrepTool", "OhMyGrep"],
            resources: [.copy("Fixtures")]
        ),
    ],
    swiftLanguageModes: [.v6]
)
