import Notch
import SwiftUI

public struct StationPlaceholderView: View {
    let route: StationRoute

    public init(route: StationRoute) {
        self.route = route
    }

    public var body: some View {
        VStack(spacing: 12) {
            Text(route.rawValue)
                .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .medium))
                .foregroundStyle(BarDS.Text.primary)

            Text("This screen is coming in a future release.")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundStyle(BarDS.Text.muted)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
    }
}
