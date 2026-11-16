//! Published identities shared by browser, CLI and other tools.
use ghostline_red::{
    assets::ResourceKey,
    tweakdb::{FlatValue, TweakId},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Catalogue {
    pub format: String,
    pub version: u32,
    pub sources: BTreeMap<String, String>,
    pub vehicles: BTreeMap<String, Vehicle>,
    pub selections: BTreeMap<String, Selection>,
    pub curves: BTreeMap<String, Value>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: String,
    pub native_id: TweakId,
    pub native_name: Option<String>,
    pub label: String,
    pub description: Option<String>,
    pub native_class: String,
    pub manufacturer: Option<String>,
    pub vehicle_type: Option<String>,
    pub properties: BTreeMap<String, FlatValue>,
    pub related: BTreeMap<String, BTreeMap<String, FlatValue>>,
    pub performance: Value,
    pub entity: Option<ResourceKey>,
    pub requested_appearance: Option<String>,
    pub selections: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Selection {
    pub id: String,
    pub resource: ResourceKey,
    pub appearance: Option<String>,
    pub records: Vec<String>,
    pub resolution: String,
    pub components: Vec<Value>,
    pub material_bindings: Vec<Value>,
    pub diagnostics: Vec<String>,
    pub preview_state: String,
    pub media: Vec<Media>,
    pub recipe: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub path: String,
    pub view: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub sha256: String,
    pub mime: String,
}

impl Catalogue {
    pub fn coverage(&self) -> Value {
        let count = |state: &str| {
            self.selections
                .values()
                .filter(|s| s.preview_state == state)
                .count()
        };
        json!({"records":self.vehicles.len(), "named_records":self.vehicles.values().filter(|v|v.native_name.is_some()).count(),
            "selections":self.selections.len(),"resolved":self.selections.values().filter(|s|s.resolution=="resolved").count(),
            "blocked":self.selections.values().filter(|s|s.resolution=="blocked").count(),
            "records_without_selections":self.vehicles.values().filter(|v|v.selections.is_empty()).count(),
            "ready":count("ready"),"pending":count("pending"),"failed":count("failed"),"not_applicable":count("not_applicable"),
            "images":self.selections.values().map(|s|s.media.len()).sum::<usize>()})
    }
}
