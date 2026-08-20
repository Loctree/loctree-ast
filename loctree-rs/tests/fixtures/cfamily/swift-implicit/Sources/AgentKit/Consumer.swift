// Real same-module consumer: references DefaultAgent by name with no
// `import` between the files. This is the one legitimate implicit edge.

import Foundation

struct AgentPanel {
    func attach() {
        let agent = DefaultAgent.instance
        try? agent.addKey()
    }
}
