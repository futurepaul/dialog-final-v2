# Frontend-to-Backend Implementation Plan

## Overview
Build the Dialog iOS app starting from the UI layer with mock data, progressively integrating UniFFI bindings, and finally connecting to the real `dialog_lib` Rust backend. This approach ensures a polished UI from day one while avoiding merge conflicts.

**Core Principle**: Start with the user experience, work backwards to the implementation.

---

## Phase 1: XcodeGen Setup & Project Structure (Day 1 Morning) ✅ COMPLETED

### 1.1 Initial Setup
```bash
# Install XcodeGen if not already installed
brew install xcodegen

# Create project structure
mkdir -p DialogApp/{Sources,Resources,Preview}
mkdir -p DialogApp/Sources/{Models,Views,ViewModels,Utils}
mkdir -p DialogApp/Resources/Assets.xcassets
```

### 1.2 Create project.yml
```yaml
name: DialogApp
options:
  bundleIdPrefix: com.dialog
  deploymentTarget:
    iOS: 18.0
  developmentLanguage: en
  xcodeVersion: "15.0"
  createIntermediateGroups: true
  groupSortPosition: top
  generateEmptyDirectories: true
  useBaseInternationalization: false

settings:
  SWIFT_VERSION: 6.0
  DEVELOPMENT_TEAM: ${DEVELOPMENT_TEAM}
  ENABLE_PREVIEWS: YES
  ENABLE_USER_SCRIPT_SANDBOXING: NO

targets:
  DialogApp:
    type: application
    platform: iOS
    sources: 
      - path: DialogApp/Sources
        name: Sources
        createIntermediateGroups: true
      - path: DialogApp/Resources
        name: Resources
        createIntermediateGroups: true
    settings:
      PRODUCT_BUNDLE_IDENTIFIER: com.dialog.app
      INFOPLIST_FILE: DialogApp/Resources/Info.plist
      ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon
      SWIFT_STRICT_CONCURRENCY: complete
      SWIFT_UPCOMING_FEATURE_EXISTENTIAL_ANY: YES
    preBuildScripts:
      - name: "SwiftLint"
        script: |
          if which swiftlint >/dev/null; then
            swiftlint
          else
            echo "warning: SwiftLint not installed"
          fi
        basedOnDependencyAnalysis: false
    scheme:
      testTargets:
        - DialogAppTests
      gatherCoverageData: true
      commandLineArguments:
        "-com.apple.CoreData.SQLDebug 1": false

  DialogAppTests:
    type: bundle.unit-test
    platform: iOS
    sources: DialogAppTests
    dependencies:
      - target: DialogApp
```

### 1.3 Build Scripts
**build-ios.sh**:
```bash
#!/bin/bash
set -e

echo "🔨 Building Dialog iOS App..."

# Clean previous builds
./clean-ios.sh

# Generate Xcode project
echo "📝 Generating Xcode project..."
xcodegen generate --spec project.yml --use-cache

# Build for simulator
echo "📱 Building for iOS Simulator..."
xcodebuild -project DialogApp.xcodeproj \
  -scheme DialogApp \
  -destination 'platform=iOS Simulator,name=iPhone 15 Pro' \
  build

echo "✅ Build complete!"
```

**clean-ios.sh**:
```bash
#!/bin/bash
echo "🧹 Cleaning iOS build artifacts..."
rm -rf DialogApp.xcodeproj
rm -rf ~/Library/Developer/Xcode/DerivedData/DialogApp-*
rm -rf .xcodegen_cache
echo "✨ Clean complete!"
```

---

## Phase 2: Mock Data Models & UI Foundation (Day 1 Afternoon - Day 2) ✅ COMPLETED

