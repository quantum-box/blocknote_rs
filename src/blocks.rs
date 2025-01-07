use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Represents the different types of DOM elements in BlockNote
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockNoteDOMElement {
    Editor,
    Block,
    BlockGroup,
    BlockContent,
    InlineContent,
}

/// Content type for a block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Inline,
    Table,
    None,
}

/// Table content structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(rename = "columnWidths")]
    pub column_widths: Vec<Option<f64>>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<Vec<InlineContent>>,
}

/// Inline content for text blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineContent {
    // TODO: Define inline content structure based on InlineContentSchema
    pub text: String,
    pub styles: HashMap<String, String>,
}

/// Configuration for file-type blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBlockConfig {
    #[serde(rename = "type")]
    pub type_name: String,
    pub caption: String,
    pub name: String,
    pub url: Option<String>,
    pub show_preview: Option<bool>,
    pub preview_width: Option<f64>,
    pub file_block_accept: Option<Vec<String>>,
}

/// Main block structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub content: BlockContent,
    pub props: HashMap<String, String>,
    #[serde(default)]
    pub children: Vec<Block>,
}

/// Content variants for blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BlockContent {
    Inline(Vec<InlineContent>),
    Table(TableContent),
    None,
}

/// Partial block structure for updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub block_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<BlockContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Block>>,
}

/// Block identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BlockIdentifier {
    Id(String),
    Block(Block),
}
