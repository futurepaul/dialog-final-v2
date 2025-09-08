import SwiftUI

struct NavigationBar: View {
    @Binding var showingTopicPicker: Bool
    let currentTag: String?
    
    @Environment(\.colorScheme) private var colorScheme
    
    var body: some View {
        HStack {
            // Left: Topic button
            Button(action: { showingTopicPicker = true }) {
                HStack(spacing: 4) {
                    Image(systemName: "tray.2")
                    Text(navigationTitle)
                        .fontWeight(.semibold)
                }
                .foregroundStyle(colorScheme == .dark ? .white : .black)
            }
            
            Spacer()
            
            // Right: Search button (future)
            Button(action: {}) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(colorScheme == .dark ? .white : .black)
            }
            .disabled(true)
            .opacity(0.3)
        }
        .padding()
        .frame(height: 60)
        .background(.ultraThinMaterial)
    }
    
    private var navigationTitle: String {
        if let tag = currentTag {
            return "#\(tag)"
        } else {
            return "Inbox"
        }
    }
}