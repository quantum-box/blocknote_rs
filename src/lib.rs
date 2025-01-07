extern crate kuchiki;

pub mod blocks;
pub mod html;
pub mod html_parser;

pub use blocks::*;
pub use html::*;
pub use html_parser::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_blocks_to_full_html() {
        let mut styles = HashMap::new();
        styles.insert("bold".to_string(), "true".to_string());

        let block = Block {
            id: "test".to_string(),
            block_type: "paragraph".to_string(),
            content: BlockContent::Inline(vec![
                InlineContent {
                    text: "Hello".to_string(),
                    styles,
                }
            ]),
            props: HashMap::new(),
            children: vec![],
        };
        
        let html = blocks_to_full_html(&[block]);
        assert!(html.contains("bn-block-group"));
        assert!(html.contains("bn-block"));
        assert!(html.contains("bn-block-content"));
        assert!(html.contains("data-style-bold=\"true\""));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_blocks_to_html_lossy() {
        let block1 = Block {
            id: "1".to_string(),
            block_type: "bulletListItem".to_string(),
            content: BlockContent::Inline(vec![
                InlineContent {
                    text: "Item 1".to_string(),
                    styles: HashMap::new(),
                }
            ]),
            props: HashMap::new(),
            children: vec![],
        };

        let block2 = Block {
            id: "2".to_string(),
            block_type: "bulletListItem".to_string(),
            content: BlockContent::Inline(vec![
                InlineContent {
                    text: "Item 2".to_string(),
                    styles: HashMap::new(),
                }
            ]),
            props: HashMap::new(),
            children: vec![],
        };
        
        let html = blocks_to_html_lossy(&[block1, block2]);
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("Item 1"));
        assert!(html.contains("Item 2"));
    }

    #[test]
    fn test_table_block_html() {
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
                                text: "Cell 1".to_string(),
                                styles: HashMap::new(),
                            }],
                            vec![InlineContent {
                                text: "Cell 2".to_string(),
                                styles: HashMap::new(),
                            }],
                        ],
                    }
                ],
            }),
            props: HashMap::new(),
            children: vec![],
        };

        let html = blocks_to_full_html(&[table]);
        assert!(html.contains("<table"));
        assert!(html.contains("bn-table"));
        assert!(html.contains("data-column-widths"));
        assert!(html.contains("Cell 1"));
        assert!(html.contains("Cell 2"));
    }
}
