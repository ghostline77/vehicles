//! Shared JSON/browser data and self-contained hosted/offline packages.
use crate::model::*;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path},
    time::Duration,
};

pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    let parent = path.parent().context("output parent")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.flush()?;
    for attempt in 0..8 {
        match temp.persist(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt == 7 || !matches!(e.error.raw_os_error(), Some(5 | 32 | 33)) {
                    return Err(e.error).with_context(|| format!("write {}", path.display()));
                }
                temp = e.file;
                std::thread::sleep(Duration::from_millis(20 << attempt));
            }
        }
    }
    unreachable!()
}

pub fn json_file(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    atomic(path, &serde_json::to_vec(value)?)
}

pub fn slug(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("~{b:02X}"));
        }
    }
    if out.is_empty() {
        out.push_str("default");
    }
    out
}

fn detail_id(kind: &str, id: &str) -> String {
    format!("data/browser/details/{kind}-{}.js", slug(id))
}

pub fn read(root: &Path) -> Result<Catalogue> {
    let d: Value = serde_json::from_slice(&fs::read(root.join("data/catalogue.json"))?)?;
    ensure!(
        d["format"] == "ghostline-vehicles" && d["version"] == 1,
        "not a supported vehicles catalogue"
    );
    let mut c = Catalogue {
        format: "ghostline-vehicles".into(),
        version: 1,
        sources: serde_json::from_value(d["sources"].clone())?,
        vehicles: BTreeMap::new(),
        selections: BTreeMap::new(),
        curves: serde_json::from_slice(&fs::read(root.join("data/curves.json"))?)?,
        diagnostics: serde_json::from_value(d["diagnostics"].clone())?,
    };
    for (kind, dest) in [("vehicles", true), ("selections", false)] {
        for file in d["collections"][kind]
            .as_array()
            .context("collection shards")?
        {
            let path = member(root, file.as_str().context("shard path")?)?;
            let rows: Vec<Value> = serde_json::from_slice(&fs::read(path)?)?;
            for row in rows {
                if dest {
                    let v: Vehicle = serde_json::from_value(row)?;
                    c.vehicles.insert(v.id.clone(), v);
                } else {
                    let s: Selection = serde_json::from_value(row)?;
                    c.selections.insert(s.id.clone(), s);
                }
            }
        }
    }
    Ok(c)
}

pub fn member(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let p = Path::new(relative);
    ensure!(
        !p.is_absolute() && p.components().all(|c| matches!(c, Component::Normal(_))),
        "unsafe package path {relative}"
    );
    let result = root.join(p);
    if result.exists() {
        ensure!(
            result.canonicalize()?.starts_with(root.canonicalize()?),
            "package member escapes its root"
        );
    }
    Ok(result)
}

/// Refreshes browser assets and their checksums without regenerating data or media.
///
/// # Errors
/// Returns an error for an invalid package manifest or failed atomic writes.
pub fn refresh_ui(root: &Path) -> Result<()> {
    let path = root.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path)?)?;
    ensure!(
        manifest["format"] == "ghostline-vehicles-package",
        "not a vehicles catalogue"
    );
    let files = manifest["files"]
        .as_array_mut()
        .context("manifest has no files")?;
    for (name, bytes) in [
        (
            "index.html",
            include_bytes!("../web/dist/index.html").as_slice(),
        ),
        (
            "assets/app.js",
            include_bytes!("../web/dist/assets/app.js").as_slice(),
        ),
        (
            "assets/style.css",
            include_bytes!("../web/dist/assets/style.css").as_slice(),
        ),
    ] {
        let entry = files
            .iter_mut()
            .find(|entry| entry["path"] == name)
            .context("browser asset absent from manifest")?;
        atomic(&root.join(name), bytes)?;
        *entry =
            json!({"path":name,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes))});
    }
    atomic(&path, &serde_json::to_vec_pretty(&manifest)?)
}

