mod generated_xgboost;

use crate::features::{ExecutableFeatures, FEATURE_COUNT, FEATURE_SCHEMA_VERSION};

pub(crate) use generated_xgboost::{
    MODEL_AVAILABLE, MODEL_MIN_PROBABILITY_MARGIN, MODEL_MIN_TOP_PROBABILITY,
};

pub(crate) fn predict_main_probability(features: &ExecutableFeatures) -> Option<f64> {
    if !model_metadata_is_compatible(
        MODEL_AVAILABLE,
        generated_xgboost::MODEL_FEATURE_SCHEMA_VERSION,
        generated_xgboost::MODEL_FEATURE_COUNT,
        MODEL_MIN_TOP_PROBABILITY,
        MODEL_MIN_PROBABILITY_MARGIN,
    ) {
        return None;
    }
    let input = features
        .model_input()
        .into_iter()
        .map(quantize_feature)
        .collect();
    let output = generated_xgboost::score(input);
    binary_main_probability(&output)
}

fn model_metadata_is_compatible(
    available: bool,
    schema_version: u32,
    feature_count: usize,
    min_top_probability: f64,
    min_probability_margin: f64,
) -> bool {
    available
        && schema_version == FEATURE_SCHEMA_VERSION
        && feature_count == FEATURE_COUNT
        && valid_probability(min_top_probability)
        && valid_probability(min_probability_margin)
}

fn binary_main_probability(output: &[f64]) -> Option<f64> {
    let [negative, positive] = output else {
        return None;
    };
    if !valid_probability(*negative)
        || !valid_probability(*positive)
        || (negative + positive - 1.0).abs() > 1e-6
    {
        return None;
    }
    Some(*positive)
}

fn valid_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn quantize_feature(value: f64) -> f64 {
    f64::from(value as f32)
}

#[cfg(test)]
mod tests {
    use super::{binary_main_probability, model_metadata_is_compatible, quantize_feature};

    #[derive(serde::Deserialize)]
    struct ParityFixture {
        feature_schema_version: u32,
        cases: Vec<ParityCase>,
    }

    #[derive(serde::Deserialize)]
    struct ParityCase {
        features: Vec<f64>,
        probabilities: Vec<f64>,
    }

    #[test]
    fn unavailable_model_metadata_is_rejected() {
        assert!(!model_metadata_is_compatible(
            false,
            crate::features::FEATURE_SCHEMA_VERSION,
            crate::features::FEATURE_COUNT,
            0.8,
            0.1,
        ));
    }

    #[test]
    fn malformed_binary_model_output_is_rejected() {
        assert!(binary_main_probability(&[]).is_none());
        assert!(binary_main_probability(&[0.2, 0.7]).is_none());
        assert!(binary_main_probability(&[f64::NAN, f64::NAN]).is_none());
        assert_eq!(binary_main_probability(&[0.2, 0.8]), Some(0.8));
    }

    #[test]
    fn generated_model_matches_python_parity_fixture_when_present() {
        if !super::MODEL_AVAILABLE {
            return;
        }
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("training/reports/parity.json");
        if !path.exists() {
            return;
        }
        let fixture: ParityFixture = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            fixture.feature_schema_version,
            crate::features::FEATURE_SCHEMA_VERSION
        );
        for case in fixture.cases {
            assert_eq!(case.features.len(), crate::features::FEATURE_COUNT);
            let input = case.features.into_iter().map(quantize_feature).collect();
            let actual = super::generated_xgboost::score(input);
            assert_eq!(actual.len(), case.probabilities.len());
            for (actual, expected) in actual.iter().zip(case.probabilities) {
                assert!((actual - expected).abs() <= 1e-6);
            }
        }
    }
}
