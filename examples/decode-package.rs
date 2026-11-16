//! Diagnose a native component package with explicit schema settings.
use anyhow::Result;
use ghostline_red::{
    redpackage::{self, PackageSettings},
    schema::RedSchema,
};
use std::{collections::BTreeSet, fs};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(args.len() == 2, "usage: decode-package INPUT OUTPUT");
    let schema = RedSchema::bundled()?;
    let bytes = fs::read(&args[0])?;
    let classes: BTreeSet<_> = schema.classes.keys().cloned().collect();
    let value = redpackage::decode(
        &bytes,
        &classes,
        PackageSettings {
            imports_as_hash: redpackage::imports_are_hashed(&bytes)?,
            handle_id_base: 1,
            enums: Some(&schema.enums),
            bitfields: Some(&schema.bitfields),
        },
    )?;
    fs::write(&args[1], serde_json::to_vec_pretty(&value)?)?;
    Ok(())
}
