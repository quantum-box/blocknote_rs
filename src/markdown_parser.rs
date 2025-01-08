use crate::blocks::{Block, BlockContent, InlineContent};
use crate::{local_name, namespace_url};
use comrak::{markdown_to_html, ComrakOptions};
use kuchiki::traits::*;
use markup5ever::QualName;

#[allow(dead_code)]
fn text_starts_with_indent(block: &Block) -> bool {
    if let BlockContent::Inline(content) = &block.content {
        if let Some(text) = content.first() {
            let trimmed = text.text.trim_start();
            let indent = text.text.len() - trimmed.len();
            return indent > 0 || text.text.contains('\n');
        }
    }
    false
}

#[allow(dead_code)]
fn get_indent_level(text: &str) -> usize {
    let trimmed = text.trim_start();
    text.len() - trimmed.len()
}
/// Returns None if parsing fails.
#[doc(hidden)]
pub fn try_parse_markdown_to_blocks(markdown: &str) -> Option<Vec<Block>> {
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.header_ids = None;
    options.render.hardbreaks = true;

    // First convert markdown to HTML using comrak
    let html = markdown_to_html(markdown, &options);

    // Debug: Print the raw HTML before processing
    println!("\nRaw HTML from comrak:");
    println!("{}", html);

    // Process lists to match BlockNote structure
    let html = html
        // First clean up newlines and whitespace
        .replace("<ul>\n", "<ul>")
        .replace("\n</ul>", "</ul>")
        .replace("<li>\n", "<li>")
        .replace("\n</li>", "</li>")
        .replace("> <", "><")
        .replace(">\n<", "><");

    // Mark list items with their type and wrap content
    let html = html
        .replace(
            "<li>",
            r#"<li data-type="bulletListItem"><div data-content="true">"#,
        )
        .replace("</li>", "</div></li>");

    // Handle nested lists by marking them as children
    let html = html
        .replace(
            r#"<div data-content="true"><ul>"#,
            r#"</div><div data-node-type="children"><ul>"#,
        )
        .replace(
            r#"<div data-content="true"><ol>"#,
            r#"</div><div data-node-type="children"><ol>"#,
        );

    // Handle block groups
    let html = html
        .replace("<ul>", r#"<div data-node-type="blockGroup"><ul>"#)
        .replace("<ol>", r#"<div data-node-type="blockGroup"><ol>"#)
        .replace("</ul></div>", "</ul></div></div>")
        .replace("</ol></div>", "</ol></div></div>");

    // Clean up and fix transitions
    let html = html
        .replace(
            r#"<div data-node-type="blockGroup"><div data-node-type="blockGroup">"#,
            r#"<div data-node-type="blockGroup">"#,
        )
        .replace("</div></div></div>", "</div></div>")
        .replace("</div></div><p>", "</div><p>")
        .replace("</div></div><h", "</div><h")
        .replace("</div></div><pre", "</div><pre");

    // Clean up any potential double-wrapping of block groups
    let html = html
        .replace(
            r#"<div data-node-type="blockGroup"><div data-node-type="blockGroup">"#,
            r#"<div data-node-type="blockGroup">"#,
        )
        .replace("</div></div></div>", "</div></div>");

    // Clean up any potential double-wrapping
    let html = html
        .replace(
            r#"<div data-node-type="blockGroup"><div data-node-type="blockGroup">"#,
            r#"<div data-node-type="blockGroup">"#,
        )
        .replace("</div></div></li>", "</div></li>");

    // Process headings to ensure correct block type and level
    let html = (1..=6).fold(html, |acc, level| {
        // Handle heading tags with or without existing attributes
        acc.replace(
            &format!("<h{}>", level),
            &format!(
                r#"<h{} data-block-type="heading" data-level="{}">"#,
                level, level
            ),
        )
    });

    // Process code blocks last to prevent interference
    let html = html
        // Convert language class to data attribute and clean up code blocks
        .replace(
            r#"<pre><code class="language-"#,
            r#"<pre><code data-language=""#,
        )
        .replace("```", "") // Remove any remaining backticks
        .replace("\n</code>", "</code>") // Remove extra newlines before closing tags
        .replace("\n</pre>", "</pre>");

    // Parse HTML using kuchiki for list processing first
    let doc = kuchiki::parse_html().one(html.as_str());
    let mut list_blocks = Vec::new();

    // Helper function to get child index, similar to BlockNote's getChildIndex
    #[allow(dead_code)]
    fn get_child_index(node: &kuchiki::NodeRef) -> usize {
        if let Some(parent) = node.parent() {
            parent
                .children()
                .position(|child| {
                    if let (Some(child_elem), Some(node_elem)) =
                        (child.as_element(), node.as_element())
                    {
                        std::ptr::eq(child_elem, node_elem)
                    } else {
                        false
                    }
                })
                .unwrap_or(0)
        } else {
            0
        }
    }

    // Helper function to check if node is whitespace, similar to BlockNote's isWhitespaceNode
    #[allow(dead_code)]
    fn is_whitespace_node(node: &kuchiki::NodeRef) -> bool {
        match node.data() {
            kuchiki::NodeData::Text(text) => text.borrow().trim().is_empty(),
            _ => false,
        }
    }

    // First step: lift nested lists to parent level
    #[allow(dead_code)]
    fn lift_nested_lists_to_parent(node: &kuchiki::NodeRef) {
        // Find all nested lists (ul or ol inside li)
        let mut nested_lists = Vec::new();
        for list_item in node.select("li > ul, li > ol").unwrap() {
            nested_lists.push(list_item.as_node().clone());
        }

        for list in nested_lists {
            let _index = get_child_index(&list);
            let parent_item = list.parent().unwrap();
            // Get siblings after the list
            let mut siblings_after = Vec::new();
            let mut current = list.next_sibling();
            while let Some(sibling) = current {
                siblings_after.push(sibling.clone());
                current = sibling.next_sibling();
            }

            // Remove list and siblings
            list.detach();
            for sibling in &siblings_after {
                sibling.detach();
            }

            // Insert list after parent
            parent_item.insert_after(list.clone());

            // Process siblings
            for sibling in siblings_after.iter().rev() {
                if !is_whitespace_node(sibling) {
                    let container = kuchiki::NodeRef::new_element(
                        QualName::new(
                            None,
                            namespace_url!("http://www.w3.org/1999/xhtml"),
                            local_name!("li"),
                        ),
                        None,
                    );
                    container.append(sibling.clone());
                    list.insert_after(container);
                }
            }

            // Remove empty parent
            if parent_item.children().count() == 0 {
                parent_item.detach();
            }
        }
    }

    // Second step: create block groups
    #[allow(dead_code)]
    fn create_groups(node: &kuchiki::NodeRef) {
        // Find all list items followed by a list
        let mut list_pairs = Vec::new();
        for list in node.select("li + ul, li + ol").unwrap() {
            if let Some(prev) = list.as_node().previous_sibling() {
                if prev
                    .as_element()
                    .map_or(false, |e| e.name.local.as_ref() == "li")
                {
                    list_pairs.push((prev.clone(), list.as_node().clone()));
                }
            }
        }

        for (list_item, _list) in list_pairs {
            // Create container div
            let block_container = kuchiki::NodeRef::new_element(
                QualName::new(
                    None,
                    namespace_url!("http://www.w3.org/1999/xhtml"),
                    local_name!("div"),
                ),
                None,
            );
            list_item.insert_after(block_container.clone());
            block_container.append(list_item);

            // Create block group
            let block_group = kuchiki::NodeRef::new_element(
                QualName::new(
                    None,
                    namespace_url!("http://www.w3.org/1999/xhtml"),
                    local_name!("div"),
                ),
                None,
            );
            block_group
                .as_element()
                .unwrap()
                .attributes
                .borrow_mut()
                .insert("data-node-type", "blockGroup".into());
            block_container.append(block_group.clone());

            // Move subsequent lists into block group
            while let Some(next) = block_container.next_sibling() {
                if let Some(elem) = next.as_element() {
                    if elem.name.local.as_ref() == "ul" || elem.name.local.as_ref() == "ol" {
                        block_group.append(next);
                        continue;
                    }
                }
                break;
            }
        }
    }

    fn process_list_item(node: &kuchiki::NodeRef, is_ordered: bool) -> Option<Block> {
        // Get direct text content
        let text_content = node.text_contents().trim().to_string();
        if text_content.is_empty() {
            return None;
        }

        // Create the list item block
        let block = Block {
            id: format!("list-{}", text_content.chars().take(10).collect::<String>()),
            block_type: if is_ordered {
                "numberedListItem".to_string()
            } else {
                "bulletListItem".to_string()
            },
            content: BlockContent::Inline(vec![InlineContent {
                text: text_content,
                styles: std::collections::HashMap::new(),
            }]),
            props: std::collections::HashMap::new(),
            children: Vec::new(),
        };

        Some(block)
    }

    // Process lists
    if let Ok(lists) = doc.select("ul, ol") {
        for list in lists {
            // Only process top-level lists (not nested within another li)
            if !list.as_node().ancestors().any(|n| {
                n.as_element()
                    .map_or(false, |e| e.name.local.eq_str_ignore_ascii_case("li"))
            }) {
                let is_ordered = list
                    .as_node()
                    .as_element()
                    .map_or(false, |e| e.name.local.eq_str_ignore_ascii_case("ol"));

                // Process only direct child list items at this level
                for child in list.as_node().children() {
                    if let Some(elem) = child.as_element() {
                        if elem.name.local.eq_str_ignore_ascii_case("li") {
                            if let Some(block) = process_list_item(&child, is_ordered) {
                                println!(
                                    "  Adding list item: {:?} with {} children",
                                    block.content,
                                    block.children.len()
                                );
                                list_blocks.push(block);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\nCreated {} list blocks", list_blocks.len());

    // Parse blocks using HTML parser to get the overall structure
    let html_blocks = crate::html_parser::try_parse_html_to_blocks(&html);

    println!("\nProcessed list blocks:");
    for block in &list_blocks {
        println!("  - {}", block.block_type);
        for child in &block.children {
            println!("    * {}", child.block_type);
        }
    }

    println!("\nHTML blocks:");
    for block in &html_blocks {
        println!("  - {}", block.block_type);
    }

    // Check if the HTML only contains lists by looking at the raw HTML structure
    let html_normalized = html.trim().replace(['\n', '\r'], "");

    // Look for list structures while being lenient with whitespace
    let only_lists = html_normalized.matches("<ul>").count()
        + html_normalized.matches("<ol>").count()
        > 0
        && html_normalized.matches("</ul>").count() + html_normalized.matches("</ol>").count() > 0;

    // Look for any non-list block content
    let has_other_content = html_normalized.contains("<p")
        || html_normalized.contains("<h1")
        || html_normalized.contains("<h2")
        || html_normalized.contains("<h3")
        || html_normalized.contains("<pre")
        || html_normalized.contains("<table");

    // Always prefer list blocks for list content
    if !list_blocks.is_empty() {
        // If we only have lists, return list blocks directly
        if only_lists && !has_other_content {
            return Some(list_blocks);
        }

        // For mixed content, merge list blocks with other blocks
        let mut blocks = Vec::new();
        let mut i = 0;

        while i < html_blocks.len() {
            let current_block = &html_blocks[i];

            if current_block.block_type.ends_with("ListItem") {
                // Found a list section - add our list blocks if we haven't already
                let list_start = i;

                // Find the end of this list section
                while i < html_blocks.len() && html_blocks[i].block_type.ends_with("ListItem") {
                    i += 1;
                }

                // Add only the top-level list blocks that correspond to this section
                let list_count = i - list_start;

                // Add blocks using enumeration
                for block in list_blocks.iter().take(list_count) {
                    // Only add blocks that don't have a parent (top-level items)
                    // and haven't been added yet
                    if !blocks.iter().any(|b: &Block| b.id == block.id) {
                        blocks.push(block.clone());
                    }
                }

                // Skip past the HTML list blocks we just processed
                i = list_start + list_count;
            } else {
                // Non-list block
                blocks.push(current_block.clone());
                i += 1;
            }
        }

        println!("\nMerged blocks:");
        for block in &blocks {
            println!(
                "  - {} (children: {})",
                block.block_type,
                block.children.len()
            );
        }
        return Some(blocks);
    }

    // If no list blocks were created, use HTML blocks
    println!("\nNo list blocks found - using HTML blocks directly");
    Some(html_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Block, BlockContent, InlineContent};

    #[test]
    fn test_parse_simple_paragraph() {
        let markdown = "Hello **world**!";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(content.len(), 3);
            assert_eq!(content[0].text, "Hello ");
            assert_eq!(content[1].text, "world");
            assert!(content[1].styles.contains_key("bold"));
            assert_eq!(content[2].text, "!");
        } else {
            panic!("Expected inline content");
        }
    }

    #[test]
    fn test_parse_code_block() {
        let markdown = "```rust\nfn main() {\n    println!(\"Hello\");\n}```";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "codeBlock");
        assert_eq!(blocks[0].props.get("language"), Some(&"rust".to_string()));
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(
                content[0].text.trim(),
                "fn main() {\n    println!(\"Hello\");\n}"
            );
        } else {
            panic!("Expected inline content");
        }
    }

    #[test]
    fn test_parse_nested_list() {
        let markdown = "- Item 1\n  - Nested 1\n  - Nested 2\n- Item 2";
        let blocks = try_parse_markdown_to_blocks(markdown).unwrap();

        // Use snapshot testing instead of manual assertions
        let html = crate::blocks_to_full_html(&blocks);
        insta::assert_snapshot!(html);
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

        // Use snapshot testing instead of manual assertions
        let html = crate::blocks_to_full_html(&blocks);
        insta::assert_snapshot!(html);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].props.get("level"), Some(&"1".to_string()));
        if let BlockContent::Inline(content) = &blocks[0].content {
            assert_eq!(content[0].text, "Main Title");
        }

        // Check first paragraph with formatting
        assert_eq!(blocks[1].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[1].content {
            assert_eq!(content.len(), 5);
            assert_eq!(content[0].text, "A paragraph with ");
            assert_eq!(content[1].text, "bold");
            assert!(content[1].styles.contains_key("bold"));
            assert_eq!(content[2].text, " and ");
            assert_eq!(content[3].text, "italic");
            assert!(content[3].styles.contains_key("italic"));
            assert_eq!(content[4].text, " text.");
        }

        // Check code block
        assert_eq!(blocks[2].block_type, "codeBlock");
        assert_eq!(blocks[2].props.get("language"), Some(&"python".to_string()));
        if let BlockContent::Inline(content) = &blocks[2].content {
            assert_eq!(
                content[0].text.trim(),
                "def hello():\n    print(\"Hello, World!\")"
            );
        }

        // Check list structure
        assert_eq!(blocks[3].block_type, "bulletListItem");
        assert_eq!(blocks[3].children.len(), 2); // Two nested items
        if let BlockContent::Inline(content) = &blocks[3].content {
            assert_eq!(content[0].text, "List item 1");
        }

        // Check final paragraph with inline code
        assert_eq!(blocks[4].block_type, "paragraph");
        if let BlockContent::Inline(content) = &blocks[4].content {
            assert_eq!(content.len(), 3);
            assert_eq!(content[0].text, "Another paragraph with ");
            assert_eq!(content[1].text, "inline code");
            assert!(content[1].styles.contains_key("code"));
            assert_eq!(content[2].text, ".");
        }
    }
}
