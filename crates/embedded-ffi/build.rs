use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    let lock_path = find_workspace_lockfile(&manifest_dir).expect("failed to locate Cargo.lock");

    println!("cargo:rerun-if-changed={}", lock_path.display());

    let lockfile = fs::read_to_string(&lock_path).expect("failed to read Cargo.lock");
    let version = find_package_version(&lockfile, "surrealdb")
        .expect("failed to find surrealdb version in Cargo.lock");
    let target = env::var("TARGET").expect("missing TARGET");

    println!("cargo:rustc-env=SURREALDB_CRATE_VERSION={version}");
    println!("cargo:rustc-env=BUILD_TARGET={target}");
}

fn find_workspace_lockfile(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join("Cargo.lock");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn find_package_version(lockfile: &str, package_name: &str) -> Option<String> {
    let mut current_name = None;
    let mut current_version = None;

    for line in lockfile.lines() {
        let line = line.trim();

        if line == "[[package]]" {
            if current_name.as_deref() == Some(package_name) {
                return current_version;
            }
            current_name = None;
            current_version = None;
            continue;
        }

        if let Some(value) = line.strip_prefix("name = ") {
            current_name = parse_toml_string(value);
            continue;
        }

        if let Some(value) = line.strip_prefix("version = ") {
            current_version = parse_toml_string(value);
        }
    }

    if current_name.as_deref() == Some(package_name) {
        current_version
    } else {
        None
    }
}

fn parse_toml_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }

    let bytes = value.as_bytes();
    let quote = bytes[0];
    if (quote == b'"' || quote == b'\'') && bytes[value.len() - 1] == quote {
        return Some(value[1..value.len() - 1].to_string());
    }

    None
}