### 2.1 Swift Data Models (Matching Rust `Note` struct)
**DialogApp/Sources/Models/Note.swift**:
```swift
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
        // Format as "2m ago", "1h ago", etc.
        RelativeDateFormatter.shared.string(from: createdAt)
    }
}

// Mock data generator
struct MockData {
    static let sampleNotes: [Note] = [
        // Work topic cluster
        Note(id: "abc123", text: "Need to review the Q4 roadmap before tomorrow's meeting", 
             tags: ["work", "planning"], createdAt: Date().addingTimeInterval(-120)),
        Note(id: "abc124", text: "Actually, let's push the deadline to Friday", 
             tags: ["work"], createdAt: Date().addingTimeInterval(-60)),
        
        // Personal cluster
        Note(id: "def456", text: "Remember to call mom about thanksgiving plans", 
             tags: ["personal", "family"], createdAt: Date().addingTimeInterval(-3600)),
        Note(id: "def457", text: "Also need to book flights ✈️", 
             tags: ["personal", "travel"], createdAt: Date().addingTimeInterval(-3540)),
        
        // Ideas topic
        Note(id: "ghi789", text: "App idea: AI that suggests recipes based on what's in your fridge. Could use vision API to scan items, then GPT to suggest combinations. Maybe partner with grocery delivery services?", 
             tags: ["ideas", "ai", "startup"], createdAt: Date().addingTimeInterval(-7200)),
        
        // Mixed
        Note(id: "jkl012", text: "The new coffee shop on 5th street is amazing! Great wifi too", 
             tags: ["coffee", "places"], createdAt: Date().addingTimeInterval(-86400)),
        Note(id: "mno345", text: "Maybe we should have our 1:1s there instead of the office", 
             tags: ["work", "coffee"], createdAt: Date().addingTimeInterval(-86340))
    ]
    
    static func notesForTag(_ tag: String?) -> [Note] {
        guard let tag = tag else { return sampleNotes }
        return sampleNotes.filter { $0.tags.contains(tag) }
    }
    
    static var allTags: [String] {
        Array(Set(sampleNotes.flatMap { $0.tags })).sorted()
    }
}
```

### 2.2 Core UI Components

**DialogApp/Sources/Views/Components/NoteBubble.swift**:
```swift
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
                    .foregroundStyle(.primary)
                
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
            .background(
                UnevenRoundedRectangle(
                    topLeadingRadius: cornerRadii.tl,
                    bottomLeadingRadius: cornerRadii.bl,
                    bottomTrailingRadius: cornerRadii.br,
                    topTrailingRadius: cornerRadii.tr,
                    style: .continuous
                )
                .fill(colorScheme == .dark ? Color(.secondarySystemBackground) : .black)
            )
            .foregroundStyle(colorScheme == .dark ? .primary : .white)
        }
        .padding(.horizontal)
        .contentShape(Rectangle())
        .onTapGesture(perform: onTap)
        .animation(.snappy(duration: 0.2), value: position)
    }
}
```

**DialogApp/Sources/Views/InboxView.swift**:
```swift
import SwiftUI

struct InboxView: View {
    @StateObject private var viewModel = InboxViewModel()
    @State private var messageText = ""
    @State private var showingTopicPicker = false
    @FocusState private var isInputFocused: Bool
    
    var body: some View {
        ZStack {
            Color(.systemBackground)
                .ignoresSafeArea()
            
            VStack(spacing: 0) {
                // Navigation bar
                NavigationBar(
                    showingTopicPicker: $showingTopicPicker,
                    currentTag: viewModel.currentTag
                )
                
                // Messages list
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(spacing: 2) {
                            ForEach(Array(viewModel.displayedNotes.enumerated()), id: \.element.id) { index, note in
                                NoteBubble(
                                    note: note,
                                    position: viewModel.bubblePosition(for: index),
                                    onTap: { viewModel.selectNote(note) }
                                )
                                .id(note.id)
                            }
                        }
                        .padding(.vertical)
                    }
                    .scrollDismissesKeyboard(.interactively)
                    .onAppear {
                        viewModel.restoreScrollPosition(using: proxy)
                    }
                    .onChange(of: viewModel.displayedNotes) { _ in
                        viewModel.saveScrollPosition(proxy: proxy)
                    }
                }
                
                // Input bar
                InputBar(
                    text: $messageText,
                    onSend: {
                        viewModel.createNote(text: messageText)
                        messageText = ""
                    },
                    isEnabled: !messageText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                )
            }
        }
        .sheet(isPresented: $showingTopicPicker) {
            TopicPickerView(selectedTag: $viewModel.currentTag)
        }
    }
}
```

### 2.3 ViewModels with State Management

