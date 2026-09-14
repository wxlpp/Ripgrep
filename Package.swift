// swift-tools-version:6.0
import Foundation
import PackageDescription

// Rewritten by scripts/set-release.sh during the release workflow.
let releaseVersion = "0.0.0"
let releaseChecksum = "0000000000000000000000000000000000000000000000000000000000000000"

// A locally built XCFramework (scripts/build-xcframework.sh) takes precedence over
// the release download, so contributors can change the Rust core.
let localXCFramework = "Frameworks/OhMyGrepCore.xcframework"
let coreBinary: Target =
    FileManager.default.fileExists(atPath: Context.packageDirectory + "/" + localXCFramework)
    ? .binaryTarget(name: "OhMyGrepCore", path: localXCFramework)
    : .binaryTarget(
        name: "OhMyGrepCore",
        url: "https://github.com/wxlpp/oh-my-grep/releases/download/\(releaseVersion)/OhMyGrepCore.xcframework.zip",
        checksum: releaseChecksum
    )

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
        coreBinary,
        .target(name: "OhMyGrepFFI",
                dependencies: ["OhMyGrepCore"],
                sources: ["OhMyGrepCore.swift"],
                swiftSettings: [.swiftLanguageMode(.v5)]),
        .target(name: "OhMyGrep",
                dependencies: ["OhMyGrepFFI"],
                resources: [.copy("PrivacyInfo.xcprivacy")]),
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
