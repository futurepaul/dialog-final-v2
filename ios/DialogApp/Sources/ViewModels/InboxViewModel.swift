import SwiftUI
import Combine
import Dialog

// ViewModel using fire-and-forget pattern
@MainActor
class InboxViewModel: ObservableObject {
    @Published var notes: [Note] = []
    @Published var currentTag: String? = nil
    @Published var allTags: [String] = []
    @Published var tagCounts: [String: Int] = [:]
    @Published var isLoading = false
    @Published var errorMessage: String?
    @Published var npub: String = ""
    
    private let client: DialogClient
    private(set) var nsecInUse: String = ""
    
    private let userDefaults = UserDefaults.standard
    private let scrollPositionKey = "dialog.scrollPosition"
    
    init() {
        // Read nsec from environment for development (set in Xcode scheme)
        let env = ProcessInfo.processInfo.environment
        if let data = KeychainService.read(key: "nsec"), let key = String(data: data, encoding: .utf8), !key.isEmpty {
            self.client = DialogClient(nsec: key)
            self.nsecInUse = key
        } else if let nsec = env["DIALOG_NSEC"], !nsec.isEmpty {
            self.client = DialogClient(nsec: nsec)
            self.nsecInUse = nsec
            _ = KeychainService.save(key: "nsec", data: Data(nsec.utf8))
        } else {
            // Auto-generate a new nsec on first run
            let helper = KeysHelper()
            let nsec = helper.generateNsec()
            self.client = DialogClient(nsec: nsec)
            self.nsecInUse = nsec
            _ = KeychainService.save(key: "nsec", data: Data(nsec.utf8))
        }
        // Derive npub once, driven by current nsec
        self.npub = client.deriveNpub(nsec: self.nsecInUse)
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
        // Use UserDefaults relay if set, else default per plan
        let relay = UserDefaults.standard.string(forKey: "DIALOG_RELAY") ?? "wss://relay.damus.io"
        client.sendCommand(cmd: Command.connectRelay(relayUrl: relay))
        
        // Get initial data (synchronous queries)
        print("[swift] getNotes/getAllTags (sync) before events")
        self.notes = client.getNotes(limit: 100, tag: currentTag)
        self.allTags = client.getAllTags()
        self.refreshTagCounts()
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
            // Always compute tags across ALL cached notes, not filtered view
            self.allTags = client.getAllTags()
            self.refreshTagCounts()
            self.isLoading = false
            
        case .noteAdded(let note):
            if let idx = self.notes.firstIndex(where: { $0.id == note.id }) {
                self.notes[idx] = note
            } else {
                self.notes.append(note)
            }
            self.notes.sort { $0.createdAt < $1.createdAt }
            // Refresh full tag list from client cache
            self.allTags = client.getAllTags()
            self.refreshTagCounts()
            
        case .noteUpdated(let note):
            if let index = notes.firstIndex(where: { $0.id == note.id }) {
                notes[index] = note
            }
            // Refresh full tag list from client cache
            self.allTags = client.getAllTags()
            self.refreshTagCounts()
            
        case .noteDeleted(let id):
            notes.removeAll { $0.id == id }
            self.refreshTagCounts()
            
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
        print("[swift] createNote -> sendCommand .createNote text='\(trimmed)' len=\(trimmed.count)")
        client.sendCommand(cmd: Command.createNote(text: trimmed))
    }
    
    func setTagFilter(_ tag: String?) {
        // Fire-and-forget command
        client.sendCommand(cmd: Command.setTagFilter(tag: tag))
    }
    
    func search(_ query: String) {
        client.sendCommand(cmd: Command.searchNotes(query: query))
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
    
    func fetchAllNotesSnapshot() -> [Note] {
        client.getNotes(limit: 1000, tag: nil)
    }
    
    func refreshTagCounts() {
        var map: [String: Int] = [:]
        for tc in client.getTagCounts() {
            map[tc.tag] = Int(tc.count)
        }
        self.tagCounts = map
    }
    
    // Settings helpers
    func connectRelay(_ url: String) {
        UserDefaults.standard.set(url, forKey: "DIALOG_RELAY")
        client.sendCommand(cmd: Command.connectRelay(relayUrl: url))
    }
    
    func clearData() {
        client.clearDataForCurrentPubkey()
    }
    
    func validate(nsec: String) -> Bool { client.validateNsec(nsec: nsec) }
    
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
