import Foundation

struct Note: Identifiable, Hashable {
    let id: String  // EventId in hex format
    let text: String
    let tags: [String]
    let createdAt: Date
    
    // UI helpers
    var displayTags: [String] {
        tags.map { "#\($0)" }
    }
    
    var relativeTime: String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: createdAt, relativeTo: Date())
    }
}