**DialogApp/Sources/ViewModels/InboxViewModel.swift**:
```swift
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
        
        // Create new note with unique ID
        let newNote = Note(
            id: UUID().uuidString,
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
            .filter { $0.hasPrefix("#") && $0.count > 1 }
            .map { String($0.dropFirst()).lowercased() }
    }
    
    func selectNote(_ note: Note) {
        // Future: Show note detail view
        print("Selected note: \(note.id)")
    }
    
    func saveScrollPosition(proxy: ScrollViewProxy) {
        if let lastNote = displayedNotes.last {
            userDefaults.set(lastNote.id, forKey: scrollPositionKey)
        }
    }
    
    func restoreScrollPosition(using proxy: ScrollViewProxy) {
        if let savedId = userDefaults.string(forKey: scrollPositionKey),
           displayedNotes.contains(where: { $0.id == savedId }) {
            proxy.scrollTo(savedId, anchor: .bottom)
        } else if let lastNote = displayedNotes.last {
            proxy.scrollTo(lastNote.id, anchor: .bottom)
        }
    }
}
```

---

## Phase 3: Topic Management & Navigation (Day 3)

### 3.1 Topic Picker View
**DialogApp/Sources/Views/TopicPickerView.swift**:
```swift
struct TopicPickerView: View {
    @Binding var selectedTag: String?
    @Environment(\.dismiss) private var dismiss
    
    private let allTags = MockData.allTags
    
    var body: some View {
        NavigationStack {
            List {
                // Inbox (all notes)
                Button {
                    selectedTag = nil
                    dismiss()
                } label: {
                    HStack {
                        Label("Inbox", systemImage: "tray")
                        Spacer()
                        if selectedTag == nil {
                            Image(systemName: "checkmark")
                                .foregroundStyle(.blue)
                        }
                    }
                }
                .foregroundStyle(.primary)
                
                Section("Topics") {
                    ForEach(allTags, id: \.self) { tag in
                        Button {
                            selectedTag = tag
                            dismiss()
                        } label: {
                            HStack {
                                Label("#\(tag)", systemImage: "tag")
                                Spacer()
                                Text("\(noteCount(for: tag))")
                                    .foregroundStyle(.secondary)
                                if selectedTag == tag {
                                    Image(systemName: "checkmark")
                                        .foregroundStyle(.blue)
                                }
                            }
                        }
                        .foregroundStyle(.primary)
                    }
                }
            }
            .navigationTitle("Topics")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }
    
    private func noteCount(for tag: String) -> Int {
        MockData.notesForTag(tag).count
    }
}
```

### 3.2 Search Functionality
**DialogApp/Sources/Views/SearchView.swift**:
```swift
struct SearchView: View {
    @State private var searchText = ""
    @State private var searchResults: [Note] = []
    
    var body: some View {
        NavigationStack {
            List(searchResults) { note in
                VStack(alignment: .leading, spacing: 4) {
                    Text(note.text)
                        .lineLimit(2)
                    HStack {
                        Text(note.relativeTime)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Spacer()
                        HStack(spacing: 4) {
                            ForEach(note.displayTags, id: \.self) { tag in
                                Text(tag)
                                    .font(.caption)
                                    .foregroundStyle(.blue)
                            }
                        }
                    }
                }
                .padding(.vertical, 4)
            }
            .searchable(text: $searchText, prompt: "Search notes")
            .onChange(of: searchText) { _, newValue in
                performSearch(newValue)
            }
            .navigationTitle("Search")
        }
    }
    
    private func performSearch(_ query: String) {
        guard !query.isEmpty else {
            searchResults = []
            return
        }
        
        searchResults = MockData.sampleNotes.filter { note in
            note.text.localizedCaseInsensitiveContains(query) ||
            note.tags.contains { $0.localizedCaseInsensitiveContains(query) }
        }
    }
}
```

---

## Phase 4: UniFFI Integration Preparation (Day 4)

