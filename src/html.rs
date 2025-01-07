use std::collections::HashSet;
use kuchiki::traits::*;
use kuchiki::{parse_html, NodeRef};

use crate::blocks::{Block, BlockContent};

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

/// Converts blocks to HTML while preserving all block structure and data.
/// This HTML can be parsed back to blocks without data loss.
pub fn blocks_to_full_html(blocks: &[Block]) -> String {
    let doc = parse_html().one("<div></div>");
    let div = doc.select_first("div").unwrap();
    
    // Create blockGroup wrapper
    let block_group = create_element("div");
    div.as_node().append(block_group.clone());
    block_group.as_element().unwrap().attributes.borrow_mut()
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
                    _ => "ul"
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

fn serialize_block_full(parent: &NodeRef, block: &Block) {
    let container = create_element("div");
    parent.append(container.clone());
    container.as_element().unwrap().attributes.borrow_mut()
        .insert("class".to_string(), "bn-block-container".to_string());
    
    let block_elem = create_element("div");
    container.append(block_elem.clone());
    let mut attrs = block_elem.as_element().unwrap().attributes.borrow_mut();
    attrs.insert("class".to_string(), "bn-block".to_string());
    attrs.insert("data-type".to_string(), block.block_type.clone());

    // Add block properties as data attributes
    for (key, value) in &block.props {
        attrs.insert(format!("data-prop-{}", key).to_string(), value.clone());
    }

    match &block.content {
        BlockContent::Inline(content) => {
            let content_div = create_element("div");
            block_elem.append(content_div.clone());
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class".to_string(), "bn-block-content".to_string());

            for inline in content {
                let span = create_element("span");
                content_div.append(span.clone());
                let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                
                // Add styles as data attributes
                for (key, value) in &inline.styles {
                    span_attrs.insert(format!("data-style-{}", key).to_string(), value.clone());
                }
                
                span.append(NodeRef::new_text(&inline.text));
            }
        },
        BlockContent::Table(table) => {
            let content_div = create_element("div");
            block_elem.append(content_div.clone());
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class".to_string(), "bn-block-content".to_string());

            let table_elem = create_element("table");
            content_div.append(table_elem.clone());
            
            // Add column widths as data attributes
            let mut table_attrs = table_elem.as_element().unwrap().attributes.borrow_mut();
            table_attrs.insert("class".to_string(), "bn-table".to_string());
            table_attrs.insert("data-type".to_string(), table.content_type.clone());
            
            let widths_json = serde_json::to_string(&table.column_widths).unwrap();
            table_attrs.insert("data-column-widths".to_string(), widths_json);

            for row in &table.rows {
                let tr = create_element("tr");
                table_elem.append(tr.clone());
                for cell in &row.cells {
                    let td = create_element("td");
                    tr.append(td.clone());
                    let content_div = create_element("div");
                    td.append(content_div.clone());
                    content_div.as_element().unwrap().attributes.borrow_mut()
                        .insert("class".to_string(), "bn-table-cell-content".to_string());

                    for inline in cell {
                        let span = create_element("span");
                        content_div.append(span.clone());
                        let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                        
                        // Add styles as data attributes
                        for (key, value) in &inline.styles {
                            span_attrs.insert(format!("data-style-{}", key).to_string(), value.clone());
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
        let child_group = create_element("div");
        block_elem.append(child_group.clone());
        child_group.as_element().unwrap().attributes.borrow_mut()
            .insert("class".to_string(), "bn-block-group".to_string());
        
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

    let block_elem = create_element(elem_name);
    parent.append(block_elem.clone());
    
    if elem_name == "div" {
        block_elem.as_element().unwrap().attributes.borrow_mut()
            .insert("data-type".to_string(), block.block_type.clone());
    }

    // Add block properties as data attributes
    if elem_name == "div" {
        let mut attrs = block_elem.as_element().unwrap().attributes.borrow_mut();
        for (key, value) in &block.props {
            attrs.insert(format!("data-prop-{}", key).to_string(), value.clone());
        }
    }

    match &block.content {
        BlockContent::Inline(content) => {
            let content_div = create_element("div");
            block_elem.append(content_div.clone());
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class".to_string(), "bn-block-content".to_string());

            for inline in content {
                let span = create_element("span");
                content_div.append(span.clone());
                let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                
                // Add styles as data attributes
                for (key, value) in &inline.styles {
                    span_attrs.insert(format!("data-style-{}", key).to_string(), value.clone());
                }
                
                span.append(NodeRef::new_text(&inline.text));
            }
        },
        BlockContent::Table(table) => {
            let content_div = create_element("div");
            block_elem.append(content_div.clone());
            content_div.as_element().unwrap().attributes.borrow_mut()
                .insert("class".to_string(), "bn-block-content".to_string());

            let table_elem = create_element("table");
            content_div.append(table_elem.clone());
            
            // Add column widths as data attributes
            let mut table_attrs = table_elem.as_element().unwrap().attributes.borrow_mut();
            table_attrs.insert("class".to_string(), "bn-table".to_string());
            table_attrs.insert("data-type".to_string(), table.content_type.clone());
            
            let widths_json = serde_json::to_string(&table.column_widths).unwrap();
            table_attrs.insert("data-column-widths".to_string(), widths_json);

            for row in &table.rows {
                let tr = create_element("tr");
                table_elem.append(tr.clone());
                for cell in &row.cells {
                    let td = create_element("td");
                    tr.append(td.clone());
                    let content_div = create_element("div");
                    td.append(content_div.clone());
                    content_div.as_element().unwrap().attributes.borrow_mut()
                        .insert("class".to_string(), "bn-table-cell-content".to_string());

                    for inline in cell {
                        let span = create_element("span");
                        content_div.append(span.clone());
                        let mut span_attrs = span.as_element().unwrap().attributes.borrow_mut();
                        
                        // Add styles as data attributes
                        for (key, value) in &inline.styles {
                            span_attrs.insert(format!("data-style-{}", key).to_string(), value.clone());
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
        let list_type = if block.block_type == "numberedListItem" { "ol" } else { "ul" };
        let child_list = create_element(list_type);
        block_elem.append(child_list.clone());
        
        for child in &block.children {
            serialize_block_lossy(&child_list, child);
        }
    }
}
