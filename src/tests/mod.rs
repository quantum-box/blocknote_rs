use blocknote_rs::{
    blocks_to_full_html, blocks_to_html_lossy,
    try_parse_html_to_blocks, blocks_to_markdown_lossy,
    try_parse_markdown_to_blocks, test_utils::*
};

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_HTML: &str = r#"<div class="bn-block-group"><div class="bn-block" data-type="heading" data-level="1"><div class="bn-block-content">Test Document</div></div><div class="bn-block" data-type="paragraph"><div class="bn-block-content">This is a <strong>bold</strong> and <em>italic</em> text.</div></div><div class="bn-block" data-type="codeBlock" data-language="rust"><div class="bn-block-content">fn main() {
    println!("Hello, World!");
}</div></div></div>"#;

    const EXPECTED_MARKDOWN: &str = r#"# Test Document

This is a **bold** and *italic* text.

```rust
fn main() {
    println!("Hello, World!");
}
```"#;

    #[test]
    fn test_html_roundtrip() {
        let blocks = create_test_document();
        
        // Test full HTML conversion (lossless)
        let html = blocks_to_full_html(&blocks);
        let parsed_blocks = try_parse_html_to_blocks(&html);
        assert_eq!(blocks, parsed_blocks);
        
        // Test lossy HTML conversion
        let lossy_html = blocks_to_html_lossy(&blocks);
        let parsed_lossy_blocks = try_parse_html_to_blocks(&lossy_html);
        // Note: We don't expect exact equality due to lossy conversion
        assert_eq!(parsed_lossy_blocks.len(), blocks.len());
    }
    
    #[test]
    fn test_markdown_conversion() {
        let blocks = create_test_document();
        
        // Test markdown conversion
        let markdown = blocks_to_markdown_lossy(&blocks);
        let parsed_blocks = try_parse_markdown_to_blocks(&markdown).unwrap();
        
        // Verify basic structure is preserved
        assert_eq!(parsed_blocks.len(), blocks.len());
        assert_eq!(parsed_blocks[0].block_type, "heading");
        assert_eq!(parsed_blocks[1].block_type, "paragraph");
        assert_eq!(parsed_blocks[2].block_type, "codeBlock");

        // Compare with expected markdown output
        assert_eq!(markdown.trim(), EXPECTED_MARKDOWN.trim());
    }
    
    #[test]
    fn test_html_formatting() {
        let blocks = create_test_document();
        
        let html = blocks_to_full_html(&blocks);
        
        // Compare with expected HTML output
        assert_eq!(html.trim(), EXPECTED_HTML.trim());
    }
}
