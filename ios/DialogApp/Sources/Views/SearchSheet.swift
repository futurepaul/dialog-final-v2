import SwiftUI
import Dialog

struct SearchSheet: View {
    @State private var query: String = ""
    @State private var results: [Note] = []
    let initialNotes: [Note]
    let dismiss: () -> Void

    var body: some View {
        NavigationStack {
            List(filteredNotes, id: \.id) { note in
                VStack(alignment: .leading, spacing: 6) {
                    Text(note.text)
                        .lineLimit(3)
                    if !note.tags.isEmpty {
                        Text("#" + note.tags.joined(separator: " #"))
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                }
                .padding(.vertical, 4)
            }
            .listStyle(.plain)
            .navigationTitle("Search")
            .toolbar { ToolbarItem(placement: .topBarTrailing) { Button("Done") { dismiss() } } }
            .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always))
            .onChange(of: query) { _, _ in
                // Local filter to avoid disturbing main chat state
                self.results = filteredNotes
            }
            .onAppear {
                self.results = initialNotes
            }
        }
    }

    private var filteredNotes: [Note] {
        let q = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !q.isEmpty else { return initialNotes }
        return initialNotes.filter { $0.text.lowercased().contains(q) }
    }
}

