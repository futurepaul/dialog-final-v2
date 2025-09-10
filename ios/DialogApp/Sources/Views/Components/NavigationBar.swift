import SwiftUI

struct NavigationBar: View {
    @Binding var showingTopicPicker: Bool
    let currentTag: String?
    let onSearchTapped: () -> Void
    
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
            
            // Right: Search button
            Button(action: { onSearchTapped() }) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(colorScheme == .dark ? .white : .black)
            }
            .disabled(false)
            .opacity(1)
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
