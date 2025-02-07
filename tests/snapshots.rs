use blocknote_rs::{
    blocks_to_full_html, blocks_to_html_lossy, blocks_to_markdown_lossy,
    test_utils::create_test_document,
};
use insta::assert_snapshot;

#[test]
fn test_full_html_conversion() {
    let blocks = create_test_document();
    let html = blocks_to_full_html(&blocks);
    assert_snapshot!(html);
}

#[test]
fn test_lossy_html_conversion() {
    let blocks = create_test_document();
    let html = blocks_to_html_lossy(&blocks);
    assert_snapshot!(html);
}

#[test]
fn test_markdown_conversion() {
    let blocks = create_test_document();
    let markdown = blocks_to_markdown_lossy(&blocks);
    assert_snapshot!(markdown);
}

#[test]
fn test_html_roundtrip() {
    let blocks = create_test_document();
    let html = blocks_to_full_html(&blocks);
    let parsed_blocks = blocknote_rs::try_parse_html_to_blocks(&html);
    let roundtrip_html = blocks_to_full_html(&parsed_blocks);
    assert_snapshot!(roundtrip_html);
}

#[test]
fn test_markdown_roundtrip() {
    let blocks = create_test_document();
    let markdown = blocks_to_markdown_lossy(&blocks);
    let parsed_blocks = blocknote_rs::try_parse_markdown_to_blocks(&markdown).unwrap();
    let roundtrip_markdown = blocks_to_markdown_lossy(&parsed_blocks);
    assert_snapshot!(roundtrip_markdown);
}
