use crate::blocks::Block;
use crate::html_parser::try_parse_html_to_blocks;
use comrak::{markdown_to_html, ComrakOptions};
use kuchiki::traits::*;

/// Parses a markdown string into a vector of blocks.
/// Returns None if parsing fails.
pub fn try_parse_markdown_to_blocks(markdown: &str) -> Option<Vec<Block>> {
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.parse.smart = true;
    options.render.unsafe_ = true;
    options.render.github_pre_lang = true;
    options.extension.table = true;
    options.parse.default_info_string = None;

    // Convert markdown to HTML using comrak
    let html = markdown_to_html(markdown, &options);
    println!("Generated HTML from markdown:\n{}", html);

    // Initialize HTML structure
    let mut blocks_html = String::new();

    // Parse the HTML using kuchiki
    let doc = kuchiki::parse_html().one(html);
    println!("Parsed HTML document: {:#?}", doc);
    if let Ok(body) = doc.select_first("body") {
        println!("Found body element");
        // Select all block-level elements
        if let Ok(elements) = body
            .as_node()
            .select("h1, h2, h3, h4, h5, h6, p, pre, ul, ol, li")
        {
            let elements: Vec<_> = elements.collect();
            println!("Found {} block-level elements", elements.len());
            // Process each block-level element
            for child in elements {
                let node = child.as_node();
                let node_html = node.to_string();
                println!("Processing node: {}", node_html);
                let node = child.as_node();
                println!("Processing node: {:#?}", node);
                if let Some(elem) = node.as_element() {
                    println!("Element type: {}", elem.name.local);
                    match elem.name.local.as_ref() {
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            let level = elem.name.local[1..].parse::<u8>().unwrap_or(1);
                            let text_contents = node.text_contents();
                            let text = text_contents.trim();
                            blocks_html.push_str(&format!(
                                "<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"heading\" data-props=\"{{&quot;level&quot;:{}}}\"><div class=\"bn-block-content\"><span>{}</span></div></div></div>",
                                level, text
                            ));
                            println!("Added heading block: {}", text);
                        }
                        "pre" => {
                            if let Ok(code) = node.select_first("code") {
                                let mut lang = String::from("text");
                                let code_text = code.text_contents();

                                // Try pre tag's lang attribute first
                                if let Some(pre_elem) = node.as_element() {
                                    let attrs = pre_elem.attributes.borrow();
                                    if let Some(lang_attr) = attrs.get("lang") {
                                        lang = lang_attr.to_string();
                                    }
                                }

                                // Then try code tag's class attribute (language-xxx)
                                if lang == "text" {
                                    if let Some(code_elem) = code.as_node().as_element() {
                                        let code_attrs = code_elem.attributes.borrow();
                                        if let Some(class_attr) = code_attrs.get("class") {
                                            if let Some(lang_class) = class_attr
                                                .split_whitespace()
                                                .find(|s| s.starts_with("language-"))
                                            {
                                                lang = lang_class
                                                    .trim_start_matches("language-")
                                                    .to_string();
                                            }
                                        }
                                    }
                                }

                                // Finally try to get language from code fence info string
                                if lang == "text" {
                                    if let Some(first_line) = code_text.lines().next() {
                                        if first_line.starts_with("```") {
                                            if let Some(lang_str) = first_line
                                                .trim_start_matches("```")
                                                .split_whitespace()
                                                .next()
                                            {
                                                if !lang_str.is_empty() {
                                                    lang = lang_str.to_string();
                                                }
                                            }
                                        }
                                    }
                                }

                                let mut code_content = code_text;
                                // Remove code fence markers and properly handle newlines
                                if code_content.starts_with("```") {
                                    if let Some(first_newline) = code_content.find('\n') {
                                        code_content =
                                            code_content[first_newline + 1..].to_string();
                                    }
                                }
                                if code_content.ends_with("```") {
                                    code_content = code_content[..code_content.len() - 3]
                                        .trim_end()
                                        .to_string();
                                }

                                // Set the language attribute on both pre and code tags
                                if let Some(pre_elem) = node.as_element() {
                                    pre_elem
                                        .attributes
                                        .borrow_mut()
                                        .insert("lang".to_string(), lang.clone());
                                }
                                if let Some(code_elem) = code.as_node().as_element() {
                                    code_elem
                                        .attributes
                                        .borrow_mut()
                                        .insert("class".to_string(), format!("language-{}", lang));
                                }

                                // Don't trim code content to preserve exact whitespace
                                blocks_html.push_str(&format!(
                                    "<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"codeBlock\" data-prop-language=\"{}\" data-language=\"{}\"><div class=\"bn-block-content\"><span>{}</span></div></div></div>",
                                    lang, lang, code_content
                                ));
                                println!("Added code block: {} (lang: {})", code_content, lang);
                            }
                        }
                        "p" => {
                            let mut content = String::new();
                            for child_node in node.children() {
                                match child_node.as_element() {
                                    Some(child_elem) => match child_elem.name.local.as_ref() {
                                        "strong" | "b" => {
                                            let text_contents = child_node.text_contents();
                                            if !text_contents.trim().is_empty() {
                                                content.push_str(&format!(
                                                    "<span style=\"font-weight: bold;\" data-style-bold=\"true\" data-style=\"{{&quot;bold&quot;:true}}\">{}</span>",
                                                    text_contents.trim()
                                                ));
                                            }
                                        }
                                        "em" | "i" => {
                                            let text_contents = child_node.text_contents();
                                            if !text_contents.trim().is_empty() {
                                                content.push_str(&format!(
                                                    "<span style=\"font-style: italic;\" data-style-italic=\"true\" data-style=\"{{&quot;italic&quot;:true}}\">{}</span>",
                                                    text_contents.trim()
                                                ));
                                            }
                                        }
                                        "code" => {
                                            let text_contents = child_node.text_contents();
                                            if !text_contents.trim().is_empty() {
                                                content.push_str(&format!(
                                                    "<span style=\"font-family: monospace;\" data-style-code=\"true\" data-style=\"{{&quot;code&quot;:true}}\">{}</span>",
                                                    text_contents.trim()
                                                ));
                                            }
                                        }
                                        _ => {
                                            let text_contents = child_node.text_contents();
                                            if !text_contents.is_empty() {
                                                content.push_str(&format!(
                                                    "<span>{}</span>",
                                                    text_contents
                                                ));
                                            }
                                        }
                                    },
                                    None => {
                                        if let Some(text) = child_node.as_text() {
                                            let borrowed = text.borrow();
                                            if !borrowed.is_empty() {
                                                content.push_str(&format!(
                                                    "<span>{}</span>",
                                                    borrowed.as_str()
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            if !content.is_empty() {
                                blocks_html.push_str(&format!(
                                    "<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"paragraph\"><div class=\"bn-block-content\">{}</div></div></div>",
                                    content
                                ));
                                println!("Added paragraph block: {}", content);
                            }
                        }
                        "li" => {
                            // Skip processing list items directly - they are handled by the ul/ol matcher
                        }
                        "ul" | "ol" => {
                            // Create block container for the list
                            blocks_html.push_str("<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"bulletedList\"><div class=\"bn-block-content\">");

                            // Process list items
                            if let Ok(items) = node.select("> li") {
                                let items: Vec<_> = items.collect();
                                for item in items {
                                    let mut item_content = String::new();
                                    let mut has_nested_list = false;

                                    // First pass: collect direct content and check for nested lists
                                    for child in item.as_node().children() {
                                        if let Some(elem) = child.as_element() {
                                            if ["ul", "ol"].contains(&elem.name.local.as_ref()) {
                                                has_nested_list = true;
                                                continue;
                                            }
                                            let text = child.text_contents();
                                            if !text.trim().is_empty() {
                                                if !item_content.is_empty() {
                                                    item_content.push(' ');
                                                }
                                                item_content.push_str(text.trim());
                                            }
                                        } else if let Some(text) = child.as_text() {
                                            let text_content = text.borrow();
                                            if !text_content.trim().is_empty() {
                                                if !item_content.is_empty() {
                                                    item_content.push(' ');
                                                }
                                                item_content.push_str(text_content.trim());
                                            }
                                        }
                                    }

                                    // Create block for current list item
                                    blocks_html.push_str(&format!(
                                        "<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"bulletListItem\"><div class=\"bn-block-content\"><span>{}</span></div></div></div>",
                                        item_content.trim()
                                    ));

                                    // Process nested lists if they exist
                                    if has_nested_list {
                                        // Add block-children container for nested list
                                        blocks_html.push_str("<div class=\"bn-block-children\">");
                                        // Create nested bulletedList block
                                        blocks_html.push_str("<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"bulletedList\"><div class=\"bn-block-content\">");
                                        for child in item.as_node().children() {
                                            if let Some(child_elem) = child.as_element() {
                                                if ["ul", "ol"]
                                                    .contains(&child_elem.name.local.as_ref())
                                                {
                                                    if let Ok(nested_items) = child.select("> li") {
                                                        for nested_item in nested_items {
                                                            let mut nested_content = String::new();

                                                            // Process direct text content of nested items
                                                            for nested_child in
                                                                nested_item.as_node().children()
                                                            {
                                                                if let Some(nested_elem) =
                                                                    nested_child.as_element()
                                                                {
                                                                    if !["ul", "ol"].contains(
                                                                        &nested_elem
                                                                            .name
                                                                            .local
                                                                            .as_ref(),
                                                                    ) {
                                                                        let text = nested_child
                                                                            .text_contents();
                                                                        if !text.trim().is_empty() {
                                                                            if !nested_content
                                                                                .is_empty()
                                                                            {
                                                                                nested_content
                                                                                    .push(' ');
                                                                            }
                                                                            nested_content
                                                                                .push_str(
                                                                                    text.trim(),
                                                                                );
                                                                        }
                                                                    }
                                                                } else if let Some(text) =
                                                                    nested_child.as_text()
                                                                {
                                                                    let text_content =
                                                                        text.borrow();
                                                                    if !text_content
                                                                        .trim()
                                                                        .is_empty()
                                                                    {
                                                                        if !nested_content
                                                                            .is_empty()
                                                                        {
                                                                            nested_content
                                                                                .push(' ');
                                                                        }
                                                                        nested_content.push_str(
                                                                            text_content.trim(),
                                                                        );
                                                                    }
                                                                }
                                                            }

                                                            // Add nested item
                                                            if !nested_content.trim().is_empty() {
                                                                blocks_html.push_str(&format!(
                                                                    "<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"bulletListItem\"><div class=\"bn-block-content\"><span>{}</span></div></div></div>",
                                                                    nested_content.trim()
                                                                ));

                                                                // Process any deeper nested lists
                                                                for nested_child in
                                                                    nested_item.as_node().children()
                                                                {
                                                                    if let Some(nested_elem) =
                                                                        nested_child.as_element()
                                                                    {
                                                                        if ["ul", "ol"].contains(
                                                                            &nested_elem
                                                                                .name
                                                                                .local
                                                                                .as_ref(),
                                                                        ) {
                                                                            // Add nested list structure
                                                                            blocks_html.push_str("<div class=\"bn-block-children\"><div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"bulletedList\"><div class=\"bn-block-content\">");
                                                                            // Recursively process deeper nested lists
                                                                            if let Ok(
                                                                                deeper_items,
                                                                            ) = nested_child
                                                                                .select("> li")
                                                                            {
                                                                                for deeper_item in
                                                                                    deeper_items
                                                                                {
                                                                                    let mut
                                                                                    deeper_content =
                                                                                        String::new(
                                                                                        );
                                                                                    for deeper_child in deeper_item.as_node().children() {
                                                                        if let Some(deeper_elem) = deeper_child.as_element() {
                                                                            if !["ul", "ol"].contains(&deeper_elem.name.local.as_ref()) {
                                                                                let text = deeper_child.text_contents();
                                                                                if !text.trim().is_empty() {
                                                                                    if !deeper_content.is_empty() {
                                                                                        deeper_content.push(' ');
                                                                                    }
                                                                                    deeper_content.push_str(text.trim());
                                                                                }
                                                                            }
                                                                        } else if let Some(text) = deeper_child.as_text() {
                                                                            let text_content = text.borrow();
                                                                            if !text_content.trim().is_empty() {
                                                                                if !deeper_content.is_empty() {
                                                                                    deeper_content.push(' ');
                                                                                }
                                                                                deeper_content.push_str(text_content.trim());
                                                                            }
                                                                        }
                                                                    }
                                                                                    if !deeper_content.trim().is_empty() {
                                                                                        blocks_html.push_str(&format!(
                                                                                            "<div class=\"bn-block-container\"><div class=\"bn-block\" data-type=\"bulletListItem\"><div class=\"bn-block-content\"><span>{}</span></div></div></div>",
                                                                                            deeper_content.trim()
                                                                                        ));
                                                                                    }
                                                                                }
                                                                            }
                                                                            // Close nested bulletedList block
                                                                            blocks_html.push_str("</div></div></div></div>");
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Close nested bulletedList block and block-children
                                        blocks_html.push_str("</div></div></div></div>");
                                    }
                                }

                                // Close the list container
                                blocks_html.push_str("</div></div></div>");
                            }
                        }
                        _ => {
                            println!("Skipping unhandled element: {}", elem.name.local);
                        }
                    }
                }
            }
        }
    }

    // Create final HTML with proper structure
    let mut final_html =
        String::from(r#"<html><head></head><body><div><div class="bn-block-group">"#);

    // Process each block and wrap it in proper BlockNote structure
    let blocks_vec: Vec<&str> = blocks_html
        .split("</div></div></div>")
        .filter(|s| !s.trim().is_empty())
        .collect();

    for block in blocks_vec {
        let block_html = format!("{}</div></div></div>", block.trim());
        if block_html.contains("data-type=\"heading\"")
            || block_html.contains("data-type=\"paragraph\"")
            || block_html.contains("data-type=\"code\"")
            || block_html.contains("data-type=\"codeBlock\"")
            || block_html.contains("data-type=\"bulletListItem\"")
        {
            final_html.push_str(&block_html);
        } else {
            println!("Skipping malformed block: {}", block_html);
        }
    }

    final_html.push_str("</div></div></body></html>");
    println!("Final processed HTML:\n{}", final_html);

    // Parse the HTML into blocks using our HTML parser
    let blocks = try_parse_html_to_blocks(&final_html);
    println!("Generated blocks: {:?}", blocks);

    // If no blocks were generated but we have content, something went wrong
    if blocks.is_empty() && !blocks_html.is_empty() {
        println!(
            "Warning: Generated blocks is empty but blocks_html contains content:\n{}",
            blocks_html
        );
        println!("Final HTML that produced no blocks:\n{}", final_html);
    }

    Some(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::BlockContent;

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
        let markdown = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
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
