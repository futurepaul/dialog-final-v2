use nostr_sdk::{prelude::Keys, ToBech32};

pub struct KeysHelper;

impl KeysHelper {
    pub fn new() -> Self { Self }

    pub fn generate_nsec(&self) -> String {
        let keys = Keys::generate();
        keys.secret_key().to_bech32().unwrap_or_default()
    }

    pub fn validate_nsec(&self, nsec: String) -> bool {
        dialog_lib::validate_nsec(&nsec).is_ok()
    }

    pub fn derive_npub(&self, nsec: String) -> String {
        match Keys::parse(&nsec) {
            Ok(keys) => keys.public_key().to_bech32().unwrap_or_default(),
            Err(_) => String::new(),
        }
    }
}
