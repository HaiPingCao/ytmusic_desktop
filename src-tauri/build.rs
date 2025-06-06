fn main() {
    let working_dir = std::env::current_dir().unwrap();
    let dist = working_dir.join("../dist");
    if !dist.exists() {
        std::fs::create_dir(&dist).unwrap();
    }
    tauri_build::build();
}
