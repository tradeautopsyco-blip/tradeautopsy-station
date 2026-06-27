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
    targets: [
        .target(
            name: "Notch",
            path: ".",
            exclude: ["Package.swift"],
            linkerSettings: [
                .linkedFramework("AVFoundation"),
                .linkedFramework("Speech"),
            ]
        ),
    ]
)
