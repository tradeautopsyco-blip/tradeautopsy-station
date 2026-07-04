import Foundation
import Notch
import SwiftUI

@MainActor
public final class HostedNotchLauncher: NotchHosting, NotchPollingControlling {
    public let viewModel: NotchViewModel
    private let launcher: NotchLauncher

    public init(viewModel: NotchViewModel) {
        self.viewModel = viewModel
        self.launcher = NotchLauncher(isHostedByStation: true, injectedViewModel: viewModel)
    }

    public func bindCoordinator(_ coordinator: StationAppCoordinator) {
        launcher.setHostedExpandedContent {
            AnyView(HostedExpandedNotchShell(coordinator: coordinator))
        }
    }

    public func configure(secret: String, port: UInt16, webBase: String) {
        launcher.configure(secret: secret, port: port, webBase: webBase)
    }

    public func start() async {
        launcher.start()
    }

    public func dismiss() {
        launcher.dismiss()
    }

    public func startPolling() {
        // `NotchLauncher.start()` already starts polling when hosted.
    }

    public func stopPolling() {
        viewModel.stopPolling()
    }

    public func toggle() {
        launcher.toggle()
    }
}
