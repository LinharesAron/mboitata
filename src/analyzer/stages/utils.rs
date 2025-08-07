use std::path::Path;

pub fn file_name(path: &str) -> String {
    let p = Path::new(path);
    match p.file_name() {
        Some(file) => file.to_string_lossy().to_string(),
        None => {
            path.rsplit('/').next().unwrap_or(path).to_string()
        }
    }
}