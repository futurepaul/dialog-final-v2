use crate::{Note};
use dialog_lib::Note as LibNote;

pub(crate) fn convert_lib_note_to_uniffi(lib_note: LibNote) -> Note {
    Note {
        id: lib_note.id.to_hex(),
        text: lib_note.text,
        tags: lib_note.tags,
        created_at: lib_note.created_at.as_u64() as i64,
        is_read: lib_note.is_read,
        is_synced: lib_note.is_synced,
    }
}

