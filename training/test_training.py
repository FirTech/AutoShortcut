import unittest

import pandas as pd

from training.train import (
    FEATURE_NAMES,
    FEATURE_SCHEMA_VERSION,
    choose_thresholds,
    grouped_split,
    metrics,
    validate_frame,
)


def frame_for(groups: int = 10) -> pd.DataFrame:
    rows = []
    for group in range(groups):
        for candidate in range(2):
            row = {
                "schema_version": FEATURE_SCHEMA_VERSION,
                "software_id": f"app-{group}",
                "label_state": "reviewed",
                "label_is_main": int(candidate == 0),
            }
            row.update({feature: 0.0 for feature in FEATURE_NAMES})
            rows.append(row)
    return pd.DataFrame(rows)


class TrainingValidationTests(unittest.TestCase):
    def test_missing_feature_column_is_rejected(self):
        frame = frame_for().drop(columns=[FEATURE_NAMES[-1]])
        with self.assertRaisesRegex(ValueError, "missing CSV columns"):
            validate_frame(frame)

    def test_partially_reviewed_software_is_excluded(self):
        frame = frame_for()
        frame.loc[0, "label_state"] = "unreviewed"
        reviewed = validate_frame(frame)
        self.assertNotIn("app-0", set(reviewed.software_id))

    def test_requires_exactly_one_positive_per_software(self):
        frame = frame_for()
        frame.loc[1, "label_is_main"] = 1
        with self.assertRaisesRegex(ValueError, "exactly one positive"):
            validate_frame(frame)

    def test_rejects_feature_order_changes(self):
        frame = frame_for()
        columns = list(frame.columns)
        first = columns.index(FEATURE_NAMES[0])
        second = columns.index(FEATURE_NAMES[1])
        columns[first], columns[second] = columns[second], columns[first]
        with self.assertRaisesRegex(ValueError, "required order"):
            validate_frame(frame[columns])

    def test_group_split_has_no_software_leakage(self):
        train, validation, test = grouped_split(validate_frame(frame_for()), 42)
        train_groups = set(train.software_id)
        validation_groups = set(validation.software_id)
        test_groups = set(test.software_id)
        self.assertFalse(train_groups & validation_groups)
        self.assertFalse(train_groups & test_groups)
        self.assertFalse(validation_groups & test_groups)

    def test_small_validation_set_uses_conservative_defaults(self):
        predictions = pd.DataFrame([
            {"top_probability": 0.95, "margin": 0.8, "correct": True}
        ])
        probability, margin, details = choose_thresholds(predictions)
        self.assertEqual(probability, 0.99)
        self.assertEqual(margin, 0.15)
        self.assertTrue(details["threshold_source"].startswith("default"))

    def test_metrics_report_accuracy_precision_coverage_and_fallback(self):
        predictions = pd.DataFrame([
            {"top_probability": 0.99, "margin": 0.8, "correct": True},
            {"top_probability": 0.60, "margin": 0.1, "correct": False},
        ])
        report = metrics(predictions, 0.90, 0.20)
        self.assertEqual(report["top1_accuracy"], 0.5)
        self.assertEqual(report["accepted_precision"], 1.0)
        self.assertEqual(report["coverage"], 0.5)
        self.assertEqual(report["fallback_rate"], 0.5)


if __name__ == "__main__":
    unittest.main()
