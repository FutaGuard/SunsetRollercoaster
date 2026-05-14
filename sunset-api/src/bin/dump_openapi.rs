use std::env;
use std::fs;
use std::path::PathBuf;

use sunset_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> anyhow::Result<()> {
    let out: PathBuf = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openapi.json"));
    let json = ApiDoc::openapi().to_pretty_json()?;
    fs::write(&out, json)?;
    println!("wrote {}", out.display());
    Ok(())
}
