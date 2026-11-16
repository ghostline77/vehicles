//! Compiled native vehicle discovery and automatic assembly binding resolution.
use crate::{model::*, performance};
use anyhow::{Context, Result};
use ghostline_red::{
    assets::{AssetReader, ResourceKey},
    schema::RedSchema,
    tweakdb::{self, FlatValue, RecordTypes, TweakDb, TweakId, TweakNames},
};
use ghostline_renderer::GameSource;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub struct Inputs<'a> {
    pub game: &'a Path,
    pub schema: Option<&'a Path>,
    pub tweakdb: &'a Path,
    pub names: Option<&'a Path>,
    pub language: &'a str,
    pub game_version: &'a str,
}

const FIELDS: &str = "displayName localizedName localizedDescription entityTemplatePath appearanceName vehicleUIData vehEngineData vehDriveModelData manufacturer type tags icon isArmoredVehicle weapons vehicleWeapon vehicleDataPackage vehWheelDimensionsSetup vehWheelSetup vehAudioResource vehCameraData price purchasePrice unlockPrereq enableDestruction";
const RELATED: &str = "name enumName info productionYear driveLayout horsepower mass total_mass engineMaxTorque maxRPM minRPM gears gearCurvesPath minSpeed maxSpeed torqueMultiplier forceMultiplier transmissionEfficiency finalGearRatio gearRatio airResistanceFactor chassis_mass wheel_mass maxWheelTurnDeg wheelTurnMaxAddPerSecond wheelTurnMaxSubPerSecond";

pub fn properties(
    db: &TweakDb,
    names: &TweakNames,
    id: TweakId,
    known: &str,
) -> BTreeMap<String, FlatValue> {
    let mut fields: BTreeSet<_> = known.split_whitespace().map(str::to_owned).collect();
    if let Some(p) = names.properties.get(&id) {
        fields.extend(p.keys().cloned());
    }
    fields
        .into_iter()
        .filter_map(|field| db.property(id, &field).map(|v| (field, v.clone())))
        .collect()
}

fn linked(db: &TweakDb, names: &TweakNames, id: TweakId, field: &str) -> Option<String> {
    let target = TweakId(db.property(id, field)?.value.as_str()?.parse().ok()?);
    if target.0 == 0 {
        return None;
    }
    ["enumName", "name"]
        .into_iter()
        .find_map(|p| db.property(target, p)?.value.as_str().map(str::to_owned))
        .or_else(|| names.records.get(&target).cloned())
        .or_else(|| Some(target.to_string()))
}

pub fn text(value: &Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.get("$value").and_then(Value::as_str))
}

