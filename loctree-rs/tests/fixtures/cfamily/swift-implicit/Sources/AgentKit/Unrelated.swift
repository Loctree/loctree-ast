// Uses only the STDLIB `Error` protocol. Must never become an "importer" of
// Agent.swift just because Agent.swift declares a nested `enum Error`.

import Foundation

struct FuzzyMatcher {
    func run() -> Error? {
        return nil
    }
}
