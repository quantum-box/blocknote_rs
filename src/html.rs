use std::collections::HashSet;
use kuchiki::traits::*;
use kuchiki::{parse_html, NodeRef};

use crate::blocks::{Block, BlockContent};

/// Converts blocks to HTML while preserving all block structure and data.
/// This HTML can be parsed back to blocks without data loss.
pub fn blocks_to_full_html(blocks: &[Block]) -> String {
    let doc = parse_html().one("<div></div>");
    let div = doc.select_first("div").unwrap();
    
    // Create blockGroup wrapper
    let block_group = div.as_node().append(NodeRef::new_element("div", None));
    block_group.as_element().unwrap().attributes.borrow_mut()
        .insert("class", "bn-block-group");

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
            if let Some(list_type) = current_list_type {
                if list_type != block.block_type {
                    // Different list type, close current list
                    current_list = None;
                }
            }

            if current_list.is_none() {
                // Start new list
                let list_elem = match block.block_type.as_str() {
                    "numberedListItem" => "ol",
                    _ => "ul"
                };
                current_list = Some(div.as_node().append(NodeRef::new_element(list_elem, None)));
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

fn serialize_block_full(parent: &NodeRef, block: &Block) {
    let container = parent.append(NodeRef::new_element("div", None));
    container.as_element().unwrap().attributes.borrow_mut()
        .insert("class", "bn-block-container");
    
    let block_elem = container.append(NodeRef::new_element("div", None));
    let mut attrs = block_elem.as_element().unwrap().attributes.borrow_mut();
    attrs.insert("class", "bn-block");
    attrs.insert("data-type", &block.block_type);

    // Add block properties as data attributes
    for (key, value) in &block.props {
        attrs.insert(&format!("data-prop-{}", key), value);
    }

    match &block.content {
        BlockContent::Inline(content) => {
            let content_div = block_elem.append(NodeRef::new_element("div", None));
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class", "bn-block-content");

            for inline in content {
                let span = content_div.append(NodeRef::new_element("span", None));
                let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                
                // Add styles as data attributes
                for (key, value) in &inline.styles {
                    span_attrs.insert(&format!("data-style-{}", key), value);
                }
                
                span.append(NodeRef::new_text(&inline.text));
            }
        },
        BlockContent::Table(table) => {
            let content_div = block_elem.append(NodeRef::new_element("div", None));
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class", "bn-block-content");

            let table_elem = content_div.append(NodeRef::new_element("table", None));
            
            // Add column widths as data attributes
            let mut table_attrs = table_elem.as_element().unwrap().attributes.borrow_mut();
            table_attrs.insert("class", "bn-table");
            table_attrs.insert("data-type", &table.content_type);
            
            let widths_json = serde_json::to_string(&table.column_widths).unwrap();
            table_attrs.insert("data-column-widths", &widths_json);

            for row in &table.rows {
                let tr = table_elem.append(NodeRef::new_element("tr", None));
                for cell in &row.cells {
                    let td = tr.append(NodeRef::new_element("td", None));
                    let content_div = td.append(NodeRef::new_element("div", None));
                    content_div.as_element().unwrap().attributes.borrow_mut()
                        .insert("class", "bn-table-cell-content");

                    for inline in cell {
                        let span = content_div.append(NodeRef::new_element("span", None));
                        let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                        
                        // Add styles as data attributes
                        for (key, value) in &inline.styles {
                            span_attrs.insert(&format!("data-style-{}", key), value);
                        }
                        
                        span.append(NodeRef::new_text(&inline.text));
                    }
                }
            }
        },
        BlockContent::None => {}
    }

    // Serialize children with proper nesting
    if !block.children.is_empty() {
        let child_group = block_elem.append(NodeRef::new_element("div", None));
        child_group.as_element().unwrap().attributes.borrow_mut()
            .insert("class", "bn-block-group");
        
        for child in &block.children {
            serialize_block_full(&child_group, child);
        }
    }
}

fn serialize_block_lossy(parent: &NodeRef, block: &Block) {
    let elem_name = if block.block_type.ends_with("ListItem") {
        "li"
    } else {
        "div"
    };

    let block_elem = parent.append(NodeRef::new_element(elem_name, None));
    
    if elem_name == "div" {
        block_elem.as_element().unwrap().attributes.borrow_mut()
            .insert("data-type", &block.block_type);
    }

    // Add block properties as data attributes
    if elem_name == "div" {
        let mut attrs = block_elem.as_element().unwrap().attributes.borrow_mut();
        for (key, value) in &block.props {
            attrs.insert(&format!("data-prop-{}", key), value);
        }
    }

    match &block.content {
        BlockContent::Inline(content) => {
            let content_div = block_elem.append(NodeRef::new_element("div", None));
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class", "bn-block-content");

            for inline in content {
                let span = content_div.append(NodeRef::new_element("span", None));
                let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                
                // Add styles as data attributes
                for (key, value) in &inline.styles {
                    span_attrs.insert(&format!("data-style-{}", key), value);
                }
                
                span.append(NodeRef::new_text(&inline.text));
            }
        },
        BlockContent::Table(table) => {
            let content_div = block_elem.append(NodeRef::new_element("div", None));
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class", "bn-block-content");

            let table_elem = content_div.append(NodeRef::new_element("table", None));
            
            // Add column widths as data attributes
            let mut table_attrs = table_elem.as_element().unwrap().attributes.borrow_mut();
            table_attrs.insert("class", "bn-table");
            table_attrs.insert("data-type", &table.content_type);
            
            let widths_json = serde_json::to_string(&table.column_widths).unwrap();
            table_attrs.insert("data-column-widths", &widths_json);

            for row in &table.rows {
                let tr = table_elem.append(NodeRef::new_element("tr", None));
                for cell in &row.cells {
                    let td = tr.append(NodeRef::new_element("td", None));
                    let content_div = td.append(NodeRef::new_element("div", None));
                    content_div.as_element().unwrap().attributes.borrow_mut()
                        .insert("class", "bn-table-cell-content");

                    for inline in cell {
                        let span = content_div.append(NodeRef::new_element("span", None));
                        let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                        
                        // Add styles as data attributes
                        for (key, value) in &inline.styles {
                            span_attrs.insert(&format!("data-style-{}", key), value);
                        }
                        
                        span.append(NodeRef::new_text(&inline.text));
                    }
                }
            }
        },
        BlockContent::None => {}
    }

    // For lossy conversion, we only preserve nesting for list items
    if !block.children.is_empty() && block.block_type.ends_with("ListItem") {
        let child_list = block_elem.append(NodeRef::new_element(
            if block.block_type == "numberedListItem" { "ol" } else { "ul" },
            None
        ));
        
        for child in &block.children {
            serialize_block_lossy(&child_list, child);
        }
    }
}
