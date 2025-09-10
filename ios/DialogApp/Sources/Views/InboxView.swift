import SwiftUI
import Dialog

struct InboxView: View {
    @StateObject private var viewModel = InboxViewModel()
    @State private var messageText = ""
    @State private var showingTopicPicker = false
    @State private var showingSettings = false
    @State private var showingSearch = false
    @FocusState private var isInputFocused: Bool
    @State private var lastVisibleNoteId: String?
    
    var body: some View {
        ZStack {
            Color(.systemBackground)
                .ignoresSafeArea()
            
            VStack(spacing: 0) {
                // Navigation bar
                NavigationBar(
                    showingTopicPicker: $showingTopicPicker,
                    currentTag: viewModel.currentTag,
                    onSearchTapped: { showingSearch = true }
                )
                
                // Messages list
                ScrollViewReader { proxy in
                    ScrollView {
                        if viewModel.displayedNotes.isEmpty {
                            VStack(alignment: .leading, spacing: 12) {
                                Text("Welcome!")
                                    .font(.title2).bold()
                                Text("Start writing notes. Everything is encrypted and saved to the cloud.")
                                Text("If you want to connect other devices or use an existing account type \"/setup\"")
                                    .foregroundStyle(.secondary)
                            }
                            .padding()
                        }
                        // No inline search; search lives in a separate sheet
                        LazyVStack(spacing: 2) {
                            ForEach(Array(viewModel.displayedNotes.enumerated()), id: \.element.id) { index, note in
                                NoteBubble(
                                    note: note,
                                    position: viewModel.bubblePosition(for: index),
                                    onTap: { viewModel.selectNote(note) }
                                )
                                .id(note.id)
                                .onAppear {
                                    // Track the last visible note for scroll position
                                    lastVisibleNoteId = note.id
                                }
                            }
                        }
                        .padding(.vertical)
                    }
                    .scrollDismissesKeyboard(.interactively)
                    .onAppear {
                        // Start the DialogClient listener
                        viewModel.start()
                        
                        // Restore scroll position or scroll to bottom
                        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                            if let savedId = viewModel.getLastScrollPosition(),
                               viewModel.displayedNotes.contains(where: { $0.id == savedId }) {
                                withAnimation {
                                    proxy.scrollTo(savedId, anchor: .bottom)
                                }
                            } else if let lastNote = viewModel.displayedNotes.last {
                                withAnimation {
                                    proxy.scrollTo(lastNote.id, anchor: .bottom)
                                }
                            }
                        }
                    }
                    .onDisappear {
                        // Stop the DialogClient listener
                        viewModel.stop()
                    }
                    .onChange(of: viewModel.displayedNotes.count) { oldCount, newCount in
                        // Scroll to new message if we added one
                        if newCount > oldCount, let lastNote = viewModel.displayedNotes.last {
                            withAnimation {
                                proxy.scrollTo(lastNote.id, anchor: .bottom)
                            }
                        }
                    }
                    .onDisappear {
                        // Save scroll position when view disappears
                        viewModel.saveScrollPosition(for: lastVisibleNoteId)
                    }
                }
                
                // Input bar
                InputBar(
                    text: $messageText,
                    onSend: {
                        let trimmed = messageText.trimmingCharacters(in: .whitespacesAndNewlines)
                        if trimmed == "/setup" {
                            showingSettings = true
                        } else {
                            viewModel.createNote(text: trimmed)
                        }
                        messageText = ""
                    },
                    isEnabled: !messageText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                )
            }
        }
        .sheet(isPresented: $showingTopicPicker) {
            TopicPickerView(
                selectedTag: $viewModel.currentTag,
                allTags: viewModel.allTags,
                tagCounts: viewModel.tagCounts,
                dismiss: { showingTopicPicker = false },
                onTagSelected: { tag in
                    viewModel.setTagFilter(tag)
                },
                onShowSettings: { showingSettings = true }
            )
        }
        .sheet(isPresented: $showingSettings) {
            SettingsView(viewModel: viewModel, dismiss: { showingSettings = false })
        }
        .sheet(isPresented: $showingSearch) {
            SearchSheet(initialNotes: viewModel.fetchAllNotesSnapshot(), dismiss: { showingSearch = false })
        }
    }
}

#Preview {
    InboxView()
}
