use ghostline_red::{
    assets::ResourceKey,
    tweakdb::{FlatValue, TweakDb, TweakId, TweakNames},
};
use ghostline_vehicles::{model::*, performance, publish, render};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

fn fixture() -> Catalogue {
    let resource = ResourceKey::parse("base/vehicles/test.ent");
    let id = format!("{resource}#paint");
    let mut c = Catalogue {
        format: "ghostline-vehicles".into(),
        version: 1,
        sources: BTreeMap::new(),
        vehicles: BTreeMap::new(),
        selections: BTreeMap::new(),
        curves: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    for (id, power) in [("1", 100), ("2", 250)] {
        c.vehicles.insert(
            id.into(),
            Vehicle {
                id: id.into(),
                native_id: TweakId(id.parse().unwrap()),
                native_name: None,
                label: format!("Vehicle {id}"),
                description: None,
                native_class: "gamedataVehicle_Record".into(),
                manufacturer: None,
                vehicle_type: None,
                properties: BTreeMap::new(),
                related: BTreeMap::new(),
                performance: json!({"specifications":{"horsepower":power}}),
                entity: Some(resource.clone()),
                requested_appearance: Some("paint".into()),
                selections: vec![format!("{resource}#paint")],
                diagnostics: vec![],
            },
        );
    }
    c.selections.insert(
        id.clone(),
        Selection {
            id,
            resource,
            appearance: Some("paint".into()),
            records: vec!["1".into(), "2".into()],
            resolution: "resolved".into(),
            components: vec![],
            material_bindings: vec![],
            diagnostics: vec![],
            preview_state: "pending".into(),
            media: vec![],
            recipe: Value::Null,
        },
    );
    c
}

#[test]
fn shared_appearance_does_not_merge_gameplay_performance_and_packages_are_portable() {
    let root = tempfile::tempdir().unwrap();
    let c = fixture();
    let source = root.path().join("source");
    publish::write(&c, &source, "").unwrap();
    publish::verify(&source).unwrap();
    let loaded = publish::read(&source).unwrap();
    assert_eq!(loaded.vehicles.len(), 2);
    assert_eq!(loaded.selections.len(), 1);
    assert_eq!(
        loaded.vehicles["1"].performance["specifications"]["horsepower"],
        100
    );
    assert_eq!(
        loaded.vehicles["2"].performance["specifications"]["horsepower"],
        250
    );
    let hosted = root.path().join("hosted");
    publish::package(&source, &hosted, "https://example.test/vehicles").unwrap();
    let offline = root.path().join("offline");
    publish::package(&hosted, &offline, "").unwrap();
    publish::verify(&offline).unwrap();
    let browser = std::fs::read_to_string(offline.join("data/browser/catalogue.js")).unwrap();
    assert!(browser.contains("\"media_base\":\"\""));
    assert!(
        !std::fs::read_to_string(offline.join("assets/app.js"))
            .unwrap()
            .contains("fetch(")
    );
    assert!(publish::member(&offline, "../outside").is_err());
    assert!(publish::member(&offline, "C:/outside").is_err());
}

#[test]
fn forward_ceiling_excludes_reverse_and_missing_hud_curve_stays_unavailable() {
    let mut c = fixture();
    let v = c.vehicles.get_mut("1").unwrap();
    v.related.insert(
        "vehEngineData".into(),
        BTreeMap::from([(
            "gears".into(),
            FlatValue {
                native_type: "array:TweakDBID".into(),
                value: json!(["10", "11", "12"]),
            },
        )]),
    );
    let mut db = TweakDb {
        records: BTreeMap::new(),
        flats: HashMap::new(),
        queries: BTreeMap::new(),
        group_tags: BTreeMap::new(),
        record_checksum: 0,
    };
    for (gear, speed) in [(10, 100.), (11, 40.), (12, 50.)] {
        db.flats.insert(
            TweakId(gear).property("maxSpeed"),
            Arc::new(FlatValue {
                native_type: "Float".into(),
                value: json!(speed),
            }),
        );
    }
    let p = performance::calculate(v, &db, &TweakNames::default(), &BTreeMap::new());
    assert_eq!(p["calculated"]["top_gear"]["limit_mps"], 50.);
    assert_eq!(p["calculated"]["top_gear"]["world_kmh"], 180.);
    assert!(p["calculated"]["top_gear"]["hud_mph"].is_null());
    assert_eq!(p["calculated"]["gears"][0]["reverse"], true);
}

#[test]
fn render_plans_keep_blocked_bindings_visible() {
    let mut c = fixture();
    let s = c.selections.values_mut().next().unwrap();
    s.resolution = "blocked".into();
    s.diagnostics.push("missing native mesh".into());
    let plan = render::plan(&c, 1024).unwrap();
    assert!(plan.jobs.is_empty());
    assert_eq!(plan.blocked.len(), 1);
    assert!(render::plan(&c, 1).is_err());
}
