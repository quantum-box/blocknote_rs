use kuchiki::traits::*;
use kuchiki::{parse_html, NodeRef};
use std::collections::HashMap;

use crate::blocks::{Block, BlockContent, InlineContent, TableContent, TableRow};

fn create_element(name: &str) -> NodeRef {
    use kuchiki::{Attribute, ExpandedName};
    use markup5ever::{local_name, namespace_url, QualName};
    let qual_name = match name {
        "div" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("div"),
        ),
        "span" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("span"),
        ),
        "p" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("p"),
        ),
        "ul" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("ul"),
        ),
        "ol" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("ol"),
        ),
        "li" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("li"),
        ),
        "table" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("table"),
        ),
        "tr" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("tr"),
        ),
        "td" => QualName::new(
            None,
            namespace_url!("http://www.w3.org/1999/xhtml"),
            local_name!("td"),
        ),
        _ => panic!("Unsupported HTML element: {}", name),
    };
    NodeRef::new_element(qual_name, std::iter::empty::<(ExpandedName, Attribute)>())
}

/// Converts blocks to HTML while preserving all block structure and data.
/// This HTML can be parsed back to blocks without data loss.
pub fn blocks_to_full_html(blocks: &[Block]) -> String {
    let doc = parse_html().one("<div></div>");
    let div = doc.select_first("div").unwrap();

    // Create blockGroup wrapper
    let block_group = create_element("div");
    div.as_node().append(block_group.clone());
    block_group
        .as_element()
        .unwrap()
        .attributes
        .borrow_mut()
        .insert("class".to_string(), "bn-block-group".to_string());

    for block in blocks {
        serialize_block_full(&block_group, block);
    }

    doc.to_string()
}