### 4.1 Protocol Definitions for Future Integration
**DialogApp/Sources/Services/DialogProtocol.swift**:
```swift
import Foundation

// Protocol that mock and real implementations will conform to
protocol DialogServiceProtocol: ObservableObject {
    func createNote(text: String) async throws -> String
    func listNotes(limit: Int, tags: [String]) async throws -> [Note]
    func watchNotes(limit: Int, tags: [String], callback: @escaping (Note) -> Void)
}

// Mock implementation for UI development
class MockDialogService: DialogServiceProtocol {
    func createNote(text: String) async throws -> String {
        // Simulate network delay
        try await Task.sleep(nanoseconds: 100_000_000) // 0.1s
        return UUID().uuidString
    }
    
    func listNotes(limit: Int, tags: [String]) async throws -> [Note] {
        try await Task.sleep(nanoseconds: 200_000_000) // 0.2s
        return MockData.notesForTag(tags.first)
    }
    
    func watchNotes(limit: Int, tags: [String], callback: @escaping (Note) -> Void) {
        // Simulate incoming notes
        Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            if Bool.random() {
                let randomNote = MockData.sampleNotes.randomElement()!
                callback(randomNote)
            }
        }
    }
}
```

### 4.2 App Structure for Dependency Injection
**DialogApp/Sources/DialogApp.swift**:
```swift
import SwiftUI

@main
struct DialogApp: App {
    @StateObject private var dialogService = MockDialogService()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(dialogService)
        }
    }
}
```

---

## Phase 5: UniFFI Bridge Implementation (Day 5-6)

### 5.1 Swift Wrapper for UniFFI
**DialogApp/Sources/Services/DialogFFIService.swift**:
```swift
import Foundation
// import DialogFFI // Generated UniFFI module

class DialogFFIService: DialogServiceProtocol {
    private var ffiDialog: FfiDialog?
    
    init() async throws {
        // Initialize with nsec from Keychain
        let nsec = try await KeychainManager.shared.getNsec()
        self.ffiDialog = try await dialogNew(opts: FfiDialogOptions(nsec: nsec))
    }
    
    func createNote(text: String) async throws -> String {
        guard let dialog = ffiDialog else { throw DialogError.notInitialized }
        
        let tags = parseHashtags(from: text)
        return try await dialogCreateNote(dialog: dialog, text: text, tags: tags)
    }
    
    func listNotes(limit: Int, tags: [String]) async throws -> [Note] {
        guard let dialog = ffiDialog else { throw DialogError.notInitialized }
        
        let ffiNotes = try await dialogListNotes(
            dialog: dialog, 
            limit: UInt32(limit), 
            tags: tags
        )
        
        return ffiNotes.map { ffiNote in
            Note(
                id: ffiNote.id,
                text: ffiNote.text,
                tags: ffiNote.tags,
                createdAt: Date(timeIntervalSince1970: TimeInterval(ffiNote.createdAt))
            )
        }
    }
    
    func watchNotes(limit: Int, tags: [String], callback: @escaping (Note) -> Void) {
        guard let dialog = ffiDialog else { return }
        
        Task {
            await dialogWatchNotes(
                dialog: dialog,
                limit: UInt32(limit),
                tags: tags,
                callback: { ffiNote in
                    let note = Note(
                        id: ffiNote.id,
                        text: ffiNote.text,
                        tags: ffiNote.tags,
                        createdAt: Date(timeIntervalSince1970: TimeInterval(ffiNote.createdAt))
                    )
                    
                    Task { @MainActor in
                        callback(note)
                    }
                }
            )
        }
    }
    
    private func parseHashtags(from text: String) -> [String] {
        text.split(separator: " ")
            .filter { $0.hasPrefix("#") && $0.count > 1 }
            .map { String($0.dropFirst()).lowercased() }
    }
}
```

### 5.2 Build Script Updates for UniFFI
**build-uniffi.sh**:
```bash
#!/bin/bash
set -e

echo "🦀 Building UniFFI bindings..."

# Build Rust library for iOS targets
cargo build --release --target aarch64-apple-ios
cargo build --release --target x86_64-apple-ios
cargo build --release --target aarch64-apple-ios-sim

# Generate Swift bindings
cargo run --bin uniffi-bindgen generate \
  --library target/aarch64-apple-ios/release/libdialog_uniffi.dylib \
  --language swift \
  --out-dir DialogApp/Generated

# Create XCFramework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libdialog_uniffi.a \
  -headers DialogApp/Generated/dialogFFI.h \
  -library target/x86_64-apple-ios/release/libdialog_uniffi.a \
  -headers DialogApp/Generated/dialogFFI.h \
  -library target/aarch64-apple-ios-sim/release/libdialog_uniffi.a \
  -headers DialogApp/Generated/dialogFFI.h \
  -output DialogApp/Frameworks/DialogFFI.xcframework

echo "✅ UniFFI build complete!"
```

