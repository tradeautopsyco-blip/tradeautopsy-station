import Foundation
@testable import Notch
@testable import Station

@MainActor
final class TrackingNotchHost: NotchHosting {
    let injectedViewModel: NotchViewModel
    private let launcher: NotchLauncher

    init(viewModel: NotchViewModel) {
        self.injectedViewModel = viewModel
        self.launcher = NotchLauncher(isHostedByStation: true, injectedViewModel: viewModel)
    }

    func start() async {
        launcher.start()
    }

    func dismiss() {
        launcher.dismiss()
    }

    func toggle() {
        launcher.toggle()
    }

    var launcherViewModel: NotchViewModel {
        launcher.viewModel
    }
}