pub fn write(c: &Catalogue, root: &Path, media_base: &str) -> Result<()> {
    if root.exists() && fs::read_dir(root)?.next().is_some() {
        let old: Value = serde_json::from_slice(
            &fs::read(root.join("manifest.json"))
                .context("output is not an existing vehicles package")?,
        )?;
        ensure!(
            old["format"] == "ghostline-vehicles-package",
            "output belongs to another application"
        );
    }
    fs::create_dir_all(root)?;
    let mut files = Vec::<String>::new();
    let mut put = |path: &str, bytes: &[u8]| -> Result<()> {
        atomic(&root.join(path), bytes)?;
        files.push(path.into());
        Ok(())
    };
    put("index.html", include_bytes!("../web/dist/index.html"))?;
    put("assets/app.js", include_bytes!("../web/dist/assets/app.js"))?;
    put(
        "assets/style.css",
        include_bytes!("../web/dist/assets/style.css"),
    )?;
    let mut collections = BTreeMap::new();
    for (name, rows) in [
        (
            "vehicles",
            c.vehicles
                .values()
                .map(serde_json::to_value)
                .collect::<std::result::Result<Vec<_>, _>>()?,
        ),
        (
            "selections",
            c.selections
                .values()
                .map(serde_json::to_value)
                .collect::<std::result::Result<Vec<_>, _>>()?,
        ),
    ] {
        let mut paths = Vec::new();
        for (i, chunk) in rows.chunks(128).enumerate() {
            let path = format!("data/{name}/{i:04}.json");
            put(&path, &serde_json::to_vec(chunk)?)?;
            paths.push(path);
        }
        collections.insert(name, paths);
    }
    put(
        "data/catalogue.json",
        &serde_json::to_vec(
            &json!({"format":c.format,"version":c.version,"sources":c.sources,"collections":collections,"diagnostics":c.diagnostics}),
        )?,
    )?;
    put("data/curves.json", &serde_json::to_vec(&c.curves)?)?;
    put("data/coverage.json", &serde_json::to_vec(&c.coverage())?)?;
    put(
        "data/binding-report.json",
        &serde_json::to_vec(&json!({"source_diagnostics":c.diagnostics,
        "records":c.vehicles.values().filter(|v|!v.diagnostics.is_empty()).map(|v|json!({"record":v.id,"diagnostics":v.diagnostics})).collect::<Vec<_>>(),
        "selections":c.selections.values().filter(|s|!s.diagnostics.is_empty()).map(|s|json!({"selection":s.id,"resolution":s.resolution,"diagnostics":s.diagnostics})).collect::<Vec<_>>()}))?,
    )?;
    let mut records = Vec::new();
    for v in c.vehicles.values() {
        let detail = detail_id("record", &v.id);
        put(
            &detail,
            format!(
                "globalThis.GHOSTLINE_VEHICLE_DETAIL={};",
                serde_json::to_string(v)?
            )
            .as_bytes(),
        )?;
        records.push(json!({"id":v.id,"native_name":v.native_name,"label":v.label,"description":v.description,"manufacturer":v.manufacturer,
            "vehicle_type":v.vehicle_type,"entity":v.entity,"requested_appearance":v.requested_appearance,"tags":v.properties.get("tags").map(|p|&p.value),
            "performance":v.performance,"selections":v.selections,"detail":detail,"diagnostics":v.diagnostics}));
    }
    let mut selections = Vec::new();
    for s in c.selections.values() {
        let detail = detail_id("selection", &s.resource.native_hash().to_string())
            + &format!(".{}.js", slug(s.appearance.as_deref().unwrap_or_default()));
        put(
            &detail,
            format!(
                "globalThis.GHOSTLINE_VEHICLE_DETAIL={};",
                serde_json::to_string(s)?
            )
            .as_bytes(),
        )?;
        selections.push(json!({"id":s.id,"resource":s.resource,"appearance":s.appearance,"records":s.records,"resolution":s.resolution,
            "preview_state":s.preview_state,"media":s.media,"diagnostics":s.diagnostics,"detail":detail}));
    }
    put("data/browser/catalogue.js",format!("globalThis.GHOSTLINE_VEHICLES={};",serde_json::to_string(&json!({"records":records,"selections":selections,"sources":c.sources,"coverage":c.coverage(),"media_base":media_base}))?).as_bytes())?;
    let entries=files.into_iter().map(|path|{let bytes=fs::read(root.join(&path))?;Ok(json!({"path":path,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes))}))}).collect::<Result<Vec<_>>>()?;
    json_file(
        &root.join("manifest.json"),
        &json!({"format":"ghostline-vehicles-package","version":1,"sources":c.sources,"media_base":media_base,"files":entries,"media":c.selections.values().flat_map(|s|s.media.iter()).collect::<Vec<_>>()}),
    )
}

