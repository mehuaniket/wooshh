import Foundation
import UserNotifications

struct Args {
    var title: String = "wooshh"
    var body: String = "Command completed"
    var kind: String = "success"
}

func parseArgs() -> Args {
    var parsed = Args()
    var i = 1
    let args = CommandLine.arguments
    while i < args.count {
        switch args[i] {
        case "--title":
            if i + 1 < args.count { parsed.title = args[i + 1]; i += 1 }
        case "--body":
            if i + 1 < args.count { parsed.body = args[i + 1]; i += 1 }
        case "--kind":
            if i + 1 < args.count { parsed.kind = args[i + 1]; i += 1 }
        default:
            break
        }
        i += 1
    }
    return parsed
}

let input = parseArgs()
let center = UNUserNotificationCenter.current()

let authSemaphore = DispatchSemaphore(value: 0)
var authorized = false
center.requestAuthorization(options: [.alert, .sound]) { granted, _ in
    authorized = granted
    authSemaphore.signal()
}
_ = authSemaphore.wait(timeout: .now() + 5.0)

if !authorized {
    fputs("Notification authorization denied.\n", stderr)
    exit(1)
}

let content = UNMutableNotificationContent()
content.title = input.title
content.body = input.body
content.sound = .default

let request = UNNotificationRequest(
    identifier: UUID().uuidString,
    content: content,
    trigger: nil
)

let sendSemaphore = DispatchSemaphore(value: 0)
var sendError: Error?
center.add(request) { error in
    sendError = error
    sendSemaphore.signal()
}
_ = sendSemaphore.wait(timeout: .now() + 2.0)

if sendError != nil {
    exit(2)
}

// Keep process alive briefly so macOS can flush delivery.
Thread.sleep(forTimeInterval: 0.25)
