use std::path::Path;

use super::{LibraryError, LibraryResult};

pub(super) fn optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}

pub(super) fn non_empty_name(value: &str) -> LibraryResult<String> {
    let name = value.trim();
    if name.is_empty() {
        return Err(LibraryError::Message("名前を入力してください".into()));
    }
    if name.chars().count() > 80 {
        return Err(LibraryError::Message(
            "名前は80文字以内にしてください".into(),
        ));
    }
    Ok(name.to_owned())
}

pub(super) fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
