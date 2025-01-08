use crate::blocks::{Block, BlockContent, InlineContent, TableContent, TableRow};
use std::collections::HashMap;

/// Creates a complex test document with various block types for testing
pub fn create_test_document() -> Vec<Block> {
    let mut blocks = Vec::new();

    // Heading
    let heading = Block {
        id: "h1".to_string(),
        block_type: "heading".to_string(),
        content: BlockContent::Inline(vec![InlineContent {
            text: "Test Document".to_string(),
            styles: HashMap::new(),
        }]),
        props: {
            let mut props = HashMap::new();
            props.insert("level".to_string(), "1".to_string());
            props
        },
        children: vec![],
    };
    blocks.push(heading);

    // Paragraph with mixed formatting
    let paragraph = Block {
        id: "p1".to_string(),
        block_type: "paragraph".to_string(),
        content: BlockContent::Inline(vec![
            InlineContent {
                text: "This is a ".to_string(),
                styles: HashMap::new(),
            },
            InlineContent {
                text: "bold".to_string(),
                styles: {
                    let mut styles = HashMap::new();
                    styles.insert("bold".to_string(), "true".to_string());
                    styles
                },
            },
            InlineContent {
                text: " and ".to_string(),
                styles: HashMap::new(),
            },
            InlineContent {
                text: "italic".to_string(),
                styles: {
                    let mut styles = HashMap::new();
                    styles.insert("italic".to_string(), "true".to_string());
                    styles
                },
            },
            InlineContent {
                text: " text.".to_string(),
                styles: HashMap::new(),
            },
        ]),
        props: HashMap::new(),
        children: vec![],
    };
    blocks.push(paragraph);

    // Code block
    let code_block = Block {
        id: "code1".to_string(),
        block_type: "codeBlock".to_string(),
        content: BlockContent::Inline(vec![InlineContent {
            text: "fn main() {\n    println!(\"Hello, World!\");\n}".to_string(),
            styles: HashMap::new(),
        }]),
        props: {
            let mut props = HashMap::new();
            props.insert("language".to_string(), "rust".to_string());
            props
        },
        children: vec![],
    };
    blocks.push(code_block);

    // Nested list
    let list_item1 = Block {
        id: "li1".to_string(),
        block_type: "bulletListItem".to_string(),
        content: BlockContent::Inline(vec![InlineContent {
            text: "First item".to_string(),
            styles: HashMap::new(),
        }]),
        props: HashMap::new(),
        children: vec![Block {
            id: "li1.1".to_string(),
            block_type: "bulletListItem".to_string(),
            content: BlockContent::Inline(vec![InlineContent {
                text: "Nested item".to_string(),
                styles: HashMap::new(),
            }]),
            props: HashMap::new(),
            children: vec![],
        }],
    };
    blocks.push(list_item1);

    // Table
    let table = Block {
        id: "table1".to_string(),
        block_type: "table".to_string(),
        content: BlockContent::Table(TableContent {
            content_type: "default".to_string(),
            column_widths: vec![Some(100.0), Some(200.0)],
            rows: vec![
                TableRow {
                    cells: vec![
                        vec![InlineContent {
                            text: "Header 1".to_string(),
                            styles: HashMap::new(),
                        }],
                        vec![InlineContent {
                            text: "Header 2".to_string(),
                            styles: HashMap::new(),
                        }],
                    ],
                },
                TableRow {
                    cells: vec![
                        vec![InlineContent {
                            text: "Cell 1".to_string(),
                            styles: HashMap::new(),
                        }],
                        vec![InlineContent {
                            text: "Cell 2".to_string(),
                            styles: HashMap::new(),
                        }],
                    ],
                },
            ],
        }),
        props: HashMap::new(),
        children: vec![],
    };
    blocks.push(table);

    blocks
}

/// Verifies that two HTML strings are equivalent in structure
pub fn assert_html_equivalent(html1: &str, html2: &str) {
    // TODO: Implement HTML comparison that ignores whitespace and formatting differences
    // For now, just normalize whitespace and compare
    let normalized1 = html1.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized2 = html2.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(normalized1, normalized2);
}
