fn main() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/dist");
    println!("cargo:rerun-if-changed={}", assets.display());
    assert!(
        assets.join("index.html").exists(),
        "Web assets are missing. Run `npm --prefix web ci && npm --prefix web run build` before building binbin-server."
    );
}
