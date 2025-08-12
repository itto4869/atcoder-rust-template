use assert_cmd::Command;
use std::{fs, path::Path};

fn run_all_cases(dir: &str, bin_name: &str) {
    let dir = Path::new(dir);
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) == Some("in") {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let input = fs::read_to_string(&path).unwrap();
            let expected = fs::read_to_string(dir.join(format!("{stem}.out"))).unwrap();

            let mut cmd = Command::cargo_bin(bin_name).unwrap();
            cmd.write_stdin(input)
                .assert()
                .success()
                .stdout(expected);
        }
    }
}
#[test]
fn d_all_cases() {
    run_all_cases("tests/d", "d");
}
