import SwiftUI

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
        VStack(alignment: .leading, spacing: 16) {
            Text("CEM Tokens")
                .font(.headline)
                .foregroundStyle(Color(hex: CEMTokens.Light.cemColorCyanXd))

            Button("Primary action") {}
                .buttonStyle(CEMPrimaryButtonStyle())

            VStack(alignment: .leading, spacing: 8) {
                Text("Comfort surface")
                    .font(.body)
                    .foregroundStyle(Color(hex: CEMTokens.Light.cemColorCyanXd))
                Text("Generated tokens drive color, radius, and spacing.")
                    .font(.caption)
                    .foregroundStyle(Color(hex: CEMTokens.Light.cemColorCyanXd))
            }
            .padding(16)
            .background(Color(hex: CEMTokens.Light.cemColorCyanXl))
            .clipShape(RoundedRectangle(cornerRadius: 12))
        }
        .padding(24)
        .background(Color(hex: CEMTokens.Light.cemColorCyanXl))
    }
}

struct CEMPrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.body.weight(.semibold))
            .foregroundStyle(Color(hex: CEMTokens.Light.cemColorBlueXd))
            .padding(.horizontal, 16)
            .frame(minHeight: 44)
            .background(Color(hex: CEMTokens.Light.cemColorBlueL))
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .opacity(configuration.isPressed ? 0.85 : 1)
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
