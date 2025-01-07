use kuchiki::traits::*;
use kuchiki::{parse_html, NodeRef};
use std::collections::HashMap;

use crate::blocks::{Block, BlockContent, InlineContent, TableContent, TableRow};

fn create_element(name: &str) -> NodeRef {
    use kuchiki::{ExpandedName, Attribute};
    use markup5ever::{namespace_url, local_name, QualName};
    let qual_name = match name {
        "div" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("div")),
        "span" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("span")),
        "p" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("p")),
        "ul" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("ul")),
        "ol" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("ol")),
        "li" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("li")),
        "table" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("table")),
        "tr" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("tr")),
        "td" => QualName::new(None, namespace_url!("http://www.w3.org/1999/xhtml"), local_name!("td")),
        _ => panic!("Unsupported HTML element: {}", name),
    };
    NodeRef::new_element(qual_name, std::iter::empty::<(ExpandedName, Attribute)>())
}

/// Converts an HTML string to BlockNote blocks.
/// This function attempts to preserve as much structure as possible.
pub fn try_parse_html_to_blocks(html: &str) -> Vec<Block> {
    let doc = parse_html().one(html);
    
    // Try to get body or main content node
    let root = match doc.select_first("body") {
        Ok(body) => body.as_node().clone(),
        Err(_) => {
            // If no body tag, use the first div or the document root
            match doc.select_first("div") {
                Ok(div) => div.as_node().clone(),
                Err(_) => doc.clone(),
            }
        }
    };
    
    let processed = process_nested_lists(&root);
    let mut blocks = Vec::new();
    
    // Process each non-whitespace child
    for child in processed.children() {
        if !is_whitespace_node(&child) {
            blocks.extend(parse_blocks(&child));
        }
    }
    
    blocks
}

/// Process nested lists to match BlockNote's structure.
/// This is similar to nestedListsToBlockNoteStructure in BlockNote.
fn process_nested_lists(root: &NodeRef) -> NodeRef {
    // First pass: lift nested lists to parent level
    let lists = root.select("li > ul, li > ol").unwrap();
    for list in lists {
        let list_ref = list.as_node();
        let parent = list_ref.parent().unwrap();
        
        // Get siblings after the list
        let mut siblings = Vec::new();
        let mut next = list_ref.next_sibling();
        while let Some(sibling) = next {
            siblings.push(sibling.clone());
            next = sibling.next_sibling();
        }
        
        // Remove list and siblings
        list_ref.detach();
        for sibling in &siblings {
            sibling.detach();
        }
        
        // Insert list after parent
        if let Some(parent_parent) = parent.parent() {
            parent_parent.append(list_ref.clone());
            
            // Re-add siblings in new li elements
            for sibling in siblings {
                if is_whitespace_node(&sibling) {
                    continue;
                }
                let li = create_element("li");
                li.append(sibling.clone());
                list_ref.insert_after(li.clone());
            }
        }
        
        // Remove empty parent li
        if !parent.children().any(|child| !is_whitespace_node(&child)) {
            parent.detach();
        }
    }
    
    // Second pass: create block groups
    let adjacent_lists = root.select("li + ul, li + ol").unwrap();
    for list in adjacent_lists {
        let list_ref = list.as_node();
        if let Some(prev_li) = list_ref.previous_sibling() {
            // Create block container
            let container = create_element("div");
            prev_li.insert_after(container.clone());
            container.append(prev_li.clone());
            
            // Create block group
            let block_group = create_element("div");
            block_group.as_element().unwrap().attributes.borrow_mut()
                .insert("data-node-type".to_string(), "blockGroup".to_string());
            container.append(block_group.clone());
            
            // Move all following lists into block group
            let mut next = container.next_sibling();
            while let Some(node) = next {
                if let Some(elem) = node.as_element() {
                    let name = elem.name.local.to_string().to_uppercase();
                    if name == "UL" || name == "OL" {
                        block_group.append(node.clone());
                        next = container.next_sibling();
                        continue;
                    }
                }
                break;
            }
        }
    }
    
    root.clone()
}

