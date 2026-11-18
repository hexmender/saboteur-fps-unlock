fn main() {
    let def = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dinput8.def");
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def.display());
    println!("cargo:rerun-if-changed=dinput8.def");
}
