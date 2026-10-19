use nanika_protocol::{TextDocumentIndex, TextRead};

#[test]
fn indexes_unicode_without_cutting_characters_and_counts_crlf_lines() {
    let text = format!(
        "{}\r\n你好\n",
        "😀".repeat(nanika_protocol::DETAIL_TEXT_BATCH_CHARS)
    );
    let index = TextDocumentIndex::new("doc".into(), "revision".into(), &text).unwrap();
    assert_eq!(
        index.document.utf16_length,
        2 * nanika_protocol::DETAIL_TEXT_BATCH_CHARS + 5
    );
    assert_eq!(index.document.line_count, 3);
    assert_eq!(
        index.document.max_line_length,
        2 * nanika_protocol::DETAIL_TEXT_BATCH_CHARS
    );
    assert_eq!(index.document.total_chunks, 2);
    let mut restored = String::new();
    for chunk in 0..2 {
        let request = TextRead {
            document_id: "doc".into(),
            revision: "revision".into(),
            index: chunk
        };
        let response = index.read(&text, &request).unwrap();
        response.validate_for(&request).unwrap();
        restored.push_str(&response.value);
    }
    assert_eq!(restored, text);
}

#[test]
fn rejects_stale_revision_wrong_document_and_out_of_bounds_reads() {
    let index = TextDocumentIndex::new("doc".into(), "current".into(), "text").unwrap();
    for request in [
        TextRead {
            document_id: "doc".into(),
            revision: "old".into(),
            index: 0
        },
        TextRead {
            document_id: "other".into(),
            revision: "current".into(),
            index: 0
        },
        TextRead {
            document_id: "doc".into(),
            revision: "current".into(),
            index: 1
        }
    ] {
        assert!(index.read("text", &request).is_err());
    }
}

#[test]
fn keeps_empty_and_boundary_documents_valid() {
    let empty = TextDocumentIndex::new("doc".into(), "empty".into(), "").unwrap();
    assert_eq!(empty.document.line_count, 1);
    assert_eq!(empty.document.max_line_length, 0);
    assert_eq!(empty.document.total_chunks, 1);
    let text = "x"
        .repeat(nanika_protocol::DETAIL_TEXT_BATCH_CHARS * nanika_protocol::MAX_DETAIL_TEXT_CHUNKS);
    let limit = TextDocumentIndex::new("doc".into(), "limit".into(), &text).unwrap();
    assert_eq!(
        limit.document.total_chunks,
        nanika_protocol::MAX_DETAIL_TEXT_CHUNKS
    );
    assert!(TextDocumentIndex::new("doc".into(), "over".into(), &(text + "x")).is_err());
}
