// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "Station",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .executable(name: "StationApp", targets: ["StationApp"]),
        .library(name: "Station", targets: ["Station"]),
    ],
    dependencies: [
        .package(path: "../notch"),
        .package(url: "https://github.com/apple/swift-testing.git", from: "0.10.0"),
    ],
    targets: [
        .target(
            name: "Station",
            dependencies: [
                .product(name: "Notch", package: "Notch"),
            ],
            path: "StationApp",
            exclude: ["StationApp.swift", "Info.plist"],
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("IOKit"),
                .linkedFramework("ServiceManagement"),
                .linkedFramework("Security"),
            ]
        ),
        .executableTarget(
            name: "StationApp",
            dependencies: [
                "Station",
                .product(name: "Notch", package: "Notch"),
            ],
            path: "StationApp",
            exclude: [
                "AgentSupervisor.swift",
                "StationAppCoordinator.swift",
                "StatusItemController.swift",
                "Info.plist",
                "Models",
                "Protocols",
            ],
            sources: ["StationApp.swift"],
            linkerSettings: [
                .unsafeFlags([
                    "-Xlinker", "-sectcreate",
                    "-Xlinker", "__TEXT",
                    "-Xlinker", "__info_plist",
                    "-Xlinker", "StationApp/Info.plist",
                ], .when(platforms: [.macOS])),
            ]
        ),
        .testTarget(
            name: "StationTests",
            dependencies: [
                "Station",
                .product(name: "Notch", package: "Notch"),
                .product(name: "Testing", package: "swift-testing"),
            ],
            path: "StationTests"
        ),
    ]
)
