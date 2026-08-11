"""Train the AppRoot main-executable selector and emit reproducible metrics."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import joblib
import numpy as np
import pandas as pd
from sklearn.model_selection import GroupShuffleSplit
from xgboost import XGBClassifier

FEATURE_SCHEMA_VERSION = 1
FEATURE_NAMES = [
    "name_root_similarity", "name_parent_similarity", "is_root_executable",
    "relative_depth", "is_launch_path", "launch_path_depth", "is_gui",
    "gui_known", "has_icon", "has_description", "description_known",
    "has_product_name", "has_company_name", "has_file_version", "is_signed",
    "signature_known", "arch_native", "arch_compatible", "arch_known",
    "file_size_log2", "size_rank_percentile", "candidate_count_log2",
    "name_has_launcher", "name_has_uninstaller", "name_has_installer",
    "name_has_updater", "name_has_service", "name_has_helper",
    "name_has_diagnostic", "name_has_plugin",
]

DEFAULT_TOP_PROBABILITY = 0.99
DEFAULT_MARGIN = 0.15
MIN_VALIDATION_GROUPS = 20


def validate_frame(frame: pd.DataFrame) -> pd.DataFrame:
    required = {"schema_version", "software_id", "label_state", "label_is_main", *FEATURE_NAMES}
    missing = sorted(required - set(frame.columns))
    if missing:
        raise ValueError(f"missing CSV columns: {', '.join(missing)}")
    actual_feature_order = [column for column in frame.columns if column in set(FEATURE_NAMES)]
    if actual_feature_order != FEATURE_NAMES:
        raise ValueError("feature columns do not match the required order")
    if set(frame["schema_version"].astype(int)) != {FEATURE_SCHEMA_VERSION}:
        raise ValueError("unsupported feature schema version")
    states = frame["label_state"].astype(str).str.lower()
    software_ids = frame["software_id"].astype(str)
    fully_reviewed = states.eq("reviewed").groupby(software_ids).all()
    reviewed_ids = set(fully_reviewed[fully_reviewed].index)
    reviewed = frame[software_ids.isin(reviewed_ids)].copy()
    if reviewed.empty:
        raise ValueError("no fully reviewed software groups available")
    reviewed["label_is_main"] = pd.to_numeric(reviewed["label_is_main"], errors="raise").astype(int)
    if not reviewed["label_is_main"].isin([0, 1]).all():
        raise ValueError("label_is_main must contain only 0 or 1 for reviewed rows")
    reviewed["software_id"] = reviewed["software_id"].astype(str)
    counts = reviewed.groupby("software_id")["label_is_main"].sum()
    invalid = counts[counts != 1]
    if not invalid.empty:
        raise ValueError(f"each software_id must have exactly one positive: {invalid.index.tolist()}")
    return reviewed


def grouped_split(frame: pd.DataFrame, seed: int) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    groups = frame["software_id"].to_numpy()
    if len(np.unique(groups)) < 5:
        raise ValueError("at least five software_id groups are required for train/validation/test split")
    first = GroupShuffleSplit(n_splits=1, test_size=0.2, random_state=seed)
    train_valid_idx, test_idx = next(first.split(frame, groups=groups))
    train_valid = frame.iloc[train_valid_idx]
    test = frame.iloc[test_idx]
    second = GroupShuffleSplit(n_splits=1, test_size=0.25, random_state=seed + 1)
    train_idx, valid_idx = next(second.split(train_valid, groups=train_valid["software_id"]))
    return train_valid.iloc[train_idx], train_valid.iloc[valid_idx], test


def group_weights(frame: pd.DataFrame) -> np.ndarray:
    sizes = frame.groupby("software_id")["software_id"].transform("count")
    return 1.0 / sizes.to_numpy(dtype=float)


def group_predictions(frame: pd.DataFrame, probabilities: np.ndarray) -> pd.DataFrame:
    result = frame[["software_id", "label_is_main"]].copy()
    result["probability"] = probabilities
    rows = []
    for software_id, group in result.groupby("software_id"):
        ordered = group.sort_values(["probability"], ascending=False)
        top = ordered.iloc[0]
        second = float(ordered.iloc[1]["probability"]) if len(ordered) > 1 else 0.0
        rows.append({
            "software_id": software_id,
            "top_probability": float(top["probability"]),
            "margin": float(top["probability"] - second),
            "correct": bool(int(top["label_is_main"]) == 1),
        })
    return pd.DataFrame(rows)


def choose_thresholds(validation: pd.DataFrame) -> tuple[float, float, dict]:
    if len(validation) < MIN_VALIDATION_GROUPS:
        return DEFAULT_TOP_PROBABILITY, DEFAULT_MARGIN, {"threshold_source": "default_insufficient_validation"}
    candidates = validation.sort_values("top_probability")["top_probability"].unique().tolist()
    candidates += [DEFAULT_TOP_PROBABILITY]
    margins = validation.sort_values("margin")["margin"].unique().tolist()
    margins += [DEFAULT_MARGIN]
    best = None
    for probability in candidates:
        for margin in margins:
            accepted = validation[(validation.top_probability >= probability) & (validation.margin >= margin)]
            if accepted.empty:
                continue
            precision = float(accepted["correct"].mean())
            if precision < 0.99:
                continue
            coverage = len(accepted) / len(validation)
            key = (coverage, probability, margin)
            if best is None or key > best[0]:
                best = (key, probability, margin, precision, len(accepted))
    if best is None:
        return DEFAULT_TOP_PROBABILITY, DEFAULT_MARGIN, {"threshold_source": "default_no_99_percent_operating_point"}
    _, probability, margin, precision, accepted = best
    return probability, margin, {
        "threshold_source": "validation",
        "validation_precision": precision,
        "validation_accepted_groups": accepted,
        "validation_groups": len(validation),
    }


def metrics(predictions: pd.DataFrame, probability: float, margin: float) -> dict:
    accepted = predictions[(predictions.top_probability >= probability) & (predictions.margin >= margin)]
    return {
        "groups": int(len(predictions)),
        "top1_accuracy": float(predictions["correct"].mean()) if len(predictions) else 0.0,
        "accepted_groups": int(len(accepted)),
        "accepted_precision": float(accepted["correct"].mean()) if len(accepted) else None,
        "coverage": float(len(accepted) / len(predictions)) if len(predictions) else 0.0,
        "fallback_rate": float(1.0 - len(accepted) / len(predictions)) if len(predictions) else 1.0,
        "top_probability_percentiles": percentile_summary(predictions["top_probability"]),
        "margin_percentiles": percentile_summary(predictions["margin"]),
    }


def percentile_summary(values: pd.Series) -> dict:
    if values.empty:
        return {}
    return {
        name: float(values.quantile(quantile))
        for name, quantile in [("p00", 0.0), ("p25", 0.25), ("p50", 0.5), ("p75", 0.75), ("p100", 1.0)]
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_csv", type=Path)
    parser.add_argument("--model-output", type=Path, required=True)
    parser.add_argument("--report-output", type=Path, required=True)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    frame = validate_frame(pd.read_csv(args.input_csv))
    train, validation, test = grouped_split(frame, args.seed)
    model = XGBClassifier(
        objective="binary:logistic",
        eval_metric="logloss",
        n_estimators=240,
        max_depth=4,
        learning_rate=0.05,
        subsample=0.9,
        colsample_bytree=0.9,
        base_score=0.5,
        random_state=args.seed,
        n_jobs=1,
    )
    model.fit(train[FEATURE_NAMES].astype(float), train["label_is_main"], sample_weight=group_weights(train))
    validation_predictions = group_predictions(validation, model.predict_proba(validation[FEATURE_NAMES].astype(float))[:, 1])
    top_probability, margin, threshold_info = choose_thresholds(validation_predictions)
    test_predictions = group_predictions(test, model.predict_proba(test[FEATURE_NAMES].astype(float))[:, 1])

    args.model_output.parent.mkdir(parents=True, exist_ok=True)
    joblib.dump({
        "model": model,
        "feature_schema_version": FEATURE_SCHEMA_VERSION,
        "feature_names": FEATURE_NAMES,
        "min_top_probability": top_probability,
        "min_probability_margin": margin,
        "parity_cases": [
            {
                "features": row,
                "probabilities": probabilities,
            }
            for row, probabilities in zip(
                test[FEATURE_NAMES].astype(float).head(10).values.tolist(),
                model.predict_proba(test[FEATURE_NAMES].astype(float).head(10)).tolist(),
            )
        ],
    }, args.model_output)
    report = {
        "feature_schema_version": FEATURE_SCHEMA_VERSION,
        "feature_count": len(FEATURE_NAMES),
        "thresholds": {"min_top_probability": top_probability, "min_probability_margin": margin, **threshold_info},
        "train": {"rows": len(train), "groups": train.software_id.nunique()},
        "validation": metrics(validation_predictions, top_probability, margin),
        "test": metrics(test_predictions, top_probability, margin),
    }
    args.report_output.parent.mkdir(parents=True, exist_ok=True)
    args.report_output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
