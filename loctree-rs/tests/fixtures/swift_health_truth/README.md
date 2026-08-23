# HealthTruth probe

A seven-file macOS app with nothing wrong with it.

- `Sources/App/HealthTruthApp.swift` — the `@main` entry point.
- `Sources/App/HealthTruthHelper.swift` — one helper, called from the entry point.
- `Sources/App/HealthTruthViewA.swift`, `…ViewB.swift` — two independent views.
  They share the member names `text`, `body` and `makeNSView` because SwiftUI and
  NSViewRepresentable require those names, not because anything is duplicated.
- `AppTests/HealthTruthHelperTests.swift` — an XCTest target.
- `scripts/build-health-truth.sh` — a release script.

Three of these files are graph roots by role: the test target, the script and this
document. Nothing imports them and nothing ever will. That is the shape of a
healthy repository, and the health score has to agree.
