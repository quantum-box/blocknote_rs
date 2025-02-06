use crate::blocks::Block;
use crate::html_parser::try_parse_html_to_blocks;
use comrak::{markdown_to_html, ComrakOptions};

/// Parses a markdown string into a vector of blocks.
/// Returns None if parsing fails.
pub fn try_parse_markdown_to_blocks(markdown: &str) -> Option<Vec<Block>> {
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.render.unsafe_ = true;

    // Convert markdown to HTML using comrak
    let html = markdown_to_html(markdown, &options);

    // Parse the HTML into blocks using our HTML parser
    Some(try_parse_html_to_blocks(&html))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Block, BlockContent};

    #[test]
    fn test_parse_simple_paragraph() {
        let markdown = "Hello **world**!";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(content.len(), 3);
            assert_eq!(content[0].text, "Hello ");
            assert_eq!(content[1].text, "world");
            assert!(content[1].styles.contains_key("bold"));
            assert_eq!(content[2].text, "!");
        } else {
            panic!("Expected inline content");
        }
    }

    #[test]
    fn test_parse_code_block() {
        let markdown = "```rust\nfn main() {\n    println!(\"Hello\");\n}```";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "codeBlock");
        assert_eq!(blocks[0].props.get("language"), Some(&"rust".to_string()));
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(
                content[0].text.trim(),
                "fn main() {\n    println!(\"Hello\");\n}"
            );
        } else {
            panic!("Expected inline content");
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let markdown = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        // Use snapshot testing instead of manual assertions
        let html = crate::blocks_to_full_html(&blocks);
        insta::assert_snapshot!(html);
    }

    #[test]
    fn test_parse_mixed_content() {
        let markdown = r#"# Main Title

A paragraph with **bold** and *italic* text.

```python
def hello():
    print("Hello, World!")
```

- List item 1
  - Nested item 1.1
  - Nested item 1.2
- List item 2

Another paragraph with `inline code`."#;

        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        // Use snapshot testing instead of manual assertions
        let html = crate::blocks_to_full_html(&blocks);
        insta::assert_snapshot!(html);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].props.get("level"), Some(&"1".to_string()));
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(content[0].text, "Main Title");
        }

        // Check first paragraph with formatting
        assert_eq!(blocks[1].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[1].content {
            assert_eq!(content.len(), 5);
            assert_eq!(content[0].text, "A paragraph with ");
            assert_eq!(content[1].text, "bold");
            assert!(content[1].styles.contains_key("bold"));
            assert_eq!(content[2].text, " and ");
            assert_eq!(content[3].text, "italic");
            assert!(content[3].styles.contains_key("italic"));
            assert_eq!(content[4].text, " text.");
        }

        // Check code block
        assert_eq!(blocks[2].block_type, "codeBlock");
        assert_eq!(blocks[2].props.get("language"), Some(&"python".to_string()));
        if let BlockContent::Inline(content) = &blocks[2].content {
            assert_eq!(
                content[0].text.trim(),
                "def hello():\n    print(\"Hello, World!\")"
            );
        }

        // Check list structure
        assert_eq!(blocks[3].block_type, "bulletListItem");
        assert_eq!(blocks[3].children.len(), 2); // Two nested items
        if let BlockContent::Inline(content) = &blocks[3].content {
            assert_eq!(content[0].text, "List item 1");
        }

        // Check final paragraph with inline code
        assert_eq!(blocks[4].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[4].content {
            assert_eq!(content.len(), 3);
            assert_eq!(content[0].text, "Another paragraph with ");
            assert_eq!(content[1].text, "inline code");
            assert!(content[1].styles.contains_key("code"));
            assert_eq!(content[2].text, ".");
        }
    }
}
