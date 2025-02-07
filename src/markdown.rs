use crate::blocks::{Block, BlockContent, InlineContent};

/// Converts blocks to Markdown with a simplified structure.
/// This produces cleaner Markdown but loses some block structure information.
pub fn blocks_to_markdown_lossy(blocks: &[Block]) -> String {
    let mut markdown = String::new();
    let mut list_stack: Vec<(&str, usize)> = Vec::new(); // (list_type, indent_level)

    for block in blocks {
        // Handle list items
        if block.block_type.ends_with("ListItem") {
            let is_ordered = block.block_type == "numberedListItem";
            let list_type = if is_ordered { "1." } else { "-" };

            // Add list marker
            markdown.push_str(list_type);
            markdown.push(' ');

            // Track current list type and level
            list_stack.clear();
            list_stack.push((list_type, 0));

            // Add content and newline after
            if let BlockContent::Inline(content) = &block.content {
                markdown.push_str(&inline_to_markdown(content));
                markdown.push('\n');
            }
        } else {
            // Reset list context for non-list blocks
            list_stack.clear();
        }

        match &block.block_type {
            t if t.ends_with("ListItem") => {
                // Handle nested lists
                if !block.children.is_empty() {
                    markdown.push('\n');
                    for child in &block.children {
                        markdown.push_str(&blocks_to_markdown_lossy(&[child.clone()]));
                    }
                }
            }
            t if t == "paragraph" => {
                if let BlockContent::Inline(content) = &block.content {
                    markdown.push_str(&inline_to_markdown(content));
                    markdown.push_str("\n\n");
                }
            }
            t if t == "heading" => {
                let level = block
                    .props
                    .get("level")
                    .map(|l| l.parse::<usize>().unwrap_or(1))
                    .unwrap_or(1);
                markdown.push_str(&"#".repeat(level));
                markdown.push(' ');
                if let BlockContent::Inline(content) = &block.content {
                    markdown.push_str(&inline_to_markdown(content));
                }
                markdown.push_str("\n\n");
            }
            t if t == "codeBlock" => {
                markdown.push_str("```");
                if let Some(language) = block.props.get("language") {
                    markdown.push_str(language);
                }
                markdown.push('\n');
                if let BlockContent::Inline(content) = &block.content {
                    markdown.push_str(&inline_to_markdown(content));
                }
                markdown.push_str("\n```\n\n");
            }
            t if t == "table" => {
                if let BlockContent::Table(table) = &block.content {
                    // Table header (using first row)
                    if !table.rows.is_empty() {
                        let first_row = &table.rows[0];
                        let mut header = String::new();
                        let mut separator = String::new();

                        for cell in &first_row.cells {
                            header.push('|');
                            header.push(' ');
                            header.push_str(&inline_to_markdown(cell));
                            header.push(' ');

                            separator.push('|');
                            separator.push_str(" --- ");
                        }
                        header.push('|');
                        separator.push('|');

                        markdown.push_str(&header);
                        markdown.push('\n');
                        markdown.push_str(&separator);
                        markdown.push('\n');

                        // Table body (remaining rows)
                        for row in table.rows.iter().skip(1) {
                            for cell in &row.cells {
                                markdown.push('|');
                                markdown.push(' ');
                                markdown.push_str(&inline_to_markdown(cell));
                                markdown.push(' ');
                            }
                            markdown.push('|');
                            markdown.push('\n');
                        }
                        markdown.push('\n');
                    }
                }
            }
            _ => {
                // Default to paragraph for unknown block types
                if let BlockContent::Inline(content) = &block.content {
                    markdown.push_str(&inline_to_markdown(content));
                    markdown.push_str("\n\n");
                }
            }
        }
    }

    markdown
}

fn inline_to_markdown(content: &[InlineContent]) -> String {
    let mut result = String::new();

    for inline in content {
        let mut text = inline.text.clone();

        // Apply styles
        if inline.styles.contains_key("bold") {
            text = format!("**{}**", text);
        }
        if inline.styles.contains_key("italic") {
            text = format!("*{}*", text);
        }
        if inline.styles.contains_key("code") {
            text = format!("`{}`", text);
        }
        if inline.styles.contains_key("underline") {
            // Markdown doesn't support underline, could use HTML but keeping it lossy
            // Skip underline since markdown doesn't support it
        }

        result.push_str(&text);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_blocks_to_markdown_simple() {
        let mut styles = HashMap::new();
        styles.insert("bold".to_string(), "true".to_string());

        let block = Block {
            id: "test".to_string(),
            block_type: "paragraph".to_string(),
            content: BlockContent::Inline(vec![
                InlineContent {
                    text: "Hello ".to_string(),
                    styles: HashMap::new(),
                },
                InlineContent {
                    text: "World".to_string(),
                    styles,
                },
            ]),
            props: HashMap::new(),
            children: vec![],
        };

        let markdown = blocks_to_markdown_lossy(&[block]);
        assert_eq!(markdown, "Hello **World**\n\n");
    }

    #[test]
    fn test_blocks_to_markdown_list() {
        let block1 = Block {
            id: "1".to_string(),
            block_type: "bulletListItem".to_string(),
            content: BlockContent::Inline(vec![InlineContent {
                text: "Item 1".to_string(),
                styles: HashMap::new(),
            }]),
            props: HashMap::new(),
            children: vec![],
        };

        let block2 = Block {
            id: "2".to_string(),
            block_type: "bulletListItem".to_string(),
            content: BlockContent::Inline(vec![InlineContent {
                text: "Item 2".to_string(),
                styles: HashMap::new(),
            }]),
            props: HashMap::new(),
            children: vec![],
        };

        let markdown = blocks_to_markdown_lossy(&[block1, block2]);
        assert_eq!(markdown, "- Item 1\n- Item 2\n");
    }

    #[test]
    fn test_blocks_to_markdown_heading() {
        let mut props = HashMap::new();
        props.insert("level".to_string(), "2".to_string());

        let block = Block {
            id: "h1".to_string(),
            block_type: "heading".to_string(),
            content: BlockContent::Inline(vec![InlineContent {
                text: "Test Heading".to_string(),
                styles: HashMap::new(),
            }]),
            props,
            children: vec![],
        };

        let markdown = blocks_to_markdown_lossy(&[block]);
        assert_eq!(markdown, "## Test Heading\n\n");
    }

    #[test]
    fn test_blocks_to_markdown_code() {
        let mut props = HashMap::new();
        props.insert("language".to_string(), "rust".to_string());

        let block = Block {
            id: "code1".to_string(),
            block_type: "codeBlock".to_string(),
            content: BlockContent::Inline(vec![InlineContent {
                text: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
                styles: HashMap::new(),
            }]),
            props,
            children: vec![],
        };

        let markdown = blocks_to_markdown_lossy(&[block]);
        assert_eq!(
            markdown,
            "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n\n"
        );
    }
}
