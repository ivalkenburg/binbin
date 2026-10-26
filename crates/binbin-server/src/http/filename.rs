pub fn safe_download_name(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " ._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .take(150)
        .collect();
    if out.trim().is_empty() {
        "download".into()
    } else {
        out
    }
}