/// Converts blocks to HTML with a simplified structure.
/// This produces cleaner HTML but loses some block structure information.
pub fn blocks_to_html_lossy(blocks: &[Block]) -> String {
    let doc = parse_html().one("<div></div>");
    let div = doc.select_first("div").unwrap();

    let mut list_types = HashSet::new();
    list_types.insert("bulletListItem".to_string());
    list_types.insert("numberedListItem".to_string());
    list_types.insert("checkListItem".to_string());

    let mut current_list: Option<NodeRef> = None;
    let mut current_list_type: Option<String> = None;

    for block in blocks {
        if list_types.contains(&block.block_type) {
            // Handle list items
            if let Some(ref list_type) = current_list_type {
                if *list_type != block.block_type {
                    // Different list type, close current list
                    current_list = None;
                }
            }

            if current_list.is_none() {
                // Start new list
                let list_elem = match block.block_type.as_str() {
                    "numberedListItem" => "ol",
                    _ => "ul",
                };
                let list = create_element(list_elem);
                div.as_node().append(list.clone());
                current_list = Some(list);
                current_list_type = Some(block.block_type.clone());
            }

            if let Some(ref list) = current_list {
                serialize_block_lossy(list, block);
            }
        } else {
            // Non-list block, close any open list
            current_list = None;
            current_list_type = None;
            serialize_block_lossy(div.as_node(), block);
        }
    }

    doc.to_string()
}

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
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                blocks.push(parse_text_block(root));
            }
            "pre" => {
                if let Ok(code) = root.select_first("code") {
                    let mut props = HashMap::new();

                    // Extract language from class attribute (language-xxx)
                    if let Some(class_attr) = code.attributes.borrow().get("class") {
                        if let Some(lang) = class_attr.split_whitespace().find(|s| s.starts_with("language-")) {
                            props.insert("language".to_string(), lang.trim_start_matches("language-").to_string());
                        }
                    }

                    // Fallback to data-language attribute
                    if !props.contains_key("language") {
                        if let Some(lang) = code.attributes.borrow().get("data-language") {
                            props.insert("language".to_string(), lang.to_string());
                        }
                    }

                    blocks.push(Block {
                        id: generate_id(),
                        block_type: "codeBlock".to_string(),
                        content: BlockContent::Inline(vec![InlineContent {
                            text: code.text_contents(),
                            styles: HashMap::new(),
                        }]),
                        props,
                        children: vec![],
                    });
                }
            }
            "p" | "div" => {
                // Check if this div contains block-level elements
                let has_block_children = root.children().any(|child| {
                    child.as_element().map_or(false, |e| {
                        matches!(
                            e.name.local.to_lowercase().as_str(),
                            "p" | "div" | "ul" | "ol" | "table"
                        )
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
            }
            "ul" | "ol" => {
                let numbered = elem.name.local.to_lowercase() == "ol";
                let mut list_items = Vec::new();

                // First pass: collect all list items
                for li in root.children().filter(|n| {
                    n.as_element()
                        .map(|e| e.name.local.to_lowercase() == "li")
                        .unwrap_or(false)
                }) {
                    let block = parse_list_item(&li, numbered);
                    list_items.push(block);
                }

                // Second pass: identify top-level items
                let mut top_level_items = Vec::new();
                for item in list_items.iter() {
                    // Check if this item is nested within any other item
                    let is_nested = list_items.iter().any(|other| {
                        if other.id == item.id {
                            return false;
                        }
                        other.children.iter().any(|child| child.id == item.id)
                    });

                    if !is_nested {
                        top_level_items.push(item.clone());
                    }
                }

                println!("Found {} top-level list items", top_level_items.len());
                blocks.extend(top_level_items);
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
    let mut block_type = "paragraph".to_string();
    let mut props = parse_block_props(node);

    if let Some(elem) = node.as_element() {
        let attrs = elem.attributes.borrow();

        // Check for data-block-type attribute first
        if let Some(type_attr) = attrs.get("data-block-type") {
            block_type = type_attr.to_string();

            // Get level from data-level if present for heading blocks
            if block_type == "heading" {
                if let Some(level) = attrs.get("data-level") {
                    props.insert("level".to_string(), level.to_string());
                }
            }
        }

        // Fall back to tag-based detection only if no data-block-type
        if !attrs.contains("data-block-type") {
            match elem.name.local.to_lowercase().as_str() {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    block_type = "heading".to_string();
                    let level = elem.name.local[1..].parse::<u8>().unwrap_or(1);
                    props.insert("level".to_string(), level.to_string());
                }
                _ => {}
            }
        }
    }

    Block {
        id: generate_id(),
        block_type,
        content: BlockContent::Inline(parse_single_node_content(node)),
        props,
        children: Vec::new(),
    }
}

fn parse_list_item(node: &NodeRef, numbered: bool) -> Block {
    let mut block = Block {
        id: generate_id(),
        block_type: "bulletListItem".to_string(),
        content: BlockContent::Inline(Vec::new()),  // Will be updated after processing children
        props: parse_block_props(node),
        children: Vec::new(),
    };

    // Check if this is a numbered list item
    if numbered {
        block.block_type = "numberedListItem".to_string();
    }

    // Check for data-type attribute first
    if let Some(elem) = node.as_element() {
        if let Some(block_type) = elem.attributes.borrow().get("data-type") {
            block.block_type = block_type.to_string();
        }
    }

    let mut inline_content = Vec::new();
    let mut nested_lists = Vec::new();
    let mut current_text = String::new();

    // First pass: separate content and nested lists
    for child in node.children() {
        if let Some(elem) = child.as_element() {
            match elem.name.local.to_lowercase().as_str() {
                "ul" | "ol" => {
                    // If we have accumulated text, create a content node for it
                    if !current_text.trim().is_empty() {
                        let text_node = NodeRef::new_text(current_text.clone());
                        inline_content.push(InlineContent {
                            text: text_node.text_contents(),
                            styles: HashMap::new(),
                        });
                        current_text.clear();
                    }
                    nested_lists.push(child.clone());
                }
                _ => {
                    // Wrap the parsed content in BlockContent::Inline before extending
                    if let BlockContent::Inline(content) = BlockContent::Inline(parse_single_node_content(&child)) {
                        inline_content.extend(content);
                    }
                }
            }
        } else if let Some(text) = child.as_text() {
            current_text.push_str(&text.borrow());
        }
    }

    // If we have any remaining text, create a content node for it
    if !current_text.trim().is_empty() {
        inline_content.push(InlineContent {
            text: current_text,
            styles: HashMap::new(),
        });
    }
    
    // Update block content with collected inline content
    block.content = BlockContent::Inline(inline_content);

    // Process nested lists
    for list in nested_lists {
        let mut child_blocks = Vec::new();
        for li in list.children().filter(|n| {
            n.as_element()
                .map(|e| e.name.local.to_lowercase() == "li")
                .unwrap_or(false)
        }) {
            let child_block = parse_list_item(&li, list.as_element().map_or(false, |e| e.name.local.to_lowercase() == "ol"));
            child_blocks.push(child_block);
        }
        block.children.extend(child_blocks);
    }

    block
}

fn parse_table_block(node: &NodeRef) -> Block {
    let mut rows = Vec::new();
    let mut column_widths = Vec::new();

    // Extract column widths from data attribute if present
    if let Some(elem) = node.as_element() {
        if let Some(widths_str) = elem.attributes.borrow().get("data-column-widths") {
            if let Ok(widths) = serde_json::from_str::<Vec<Option<f64>>>(widths_str) {
                column_widths = widths;
            }
        }
    }

    // Process rows
    for tr in node.children().filter(|n| {
        n.as_element()
            .map(|e| e.name.local.to_lowercase() == "tr")
            .unwrap_or(false)
    }) {
        let mut cells = Vec::new();

        // Process cells
        for td in tr.children().filter(|n| {
            n.as_element()
                .map(|e| e.name.local.to_lowercase() == "td")
                .unwrap_or(false)
        }) {
            cells.push(BlockContent::Inline(parse_single_node_content(&td)));
        }

        rows.push(TableRow { cells });
    }

    Block {
        id: generate_id(),
        block_type: "table".to_string(),
        content: BlockContent::Table(TableContent {
            content_type: "table".to_string(),
            column_widths,
            rows,
        }),
        props: parse_block_props(node),
        children: Vec::new(),
    }
}

fn parse_block_props(node: &NodeRef) -> HashMap<String, String> {
    let mut props = HashMap::new();

    if let Some(elem) = node.as_element() {
        let attrs = elem.attributes.borrow();
        for (key, value) in attrs.map.iter() {
            if key.local.to_string().starts_with("data-prop-") {
                let prop_name = key.local.to_string().trim_start_matches("data-prop-").to_string();
                props.insert(prop_name, value.to_string());
            }
        }
    }

    props
}

fn parse_single_node_content(node: &NodeRef) -> Vec<InlineContent> {
    let mut content = Vec::new();
    let mut current_text = String::new();
    let mut current_styles = HashMap::new();

    // Helper function to flush accumulated text
    let mut flush_text = |text: &str, styles: &HashMap<String, String>| {
        if !text.is_empty() {
            content.push(InlineContent {
                text: text.to_string(),
                styles: styles.clone(),
            });
        }
    };

    // Process content div if it exists
    let content_node = if let Ok(content_div) = node.select_first("[data-content='true']") {
        content_div.as_node().clone()
    } else {
        node.clone()
    };

    for child in content_node.children() {
        if let Some(elem) = child.as_element() {
            // Flush any accumulated text before processing the element
            flush_text(&current_text, &current_styles);
            current_text.clear();

            // Extract styles from data attributes
            let mut span_styles = HashMap::new();
            let attrs = elem.attributes.borrow();
            for (key, value) in attrs.map.iter() {
                if key.local.to_string().starts_with("data-style-") {
                    let style_name = key.local.to_string().trim_start_matches("data-style-").to_string();
                    span_styles.insert(style_name, value.to_string());
                }
            }

            // Process child text with these styles
            for text_child in child.children() {
                if let Some(text) = text_child.as_text() {
                    content.push(InlineContent {
                        text: text.borrow().to_string(),
                        styles: span_styles.clone(),
                    });
                }
            }
        } else if let Some(text) = child.as_text() {
            current_text.push_str(&text.borrow());
        }
    }

    // Flush any remaining text
    flush_text(&current_text, &current_styles);

    content
}

fn generate_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen();
    format!("block_{}", id)
}

fn is_whitespace_node(node: &NodeRef) -> bool {
    if let Some(text) = node.as_text() {
        text.borrow().trim().is_empty()
    } else {
        false
    }
}
