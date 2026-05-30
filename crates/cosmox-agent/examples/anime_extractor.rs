use rig::{
    client::CompletionClient,
    providers::llamafile::{self},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Anime {
    /// The main title of the anime. Supports multiple potential JSON keys.
    #[serde(alias = "name", alias = "title", alias = "Title")]
    #[schemars(description = "The primary title of the anime. Example: 'Serial Experiments Lain'")]
    pub name: Option<String>,

    /// Release format such as TV, OVA, Movie.
    #[serde(alias = "format", alias = "type", alias = "media")]
    #[schemars(description = "The format of the release (TV, OVA, Movie, BDRip).")]
    pub format: Option<String>,

    /// Encoding and language info like HEVC, 10bit.
    #[serde(alias = "language", alias = "encoding", alias = "codec")]
    #[schemars(description = "Encoding details and languages (e.g., HEVC, 10bit, Japanese).")]
    pub language: Option<String>,

    /// Fansub group or uploader.
    #[serde(alias = "subtitle_groups", alias = "group", alias = "sub_group")]
    #[schemars(description = "The fansub group or the uploader's name inside brackets.")]
    pub subtitle_groups: Option<String>,

    /// Video resolution like 1080P, 4K.
    #[serde(alias = "resolution", alias = "res")]
    #[schemars(description = "The video resolution (e.g., 2160P, 4K, 1080P).")]
    pub resolution: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ResultCollection {
    #[schemars(description = "A list of parsed anime items.")]
    pub items: Vec<Anime>,
    #[schemars(description = "The total number of items found in the input text.")]
    pub count: usize,
}

#[tokio::main]
async fn main() {
    let client = llamafile::Client::from_url("http://192.168.1.132:8080");
    let ext = client
    .extractor::<ResultCollection>("Qwen3.5-9B-Q4_K_M.gguf")

    .preamble(
    r#"
    Please answer the question directly and immediately. Do not engage in any deep thinking, chain-of-thought reasoning, or step-by-step analysis

    You are a precise data extraction engine. Your task is to parse anime release titles into structured JSON.

### Extraction Rules:
- **name**: Clean title (e.g., 'Babel II', 'Yosuga no Sora').
- **format**: Media type like TV, OVA, ONA, Movie, BDRip, Blu-ray.
- **language**: Encoding and audio info (e.g., HEVC, 10bit, x265, JP).
- **subtitle_groups**: Name of the fansub group, usually inside the first or second set of [brackets].
- **resolution**: Standard sizes like 2160P, 1080P, 720P, or 4K.

### Examples:
Input: [LuQiAiEr][Yosuga no Sora][1-12][2160P][BDRip][HEVC-10bit]
Output: {"name": "Yosuga no Sora", "format": "BDRip", "language": "HEVC-10bit", "subtitle_groups": "LuQiAiEr", "resolution": "2160P"}

### Critical Instruction:
You MUST fill every field if the information exists. Do not return null if the data is present in the title.
Respond ONLY with a JSON object containing an 'items' array and a 'count' integer."#
)
    .build();
    let mut idx = 0;
    loop {
        let data = ext.extract(include_str!("../test.lst")).await;
        if let Ok(data) = data {
            println!("{data:#?}");
            break;
        } else {
            println!("{data:?}");
        }
        println!("retry #{idx}");
        idx += 1;
    }
}
