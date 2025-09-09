use clap::{Parser, Subcommand};
use dialog_lib::Dialog;
use nostr_sdk::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
enum CliError {
    #[error("Dialog error: {0}")]
    Dialog(#[from] dialog_lib::DialogError),
    #[error("Bech32 error: {0}")]
    Bech32(#[from] nostr_sdk::nips::nip19::Error),
    #[error("Key parse error: {0}")]
    Keys(#[from] nostr_sdk::key::Error),
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
}

type Result<T> = std::result::Result<T, CliError>;

#[derive(Parser)]
#[command(name = "dialog")]
#[command(about = "A privacy-first note-taking system on Nostr", long_about = None)]
struct Cli {
    /// Print resolved configuration and exit
    #[arg(long)]
    print_config: bool,

    /// Override the relay URL (default: ws://localhost:10548)
    #[arg(short, long)]
    relay: Option<String>,

    /// Set the data directory (default: OS-specific)
    #[arg(short, long)]
    data_dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new note
    #[command(arg_required_else_help = true)]
    Create {
        /// Note text (hashtags will be parsed automatically)
        text: String,
    },

    /// List notes
    List {
        /// Maximum number of notes to display
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Watch for new notes in real-time
        #[arg(long)]
        watch: bool,
    },

    /// Show your public key
    Pubkey,
}

fn get_nsec() -> Result<String> {
    std::env::var("DIALOG_NSEC").map_err(|_| {
        CliError::MissingEnv(
            "DIALOG_NSEC environment variable not set.\n\
            Please set it to your nsec key:\n  \
            export DIALOG_NSEC=nsec1..."
                .to_string(),
        )
    })
}

fn get_relay_url(cli_override: Option<String>) -> String {
    cli_override
        .or_else(|| std::env::var("DIALOG_RELAY").ok())
        .unwrap_or_else(|| "ws://localhost:10548".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Get nsec from environment
    let nsec = get_nsec()?;

    // Resolve relay and data dir before constructing client
    let relay_url = get_relay_url(cli.relay.clone());
    let data_dir_env = std::env::var("DIALOG_DATA_DIR").ok();

    if cli.print_config {
        let keys = Keys::parse(&nsec)?;
        let pubkey = keys.public_key();
        println!("Config:");
        println!("  Pubkey: {}", pubkey.to_bech32()?);
        println!("  Relay:  {}", relay_url);
        match data_dir_env {
            Some(ref dir) => println!("  DataDir: {}", dir),
            None => println!("  DataDir: <OS default>"),
        }
        return Ok(());
    }

    // Set data dir if provided
    if let Some(data_dir) = &cli.data_dir {
        unsafe {
            std::env::set_var("DIALOG_DATA_DIR", data_dir);
        }
    }

    // Create dialog instance
    let dialog = Dialog::new(&nsec).await?;

    // Connect to relay
    eprintln!("Using relay: {}", relay_url);
    if let Err(e) = dialog.connect_relay(&relay_url).await {
        eprintln!("Warning: Could not connect to relay {}: {}", relay_url, e);
        eprintln!("Running in offline mode.");
    }

    // Handle commands
    match cli.command {
        Commands::Create { text } => {
            let id = dialog.create_note(&text).await?;
            println!("Created note: {}", id.to_bech32()?);

            // Parse and display tags
            let tags: Vec<_> = text
                .split_whitespace()
                .filter(|w| w.starts_with('#') && w.len() > 1)
                .map(|t| &t[1..])
                .collect();

            if !tags.is_empty() {
                println!("Tags: {}", tags.join(", "));
            }
        }

        Commands::List { limit, tag, watch } => {
            if watch {
                // Watch mode - show existing notes first, then subscribe to new ones
                println!("Entering watch mode. Press Ctrl+C to exit.\n");

                // First, show existing notes
                let existing_notes = if let Some(ref tag) = tag {
                    println!("=== Existing notes with tag: #{} ===", tag);
                    dialog.list_by_tag(tag, limit).await?
                } else {
                    println!("=== Recent notes ===");
                    dialog.list_notes(limit).await?
                };

                if existing_notes.is_empty() {
                    println!("No existing notes found.");
                } else {
                    for note in &existing_notes {
                        println!("\n[{}]", note.created_at.to_human_datetime());
                        println!("{}", note.text);
                        if !note.tags.is_empty() {
                            println!("Tags: #{}", note.tags.join(" #"));
                        }
                    }
                    println!("\n---");
                }

                // Now watch for notes using subscribe - runs forever
                println!("\nWatching for new notes...");
                let mut receiver = dialog.watch_notes().await?;

                // Handle incoming notes
                while let Some(note) = receiver.recv().await {
                    println!("\n🆕 [{}]", note.created_at.to_human_datetime());
                    println!("{}", note.text);
                    if !note.tags.is_empty() {
                        println!("Tags: #{}", note.tags.join(" #"));
                    }
                }
            } else {
                // Regular list mode
                let notes = if let Some(tag) = tag {
                    println!("Listing notes with tag: #{}", tag);
                    dialog.list_by_tag(&tag, limit).await?
                } else {
                    dialog.list_notes(limit).await?
                };

                if notes.is_empty() {
                    println!("No notes found.");
                } else {
                    for note in &notes {
                        println!("\n[{}]", note.created_at.to_human_datetime());
                        println!("{}", note.text);
                        if !note.tags.is_empty() {
                            println!("Tags: #{}", note.tags.join(" #"));
                        }
                    }
                    println!("\nTotal: {} note(s)", notes.len());
                }
            }
        }

        Commands::Pubkey => {
            println!("Your public key: {}", dialog.public_key().to_bech32()?);
        }
    }

    Ok(())
}
