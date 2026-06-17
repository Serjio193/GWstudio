use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn portable_asset_target(file_name: &str) -> Option<&'static str> {
    match file_name {
        "tools.zip" => Some("tools"),
        "sources.zip" => Some("sources"),
        _ => None,
    }
}

fn portable_dir() -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("portable")
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let generated_path = out_dir.join("portable_assets.rs");
    let portable_dir = portable_dir();
    println!("cargo:rerun-if-changed={}", portable_dir.display());

    let mut generated = String::from(
        "const PORTABLE_ASSETS: &[(&str, &str, &[u8])] = &[\n",
    );

    if let Ok(entries) = fs::read_dir(&portable_dir) {
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        files.sort();
        for path in files {
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(target_dir) = portable_asset_target(file_name) else {
                continue;
            };
            println!("cargo:rerun-if-changed={}", path.display());
            generated.push_str(&format!(
                "    ({file_name:?}, {target_dir:?}, include_bytes!(r#\"{}\"#)),\n",
                path.display()
            ));
        }
    }

    generated.push_str("];\n");
    fs::write(generated_path, generated).unwrap();

    tauri_build::build()
}
