use crate::models::Note;
use chrono::{Duration, Utc};

pub fn generate_mock_notes() -> Vec<Note> {
    let now = Utc::now();
    vec![
        // Work topic cluster (recent)
        Note {
            id: hex_id(1),
            text: "Need to review the Q4 roadmap before tomorrow's meeting".to_string(),
            tags: vec!["work".to_string(), "planning".to_string()],
            created_at: (now - Duration::minutes(2)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(2),
            text: "Actually, let's push the deadline to Friday".to_string(),
            tags: vec!["work".to_string()],
            created_at: (now - Duration::minutes(1)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(3),
            text: "Don't forget to include the mobile strategy slides".to_string(),
            tags: vec!["work".to_string(), "planning".to_string()],
            created_at: (now - Duration::seconds(30)).timestamp() ,
            is_read: false,
            is_synced: true,
        },
        
        // Personal cluster (1 hour ago)
        Note {
            id: hex_id(4),
            text: "Remember to call mom about thanksgiving plans 🦃".to_string(),
            tags: vec!["personal".to_string(), "family".to_string()],
            created_at: (now - Duration::hours(1)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(5),
            text: "Also need to book flights ✈️".to_string(),
            tags: vec!["personal".to_string(), "travel".to_string()],
            created_at: (now - Duration::minutes(59)).timestamp() ,
            is_read: true,
            is_synced: false,
        },
        
        // Ideas (2 hours ago)
        Note {
            id: hex_id(6),
            text: "App idea: AI that suggests recipes based on what's in your fridge. Could use vision API to scan items, then GPT to suggest combinations. Maybe partner with grocery delivery services? Could also track expiration dates and suggest meals to avoid waste. Premium feature: meal planning for the week with automatic shopping list generation.".to_string(),
            tags: vec!["ideas".to_string(), "ai".to_string(), "startup".to_string()],
            created_at: (now - Duration::hours(2)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        
        // Coffee notes (yesterday)
        Note {
            id: hex_id(7),
            text: "The new coffee shop on 5th street is amazing! Great wifi too ☕".to_string(),
            tags: vec!["coffee".to_string(), "places".to_string()],
            created_at: (now - Duration::days(1)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(8),
            text: "Ethiopian single origin, notes of blueberry and chocolate".to_string(),
            tags: vec!["coffee".to_string()],
            created_at: (now - Duration::days(1) + Duration::minutes(1)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(9),
            text: "Maybe we should have our 1:1s there instead of the office".to_string(),
            tags: vec!["work".to_string(), "coffee".to_string()],
            created_at: (now - Duration::days(1) + Duration::minutes(2)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        
        // Random thoughts
        Note {
            id: hex_id(10),
            text: "Why do we say 'heads up' when we mean 'duck'? 🤔".to_string(),
            tags: vec!["random".to_string()],
            created_at: (now - Duration::days(2)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(11),
            text: "Learn Rust".to_string(),
            tags: vec!["todo".to_string()],
            created_at: (now - Duration::days(3)).timestamp() ,
            is_read: false,
            is_synced: true,
        },
        Note {
            id: hex_id(12),
            text: "That thing Sarah said about compound interest was really insightful. The example about starting to invest at 25 vs 35 was eye-opening. Even small amounts compound significantly over decades.".to_string(),
            tags: vec!["finance".to_string(), "learning".to_string()],
            created_at: (now - Duration::days(4)).timestamp() ,
            is_read: true,
            is_synced: true,
        },
        
        // Dev testing burst
        Note {
            id: hex_id(13),
            text: "Testing".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(10)).timestamp() ,
            is_read: false,
            is_synced: false,
        },
        Note {
            id: hex_id(14),
            text: "One".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(9)).timestamp() ,
            is_read: false,
            is_synced: false,
        },
        Note {
            id: hex_id(15),
            text: "Two".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(8)).timestamp() ,
            is_read: false,
            is_synced: false,
        },
        Note {
            id: hex_id(16),
            text: "Three".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(7)).timestamp() ,
            is_read: false,
            is_synced: false,
        },
    ]
}

fn hex_id(n: u32) -> String {
    format!("{:064x}", n)
}