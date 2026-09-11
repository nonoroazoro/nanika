/// Report startup failure before the shared WebView can be created.
pub fn report(message: &str) {
    eprintln!("{message}");
}
