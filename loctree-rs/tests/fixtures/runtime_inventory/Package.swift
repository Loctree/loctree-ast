import PackageDescription

let package = Package(
    name: "RuntimeFixture",
    products: [.executable(name: "RuntimeFixture", targets: ["RuntimeFixture"])],
    targets: [.executableTarget(name: "RuntimeFixture")]
)
