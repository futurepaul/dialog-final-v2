import SwiftUI

struct TopicPickerView: View {
    @Binding var selectedTag: String?
    let dismiss: () -> Void
    
    @Environment(\.colorScheme) private var colorScheme
    
    private let allTags = MockData.allTags
    private let tagCounts = MockData.tagCounts
    
    var body: some View {
        NavigationStack {
            List {
                // Inbox (all notes)
                Button {
                    selectedTag = nil
                    dismiss()
                } label: {
                    HStack {
                        Label("Inbox", systemImage: "tray.2")
                            .foregroundStyle(.primary)
                        Spacer()
                        Text("\(MockData.sampleNotes.count)")
                            .foregroundStyle(.secondary)
                        if selectedTag == nil {
                            Image(systemName: "checkmark")
                                .foregroundStyle(.blue)
                        }
                    }
                }
                .listRowBackground(selectedTag == nil ? Color.blue.opacity(0.1) : Color.clear)
                
                Section("Topics") {
                    ForEach(allTags, id: \.self) { tag in
                        Button {
                            selectedTag = tag
                            dismiss()
                        } label: {
                            HStack {
                                Label("#\(tag)", systemImage: "tag")
                                    .foregroundStyle(.primary)
                                Spacer()
                                Text("\(tagCounts[tag] ?? 0)")
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
        dismiss: {}
    )
}