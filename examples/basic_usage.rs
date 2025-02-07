use blocknote_rs::{try_parse_html_to_blocks, try_parse_markdown_to_blocks};

fn main() {
    // HTMLからブロックへの変換例
    let html = r#"
        <h1>タイトル</h1>
        <p>段落テキスト</p>
        <ul>
            <li>リストアイテム1</li>
            <li>リストアイテム2
                <ul>
                    <li>ネストされたアイテム</li>
                </ul>
            </li>
        </ul>
    "#;
    let blocks = try_parse_html_to_blocks(html);
    println!("HTMLから変換されたブロック: {:#?}", blocks);

    // Markdownからブロックへの変換例
    let markdown = r#"
# タイトル

段落テキスト

- リストアイテム1
- リストアイテム2
  - ネストされたアイテム
    "#;
    if let Some(blocks) = try_parse_markdown_to_blocks(markdown) {
        println!("Markdownから変換されたブロック: {:#?}", blocks);
    }
}
