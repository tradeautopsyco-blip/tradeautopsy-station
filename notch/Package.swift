// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "Notch",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .library(
            name: "Notch",
            targets: ["Notch"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-testing.git", from: "0.10.0"),
    ],
    targets: [
        .target(
            name: "Notch",
            path: ".",
            exclude: ["Package.swift", "Tests"],
            linkerSettings: [
                .linkedFramework("AVFoundation"),
                .linkedFramework("Speech"),
            ]
        ),
        .testTarget(
            name: "NotchTests",
            dependencies: [
                "Notch",
                .product(name: "Testing", package: "swift-testing"),
            ],
            path: "Tests/NotchTests"
        ),
    ]
)
