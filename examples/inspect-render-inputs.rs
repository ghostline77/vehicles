//! Inspect native component graphs and weighted mesh palettes without rendering.
use anyhow::{Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::GameSource;
use serde_json::json;
use std::{collections::BTreeSet, path::Path};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: inspect-render-inputs GAME mesh|assembly RESOURCE[#APPEARANCE]"
    );
    let source = GameSource::open(Path::new(&args[0]), None)?;
    let (key, appearance) = args[2]
        .split_once('#')
        .map_or((args[2].as_str(), None), |(a, b)| (a, Some(b)));
    let key = ResourceKey::parse(key);
    let output = if args[1] == "mesh" {
        let mesh = source.load_geometry(&key)?;
        let used: BTreeSet<_> = mesh
            .chunks()
            .iter()
            .flat_map(|chunk| chunk.skin.iter())
            .flat_map(|vertex| vertex.joints.iter().copied().zip(vertex.weights))
            .filter_map(|(joint, weight)| (weight > 0.).then_some(usize::from(joint)))
            .collect();
        json!({"appearances":mesh.appearance_names(),"shadow":mesh.is_shadow_only(),"bones":mesh.skin().map(|skin|skin.names.iter().enumerate().map(|(i,name)|json!({"index":i,"name":name,"used":used.contains(&i)})).collect::<Vec<_>>()),"chunks":mesh.chunks().iter().map(|chunk|json!({"index":chunk.chunk_index,"vertices":chunk.positions.len(),"triangles":chunk.indices.len()/3,"weighted_vertices":chunk.skin.iter().filter(|v|v.weights.iter().any(|w|*w>0.)).count()})).collect::<Vec<_>>()})
    } else {
        let assembly = source.resolve_assembly(&key, appearance)?;
        json!({"components":assembly.components.iter().map(|c|json!({"name":c.name,"kind":c.kind,"mesh":c.mesh,"appearance":c.mesh_appearance,"enabled":c.enabled,"chunk_mask":c.chunk_mask.to_string(),"parent":{"component":c.parent.component,"slot":c.parent.slot},"control":{"component":c.control.component,"slot":c.control.slot},"rig":c.rig,"slots":c.slots.iter().map(|s|json!({"name":s.name,"bone":s.bone})).collect::<Vec<_>>(),"template":&*c.template})).collect::<Vec<_>>()})
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