fn is_whitespace_node(node: &NodeRef) -> bool {
    if let Some(text) = node.as_text() {
        text.borrow().trim().is_empty()
    } else {
        false
    }
}

/// Parse a DOM node into BlockNote blocks
fn parse_blocks(root: &NodeRef) -> Vec<Block> {
    let mut blocks = Vec::new();
    
    // Handle block groups
    if let Some(elem) = root.as_element() {
        if elem.name.local.to_lowercase() == "div" {
            let attrs = elem.attributes.borrow();
            if attrs.get("data-node-type") == Some("blockGroup") {
                for child in root.children() {
                    if !is_whitespace_node(&child) {
                        blocks.extend(parse_blocks(&child));
                    }
                }
                return blocks;
            }
        }
    }
    
    // Handle different block types
    if let Some(elem) = root.as_element() {
        match elem.name.local.to_lowercase().as_str() {
            "p" | "div" => {
                // Check if this div contains block-level elements
                let has_block_children = root.children().any(|child| {
                    child.as_element().map_or(false, |e| {
                        matches!(e.name.local.to_lowercase().as_str(),
                            "p" | "div" | "ul" | "ol" | "table")
                    })
                });
                
                if has_block_children {
                    // Process children as blocks
                    for child in root.children() {
                        if !is_whitespace_node(&child) {
                            blocks.extend(parse_blocks(&child));
                        }
                    }
                } else {
                    // Treat as text block
                    blocks.push(parse_text_block(root));
                }
            },
            "ul" | "ol" => {
                for li in root.children().filter(|n| {
                    n.as_element()
                        .map(|e| e.name.local.to_lowercase() == "li")
                        .unwrap_or(false)
                }) {
                    blocks.push(parse_list_item(&li, elem.name.local.to_lowercase() == "ol"));
                }
            }
            "table" => blocks.push(parse_table_block(root)),
            _ => {
                // Try to parse children for unknown elements
                for child in root.children() {
                    if !is_whitespace_node(&child) {
                        blocks.extend(parse_blocks(&child));
                    }
                }
            }
        }
    }
    
    blocks
}

fn parse_text_block(node: &NodeRef) -> Block {
    Block {
        id: generate_id(),
        block_type: "paragraph".to_string(),
        content: BlockContent::Inline(parse_inline_content(node)),
        props: parse_block_props(node),
        children: Vec::new(),
    }
}

fn parse_list_item(node: &NodeRef, numbered: bool) -> Block {
    Block {
        id: generate_id(),
        block_type: if numbered {
            "numberedListItem".to_string()
        } else {
            "bulletListItem".to_string()
        },
        content: BlockContent::Inline(parse_inline_content(node)),
        props: parse_block_props(node),
        children: Vec::new(),
    }
}

fn parse_table_block(node: &NodeRef) -> Block {
    let mut rows = Vec::new();
    let mut column_widths = Vec::new();
    
    // Parse rows
    for tr in node.select("tr").unwrap() {
        let mut cells = Vec::new();
        for td in tr.as_node().select("td").unwrap() {
            cells.push(parse_inline_content(td.as_node()));
        }
        rows.push(TableRow { cells });
    }
    
    // Get column widths from data attribute or default to None
    if let Some(elem) = node.as_element() {
        if let Some(widths) = elem.attributes.borrow().get("data-column-widths") {
            if let Ok(parsed) = serde_json::from_str::<Vec<Option<f64>>>(widths) {
                column_widths = parsed;
            }
        }
    }
    
    Block {
        id: generate_id(),
        block_type: "table".to_string(),
        content: BlockContent::Table(TableContent {
            content_type: "default".to_string(),
            column_widths,
            rows,
        }),
        props: parse_block_props(node),
        children: Vec::new(),
    }
}

