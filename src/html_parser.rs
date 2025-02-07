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
                } else if let Some(text) = child.as_text() {
                    if !text.borrow().trim().is_empty() {
                        content.push(child.clone());
                        has_content = true;
                    }
                }
            }

            // Clear remaining content
            while let Some(child) = li_ref.first_child() {
                child.detach();
            }

            // Create content wrapper
            let content_wrapper = create_element("div");
            content_wrapper
                .as_element()
                .unwrap()
                .attributes
                .borrow_mut()
                .insert("data-content".to_string(), "true".to_string());

            if has_content {
                for node in content {
                    content_wrapper.append(node);
                }
            } else {
                // Add empty text node if no content
                let text_node = NodeRef::new_text("");
                content_wrapper.append(text_node);
            }
            li_ref.append(content_wrapper);

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

    // Handle block groups and containers
    if let Some(elem) = root.as_element() {
        let name = elem.name.local.to_lowercase();
        let attrs = elem.attributes.borrow();

        match name.as_str() {
            "div" => {
                // Handle BlockNote container and block group
                if attrs
                    .get("class")
                    .map_or(false, |c| c.contains("bn-container"))
                    || attrs.get("data-node-type") == Some("blockGroup")
                {
                    for child in root.children() {
                        if !is_whitespace_node(&child) {
                            blocks.extend(parse_blocks(&child));
                        }
                    }
                    return blocks;
                }

                // Handle block containers
                if attrs
                    .get("class")
                    .map_or(false, |c| c.contains("bn-block-container"))
                {
                    if let Ok(block_elem) = root.select_first(".bn-block") {
                        let block_node = block_elem.as_node();
                        if let Some(block_type) = block_elem.attributes.borrow().get("data-type") {
                            let mut block = Block {
                                id: generate_id(),
                                block_type: block_type.to_string(),
                                content: BlockContent::Inline(vec![]),
                                props: parse_block_props(block_node),
                                children: vec![],
                            };

                            // Parse block content
                            if let Ok(content_elem) = block_node.select_first(".bn-block-content") {
                                block.content = BlockContent::Inline(parse_single_node_content(
                                    content_elem.as_node(),
                                ));
                            }

                            // Handle nested blocks (for lists)
                            if let Ok(nested_blocks) =
                                block_node.select(".bn-block-group > .bn-block-container")
                            {
                                for nested_block in nested_blocks {
                                    blocks.extend(parse_blocks(nested_block.as_node()));
                                }
                            }

                            blocks.push(block);
                            return blocks;
                        }
                    }
                }
            }
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                // Handle direct HTML elements
                let block_type = if name == "p" { "paragraph" } else { "heading" };
                let mut props = HashMap::new();
                if block_type == "heading" {
                    props.insert("level".to_string(), name[1..].to_string());
                }

                let block = Block {
                    id: generate_id(),
                    block_type: block_type.to_string(),
                    content: BlockContent::Inline(parse_single_node_content(root)),
                    props,
                    children: vec![],
                };
                blocks.push(block);
                return blocks;
            }
            _ => {}
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
                    let mut block_type = "codeBlock".to_string();

                    // Check for data-type attribute first
                    if let Some(type_attr) = code.attributes.borrow().get("data-type") {
                        block_type = type_attr.to_string();
                    }

                    // First try to get language from pre tag's lang attribute
                    if let Some(elem) = root.as_element() {
                        let attrs = elem.attributes.borrow();
                        if let Some(lang_attr) = attrs.get("lang") {
                            if !lang_attr.is_empty() {
                                props.insert("language".to_string(), lang_attr.to_string());
                            }
                        }
                    }

                    // Then try code tag's class attribute (language-xxx)
                    if !props.contains_key("language") {
                        let code_attrs = code.attributes.borrow();
                        if let Some(class_attr) = code_attrs.get("class") {
                            if let Some(lang) = class_attr
                                .split_whitespace()
                                .find(|s| s.starts_with("language-"))
                            {
                                let lang_value = lang.trim_start_matches("language-").to_string();
                                if !lang_value.is_empty() {
                                    props.insert("language".to_string(), lang_value.clone());
                                }
                            }
                        }
                    }

                    // Then try data-language attribute
                    if !props.contains_key("language") {
                        if let Some(lang) = code.attributes.borrow().get("data-language") {
                            if !lang.is_empty() {
                                props.insert("language".to_string(), lang.to_string());
                            }
                        }
                    }

                    // Then try code fence info string
                    if !props.contains_key("language") {
                        let code_text = code.text_contents();
                        if let Some(first_line) = code_text.lines().next() {
                            if first_line.starts_with("```") {
                                if let Some(lang_str) = first_line
                                    .trim_start_matches("```")
                                    .split_whitespace()
                                    .next()
                                {
                                    if !lang_str.is_empty() {
                                        props.insert("language".to_string(), lang_str.to_string());
                                    }
                                }
                            }
                        }
                    }

                    // If no language specified, use "text" as default
                    if !props.contains_key("language") {
                        props.insert("language".to_string(), "text".to_string());
                    }

                    // Set the language on both pre and code tags for consistency
                    if let Some(lang) = props.get("language") {
                        if let Some(elem) = root.as_element() {
                            elem.attributes
                                .borrow_mut()
                                .insert("lang".to_string(), lang.clone());
                        }
                        if let Some(code_elem) = code.as_node().as_element() {
                            code_elem
                                .attributes
                                .borrow_mut()
                                .insert("class".to_string(), format!("language-{}", lang));
                        }
                    }

                    let mut text_content = code.text_contents();
                    // Remove trailing backticks and properly handle newlines
                    if text_content.ends_with("```") {
                        text_content = text_content[..text_content.len() - 3].to_string();
                    }
                    text_content = text_content.trim().to_string();

                    let block = Block {
                        id: generate_id(),
                        block_type,
                        content: BlockContent::Inline(vec![InlineContent {
                            text: text_content,
                            styles: HashMap::new(),
                        }]),
                        props,
                        children: vec![],
                    };
                    blocks.push(block);
                    return blocks;
                }
                // If no code element found, try to parse as text block
                blocks.push(parse_text_block(root));
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

        // Check data-type attribute first
        if let Some(type_attr) = attrs.get("data-type") {
            block_type = type_attr.to_string();
        }

        // Then check HTML structure for additional properties
        match elem.name.local.to_lowercase().as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if block_type == "paragraph" {
                    block_type = "heading".to_string();
                }
                let level = elem.name.local[1..].parse::<u8>().unwrap_or(1);
                props.insert("level".to_string(), level.to_string());
            }
            "pre" => {
                if let Ok(code) = node.select_first("code") {
                    if block_type == "paragraph" {
                        block_type = "codeBlock".to_string();
                    }
                    // Extract language from class attribute or data-language
                    if let Some(lang) = code.attributes.borrow().get("data-language") {
                        props.insert("language".to_string(), lang.to_string());
                    } else if let Some(class_attr) = code.attributes.borrow().get("class") {
                        if let Some(lang) = class_attr
                            .split_whitespace()
                            .find(|s| s.starts_with("language-"))
                        {
                            props.insert(
                                "language".to_string(),
                                lang.trim_start_matches("language-").to_string(),
                            );
                        }
                    }
                }
            }
            _ => (),
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
        content: BlockContent::Inline(vec![]),
        props: parse_block_props(node),
        children: Vec::new(),
    };

    // Process content first
    if let Ok(content_div) = node.select_first("div[data-content='true']") {
        block.content = BlockContent::Inline(parse_single_node_content(content_div.as_node()));
    } else {
        // If no content wrapper, parse direct content
        block.content = BlockContent::Inline(parse_single_node_content(node));
    }

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
    let mut current_text = String::new();

    // First pass: separate content and nested lists
    for child in node.children() {
        if let Some(elem) = child.as_element() {
            match elem.name.local.to_lowercase().as_str() {
                "ul" | "ol" => {
                    // If we have accumulated text, create a content node for it
                    if !current_text.trim().is_empty() {
                        let text_node = NodeRef::new_text(current_text.clone());
                        content_nodes.push(text_node);
                        current_text.clear();
                    }

                    let is_ordered = elem.name.local.to_lowercase() == "ol";

                    // Process nested list items
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
        } else if let Some(text) = child.as_text() {
            current_text.push_str(&text.borrow());
        }
    }

    // Set content if any content nodes exist
    if !content_nodes.is_empty() {
        let parsed_content = parse_inline_content(&content_nodes);
        if !parsed_content.is_empty() {
            block.content = BlockContent::Inline(parsed_content);
        }
    } else if !current_text.trim().is_empty() {
        // Use any remaining text as content
        block.content = BlockContent::Inline(vec![InlineContent {
            text: current_text.trim().to_string(),
            styles: HashMap::new(),
        }]);
    }

    // Add nested lists as children if they exist
    if !nested_lists.is_empty() {
        block.children = nested_lists;
    }

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
                            // Check for data-style attributes first
                            let attrs = elem.attributes.borrow();
                            if let Some(style_attr) = attrs.get("data-style") {
                                if style_attr.contains("bold") {
                                    current_styles.insert("bold".to_string(), "true".to_string());
                                }
                                if style_attr.contains("italic") {
                                    current_styles.insert("italic".to_string(), "true".to_string());
                                }
                                if style_attr.contains("underline") {
                                    current_styles
                                        .insert("underline".to_string(), "true".to_string());
                                }
                                if style_attr.contains("code") {
                                    current_styles.insert("code".to_string(), "true".to_string());
                                }
                            }

                            // Then check element types
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

                    // Preserve exact whitespace and styles for text content
                    content.push(InlineContent {
                        text: text_str,
                        styles: current_styles.clone(),
                    });
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
        let attrs = elem.attributes.borrow();

        // First check for data-prop-* attributes
        for (key, value) in attrs.map.iter() {
            if key.local.starts_with("data-prop-") {
                let prop_name = key.local.trim_start_matches("data-prop-").to_string();
                props.insert(prop_name, value.value.to_string());
            }
        }

        // For code blocks, also check lang attribute if language prop not set
        if !props.contains_key("language") {
            if let Some(lang) = attrs.get("lang") {
                if !lang.is_empty() {
                    props.insert("language".to_string(), lang.to_string());
                }
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
        // Test with lang attribute on pre tag
        let html = r#"<pre lang="rust"><code>fn main() {
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

        // Test with data-language attribute on code tag
        let html2 = r#"<pre><code data-language="rust">fn test() {}</code></pre>"#;
        let blocks2 = try_parse_html_to_blocks(html2);
        assert_eq!(blocks2[0].props.get("language"), Some(&"rust".to_string()));

        // Test with language-xxx class on code tag
        let html3 = r#"<pre><code class="language-rust">fn test() {}</code></pre>"#;
        let blocks3 = try_parse_html_to_blocks(html3);
        assert_eq!(blocks3[0].props.get("language"), Some(&"rust".to_string()));
    }
}
