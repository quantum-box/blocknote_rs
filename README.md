# BlockNote Rust サーバーサイド実装

## 実装タスク

📝 **実装タスク一覧**

✅ 基本構造の実装
- ✅ `Block`構造体と関連型の定義
- ✅ HTML/Markdownパーサーの実装
- ✅ テストケースの作成

✅ 主要機能の実装
- ✅ HTMLからBlockへの変換
- ✅ Markdownからブロックへの変換
- ✅ エラーハンドリングの追加

🔄 ユースケースの実装
- ✅ ネストされたリストの処理
- ✅ 混合コンテンツの処理
- 🔄 統合テストの作成

## 主要な要素

- **Block構造体**: エディタのブロックを表現する基本構造
- **BlockContent**: ブロックの内容を表現する列挙型
- **InlineContent**: インラインテキストとスタイルを管理する構造体

## `Block` 構造体

`Block`構造体は、BlockNoteエディタの各ブロック要素を表現します。

```rust
pub struct Block {
    pub id: String,
    pub block_type: String,
    pub content: BlockContent,
    pub props: HashMap<String, String>,
    pub children: Vec<Block>,
}
```

- `id`: ブロックの一意識別子
- `block_type`: ブロックの種類（paragraph, heading, list等）
- `content`: ブロックの内容（BlockContent型）
- `props`: ブロックのプロパティ
- `children`: 子ブロック（ネストされたリスト等）

### パーサー関数

- `parse_html_to_blocks(html: &str) -> Vec<Block>`: HTMLをブロック構造に変換
- `parse_markdown_to_blocks(markdown: &str) -> Option<Vec<Block>>`: Markdownをブロック構造に変換

## 使用例

基本的な使用例は `examples/basic_usage.rs` を参照してください。

### 1. HTMLパース

**目的**: HTMLコンテンツをBlockNote形式に変換

**実装済み機能**:
- ✅ 基本的なHTML要素のパース
- ✅ ネストされたリストの処理
- ✅ インラインスタイルの保持
- ✅ カスタム属性の処理

**ユースケース例**:
- 既存のHTMLコンテンツをBlockNoteエディタに読み込む
- HTMLフォーマットされたコンテンツの変換
- リッチテキストデータの統合

### 2. Markdownパース

**目的**: MarkdownテキストをBlockNote形式に変換

**実装済み機能**:
- ✅ 基本的なMarkdown記法のパース
- ✅ ネストされたリストの処理
- ✅ インラインスタイルの変換
- ✅ コードブロックの処理

**ユースケース例**:
- Markdownドキュメントの読み込み
- プレーンテキストからリッチテキストへの変換
- 技術文書の統合

## 今後の展開

- 🔄 双方向変換の実装（BlockNote → HTML/Markdown）
- 📝 カスタムブロックタイプのサポート
- 📝 より高度なスタイリングのサポート
