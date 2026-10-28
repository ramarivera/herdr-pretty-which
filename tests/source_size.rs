use std::fs;
use std::path::Path;

fn check_directory(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            check_directory(&path);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let lines = fs::read_to_string(&path).unwrap().lines().count();
            assert!(
                lines <= 800,
                "{} has {lines} lines; split it by responsibility",
                path.display()
            );
        }
    }
}

#[test]
fn rust_modules_stay_within_eight_hundred_lines() {
    check_directory(Path::new("src"));
    check_directory(Path::new("tests"));
}
