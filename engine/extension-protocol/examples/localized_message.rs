use nanika_protocol::message;

fn main() -> Result<(), serde_json::Error> {
    let title = message(
        "files-more",
        [
            ("name".into(), "report.pdf".into()),
            ("count".into(), 2.into())
        ]
    );
    println!("{}", serde_json::to_string(&title)?);
    Ok(())
}
