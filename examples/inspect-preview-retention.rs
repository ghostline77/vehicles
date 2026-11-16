//! Explain native input differences between a saved preview and a fresh build.
use anyhow::{Result, ensure};
use ghostline_red::assets::same_native_value;
use ghostline_vehicles::{model::Selection, publish, render};
use serde_json::json;
use std::{fs, path::Path};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2,
        "usage: inspect-preview-retention CATALOGUE RECEIPT"
    );
    let old: Selection = serde_json::from_slice(&fs::read(&args[1])?)?;
    let catalogue = publish::read(Path::new(&args[0]))?;
    let new = &catalogue.selections[&old.id];
    println!(
        "{}",
        json!({"selection":old.id,"matches":render::same_preview_inputs(&old,new),
        "old_resolution":old.resolution,"new_resolution":new.resolution,
        "old_components":old.components.len(),"new_components":new.components.len(),
        "bindings_match":same_native_value(&json!(old.material_bindings),&json!(new.material_bindings))})
    );
    for a in &old.components {
        if let Some(b) = new.components.iter().find(|b| a["name"] == b["name"]) {
            for (key, value) in a.as_object().into_iter().flatten() {
                if !same_native_value(value, &b[key]) {
                    println!(
                        "{}",
                        json!({"component":a["name"],"field":key,"old":value,"new":b[key]})
                    );
                }
            }
        }
    }
    Ok(())
}
