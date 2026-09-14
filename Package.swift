// swift-tools-version:6.0
import PackageDescription

// On main the package builds against a local XCFramework (scripts/build-xcframework.sh).
// Release tags are cut from a commit where scripts/set-release.sh switches to the
// published zip. A static flag, not a file check: SwiftPM caches manifest results.
let useLocalBinary = true
let releaseVersion = "0.0.0"
let releaseChecksum = "0000000000000000000000000000000000000000000000000000000000000000"

let coreBinary: Target = useLocalBinary
    ? .binaryTarget(name: "OhMyGrepCore", path: "Frameworks/OhMyGrepCore.xcframework")
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
