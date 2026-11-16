//! Vehicle specifications and source-backed gearing ceilings; no runtime simulation.
use crate::{build::properties, model::Vehicle};
use ghostline_red::{
    curves::Curve,
    tweakdb::{TweakDb, TweakId, TweakNames},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const HUD_RESOURCE: &str = "base/gameplay/curves/vehicle/vehicle_ui.curveset";

fn curve(bundle: &Value, name: &str) -> Option<Curve> {
    let entries = bundle.pointer("/document/curves")?.as_array()?;
    let entry = entries
        .iter()
        .find(|v| crate::build::text(&v["name"]) == Some(name))?;
    Curve::from_value(&entry["curve"]).ok()
}

pub fn calculate(
    v: &Vehicle,
    db: &TweakDb,
    names: &TweakNames,
    curves: &BTreeMap<String, Value>,
) -> Value {
    let prop = |group: &str, field: &str| {
        v.related
            .get(group)
            .and_then(|m| m.get(field))
            .map(|p| p.value.clone())
            .unwrap_or(Value::Null)
    };
    let engine = prop("vehEngineData", "gears");
    let hud = curves
        .get(HUD_RESOURCE)
        .and_then(|b| curve(b, "speed_to_multiplier"));
    let mut issues = Vec::new();
    if hud.is_none() {
        issues.push("HUD speed curve unavailable or unsupported; HUD ceilings unavailable");
    }
    let mut gears = Vec::new();
    for (index, id) in engine.as_array().into_iter().flatten().enumerate() {
        let fields = id
            .as_str()
            .and_then(|s| s.parse().ok())
            .map(TweakId)
            .map(|id| {
                properties(
                    db,
                    names,
                    id,
                    "minSpeed maxSpeed torqueMultiplier gearRatio",
                )
            });
        let speed = fields
            .as_ref()
            .and_then(|m| m.get("maxSpeed"))
            .and_then(|v| v.value.as_f64())
            .filter(|v| v.is_finite());
        let hud_mph =
            speed.and_then(|s| hud.as_ref().and_then(|c| c.evaluate(s).ok()).map(|m| s * m));
        gears.push(json!({"index":index,"reverse":index==0,"record":id,"properties":fields,
            "limit_mps":speed,"world_mph":speed.map(|s|s*3600./1609.344),"world_kmh":speed.map(|s|s*3.6),"hud_mph":hud_mph,"hud_kmh":hud_mph.map(|s|s*1.60934)}));
    }
    let top = gears
        .iter()
        .filter(|g| g["reverse"] == false)
        .filter_map(|g| g["limit_mps"].as_f64().filter(|s| *s > 0.).map(|s| (s, g)))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, g)| g.clone());
    let torque_path = prop("vehEngineData", "gearCurvesPath");
    let torque=torque_path.as_str().and_then(|p|curves.get(p)).and_then(|b|b.pointer("/document/curves")).and_then(Value::as_array).map(|entries|entries.iter().map(|entry|json!({"name":entry["name"],"at_normalized_rpm_1":Curve::from_value(&entry["curve"]).and_then(|c|c.evaluate(1.)).ok()})).collect::<Vec<_>>());
    json!({"specifications":{"evidence":"authored_compiled_parameters","horsepower":prop("vehicleUIData","horsepower"),
        "ui_mass_kg":prop("vehicleUIData","mass"),"physics_mass_kg":prop("vehDriveModelData","total_mass"),
        "production_year":prop("vehicleUIData","productionYear"),"drivetrain":prop("vehicleUIData","driveLayout"),
        "engine_torque":prop("vehEngineData","engineMaxTorque"),"max_rpm":prop("vehEngineData","maxRPM")},
        "calculated":{"evidence":"calculated","method":"highest_positive_forward_gear_limit","top_gear":top,
        "gears":gears,"hud_curve_resource":HUD_RESOURCE,"torque_curve_resource":torque_path,"torque_curve_endpoints":torque,
        "formula":"world_kmh = mps * 3.6; world_mph = mps * 3600 / 1609.344; HUD mph = mps * Bezier(speed_to_multiplier,mps); HUD kmh = HUD mph * 1.60934",
        "limitations":"Configured gearing ceilings, not measured sustained speed or a drag/traction simulation"},
        "measurements":[],"diagnostics":issues})
}