pub fn verify(root: &Path) -> Result<Value> {
    let c = read(root)?;
    let m: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    for file in m["files"]
        .as_array()
        .context("manifest files")?
        .iter()
        .chain(m["media"].as_array().context("manifest media")?.iter())
    {
        let relative = file["path"].as_str().context("manifest path")?;
        let bytes = fs::read(member(root, relative)?)
            .with_context(|| format!("missing package file {relative}"))?;
        ensure!(
            file["bytes"].as_u64() == Some(bytes.len() as u64),
            "size mismatch {relative}"
        );
        ensure!(
            file["sha256"].as_str() == Some(format!("{:x}", Sha256::digest(&bytes)).as_str()),
            "hash mismatch {relative}"
        );
    }
    for v in c.vehicles.values() {
        for id in &v.selections {
            ensure!(c.selections.contains_key(id), "missing selection {id}");
        }
    }
    for s in c.selections.values() {
        ensure!(s.resolution != "pending", "unfinished binding {}", s.id);
        for id in &s.records {
            ensure!(c.vehicles.contains_key(id), "missing record {id}");
        }
        ensure!(
            s.preview_state != "ready" || !s.media.is_empty(),
            "ready selection has no images: {}",
            s.id
        );
    }
    Ok(c.coverage())
}

pub fn package(source: &Path, output: &Path, media_base: &str) -> Result<()> {
    verify(source)?;
    fs::create_dir_all(output)?;
    let input = source.canonicalize()?;
    let dest = output.canonicalize()?;
    ensure!(
        !dest.starts_with(&input) && !input.starts_with(&dest),
        "package source and destination must be separate"
    );
    let c = read(source)?;
    write(&c, output, media_base)?;
    for media in c.selections.values().flat_map(|s| &s.media) {
        let target = member(output, &media.path)?;
        fs::create_dir_all(target.parent().context("media parent")?)?;
        fs::copy(member(source, &media.path)?, target)?;
    }
    prune_media(&c, output)?;
    verify(output)?;
    Ok(())
}

/// Removes unreferenced generated images and their empty directories.
///
/// # Errors
/// Rejects symlinks, paths outside the owned package, and filesystem failures.
pub fn prune_media(c: &Catalogue, root: &Path) -> Result<usize> {
    let root = fs::canonicalize(root)?;
    let media = root.join("media");
    if !media.exists() {
        return Ok(0);
    }
    ensure!(
        !fs::symlink_metadata(&media)?.file_type().is_symlink(),
        "media directory cannot be a symlink"
    );
    ensure!(
        fs::canonicalize(&media)?.starts_with(&root),
        "media escapes catalogue"
    );
    let used: std::collections::BTreeSet<_> = c
        .selections
        .values()
        .flat_map(|selection| selection.media.iter())
        .map(|image| {
            if cfg!(windows) {
                image.path.replace('\\', "/").to_lowercase()
            } else {
                image.path.replace('\\', "/")
            }
        })
        .collect();
    let mut pending = vec![media.clone()];
    let mut visited = Vec::new();
    let mut removed = 0;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            ensure!(!kind.is_symlink(), "media entries cannot be symlinks");
            ensure!(
                fs::canonicalize(&path)?.starts_with(&media),
                "media entry escapes owned media"
            );
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                let relative = path
                    .strip_prefix(&root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                let relative = if cfg!(windows) {
                    relative.to_lowercase()
                } else {
                    relative
                };
                if !used.contains(&relative) {
                    fs::remove_file(&path)
                        .with_context(|| format!("remove obsolete preview {}", path.display()))?;
                    removed += 1;
                }
            }
        }
        visited.push(directory);
    }
    for directory in visited.into_iter().rev() {
        if directory != media && fs::read_dir(&directory)?.next().is_none() {
            fs::remove_dir(&directory)
                .with_context(|| format!("remove empty media directory {}", directory.display()))?;
        }
    }
    Ok(removed)
}
