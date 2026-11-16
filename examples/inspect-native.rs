//! Export selected native documents for developer inspection without GPU allocation.
use anyhow::Result;
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::GameSource;
use std::{fs, path::Path};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() >= 3,
        "usage: inspect-native GAME OUTPUT RESOURCE..."
    );
    let source = GameSource::open(Path::new(&args[0]), None)?;
    let out = source.validate_output(Path::new(&args[1]))?;
    fs::create_dir_all(&out)?;
    let mut reader = source.resources();
    for resource in &args[2..] {
        if let Some((resource, appearance)) = resource.split_once('#') {
            let key = ResourceKey::parse(resource);
            let assembly = reader.assembly_with_resource_overrides(
                &key,
                Some(appearance),
                &Default::default(),
                &ghostline_vehicles::build::resource_overrides(),
            )?;
            let path = out.join(format!("{}-assembly.json", key.native_hash()));
            let value = serde_json::json!({"components":assembly.components.iter().map(|c|serde_json::json!({"name":c.name,"parent":c.parent.component,"control":c.control.component,"rig":c.rig,
                "slots":c.slots.iter().map(|s|serde_json::json!({"name":s.name,"bone":s.bone})).collect::<Vec<_>>() })).collect::<Vec<_>>(),
                "rigs":assembly.rigs.iter().map(|(key,rig)|serde_json::json!({"resource":key,"bones":rig.bones.iter().map(|bone|&bone.name).collect::<Vec<_>>()})).collect::<Vec<_>>()});
            fs::write(&path, serde_json::to_vec_pretty(&value)?)?;
            println!("{}", path.display());
            continue;
        }
        let key = ResourceKey::parse(resource);
        let doc = match reader.document(&key) {
            Ok(doc) => doc,
            Err(error) => {
                eprintln!("decode {resource}: {error}");
                continue;
            }
        };
        let path = out.join(format!("{}.json", key.native_hash()));
        fs::write(&path, serde_json::to_vec_pretty(&*doc)?)?;
        println!("{}\t{resource}", path.display());
    }
    Ok(())
}
