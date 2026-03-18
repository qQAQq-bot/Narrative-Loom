use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct RecallReport {
    pub character_recall: f32,
    pub setting_recall: f32,
    pub event_recall: f32,
    pub macro_recall: f32,
}

#[derive(Debug, Deserialize)]
struct RecallCase {
    character_recall: f32,
    setting_recall: f32,
    event_recall: f32,
}

#[derive(Debug, Deserialize)]
struct RecallFixture {
    cases: Vec<RecallCase>,
}

fn run_entity_recall_metrics() -> RecallReport {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("entity_recall")
        .join("metrics_fixture.json");

    let raw = fs::read_to_string(fixture_path).expect("failed to read metrics fixture");
    let fixture: RecallFixture = serde_json::from_str(&raw).expect("invalid metrics fixture json");
    assert!(!fixture.cases.is_empty(), "metrics fixture must contain at least one case");

    let count = fixture.cases.len() as f32;
    let character_recall = fixture.cases.iter().map(|c| c.character_recall).sum::<f32>() / count;
    let setting_recall = fixture.cases.iter().map(|c| c.setting_recall).sum::<f32>() / count;
    let event_recall = fixture.cases.iter().map(|c| c.event_recall).sum::<f32>() / count;
    let macro_recall = (character_recall + setting_recall + event_recall) / 3.0;

    RecallReport {
        character_recall,
        setting_recall,
        event_recall,
        macro_recall,
    }
}

#[test]
fn test_macro_recall_not_below_baseline() {
    let report = run_entity_recall_metrics();
    assert!(report.macro_recall >= 0.72);
}
