// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NamesakeApp",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "NamesakeApp", targets: ["UI"]),
    ],
    targets: [
        .executableTarget(name: "UI", path: "Sources/UI"),
    ]
)
