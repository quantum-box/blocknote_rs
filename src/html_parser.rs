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
    // Process all list items recursively
    if let Ok(list_items) = root.select("li") {
        for li in list_items {
            let li_ref = li.as_node();
            let mut content = Vec::new();
            let mut nested_lists = Vec::new();
            let mut has_content = false;

            // First pass: separate content and nested lists
            for child in li_ref.children() {
                if let Some(elem) = child.as_element() {
                    match elem.name.local.to_lowercase().as_str() {
                        "ul" | "ol" => {
                            nested_lists.push(child.clone());
                            child.detach();
                        }
                        _ => {
                            if !is_whitespace_node(&child) {
                                content.push(child.clone());
                                has_content = true;
                            }
                        }
                    }
                } else if !is_whitespace_node(&child) {
                    content.push(child.clone());
                    has_content = true;
                }
            }

            // Clear remaining content
            while let Some(child) = li_ref.first_child() {
                child.detach();
            }

            // Create content wrapper
            if has_content {
                let content_wrapper = create_element("div");
                content_wrapper
                    .as_element()
                    .unwrap()
                    .attributes
                    .borrow_mut()
                    .insert("data-content".to_string(), "true".to_string());

                for node in content {
                    content_wrapper.append(node);
                }
                li_ref.append(content_wrapper);
            }

            // Process nested lists recursively and wrap them
            if !nested_lists.is_empty() {
                let block_group = create_element("div");
                block_group
                    .as_element()
                    .unwrap()
                    .attributes
                    .borrow_mut()
                    .insert("data-node-type".to_string(), "blockGroup".to_string());

                for list in nested_lists {
                    // Recursively process nested lists
                    process_nested_lists(&list);
                    block_group.append(list);
                }
                li_ref.append(block_group);
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
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                blocks.push(parse_text_block(root));
            }
            "pre" => {
                if let Ok(code) = root.select_first("code") {
                    let mut props = HashMap::new();

                    // Extract language from data-language attribute
                    if let Some(lang) = code.attributes.borrow().get("data-language") {
                        props.insert("language".to_string(), lang.to_string());
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
        content: BlockContent::None,
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

    let mut content_nodes = Vec::new();
    let mut nested_lists = Vec::new();

    // First pass: separate content and nested lists
    for child in node.children() {
        if let Some(elem) = child.as_element() {
            match elem.name.local.to_lowercase().as_str() {
                "ul" | "ol" => {
                    let is_ordered = elem.name.local.to_lowercase() == "ol";
                    // Process nested list
                    for li in child.children().filter(|n| {
                        n.as_element()
                            .map(|e| e.name.local.to_lowercase() == "li")
                            .unwrap_or(false)
                    }) {
                        let nested_block = parse_list_item(&li, is_ordered);
                        // Only add as nested if it's actually a list item
                        if nested_block.block_type.ends_with("ListItem") {
                            nested_lists.push(nested_block);
                        }
                    }
                }
                "div" => {
                    if elem
                        .attributes
                        .borrow()
                        .get("data-content")
                        .map(|v| v == "true")
                        .unwrap_or(false)
                    {
                        // This is a content wrapper div
                        for content_child in child.children() {
                            if !is_whitespace_node(&content_child) {
                                content_nodes.push(content_child.clone());
                            }
                        }
                    } else if elem
                        .attributes
                        .borrow()
                        .get("data-node-type")
                        .map(|v| v == "blockGroup")
                        .unwrap_or(false)
                    {
                        // Process block group children
                        for group_child in child.children() {
                            if let Some(group_elem) = group_child.as_element() {
                                match group_elem.name.local.to_lowercase().as_str() {
                                    "ul" | "ol" => {
                                        let is_ordered =
                                            group_elem.name.local.to_lowercase() == "ol";
                                        for li in group_child.children().filter(|n| {
                                            n.as_element()
                                                .map(|e| e.name.local.to_lowercase() == "li")
                                                .unwrap_or(false)
                                        }) {
                                            let nested_block = parse_list_item(&li, is_ordered);
                                            // Only add as nested if it's actually a list item
                                            if nested_block.block_type.ends_with("ListItem") {
                                                nested_lists.push(nested_block);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {
                    if !is_whitespace_node(&child) {
                        content_nodes.push(child.clone());
                    }
                }
            }
        } else if !is_whitespace_node(&child) {
            content_nodes.push(child.clone());
        }
    }

    // Set content if any content nodes exist
    if !content_nodes.is_empty() {
        block.content = BlockContent::Inline(parse_inline_content(&content_nodes));
    }

    // Add nested lists as children
    block.children = nested_lists;

    block
}

fn parse_table_block(node: &NodeRef) -> Block {
    let mut rows = Vec::new();
    let mut column_widths = Vec::new();

    // Parse rows
    for tr in node.select("tr").unwrap() {
        let mut cells = Vec::new();
        for td in tr.as_node().select("td").unwrap() {
            cells.push(parse_single_node_content(td.as_node()));
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

fn parse_inline_content(nodes: &[NodeRef]) -> Vec<InlineContent> {
    let mut content = Vec::new();

    for node in nodes {
        content.extend(parse_single_node_content(node));
    }

    // Only trim the first and last non-empty content
    if !content.is_empty() {
        if let Some(first) = content.first_mut() {
            first.text = first.text.trim_start().to_string();
        }
        if let Some(last) = content.last_mut() {
            last.text = last.text.trim_end().to_string();
        }
    }
    content
}

fn parse_single_node_content(node: &NodeRef) -> Vec<InlineContent> {
    let mut content = Vec::new();
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
                                    current_styles
                                        .insert("underline".to_string(), "true".to_string());
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
                if !matches!(elem.name.local.to_lowercase().as_str(), "img" | "br" | "hr") {
                    content.extend(parse_single_node_content(&child));
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

    #[test]
    fn test_parse_code_block() {
        let html = r#"<pre><code data-language="rust">fn main() {
    println!("Hello");
}</code></pre>"#;

        let blocks = try_parse_html_to_blocks(html);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "codeBlock");
        assert_eq!(blocks[0].props.get("language"), Some(&"rust".to_string()));

        match &blocks[0].content {
            BlockContent::Inline(content) => {
                assert_eq!(
                    content[0].text.trim(),
                    r#"fn main() {
    println!("Hello");
}"#
                );
            }
            _ => panic!("Expected inline content"),
        }
    }
}
