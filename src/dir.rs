use camino::Utf8PathBuf;

pub fn current_dir() -> Utf8PathBuf {
    let current_dir = std::env::current_dir().unwrap();
    Utf8PathBuf::from_path_buf(current_dir).unwrap()
}

pub fn current_dir_is_simpleinfra() -> bool {
    let current_dir = current_dir();
    current_dir.ends_with("simpleinfra")
}
