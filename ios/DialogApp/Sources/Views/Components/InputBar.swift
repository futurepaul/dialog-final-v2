import SwiftUI

struct InputBar: View {
    @Binding var text: String
    let onSend: () -> Void
    let isEnabled: Bool
    
    @FocusState private var isInputFocused: Bool
    @Environment(\.colorScheme) private var colorScheme
    
    private var buttonColor: Color {
        isEnabled ? .blue : Color(UIColor.systemGray3)
    }
    
    var body: some View {
        HStack(spacing: 12) {
            TextField("What's on your mind?", text: $text, axis: .vertical)
                .textFieldStyle(.plain)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(Color(.secondarySystemBackground))
                .cornerRadius(20)
                .focused($isInputFocused)
                .onSubmit {
                    if isEnabled {
                        onSend()
                    }
                }
                .lineLimit(1...5)
            
            Button(action: onSend) {
                Image(systemName: "arrow.up.circle.fill")
                    .font(.system(size: 32))
                    .foregroundColor(buttonColor)
            }
            .disabled(!isEnabled)
            .animation(.snappy(duration: 0.2), value: isEnabled)
        }
        .padding()
    }
}