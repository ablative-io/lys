import Foundation
import LocalAuthentication
import Darwin

let context = LAContext()
context.touchIDAuthenticationAllowableReuseDuration = 0
context.localizedCancelTitle = "Cancel"
var failure: NSError?
guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &failure) else {
    print("unavailable")
    exit(2)
}
context.evaluatePolicy(
    .deviceOwnerAuthentication,
    localizedReason: "Authorize a one-time administrator setup or password recovery code for Lys."
) { authenticated, error in
    if authenticated {
        print("authenticated")
        exit(0)
    }
    let code = (error as? LAError)?.code
    switch code {
    case .userCancel, .appCancel, .systemCancel:
        print("cancelled")
    case .authenticationFailed:
        print("refused")
    default:
        print("unavailable")
    }
    exit(2)
}
RunLoop.main.run()