fn parse_inline_content(node: &NodeRef) -> Vec<InlineContent> {
    let mut content = Vec::new();
    let mut current_text = String::new();
    let mut current_styles = HashMap::new();
    
    // Process text nodes and collect styles
    for child in node.children() {
        match child.data() {
            kuchiki::NodeData::Text(text) => {
                let text_str = text.borrow().to_string();
                if !text_str.trim().is_empty() {
                    // Get all parent element styles
                    let mut parent = child.parent();
                    while let Some(p) = parent {
                        if let Some(elem) = p.as_element() {
                            match elem.name.local.to_lowercase().as_str() {
                                "strong" | "b" => {
                                    current_styles.insert("bold".to_string(), "true".to_string());
                                }
                                "em" | "i" => {
                                    current_styles.insert("italic".to_string(), "true".to_string());
                                }
                                "u" => {
                                    current_styles.insert("underline".to_string(), "true".to_string());
                                }
                                "code" => {
                                    current_styles.insert("code".to_string(), "true".to_string());
                                }
                                _ => {}
                            }
                        }
                        parent = p.parent();
                    }
                    
                    content.push(InlineContent {
                        text: text_str,
                        styles: current_styles.clone(),
                    });
                    current_styles.clear();
                }
            }
            kuchiki::NodeData::Element(elem) => {
                // Only recurse if this element might contain text
                if !matches!(elem.name.local.to_lowercase().as_str(), 
                    "img" | "br" | "hr") {
                    content.extend(parse_inline_content(&child));
                }
            }
            _ => {}
        }
    }
    
    content
}

fn parse_block_props(node: &NodeRef) -> HashMap<String, String> {
    let mut props = HashMap::new();
    
    if let Some(elem) = node.as_element() {
        for (key, value) in elem.attributes.borrow().map.iter() {
            if key.local.starts_with("data-prop-") {
                let prop_name = key.local.trim_start_matches("data-prop-").to_string();
                props.insert(prop_name, value.value.to_string());
            }
        }
    }
    
    props
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("block_{}", now)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_paragraph() {
        let html = r#"<div><p>Hello <strong>world</strong>!</p></div>"#;
        let blocks = try_parse_html_to_blocks(html);
        
        assert_eq!(blocks.len(), 1);
        match &blocks[0].content {
            BlockContent::Inline(content) => {
                assert_eq!(content.len(), 3);
                assert_eq!(content[0].text, "Hello ");
                assert_eq!(content[1].text, "world");
                assert!(content[1].styles.contains_key("bold"));
                assert_eq!(content[2].text, "!");
            }
            _ => panic!("Expected inline content"),
        }
    }
    
    #[test]
    fn test_parse_nested_lists() {
        let html = r#"
        <ul>
            <li>Item 1</li>
            <li>
                <ul>
                    <li>Nested 1</li>
                    <li>Nested 2</li>
                </ul>
            </li>
            <li>Item 2</li>
        </ul>"#;
        
        let blocks = try_parse_html_to_blocks(html);
        assert!(blocks.len() > 1);
        assert_eq!(blocks[0].block_type, "bulletListItem");
        
        // Check that nested items are properly structured
        let html_output = crate::html::blocks_to_full_html(&blocks);
        assert!(html_output.contains("bn-block-group"));
    }
    
    #[test]
    fn test_parse_table() {
        let html = r#"
        <table data-column-widths="[100,200]">
            <tr>
                <td>Cell 1</td>
                <td>Cell 2</td>
            </tr>
        </table>"#;
        
        let blocks = try_parse_html_to_blocks(html);
        assert_eq!(blocks.len(), 1);
        
        match &blocks[0].content {
            BlockContent::Table(table) => {
                assert_eq!(table.column_widths, vec![Some(100.0), Some(200.0)]);
                assert_eq!(table.rows.len(), 1);
                assert_eq!(table.rows[0].cells.len(), 2);
            }
            _ => panic!("Expected table content"),
        }
    }
}
