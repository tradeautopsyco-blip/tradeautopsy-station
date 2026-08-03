import SwiftUI

public struct StationPlaceholderView: View {
    let route: StationRoute

    public init(route: StationRoute) {
        self.route = route
    }

    public var body: some View {
        VStack(spacing: 12) {
            Text(route.rawValue)
                .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .medium))
                .foregroundStyle(StationDS.Text.primary)

            Text("This screen is coming in a future release.")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .regular))
                .foregroundStyle(StationDS.Text.muted)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
