import Foundation

struct MockData {
    static let sampleNotes: [Note] = [
        // Work topic cluster (recent, grouped together)
        Note(
            id: "abc123def456789012345678901234567890123456789012345678901234567",
            text: "Need to review the Q4 roadmap before tomorrow's meeting",
            tags: ["work", "planning"],
            createdAt: Date().addingTimeInterval(-120)
        ),
        Note(
            id: "abc124def456789012345678901234567890123456789012345678901234568",
            text: "Actually, let's push the deadline to Friday",
            tags: ["work"],
            createdAt: Date().addingTimeInterval(-60)
        ),
        Note(
            id: "abc125def456789012345678901234567890123456789012345678901234569",
            text: "Don't forget to include the mobile strategy slides",
            tags: ["work", "planning"],
            createdAt: Date().addingTimeInterval(-30)
        ),
        
        // Personal cluster (1 hour ago)
        Note(
            id: "def456abc123789012345678901234567890123456789012345678901234567",
            text: "Remember to call mom about thanksgiving plans 🦃",
            tags: ["personal", "family"],
            createdAt: Date().addingTimeInterval(-3600)
        ),
        Note(
            id: "def457abc123789012345678901234567890123456789012345678901234568",
            text: "Also need to book flights ✈️",
            tags: ["personal", "travel"],
            createdAt: Date().addingTimeInterval(-3540)
        ),
        
        // Ideas topic (standalone, longer text)
        Note(
            id: "ghi789def456123012345678901234567890123456789012345678901234567",
            text: "App idea: AI that suggests recipes based on what's in your fridge. Could use vision API to scan items, then GPT to suggest combinations. Maybe partner with grocery delivery services? Could also track expiration dates and suggest meals to avoid waste. Premium feature: meal planning for the week with automatic shopping list generation.",
            tags: ["ideas", "ai", "startup"],
            createdAt: Date().addingTimeInterval(-7200)
        ),
        
        // Coffee notes (yesterday)
        Note(
            id: "jkl012ghi789456012345678901234567890123456789012345678901234567",
            text: "The new coffee shop on 5th street is amazing! Great wifi too ☕",
            tags: ["coffee", "places"],
            createdAt: Date().addingTimeInterval(-86400)
        ),
        Note(
            id: "jkl013ghi789456012345678901234567890123456789012345678901234568",
            text: "Ethiopian single origin, notes of blueberry and chocolate",
            tags: ["coffee"],
            createdAt: Date().addingTimeInterval(-86340)
        ),
        Note(
            id: "mno345jkl012789012345678901234567890123456789012345678901234567",
            text: "Maybe we should have our 1:1s there instead of the office",
            tags: ["work", "coffee"],
            createdAt: Date().addingTimeInterval(-86280)
        ),
        
        // Random thoughts (various times)
        Note(
            id: "pqr678mno345012012345678901234567890123456789012345678901234567",
            text: "Why do we say 'heads up' when we mean 'duck'? 🤔",
            tags: ["random"],
            createdAt: Date().addingTimeInterval(-172800)
        ),
        Note(
            id: "stu901pqr678345012345678901234567890123456789012345678901234567",
            text: "Learn Rust",
            tags: ["todo"],
            createdAt: Date().addingTimeInterval(-259200)
        ),
        Note(
            id: "vwx234stu901678012345678901234567890123456789012345678901234567",
            text: "That thing Sarah said about compound interest was really insightful. The example about starting to invest at 25 vs 35 was eye-opening. Even small amounts compound significantly over decades.",
            tags: ["finance", "learning"],
            createdAt: Date().addingTimeInterval(-345600)
        ),
        
        // Recent burst (testing grouping)
        Note(
            id: "yz012vwx234567890123456789012345678901234567890123456789012345a",
            text: "Testing",
            tags: ["dev"],
            createdAt: Date().addingTimeInterval(-10)
        ),
        Note(
            id: "yz013vwx234567890123456789012345678901234567890123456789012345b",
            text: "One",
            tags: ["dev"],
            createdAt: Date().addingTimeInterval(-9)
        ),
        Note(
            id: "yz014vwx234567890123456789012345678901234567890123456789012345c",
            text: "Two",
            tags: ["dev"],
            createdAt: Date().addingTimeInterval(-8)
        ),
        Note(
            id: "yz015vwx234567890123456789012345678901234567890123456789012345d",
            text: "Three",
            tags: ["dev"],
            createdAt: Date().addingTimeInterval(-7)
        )
    ]
    
    static func notesForTag(_ tag: String?) -> [Note] {
        guard let tag = tag else { return sampleNotes }
        return sampleNotes.filter { $0.tags.contains(tag) }
    }
    
    static var allTags: [String] {
        let tags = sampleNotes.flatMap { $0.tags }
        let uniqueTags = Set(tags)
        return Array(uniqueTags).sorted()
    }
    
    static var tagCounts: [String: Int] {
        var counts: [String: Int] = [:]
        for note in sampleNotes {
            for tag in note.tags {
                counts[tag, default: 0] += 1
            }
        }
        return counts
    }
}