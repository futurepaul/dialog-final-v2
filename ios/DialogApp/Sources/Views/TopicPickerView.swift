import SwiftUI
import Dialog

struct TopicPickerView: View {
    @Binding var selectedTag: String?
    let allTags: [String]
    let allNotes: [Note]
    let dismiss: () -> Void
    let onTagSelected: (String?) -> Void
    
    @Environment(\.colorScheme) private var colorScheme
    
    private func noteCount(for tag: String?) -> Int {
        if let tag = tag {
            return allNotes.filter { $0.tags.contains(tag) }.count
        } else {
            return allNotes.count
        }
    }
    
    var body: some View {
        NavigationStack {
            List {
                // Inbox (all notes)
                Button {
                    selectedTag = nil
                    onTagSelected(nil)
                    dismiss()
                } label: {
                    HStack {
                        Label("Inbox", systemImage: "tray.2")
                            .foregroundStyle(.primary)
                        Spacer()
                        Text("\(noteCount(for: nil))")
                            .foregroundStyle(.secondary)
                        if selectedTag == nil {
                            Image(systemName: "checkmark")
                                .foregroundStyle(.blue)
                        }
                    }
                }
                .listRowBackground(selectedTag == nil ? Color.blue.opacity(0.1) : Color.clear)
                
                if !allTags.isEmpty {
                    Section("Topics") {
                        ForEach(allTags, id: \.self) { tag in
                            Button {
                                selectedTag = tag
                                onTagSelected(tag)
                                dismiss()
                            } label: {
                                HStack {
                                    Label("#\(tag)", systemImage: "tag")
                                        .foregroundStyle(.primary)
                                    Spacer()
                                    Text("\(noteCount(for: tag))")
                                        .foregroundStyle(.secondary)
                                    if selectedTag == tag {
                                        Image(systemName: "checkmark")
                                            .foregroundStyle(.blue)
                                    }
                                }
                            }
                            .listRowBackground(selectedTag == tag ? Color.blue.opacity(0.1) : Color.clear)
                        }
                    }
                }
            }
            .navigationTitle("Topics")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
    }
}

#Preview {
    TopicPickerView(
        selectedTag: .constant(nil),
        allTags: ["work", "personal", "ideas"],
        allNotes: [],
        dismiss: {},
        onTagSelected: { _ in }
    )
}