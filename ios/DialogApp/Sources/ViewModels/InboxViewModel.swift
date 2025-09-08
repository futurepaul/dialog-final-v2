import SwiftUI
import Combine

@MainActor
class InboxViewModel: ObservableObject {
    @Published var notes: [Note] = MockData.sampleNotes
    @Published var currentTag: String? = nil
    @Published var scrollAnchor: String? = nil
    
    private let userDefaults = UserDefaults.standard
    private let scrollPositionKey = "dialog.scrollPosition"
    
    var displayedNotes: [Note] {
        MockData.notesForTag(currentTag)
            .sorted { $0.createdAt < $1.createdAt }
    }
    
    var navigationTitle: String {
        if let tag = currentTag {
            return "#\(tag)"
        } else {
            return "Inbox"
        }
    }
    
    func bubblePosition(for index: Int) -> BubblePosition {
        guard index >= 0 && index < displayedNotes.count else { return .solo }
        
        let note = displayedNotes[index]
        let hasNext = index + 1 < displayedNotes.count && 
                     isWithinTimeThreshold(note, displayedNotes[index + 1])
        let hasPrev = index > 0 && 
                     isWithinTimeThreshold(displayedNotes[index - 1], note)
        
        switch (hasNext, hasPrev) {
        case (false, false): return .solo
        case (true, false):  return .top
        case (false, true):  return .bottom
        case (true, true):   return .middle
        }
    }
    
    private func isWithinTimeThreshold(_ note1: Note, _ note2: Note) -> Bool {
        abs(note1.createdAt.timeIntervalSince(note2.createdAt)) <= 60
    }
    
    func createNote(text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        
        // Parse tags from text
        let tags = parseHashtags(from: trimmed)
        
        // Generate a random hex string for the ID (64 chars like EventId)
        let id = (0..<32).map { _ in 
            String(format: "%02x", Int.random(in: 0...255))
        }.joined()
        
        // Create new note
        let newNote = Note(
            id: id,
            text: trimmed,
            tags: tags,
            createdAt: Date()
        )
        
        // Add with animation
        withAnimation(.snappy(duration: 0.3)) {
            notes.append(newNote)
        }
    }
    
    private func parseHashtags(from text: String) -> [String] {
        text.split(separator: " ")
            .compactMap { word in
                if word.hasPrefix("#") && word.count > 1 {
                    let tag = String(word.dropFirst())
                    // Remove any trailing punctuation
                    let cleaned = tag.trimmingCharacters(in: .punctuationCharacters)
                    return cleaned.lowercased()
                }
                return nil
            }
    }
    
    func selectNote(_ note: Note) {
        // Future: Show note detail view
        print("Selected note: \(note.id)")
    }
    
    func saveScrollPosition(for noteId: String?) {
        if let noteId = noteId {
            let key = scrollPositionKey + (currentTag ?? "inbox")
            userDefaults.set(noteId, forKey: key)
        }
    }
    
    func getLastScrollPosition() -> String? {
        let key = scrollPositionKey + (currentTag ?? "inbox")
        return userDefaults.string(forKey: key)
    }
}