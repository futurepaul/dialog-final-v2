import SwiftUI
import Combine
import Dialog

// ViewModel using fire-and-forget pattern
@MainActor
class InboxViewModel: ObservableObject {
    @Published var notes: [Note] = []
    @Published var currentTag: String? = nil
    @Published var allTags: [String] = []
    @Published var isLoading = false
    @Published var errorMessage: String?
    
    private let client: DialogClient
    
    private let userDefaults = UserDefaults.standard
    private let scrollPositionKey = "dialog.scrollPosition"
    
    init() {
        // For testing - in production this would come from secure storage
        let testNsec = "nsec1ufnus6pju578ste3v90xd5m2decpuzpql2295m3sknqcjzyys9ls0qlc85"
        self.client = DialogClient(nsec: testNsec)
    }
    
    var displayedNotes: [Note] {
        notes.sorted { $0.createdAt < $1.createdAt }
    }
    
    var navigationTitle: String {
        if let tag = currentTag {
            return "#\(tag)"
        } else {
            return "Inbox"
        }
    }
    
    func start() {
        print("[swift] start() called")
        // Create listener to receive events from Rust
        let listener = SwiftDialogListener { [weak self] event in
            Task { @MainActor in
                print("[swift] onEvent ->", String(describing: event))
                self?.handleEvent(event)
            }
        }
        
        // Start the client with the listener (fire-and-forget)
        client.start(listener: listener)
        
        // Connect to a relay so create/list/watch work
        // Hardcode relay for reliability during development
        client.sendCommand(cmd: Command.connectRelay(relayUrl: "wss://nos.lol"))
        
        // Get initial data (synchronous queries)
        print("[swift] getNotes/getAllTags (sync) before events")
        self.notes = client.getNotes(limit: 100, tag: currentTag)
        self.allTags = client.getAllTags()
        print("[swift] initial notes count", self.notes.count)
    }
    
    func stop() {
        client.stop()
    }
    
    private func handleEvent(_ event: Event) {
        switch event {
        case .ready:
            // Dialog is ready
            break
            
        case .notesLoaded(let notes):
            // Deduplicate by id in case of repeats
            var unique: [String: Note] = [:]
            for n in notes { unique[n.id] = n }
            self.notes = Array(unique.values)
            self.isLoading = false
            
        case .noteAdded(let note):
            if let idx = self.notes.firstIndex(where: { $0.id == note.id }) {
                self.notes[idx] = note
            } else {
                self.notes.append(note)
            }
            self.notes.sort { $0.createdAt < $1.createdAt }
            
        case .noteUpdated(let note):
            if let index = notes.firstIndex(where: { $0.id == note.id }) {
                notes[index] = note
            }
            
        case .noteDeleted(let id):
            notes.removeAll { $0.id == id }
            
        case .tagFilterChanged(let tag):
            self.currentTag = tag
            
        case .syncStatusChanged(let syncing):
            self.isLoading = syncing
            
        case .error(let message):
            self.errorMessage = message
        }
    }
    
    func createNote(text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        
        // Fire-and-forget command
        print("[swift] createNote -> sendCommand .createNote len=\(trimmed.count)")
        client.sendCommand(cmd: Command.createNote(text: trimmed))
    }
    
    func setTagFilter(_ tag: String?) {
        // Fire-and-forget command
        client.sendCommand(cmd: Command.setTagFilter(tag: tag))
    }
    
    func markAsRead(_ noteId: String) {
        // Fire-and-forget command
        client.sendCommand(cmd: Command.markAsRead(id: noteId))
    }
    
    func selectNote(_ note: Note) {
        // Mark as read when selected
        markAsRead(note.id)
        print("Selected note: \(note.id)")
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
        // createdAt is now a UInt64 timestamp in seconds
        abs(Int(note1.createdAt) - Int(note2.createdAt)) <= 60
    }
    
    var unreadCount: Int {
        Int(client.getUnreadCount(tag: currentTag))
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

// Helper class to implement DialogListener protocol
final class SwiftDialogListener: DialogListener, @unchecked Sendable {
    private let onEventCallback: @Sendable (Event) -> Void
    
    init(onEvent: @escaping @Sendable (Event) -> Void) {
        self.onEventCallback = onEvent
    }
    
    func onEvent(event: Event) {
        onEventCallback(event)
    }
}
