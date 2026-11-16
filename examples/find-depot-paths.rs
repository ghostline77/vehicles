//! Compare known native depot path spellings with the installed archive index.
use anyhow::{Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::GameSource;
use std::{fs, path::Path};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() >= 3,
        "usage: find-depot-paths GAME PATH_DICTIONARY QUERY..."
    );
    let source = GameSource::open(Path::new(&args[0]), None)?;
    let bytes = fs::read(&args[1])?;
    let bytes = if bytes.starts_with(b"KARK") {
        ensure!(bytes.len() >= 8, "truncated KARK dictionary");
        let size = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
        ghostline_red::archive::decompress_payload(&bytes[8..], size)?
    } else {
        bytes
    };
    let paths = String::from_utf8(bytes)?;
    let reader = source.resources();
    for query in &args[2..] {
        let matches: Vec<_> = paths.lines().filter(|path|path.contains(query))
            .map(|path| {let path=path.trim_end_matches('\r');let key=ResourceKey::parse(path);
                serde_json::json!({"path":path,"hash":key.native_hash().to_string(),"installed":reader.resource_exists(&key)})})
            .collect();
        println!(
            "{}",
            serde_json::json!({"query":query,"count":matches.len(),"matches":matches})
        );
    }
    Ok(())
}
