import SwiftUI
import Foundation
import CEMTokens

@main
struct CEMTokensExampleApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

struct ContentView: View {
    var body: some View {
        VStack(alignment: .leading, spacing: CEMTokens.Light.cemGapBlock.cemPoints) {
            Text("CEM Tokens")
                .font(.system(
                    size: CEMTokens.Light.cemTypographySizeL.cemPoints,
                    weight: CEMTokens.Light.cemThicknessBold.cemFontWeight
                ))
                .foregroundStyle(Color(hex: CEMTokens.Light.cemColorCyanXd))

            Button("Primary action") {}
                .buttonStyle(CEMPrimaryButtonStyle())

            VStack(alignment: .leading, spacing: CEMTokens.Light.cemGapRelated.cemPoints) {
                Text("Comfort surface")
                    .font(.system(size: CEMTokens.Light.cemTypographySizeM.cemPoints))
                    .foregroundStyle(Color(hex: CEMTokens.Light.cemColorCyanXd))
                Text("Generated tokens drive color, radius, and spacing.")
                    .font(.system(size: CEMTokens.Light.cemTypographySizeS.cemPoints))
                    .foregroundStyle(Color(hex: CEMTokens.Light.cemColorCyanXd))
            }
            .padding(CEMTokens.Light.cemInsetContainer.cemPoints)
            .background(Color(hex: CEMTokens.Light.cemColorCyanXl))
            .clipShape(RoundedRectangle(cornerRadius: CEMTokens.Light.cemBendSurface.cemPoints))
        }
        .padding(CEMTokens.Light.cemInsetSurface.cemPoints)
        .background(Color(hex: CEMTokens.Light.cemColorCyanXl))
    }
}

struct CEMPrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(
                size: CEMTokens.Light.cemTypographySizeM.cemPoints,
                weight: CEMTokens.Light.cemThicknessBold.cemFontWeight
            ))
            .foregroundStyle(Color(hex: CEMTokens.Light.cemColorBlueXd))
            .padding(.horizontal, CEMTokens.Light.cemInsetContainer.cemPoints)
            .frame(minHeight: CEMTokens.Light.cemCouplingZoneMin.cemPoints)
            .background(Color(hex: CEMTokens.Light.cemColorBlueL))
            .clipShape(RoundedRectangle(cornerRadius: CEMTokens.Light.cemBendControl.cemPoints))
            .opacity(configuration.isPressed ? 0.85 : 1)
    }
}

extension String {
    var cemPoints: CGFloat {
        let normalized = trimmingCharacters(in: .whitespacesAndNewlines)
        if normalized.hasSuffix("rem") {
            return CGFloat(Double(normalized.dropLast(3)) ?? 0) * 16
        }
        if normalized.hasSuffix("px") {
            return CGFloat(Double(normalized.dropLast(2)) ?? 0)
        }
        return CGFloat(Double(normalized) ?? 0)
    }

    var cemFontWeight: Font.Weight {
        switch Int(trimmingCharacters(in: .whitespacesAndNewlines)) ?? 400 {
        case 700...: return .bold
        case 600...: return .semibold
        case 500...: return .medium
        case ..<400: return .light
        default: return .regular
        }
    }
}

extension Color {
    init(hex: String) {
        let normalized = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        let value = UInt64(normalized, radix: 16) ?? 0
        let red = Double((value >> 16) & 0xff) / 255
        let green = Double((value >> 8) & 0xff) / 255
        let blue = Double(value & 0xff) / 255
        self.init(red: red, green: green, blue: blue)
    }
}