fn localization(
    reader: &mut AssetReader<'_>,
    language: &str,
    diagnostics: &mut Vec<String>,
) -> BTreeMap<String, String> {
    fn visit(v: &Value, out: &mut BTreeMap<String, String>) {
        if let Some(key) = v.get("primaryKey").and_then(text)
            && let Some(label) = ["femaleVariant", "maleVariant"]
                .into_iter()
                .find_map(|field| v.get(field).and_then(text).filter(|s| !s.is_empty()))
        {
            out.insert(key.into(), label.into());
            if let Some(key) = v
                .get("secondaryKey")
                .and_then(text)
                .filter(|s| !s.is_empty())
            {
                out.insert(key.into(), label.into());
            }
        }
        match v {
            Value::Object(map) => {
                for v in map.values() {
                    visit(v, out)
                }
            }
            Value::Array(list) => {
                for v in list {
                    visit(v, out)
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeMap::new();
    for prefix in ["base", "ep1"] {
        let resource = ResourceKey::parse(&format!(
            "{prefix}/localization/{language}/onscreens/onscreens_final.json"
        ));
        match reader.document(&resource) {
            Ok(v) => visit(&v, &mut out),
            Err(e) => diagnostics.push(format!("localization {resource}: {e}")),
        }
    }
    out
}

fn resource_aliases(reader: &mut AssetReader<'_>) -> BTreeMap<u64, ResourceKey> {
    let mut aliases = BTreeMap::new();
    for prefix in ["base", "ep1"] {
        let Ok(registry) = reader.table(&ResourceKey::parse(&format!(
            "{prefix}/gameplay/factories.csv"
        ))) else {
            continue;
        };
        for path in registry.rows.iter().filter_map(|r| r.first()) {
            let Ok(table) = reader.table(&ResourceKey::parse(path)) else {
                continue;
            };
            let Some(column) = table.headers.iter().position(|h| h == "path") else {
                continue;
            };
            for path in table
                .rows
                .iter()
                .filter_map(|row| row.get(column))
                .filter(|s| !s.is_empty())
            {
                let key = ResourceKey::parse(path);
                aliases.insert(key.native_hash(), key);
            }
        }
    }
    aliases
}

pub fn build(input: &Inputs<'_>) -> Result<Catalogue> {
    let source = GameSource::open(input.game, input.schema)?;
    let schema = RedSchema::load(Some(input.game), input.schema)?;
    let types = RecordTypes::new(&schema);
    let bytes = fs::read(input.tweakdb)?;
    let checksum = format!("{:x}", Sha256::digest(&bytes));
    let db = TweakDb::from_bytes(&bytes)?;
    drop(bytes);
    let names = if let Some(path) = input.names {
        TweakNames::resolve(&db, tweakdb::read_names(&fs::read(path)?)?)
    } else {
        TweakNames::default()
    };
    let mut c=Catalogue{format:"ghostline-vehicles".into(),version:1,sources:BTreeMap::from([
        ("game_version".into(),input.game_version.into()),("language".into(),input.language.into()),
        ("tweakdb_sha256".into(),checksum),("archives_schema_identity".into(),source.content_identity()),
        ("tweakdb_file".into(),input.tweakdb.file_name().unwrap_or_default().to_string_lossy().into()),
        ("scope".into(),"Selected complete compiled cache and unmodified base/EP1 archives; no runtime substitutions or inferred spawnability".into())]),
        vehicles:BTreeMap::new(),selections:BTreeMap::new(),curves:BTreeMap::new(),diagnostics:Vec::new()};
    if let Some(path) = input.names {
        c.sources.insert(
            "names_sha256".into(),
            format!("{:x}", Sha256::digest(fs::read(path)?)),
        );
    }
    let localized = localization(&mut source.resources(), input.language, &mut c.diagnostics);
    let aliases = resource_aliases(&mut source.resources());
    let mut entities = BTreeMap::<String, Vec<String>>::new();
    for (&id, &class) in &db.records {
        if !types.is_a(class, "gamedataVehicle_Record") {
            continue;
        }
        let props = properties(&db, &names, id, FIELDS);
        let mut related = BTreeMap::new();
        for field in ["vehicleUIData", "vehEngineData", "vehDriveModelData"] {
            if let Some(target) = props
                .get(field)
                .and_then(|p| p.value.as_str())
                .and_then(|s| s.parse().ok())
                .map(TweakId)
            {
                related.insert(field.into(), properties(&db, &names, target, RELATED));
            }
        }
        let label = props
            .get("displayName")
            .and_then(|p| p.value.as_str())
            .and_then(|s| localized.get(s))
            .cloned()
            .or_else(|| names.records.get(&id).cloned())
            .unwrap_or_else(|| format!("Vehicle {id}"));
        let description = related
            .get("vehicleUIData")
            .and_then(|p| p.get("info"))
            .and_then(|p| p.value.as_str())
            .and_then(|s| localized.get(s))
            .cloned();
        let entity = props
            .get("entityTemplatePath")
            .and_then(|p| p.value.as_str())
            .filter(|s| !s.is_empty() && *s != "0")
            .map(ResourceKey::parse)
            .map(|key| aliases.get(&key.native_hash()).cloned().unwrap_or(key));
        let requested_appearance = props
            .get("appearanceName")
            .and_then(|p| p.value.as_str())
            .filter(|s| !s.is_empty() && *s != "None")
            .map(str::to_owned);
        let key = id.to_string();
        if let Some(entity) = &entity {
            entities
                .entry(entity.to_string())
                .or_default()
                .push(key.clone());
        }
        c.vehicles.insert(
            key.clone(),
            Vehicle {
                id: key,
                native_id: id,
                native_name: names.records.get(&id).cloned(),
                label,
                description,
                native_class: types.name(class).unwrap_or("unknown").into(),
                manufacturer: linked(&db, &names, id, "manufacturer"),
                vehicle_type: linked(&db, &names, id, "type"),
                properties: props,
                related,
                performance: Value::Null,
                entity,
                requested_appearance,
                selections: Vec::new(),
                diagnostics: Vec::new(),
            },
        );
    }
    anyhow::ensure!(
        !c.vehicles.is_empty(),
        "selected compiled cache contains no recognized vehicle records"
    );
    eprintln!(
        "Discovered {} vehicle records and {} native roots",
        c.vehicles.len(),
        entities.len()
    );
    let mut reader = source.resources();
    let mut curve_paths = BTreeSet::from([performance::HUD_RESOURCE.to_owned()]);
    for v in c.vehicles.values() {
        if let Some(path) = v
            .related
            .get("vehEngineData")
            .and_then(|p| p.get("gearCurvesPath"))
            .and_then(|p| p.value.as_str())
            .filter(|s| *s != "0")
        {
            curve_paths.insert(path.into());
        }
    }
    for path in curve_paths {
        match reader.document(&ResourceKey::parse(&path)) {
            Ok(doc) => {
                let data = doc.pointer("/Data/RootChunk").unwrap_or(&doc);
                c.curves.insert(path,json!({"document":data,"sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(data)?))}));
            }
            Err(e) => c.diagnostics.push(format!("curve {path}: {e}")),
        }
    }
    drop(reader);
    for v in c.vehicles.values_mut() {
        v.performance = performance::calculate(v, &db, &names, &c.curves);
    }
    resolve_roots(&mut c, &source, entities)?;
    for v in c.vehicles.values_mut().filter(|v| v.entity.is_none()) {
        v.diagnostics
            .push("No native entity reference; metadata-only record".into());
    }
    eprintln!("Catalogue coverage: {}", c.coverage());
    Ok(c)
}

fn resolve_roots(
    c: &mut Catalogue,
    source: &GameSource,
    entities: BTreeMap<String, Vec<String>>,
) -> Result<()> {
    use std::{
        collections::VecDeque,
        sync::{Mutex, mpsc},
    };
    let count = entities.len();
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8)
        .min(count);
    let queue = Mutex::new(VecDeque::from_iter(entities));
    let (tx, rx) = mpsc::channel();
    eprintln!("Resolving native roots with {workers} workers");
    std::thread::scope(|scope| -> Result<()> {
        for _ in 0..workers {
            let queue = &queue;
            let tx = tx.clone();
            scope.spawn(move || {
                let mut bindings = BTreeMap::new();
                loop {
                    let Some((resource, records)) = queue
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .pop_front()
                    else {
                        break;
                    };
                    let resource = ResourceKey::parse(&resource);
                    let result = (|| -> std::result::Result<Vec<Selection>, String> {
                        let mut reader = source.resources();
                        let entries = reader.appearances(&resource).map_err(|e| e.to_string())?;
                        let appearances: Vec<_> = if entries.is_empty() {
                            vec![None]
                        } else {
                            entries.into_iter().map(|v| Some(v.name)).collect()
                        };
                        Ok(appearances
                            .into_iter()
                            .map(|appearance| {
                                let id = format!(
                                    "{}#{}",
                                    resource.native_hash(),
                                    appearance.as_deref().unwrap_or_default()
                                );
                                let mut s = Selection {
                                    id,
                                    resource: resource.clone(),
                                    appearance,
                                    records: Vec::new(),
                                    resolution: "blocked".into(),
                                    components: Vec::new(),
                                    material_bindings: Vec::new(),
                                    diagnostics: Vec::new(),
                                    preview_state: "blocked".into(),
                                    media: Vec::new(),
                                    recipe: Value::Null,
                                };
                                resolve(&mut s, &mut reader, &mut bindings);
                                s
                            })
                            .collect())
                    })();
                    if tx.send((resource, records, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (index, (resource, records, result)) in rx.into_iter().enumerate() {
            match result {
                Err(error) => {
                    for id in &records {
                        c.vehicles
                            .get_mut(id)
                            .context("known vehicle")?
                            .diagnostics
                            .push(format!("appearance discovery {resource}: {error}"));
                    }
                }
                Ok(selections) => {
                    for mut s in selections {
                        for id in &records {
                            let v = c.vehicles.get_mut(id).context("known vehicle")?;
                            if v.requested_appearance
                                .as_deref()
                                .is_none_or(|a| a == "random" || Some(a) == s.appearance.as_deref())
                            {
                                v.selections.push(s.id.clone());
                                s.records.push(id.clone());
                            }
                        }
                        c.selections.insert(s.id.clone(), s);
                    }
                    for id in &records {
                        let v = c.vehicles.get_mut(id).context("known vehicle")?;
                        if v.selections.is_empty() {
                            v.diagnostics.push(format!(
                                "No authored appearance matches {:?} on {resource}",
                                v.requested_appearance
                            ));
                        }
                    }
                }
            }
            if (index + 1).is_multiple_of(10) || index + 1 == count {
                eprintln!(
                    "Resolved {}/{} roots; {} selections",
                    index + 1,
                    count,
                    c.selections.len()
                );
            }
        }
        Ok(())
    })
}

pub fn resource_overrides() -> BTreeMap<String, ResourceKey> {
    BTreeMap::from([
        (
            "8643518564535277504".into(),
            ResourceKey::parse(
                r"base\vehicles\special\av_zetatech_atlus\rig\av_zetatech_atlus_skeleton.rig",
            ),
        ),
        (
            "16913330955692710019".into(),
            ResourceKey::parse(
                r"base\environment\decoration\misc\music\electric_guitar\electric_guitar_a.mesh",
            ),
        ),
        (
            "1649818027137861327".into(),
            ResourceKey::parse(
                r"base\environment\decoration\misc\music\electric_guitar\electric_guitar_b.mesh",
            ),
        ),
        (
            "4278970978671007791".into(),
            ResourceKey::parse(
                r"base\environment\decoration\misc\music\electric_guitar\electric_guitar_c.mesh",
            ),
        ),
        (
            "11574223278973522291".into(),
            ResourceKey::parse(
                r"base\environment\decoration\misc\music\electric_guitar\electric_guitar_d.mesh",
            ),
        ),
    ])
}

fn resolve(
    s: &mut Selection,
    reader: &mut AssetReader<'_>,
    bindings: &mut BTreeMap<
        (u64, String),
        std::result::Result<ghostline_red::assets::MeshAppearanceBinding, String>,
    >,
) {
    let shadows = ["stwheel_01_shadow", "shadowmesh_ncl1_12"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let assembly = match reader.assembly_with_resource_overrides(
        &s.resource,
        s.appearance.as_deref(),
        &shadows,
        &resource_overrides(),
    ) {
        Ok(v) => v,
        Err(e) => {
            s.diagnostics.push(format!("assembly: {e}"));
            return;
        }
    };
    for component in &assembly.components {
        s.components.push(json!({"name":component.name,"class":component.kind,"mesh":component.mesh,
            "requested_mesh_appearance":component.mesh_appearance_requested,"effective_mesh_appearance":component.mesh_appearance,
            "enabled":component.enabled,"chunk_mask":component.chunk_mask.to_string(),"local_transform":component.local.to_cols_array(),
            "rig":component.rig,"parent":{"component":component.parent.component,"slot":component.parent.slot},
            "control":{"component":component.control.component,"slot":component.control.slot}}));
        for (field, effective) in [
            ("mesh", component.mesh.as_ref()),
            ("rig", component.rig.as_ref()),
        ] {
            if let Ok(Some(requested)) = component
                .template
                .get(field)
                .map(ResourceKey::from_value)
                .transpose()
                .map(Option::flatten)
                && let Some(effective) = effective
                && requested.native_hash() != effective.native_hash()
            {
                s.components.last_mut().expect("just inserted component")
                    [format!("requested_{field}")] = json!(requested);
            }
        }
        if shadows.contains(&component.name) || component.is_collapsed() {
            if component.is_collapsed() {
                s.components.last_mut().expect("just inserted component")["native_visual_scale"] =
                    component.template["visualScale"].clone();
            }
            let reason = if component.is_collapsed() {
                "Native visual scale collapses the mesh"
            } else {
                "Shadow caster omitted from the unshadowed catalogue preview"
            };
            let missing = component
                .mesh
                .as_ref()
                .is_some_and(|mesh| !reader.resource_exists(mesh));
            s.material_bindings
                .push(json!({"component":component.name,"resource":component.mesh,
                "preview_excluded":true,"reason":reason,"native_resource_missing":missing}));
            if missing {
                s.diagnostics.push(format!(
                    "preview: {}: {reason}; native mesh {:?} is unresolved",
                    component.name, component.mesh
                ));
            }
            continue;
        }
        if let Some(mesh) = component.mesh.as_ref().filter(|_| {
            component.enabled
                && component.chunk_mask != 0
                && component.kind != "entAppearanceProxyMeshComponent"
        }) {
            let request = component
                .mesh_appearance_requested
                .as_deref()
                .unwrap_or("default");
            let binding = bindings
                .entry((mesh.native_hash(), request.into()))
                .or_insert_with(|| {
                    reader
                        .mesh_appearance_binding(mesh, request)
                        .map_err(|e| e.to_string())
                });
            match binding {
                Ok(binding) => s
                    .material_bindings
                    .push(json!({"component":component.name,"binding":binding})),
                Err(e) => s
                    .diagnostics
                    .push(format!("component {} ({mesh}): {e}", component.name)),
            }
        }
    }
    if s.diagnostics
        .iter()
        .all(|diagnostic| diagnostic.starts_with("preview: "))
    {
        s.resolution = "resolved".into();
        s.preview_state = if s
            .material_bindings
            .iter()
            .any(|v| v["preview_excluded"] != true && v["binding"]["shadow_only"] != true)
        {
            "pending"
        } else {
            "not_applicable"
        }
        .into();
    }
}
