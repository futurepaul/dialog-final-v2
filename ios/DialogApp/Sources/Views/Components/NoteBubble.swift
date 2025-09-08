import SwiftUI

enum BubblePosition {
    case solo
    case top
    case middle
    case bottom
}

struct NoteBubble: View {
    let note: Note
    let position: BubblePosition
    let onTap: () -> Void
    
    @Environment(\.colorScheme) private var colorScheme
    
    private var cornerRadii: (tl: CGFloat, tr: CGFloat, bl: CGFloat, br: CGFloat) {
        let sharp: CGFloat = 8
        let round: CGFloat = 16
        
        switch position {
        case .solo:   return (round, round, round, round)
        case .top:    return (round, round, sharp, sharp)
        case .middle: return (sharp, sharp, sharp, sharp)
        case .bottom: return (sharp, sharp, round, round)
        }
    }
    
    private var bubbleColor: Color {
        // Light gray for both light and dark mode
        Color(UIColor.systemGray6)
    }
    
    private var textColor: Color {
        // Black text for both modes
        .black
    }
    
    var body: some View {
        VStack(alignment: .trailing, spacing: 4) {
            // Show timestamp only for solo or top messages
            if position == .solo || position == .top {
                Text(note.relativeTime)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 4)
            }
            
            VStack(alignment: .leading, spacing: 8) {
                Text(note.text)
                    .foregroundStyle(textColor)
                
                if !note.tags.isEmpty {
                    HStack(spacing: 6) {
                        ForEach(note.displayTags, id: \.self) { tag in
                            Text(tag)
                                .font(.caption)
                                .foregroundStyle(.blue)
                        }
                    }
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                UnevenRoundedRectangle(
                    topLeadingRadius: cornerRadii.tl,
                    bottomLeadingRadius: cornerRadii.bl,
                    bottomTrailingRadius: cornerRadii.br,
                    topTrailingRadius: cornerRadii.tr,
                    style: .continuous
                )
                .fill(bubbleColor)
            )
        }
        .padding(.horizontal)
        .contentShape(Rectangle())
        .onTapGesture(perform: onTap)
        .animation(.snappy(duration: 0.2), value: position)
    }
}

#Preview("Solo Message") {
    NoteBubble(
        note: MockData.sampleNotes[0],
        position: .solo,
        onTap: {}
    )
}

#Preview("Message Group") {
    VStack(spacing: 2) {
        NoteBubble(
            note: MockData.sampleNotes[0],
            position: .top,
            onTap: {}
        )
        NoteBubble(
            note: MockData.sampleNotes[1],
            position: .middle,
            onTap: {}
        )
        NoteBubble(
            note: MockData.sampleNotes[2],
            position: .bottom,
            onTap: {}
        )
    }
}