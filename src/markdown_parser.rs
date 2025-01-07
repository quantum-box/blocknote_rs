use comrak::{markdown_to_html, ComrakOptions};
use crate::blocks::{Block, BlockContent};
/// Returns None if parsing fails.
pub fn try_parse_markdown_to_blocks(markdown: &str) -> Option<Vec<Block>> {
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.header_ids = None;
    
    // First convert markdown to HTML using comrak
    let html = markdown_to_html(markdown, &options);
    
    // Process code blocks to use data-language attribute
    let html = html.replace(
        r#"<pre><code class="language-"#,
        r#"<pre><code data-language=""#
    );
    
    // Process nested lists to match BlockNote structure
    let html = html
        .replace("<ul>\n<li>", "<ul><li>")
        .replace("</li>\n</ul>", "</li></ul>")
        .replace("</li>\n<li>", "</li><li>");
    
    // Then use our HTML parser to convert to blocks
    Some(crate::html_parser::try_parse_html_to_blocks(&html))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_paragraph() {
        let markdown = "Hello **world**!";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();
        
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(content.len(), 2);
            assert_eq!(content[0].text, "Hello ");
            assert_eq!(content[1].text, "world!");
            assert!(content[1].styles.contains_key("bold"));
        } else {
            panic!("Expected inline content");
        }
    }
    
    #[test]
    fn test_parse_code_block() {
        let markdown = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();
        
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "codeBlock");
        assert_eq!(blocks[0].props.get("language"), Some(&"rust".to_string()));
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(content[0].text, "fn main() {\n    println!(\"Hello\");\n}");
        } else {
            panic!("Expected inline content");
        }
    }
    
    #[test]
    fn test_parse_nested_list() {
        let markdown = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();
        
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type, "bulletListItem");
        assert_eq!(blocks[1].block_type, "bulletListItem");
        
        // Check first item's nested list
        assert_eq!(blocks[0].children.len(), 2);
        assert_eq!(blocks[0].children[0].block_type, "bulletListItem");
        assert_eq!(blocks[0].children[1].block_type, "bulletListItem");
        
        if let BlockContent::Inline(content) = &blocks[0].children[0].content {
            assert_eq!(content[0].text, "Nested 1");
        } else {
            panic!("Expected inline content");
        }
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
        
        // Verify overall structure
        assert!(blocks.len() >= 4); // At least heading, paragraph, code block, and list
        
        // Check heading
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].props.get("level"), Some(&"1".to_string()));
        
        // Check first paragraph with formatting
        assert_eq!(blocks[1].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[1].content {
            assert!(content.iter().any(|c| c.styles.contains_key("bold")));
            assert!(content.iter().any(|c| c.styles.contains_key("italic")));
        }
        
        // Check code block
        assert_eq!(blocks[2].block_type, "codeBlock");
        assert_eq!(blocks[2].props.get("language"), Some(&"python".to_string()));
        
        // Check list structure
        assert_eq!(blocks[3].block_type, "bulletListItem");
        assert_eq!(blocks[3].children.len(), 2); // Two nested items
        
        // Check final paragraph with inline code
        let last_block = blocks.last().unwrap();
        assert_eq!(last_block.block_type, "paragraph");
        if let BlockContent::Inline(content) = &last_block.content {
            assert!(content.iter().any(|c| c.styles.contains_key("code")));
        }
    }
}
