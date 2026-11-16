//! Concurrent GPU vehicle previews, validated renderer receipts and publication.
use crate::{model::*, publish};
use anyhow::{Context, Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::{
    AssemblyOptions, BatchJob, BatchLimits, BatchSource, Camera, Cancellation, GameSource,
    RenderOptions, Renderer, View,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::Path,
    sync::{Mutex, mpsc},
    time::Instant,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub origin_anchors: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slot_rig_overrides: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub duplicate_components: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resource_overrides: BTreeMap<String, ResourceKey>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub excluded_components: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slot_bone_overrides: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parent_component_overrides: BTreeMap<String, String>,
    pub size: u32,
    pub views: Vec<View>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Plan {
    pub format: String,
    pub version: u32,
    pub sources: BTreeMap<String, String>,
    pub jobs: Vec<Job>,
    pub blocked: Vec<Value>,
    pub not_applicable: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub selection: String,
    pub resource: ResourceKey,
    pub appearance: Option<String>,
    pub recipe: Recipe,
}

pub fn plan(c: &Catalogue, size: u32) -> Result<Plan> {
    ensure!((64..=4096).contains(&size), "size must be 64..4096");
    let mut p = Plan {
        format: "ghostline-vehicles-render-plan".into(),
        version: 1,
        sources: c.sources.clone(),
        jobs: Vec::new(),
        blocked: Vec::new(),
        not_applicable: Vec::new(),
    };
    for s in c.selections.values() {
        if s.resolution != "resolved" {
            p.blocked
                .push(json!({"selection":s.id,"diagnostics":s.diagnostics}));
        } else if s.preview_state == "not_applicable" {
            p.not_applicable.push(s.id.clone());
        } else {
            p.jobs.push(Job {
                selection: s.id.clone(),
                resource: s.resource.clone(),
                appearance: s.appearance.clone(),
                recipe: Recipe {
                    origin_anchors: match s.resource.native_hash() {
                        2268554714971898408 | 8190325089569015717 | 13701316129316251616 => {
                            ["TransformAnimator3503"]
                                .into_iter()
                                .map(str::to_owned)
                                .collect()
                        }
                        15308668192881310443 | 7429546577283304759 | 531571981639560208 => {
                            ["ChassisMesh"].into_iter().map(str::to_owned).collect()
                        }
                        15322297058930386240 => {
                            ["External"].into_iter().map(str::to_owned).collect()
                        }
                        1691620158560845678 => {
                            ["camera_slots"].into_iter().map(str::to_owned).collect()
                        }
                        8079861380283144540 => ["root"].into_iter().map(str::to_owned).collect(),
                        14921115243869866848 => ["TransformAnimator4677"]
                            .into_iter()
                            .map(str::to_owned)
                            .collect(),
                        _ => BTreeSet::new(),
                    },
                    slot_rig_overrides: if matches!(
                        s.resource.native_hash(),
                        2877158406812214398 | 17078767364920982950
                    ) {
                        BTreeMap::from([("Slot3612".into(), "deformation_rig".into())])
                    } else {
                        BTreeMap::new()
                    },
                    duplicate_components: if s.appearance.as_deref()
                        == Some("villefort_cortes__basic_sq012_peralez")
                    {
                        BTreeMap::from([(
                            "v_standard2_villefort_cortes__int01_armrest_01".into(),
                            "armrest_01".into(),
                        )])
                    } else {
                        BTreeMap::new()
                    },
                    resource_overrides: s
                        .components
                        .iter()
                        .flat_map(|component| {
                            ["requested_mesh", "requested_rig"]
                                .into_iter()
                                .filter_map(|field| {
                                    serde_json::from_value::<ResourceKey>(component[field].clone())
                                        .ok()
                                })
                        })
                        .filter_map(|key| {
                            crate::build::resource_overrides()
                                .get(&key.native_hash().to_string())
                                .cloned()
                                .map(|value| (key.native_hash().to_string(), value))
                        })
                        .collect(),
                    excluded_components: s
                        .material_bindings
                        .iter()
                        .filter(|binding| binding["preview_excluded"] == true)
                        .filter_map(|binding| binding["component"].as_str().map(str::to_owned))
                        .collect(),
                    slot_bone_overrides: if s.resource.native_hash() == 18_254_919_793_398_716_638 {
                        // This intercom retains chassis-space local coordinates;
                        // its obsolete trunk_d slot has no bone in the van rig.
                        // Freeze that static decoration to the native Base frame.
                        BTreeMap::from([("vehicle_slots/trunk_d".into(), "Base".into())])
                    } else if s.resource.native_hash() == 5_376_322_257_826_956_230 {
                        // Valgus Slot4436 references lowercase `base`; its native
                        // owning skeleton declares the exact CName `Base`.
                        ["fake_light1", "fake_light2", "light", "scan_fx"]
                            .into_iter()
                            .map(|slot| (format!("Slot4436/{slot}"), "Base".into()))
                            .collect()
                    } else {
                        BTreeMap::new()
                    },
                    parent_component_overrides: preview_parent_corrections(s),
                    size,
                    views: vec![View::Hero, View::Front, View::Back, View::Left, View::Right],
                },
            });
        }
    }
    Ok(p)
}

// These stock Colby sticker components retain obsolete host names. Keep the
// native bindings in data, and use a reviewed preview correction only when the
// exact canonical window mesh and component are present and the old host is not.
fn preview_parent_corrections(selection: &Selection) -> BTreeMap<String, String> {
    let mut corrections = BTreeMap::new();
    if selection.resource.native_hash() == 13_676_228_406_475_381_527
        && selection.components.iter().any(|component|component["name"] == "root"
            && serde_json::from_value::<ResourceKey>(component["rig"].clone()).is_ok_and(|rig|
                rig.native_hash() == ResourceKey::parse(r"base\vehicles\special\av_zetatech_bombus\rig\av_zetatech_bombus__basic_01.rig").native_hash())) {
        // The isolated Nanny shell needs its owning Bombus skeleton rather than
        // the external actor binding supplied only by a runtime composition.
        for component in &selection.components {
            if component["parent"]["component"] == "External0367"
                && component["class"] == "entSkinnedMeshComponent"
                && let Some(name) = component["name"].as_str() {
                corrections.insert(name.to_owned(), "root".into());
            }
        }
    }
    for (old, host, mesh) in [
        (
            "v_standard25_thorton_colby_pickup__ext01_window_fr_01",
            "window_fr",
            10_314_678_298_949_433_143_u64,
        ),
        (
            "v_standard25_thorton_colby_pickup__ext01_window_fl_01",
            "window_fl",
            6_875_084_270_591_402_521_u64,
        ),
    ] {
        if selection
            .components
            .iter()
            .any(|component| component["name"] == old)
        {
            continue;
        }
        if !selection.components.iter().any(|component| {
            component["name"] == host
                && serde_json::from_value::<ResourceKey>(component["mesh"].clone())
                    .is_ok_and(|key| key.native_hash() == mesh)
        }) {
            continue;
        }
        for component in &selection.components {
            if component["parent"]["component"] == old
                && let Some(child) = component["name"].as_str()
            {
                corrections.insert(child.to_owned(), host.to_owned());
            }
        }
    }
    corrections
}

pub struct RunOptions<'a> {
    pub workers: usize,
    pub cancel: &'a Cancellation,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Compares the visible native graph, excluding unrelated editor/physics nodes.
pub fn same_preview_inputs(a: &Selection, b: &Selection) -> bool {
    fn canonical(mut value: Value) -> Value {
        fn visit(value: &mut Value) {
            if value.as_object().is_some_and(|fields| {
                fields.len() == 1 && (fields.contains_key("Hash") || fields.contains_key("Path"))
            }) && let Ok(resource) = serde_json::from_value::<ResourceKey>(value.clone())
            {
                *value = json!({"Hash":resource.native_hash().to_string()});
                return;
            }
            match value {
                Value::Object(fields) => {
                    for child in fields.values_mut() {
                        visit(child);
                    }
                }
                Value::Array(values) => {
                    for child in values {
                        visit(child);
                    }
                }
                _ => {}
            }
        }
        visit(&mut value);
        value
    }
    let excluded: BTreeSet<_> = [a, b]
        .into_iter()
        .flat_map(|selection| selection.material_bindings.iter())
        .filter(|binding| binding["preview_excluded"] == true)
        .filter_map(|binding| binding["component"].as_str().map(str::to_owned))
        .collect();
    let graph = |selection: &Selection| {
        let components: BTreeMap<_, _> = selection
            .components
            .iter()
            .filter_map(|component| Some((component["name"].as_str()?, component)))
            .collect();
        let shadows: BTreeSet<_> = selection
            .material_bindings
            .iter()
            .filter(|material| material["binding"]["shadow_only"] == true)
            .filter_map(|material| material["component"].as_str())
            .collect();
        let mut pending: Vec<_> = components
            .iter()
            .filter(|(name, component)| {
                !component["mesh"].is_null()
                    && component["enabled"] == true
                    && component["chunk_mask"] != "0"
                    && component["class"] != "entAppearanceProxyMeshComponent"
                    && !shadows.contains(*name)
                    && !excluded.contains(**name)
            })
            .map(|(name, _)| *name)
            .collect();
        let mut graph = BTreeMap::new();
        while let Some(name) = pending.pop() {
            if graph.contains_key(name) {
                continue;
            }
            let component = components.get(name).copied();
            graph.insert(name, component.cloned().unwrap_or(Value::Null));
            if let Some(component) = component {
                for key in ["parent", "control"] {
                    if let Some(name) = component[key]["component"].as_str()
                        && !name.is_empty()
                        && !name.eq_ignore_ascii_case("none")
                    {
                        pending.push(name);
                    }
                }
            }
        }
        json!(graph)
    };
    a.resource == b.resource
        && a.appearance == b.appearance
        && a.resolution == "resolved"
        && b.resolution == "resolved"
        && ghostline_red::assets::same_native_value(&canonical(graph(a)), &canonical(graph(b)))
        && ghostline_red::assets::same_native_value(
            &canonical(json!(
                a.material_bindings
                    .iter()
                    .filter(|binding| binding["component"]
                        .as_str()
                        .is_none_or(|name| !excluded.contains(name)))
                    .collect::<Vec<_>>()
            )),
            &canonical(json!(
                b.material_bindings
                    .iter()
                    .filter(|binding| binding["component"]
                        .as_str()
                        .is_none_or(|name| !excluded.contains(name)))
                    .collect::<Vec<_>>()
            )),
        )
}

/// Republishes completed frames from a same-source render cache. Native bindings
/// and saved frame hashes must still agree; normal recipe checks remain in run.
pub fn restore_cached(c: &mut Catalogue, package: &Path, work: &Path) -> Result<usize> {
    let receipts = work.join("receipts");
    let workers = work.join("workers");
    if !receipts.is_dir() || !workers.is_dir() {
        return Ok(0);
    }
    let mut directories = BTreeMap::new();
    for worker in fs::read_dir(workers)? {
        let worker = worker?.path();
        if !worker.is_dir() {
            continue;
        }
        for job in fs::read_dir(worker)? {
            let directory = job?.path();
            let path = directory.join("assembly-inspection.json");
            if !path.is_file() {
                continue;
            }
            let inspection: Value = serde_json::from_slice(&fs::read(path)?)?;
            let key: ResourceKey = serde_json::from_value(inspection["source"].clone())?;
            let id = format!(
                "{}#{}",
                key.native_hash(),
                inspection["appearance"].as_str().unwrap_or_default()
            );
            directories.insert(id, directory);
        }
    }
    let mut restored = 0;
    for receipt in fs::read_dir(receipts)? {
        let path = receipt?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let prior: Selection = serde_json::from_slice(&fs::read(path)?)?;
        if prior.preview_state != "ready" || prior.media.is_empty() {
            continue;
        }
        let Some(selection) = c.selections.get_mut(&prior.id) else {
            continue;
        };
        if !same_preview_inputs(selection, &prior) {
            continue;
        }
        let Some(directory) = directories.get(&prior.id) else {
            continue;
        };
        let frames = prior
            .media
            .iter()
            .enumerate()
            .map(|(i, media)| {
                let bytes = fs::read(directory.join(format!("images/{i:06}.png")))?;
                ensure!(
                    bytes.len() as u64 == media.bytes
                        && format!("{:x}", Sha256::digest(&bytes)) == media.sha256,
                    "cached frame differs from receipt for {}",
                    prior.id
                );
                Ok(bytes)
            })
            .collect::<Result<Vec<_>>>()?;
        for (media, bytes) in prior.media.iter().zip(frames) {
            let path = publish::member(package, &media.path)?;
            publish::atomic(&path, &bytes)?;
        }
        selection.media = prior.media;
        selection.preview_state = "ready".into();
        selection.recipe = prior.recipe;
        selection.diagnostics.retain(|d| !d.starts_with("render: "));
        restored += 1;
    }
    eprintln!(
        "Restored {restored} completed previews from {}",
        work.display()
    );
    Ok(restored)
}

pub fn run(
    c: &mut Catalogue,
    source: &GameSource,
    plan: &Plan,
    package: &Path,
    work: &Path,
    options: RunOptions<'_>,
) -> Result<Value> {
    ensure!(
        plan.format == "ghostline-vehicles-render-plan"
            && plan.version == 1
            && plan.sources == c.sources,
        "plan does not match catalogue sources"
    );
    ensure!(options.workers > 0, "workers must be positive");
    ensure!(
        options.offset <= plan.jobs.len(),
        "offset exceeds render plan"
    );
    // Catalogue provenance may predate a game update. Resolve selected native
    // resources from the installed game and scope new previews to that source.
    let source_identity = source.content_identity();
    let mut queue = VecDeque::new();
    let mut reused = 0;
    let mut unique = BTreeSet::new();
    for job in plan
        .jobs
        .iter()
        .skip(options.offset)
        .take(options.limit.unwrap_or(usize::MAX))
    {
        ensure!(
            (64..=4096).contains(&job.recipe.size)
                && !job.recipe.views.is_empty()
                && job.recipe.views.len() <= 5,
            "invalid recipe"
        );
        let s = c
            .selections
            .get_mut(&job.selection)
            .context("selection absent")?;
        ensure!(
            unique.insert(&job.selection),
            "duplicate render selection {}",
            job.selection
        );
        let views = job
            .recipe
            .views
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        ensure!(
            views.len() == job.recipe.views.len(),
            "duplicate semantic render views"
        );
        ensure!(
            s.resolution == "resolved"
                && s.resource == job.resource
                && s.appearance == job.appearance,
            "job binding mismatch"
        );
        let recipe = json!({"recipe":job.recipe,"renderer":ghostline_renderer::BUILD_ID,"source_identity":source_identity});
        if s.recipe["identity"] == recipe
            && s.preview_state == "ready"
            && s.media.len() == job.recipe.views.len()
            && s.media
                .iter()
                .all(|m| fs::metadata(package.join(&m.path)).is_ok_and(|f| f.len() == m.bytes))
        {
            reused += 1;
            continue;
        }
        for old in &s.media {
            let path = publish::member(package, &old.path)?;
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
        s.media.clear();
        s.preview_state = "pending".into();
        s.recipe = json!({"identity":recipe});
        queue.push_back(job.clone());
    }
    let scheduled = queue.len();
    eprintln!(
        "Reusing {reused} previews; {scheduled} selections scheduled on {} GPU workers",
        options.workers
    );
    fs::create_dir_all(work)?;
    let media_base = fs::read(package.join("manifest.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|m| m["media_base"].as_str().map(str::to_owned))
        .unwrap_or_default();
    publish::write(c, package, &media_base)?;
    let queue = Mutex::new(queue);
    let (tx, rx) = mpsc::channel();
    let mut outcomes = Vec::new();
    let mut checkpoint = Instant::now();
    let execution = std::thread::scope(|scope| -> Result<()> {
        for worker in 0..options.workers.min(scheduled) {
            let tx = tx.clone();
            let queue = &queue;
            let cancel = options.cancel;
            scope.spawn(move || {
                let renderer = match pollster::block_on(Renderer::new()) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("GPU worker {worker}: {e}")));
                        return;
                    }
                };
                eprintln!("GPU worker {worker}: {:?}", renderer.adapter_info());
                loop {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let job = queue
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .pop_front();
                    let Some(job) = job else {
                        break;
                    };
                    let result = (|| -> Result<_> {
                        let request = BatchJob {
                            native_frame: None,
                            name: job.selection.clone(),
                            source: BatchSource::Assembly {
                                resource: job.resource.clone(),
                                appearance: job.appearance.clone(),
                                options: Box::new(AssemblyOptions {
                                    origin_anchors: job.recipe.origin_anchors.clone(),
                                    slot_rig_overrides: job.recipe.slot_rig_overrides.clone(),
                                    duplicate_components: job.recipe.duplicate_components.clone(),
                                    resource_overrides: job.recipe.resource_overrides.clone(),
                                    excluded_components: job.recipe.excluded_components.clone(),
                                    slot_bone_overrides: job.recipe.slot_bone_overrides.clone(),
                                    parent_component_overrides: job
                                        .recipe
                                        .parent_component_overrides
                                        .clone(),
                                    omit_unbound_duplicates: true,
                                    ..Default::default()
                                }),
                            },
                            views: job
                                .recipe
                                .views
                                .iter()
                                .map(|v| RenderOptions {
                                    size: job.recipe.size,
                                    camera: Camera::FrameProjected(*v),
                                })
                                .collect(),
                        };
                        let root = work.join(format!("workers/{worker}"));
                        let report = renderer.run_batch(
                            source,
                            &[request],
                            &root,
                            BatchLimits {
                                views_per_job: 5,
                                ..Default::default()
                            },
                            cancel,
                            |_| {},
                        )?;
                        Ok((job, root, report))
                    })();
                    if tx.send(result).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for result in rx {
            let (job, root, report) = match result {
                Ok(v) => v,
                Err(e) => {
                    options.cancel.cancel();
                    return Err(e);
                }
            };
            let Some(result) = report.jobs.first() else {
                continue;
            };
            let s = c
                .selections
                .get_mut(&job.selection)
                .context("known selection")?;
            s.diagnostics.retain(|d| !d.starts_with("render: "));
            if result.non_visual {
                s.preview_state = "not_applicable".into();
                s.media.clear();
            } else if let Some(error) = &result.error {
                s.preview_state = "failed".into();
                s.diagnostics.push(format!("render: {error}"));
            } else if !report.cancelled && result.output.is_some() {
                let mut media = Vec::new();
                for (index, view) in job.recipe.views.iter().enumerate() {
                    let view = serde_json::to_value(view)?
                        .as_str()
                        .context("semantic view")?
                        .to_owned();
                    let path = format!(
                        "media/images/{}/{}/{}px/{view}.png",
                        job.resource.native_hash(),
                        publish::slug(job.appearance.as_deref().unwrap_or_default()),
                        job.recipe.size
                    );
                    let image = root
                        .join(&result.directory)
                        .join("images")
                        .join(format!("{index:06}.png"));
                    let bytes = fs::read(&image)
                        .with_context(|| format!("missing rendered image {}", image.display()))?;
                    publish::atomic(&package.join(&path), &bytes)?;
                    media.push(Media {
                        path,
                        view,
                        width: job.recipe.size,
                        height: job.recipe.size,
                        bytes: bytes.len() as u64,
                        sha256: format!("{:x}", Sha256::digest(&bytes)),
                        mime: "image/png".into(),
                    });
                }
                s.media = media;
                s.preview_state = "ready".into();
                let inspection: Value = serde_json::from_slice(&fs::read(
                    root.join(&result.directory)
                        .join("assembly-inspection.json"),
                )?)?;
                let notes = inspection["assets"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|asset| asset["materials"].as_array().into_iter().flatten())
                    .flat_map(|material| material["notes"].as_array().into_iter().flatten())
                    .filter_map(Value::as_str)
                    .chain(
                        inspection["preview_notes"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str),
                    )
                    .collect::<BTreeSet<_>>();
                s.recipe["inspection"] = json!({"cameras":inspection["cameras"],"lighting":inspection["lighting"],
                    "coordinate_system":inspection["coordinate_system"],"material_notes":notes});
            }
            let outcome = json!({"selection":s.id,"state":s.preview_state,"error":result.error,"output":result.output});
            eprintln!("{}: {}", s.id, s.preview_state);
            publish::json_file(
                &work.join(format!(
                    "receipts/{}-{}.json",
                    s.resource.native_hash(),
                    publish::slug(s.appearance.as_deref().unwrap_or_default())
                )),
                s,
            )?;
            outcomes.push(outcome);
            publish::json_file(
                &work.join("render-progress.json"),
                &json!({"scheduled":scheduled,"reported":outcomes.len(),"reused":reused,"last":outcomes.last()}),
            )?;
            if checkpoint.elapsed().as_secs() >= 30 {
                publish::write(c, package, &media_base)?;
                checkpoint = Instant::now();
            }
        }
        Ok(())
    });
    publish::write(c, package, &media_base)?;
    publish::prune_media(c, package)?;
    execution?;
    let report = json!({"scheduled":scheduled,"reused":reused,"cancelled":options.cancel.is_cancelled(),"outcomes":outcomes});
    publish::json_file(&work.join("render-report.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod retention_tests {
    use super::*;
    #[test]
    fn unrelated_physics_nodes_do_not_invalidate_visible_graphs() {
        let make = || {
            serde_json::from_value::<Selection>(json!({
            "id":"42#default","resource":{"Hash":"42"},"appearance":"default",
            "records":[],"resolution":"resolved","components":[{
                "name":"body","class":"entMeshComponent","mesh":{"Hash":"43"},
                "enabled":true,"chunk_mask":"1","local_transform":[0.09999847412109376],
                "parent":{"component":""},"control":{"component":""}}],
            "material_bindings":[],"diagnostics":[],"preview_state":"ready","media":[],"recipe":{}
        })).unwrap()
        };
        let prior = make();
        let mut current = make();
        current.components[0]["local_transform"][0] = json!(0.09999847412109375);
        current
            .components
            .push(json!({"name":"Chassis","class":"vehicleChassisComponent","mesh":null}));
        assert!(same_preview_inputs(&prior, &current));
        current.components[0]["parent"]["component"] = json!("Chassis");
        assert!(!same_preview_inputs(&prior, &current));
    }
}
