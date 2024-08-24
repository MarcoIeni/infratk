use camino::{Utf8Path, Utf8PathBuf};

pub fn current_dir() -> Utf8PathBuf {
    let current_dir = std::env::current_dir().unwrap();
    Utf8PathBuf::from_path_buf(current_dir).unwrap()
}

pub fn strip_current_dir(path: &Utf8Path) -> Utf8PathBuf {
    let curr_dir = current_dir();
    if let Ok(stripped_path) = path.strip_prefix(&curr_dir) {
        stripped_path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

pub fn current_dir_is_simpleinfra() -> bool {
    let current_dir = current_dir();
    current_dir.ends_with("simpleinfra")
}
