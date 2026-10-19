use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let inventory = root.join("inventory.json");
    println!("cargo:rerun-if-changed={}", inventory.display());
    let directories: Vec<String> = serde_json::from_slice(&fs::read(inventory).unwrap()).unwrap();
    let mut output = String::from("pub const MANIFESTS: &[&str] = &[\n");
    for directory in directories {
        let path = root.join(directory).join("manifest.jsonc");
        println!("cargo:rerun-if-changed={}", path.display());
        output.push_str(&format!("include_str!({:?}),\n", path));
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("inventory.rs"),
        output
    )
    .unwrap();
}