---

## Phase 6: Real Backend Integration (Day 7-8)

### 6.1 Update App to Use Real Service
```swift
@main
struct DialogApp: App {
    @StateObject private var dialogService: any DialogServiceProtocol
    
    init() {
        // Switch between mock and real based on build configuration
        #if DEBUG && MOCK_DATA
        _dialogService = StateObject(wrappedValue: MockDialogService())
        #else
        _dialogService = StateObject(wrappedValue: DialogFFIService())
        #endif
    }
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(dialogService)
        }
    }
}
```

### 6.2 Update ViewModels to Use Service
```swift
@MainActor
class InboxViewModel: ObservableObject {
    @Published var notes: [Note] = []
    @Published var currentTag: String? = nil
    
    private let dialogService: any DialogServiceProtocol
    
    init(dialogService: any DialogServiceProtocol) {
        self.dialogService = dialogService
        loadNotes()
        startWatching()
    }
    
    private func loadNotes() {
        Task {
            do {
                let tags = currentTag.map { [$0] } ?? []
                notes = try await dialogService.listNotes(limit: 50, tags: tags)
            } catch {
                print("Failed to load notes: \(error)")
            }
        }
    }
    
    private func startWatching() {
        let tags = currentTag.map { [$0] } ?? []
        dialogService.watchNotes(limit: 50, tags: tags) { [weak self] note in
            Task { @MainActor in
                self?.notes.append(note)
            }
        }
    }
    
    func createNote(text: String) {
        Task {
            do {
                let noteId = try await dialogService.createNote(text: text)
                print("Created note: \(noteId)")
                // Note will appear via watch callback
            } catch {
                print("Failed to create note: \(error)")
            }
        }
    }
}
```

---

## Testing Strategy

### UI Testing with Mock Data
1. Test all bubble positions render correctly
2. Test smooth scrolling and animations
3. Test topic filtering works
4. Test search functionality
5. Verify scroll position persistence

### Integration Testing Checkpoints
1. **After Phase 2**: Full UI working with mock data
2. **After Phase 4**: Mock service protocol working
3. **After Phase 5**: UniFFI bindings compile and link
4. **After Phase 6**: Real data flowing through app

---

## Key Decisions & Rationale

### Why Frontend-First?
- Immediate visual feedback for development
- Can iterate on UX without backend delays
- Parallel development possible (UI team vs Rust team)
- Easier to demo and get feedback early

### Why XcodeGen from Start?
- No merge conflicts in .xcodeproj files
- Reproducible builds across team
- Easy to add/remove files
- Works well with CI/CD

### Why Simple Value Types?
- No CoreData complexity
- Easy to bridge with UniFFI
- Immutable by default (safer)
- Better for SwiftUI performance

### Why Protocol-Based Service Layer?
- Easy to swap mock/real implementations
- Testable ViewModels
- Clear separation of concerns
- Smooth transition from mock to real data

---

## Success Metrics

### Phase 1-2 (UI with Mock Data) ✅ COMPLETED
✅ App launches and shows mock notes
✅ Smooth animations under 16ms frame time
✅ Topic filtering works instantly
✅ Scroll position persists between launches
✅ XcodeGen project generation working
✅ Environment variables for team ID
✅ Clean file organization in ios/ folder
✅ Beautiful UI with light gray bubbles and black text

### Phase 3-4 (Service Layer)
✅ Mock service provides realistic delays
✅ ViewModels are testable
✅ No UI changes needed for service swap

### Phase 5-6 (Real Integration)
✅ UniFFI bindings compile without warnings
✅ Real notes appear in UI
✅ Encryption/decryption works
✅ Offline-first: works without network

---

## Timeline Summary

- **Day 1**: XcodeGen setup + Basic UI components
- **Day 2**: Complete mock UI with all views
- **Day 3**: Topic management and search
- **Day 4**: Service protocol layer
- **Day 5-6**: UniFFI integration
- **Day 7-8**: Real backend connection
- **Day 9**: Testing and polish
- **Day 10**: Documentation and handoff

This plan ensures we have a working, polished UI from day one while systematically building towards the full Rust backend integration.