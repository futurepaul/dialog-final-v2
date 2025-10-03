import SwiftUI
import Dialog

struct TopicPickerView: View {
    @Binding var selectedTag: String?
    let allTags: [String]
    let tagCounts: [String: Int]
    let dismiss: () -> Void
    let onTagSelected: (String?) -> Void
    let onShowSettings: () -> Void
    
    @Environment(\.colorScheme) private var colorScheme
    
    private func countForTag(_ tag: String?) -> Int {
        if let tag = tag { return tagCounts[tag] ?? 0 }
        // Inbox count is sum of all notes; if tagCounts empty, show 0
        return tagCounts.values.reduce(0, +)
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
                        Text("\(countForTag(nil))")
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
                                    Text("\(countForTag(tag))")
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
                Section {
                    Button {
                        dismiss()
                        onShowSettings()
                    } label: {
                        HStack {
                            Label("Settings", systemImage: "gearshape")
                                .foregroundStyle(.primary)
                            Spacer()
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
        tagCounts: ["work": 3, "personal": 5, "ideas": 2],
        dismiss: {},
        onTagSelected: { _ in },
        onShowSettings: {}
    )
}
