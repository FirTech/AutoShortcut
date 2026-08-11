# Main executable model training

1. Export reviewed candidates from the application:

```powershell
AutoShortcut.exe D:\Apps --export-features training\data\candidates.csv
```

2. For each software you want to train on, mark every candidate row as `label_state=reviewed`. Mark exactly one candidate with `label_is_main=1` and all other candidates with `label_is_main=0`. Partially reviewed software groups are excluded.

3. Create Python virtual environment and install `requirements.txt`:

```powershell
cd training
python -m venv .venv
.venv\Scripts\Activate.ps1
python -m pip install -r requirements.txt
```

4. Train model and generate Rust code:

```powershell
python train.py data\candidates.csv --model-output reports\model.joblib --report-output reports\metrics.json
python generate_model.py reports\model.joblib ..\src\model\generated_xgboost.rs --parity-output reports\parity.json
cargo fmt
cargo test
```

The CSV paths are relative to the scan root. The generated model contains only numeric decision logic and model metadata; it does not contain software paths.

## CSV metadata

| Column                 | Meaning                                                                            |
| ---------------------- | ---------------------------------------------------------------------------------- |
| `schema_version`       | Feature and CSV schema version checked by the trainer.                             |
| `sample_id`            | Relative, stable identifier for one executable candidate.                          |
| `software_id`          | Group identifier shared by every candidate in one AppRoot.                         |
| `app_root_relative`    | AppRoot path relative to the scanned directory.                                    |
| `candidate_relative`   | Candidate path relative to the scanned directory.                                  |
| `candidate_name`       | Candidate EXE file name.                                                           |
| `directory_role`       | Directory analyzer role; exported rows are AppRoot candidates.                     |
| `directory_confidence` | Confidence assigned by the rule-based boundary analyzer.                           |
| `candidate_count`      | Number of executable candidates owned by the AppRoot.                              |
| `product_name`         | PE product name, or empty when unavailable.                                        |
| `file_description`     | PE file description, or empty when unavailable.                                    |
| `company_name`         | PE company name, or empty when unavailable.                                        |
| `file_version`         | PE product version, or empty when unavailable.                                     |
| `rule_score`           | Score produced by the fallback rule selector.                                      |
| `rule_rank`            | Candidate rank produced by the fallback rule selector.                             |
| `rule_selected`        | Whether the fallback selector would choose this candidate.                         |
| `label_state`          | `unreviewed` or `reviewed`; training uses only fully reviewed software groups.     |
| `label_is_main`        | Human label: `1` for the single main executable and `0` for every other candidate. |
| `label_source`         | Optional label provenance, such as `manual`.                                       |
| `label_note`           | Optional annotation note.                                                          |

## Numeric features

| Feature                                 | Meaning                                                                          |
| --------------------------------------- | -------------------------------------------------------------------------------- |
| `name_root_similarity`                  | Similarity between EXE stem and AppRoot directory name.                          |
| `name_parent_similarity`                | Similarity between EXE stem and its direct parent directory.                     |
| `is_root_executable`                    | Whether the EXE is directly inside the AppRoot.                                  |
| `relative_depth`                        | Candidate parent-directory depth below the AppRoot.                              |
| `is_launch_path`                        | Whether the relative path contains a common launch directory.                    |
| `launch_path_depth`                     | Number of common launch-directory components in the relative path.               |
| `is_gui` / `gui_known`                  | PE GUI subsystem result and whether it could be read.                            |
| `has_icon`                              | Whether the executable contains an icon resource.                                |
| `has_description` / `description_known` | Whether a PE description exists and whether metadata could be read.              |
| `has_product_name`                      | Whether PE product name metadata exists.                                         |
| `has_company_name`                      | Whether PE company metadata exists.                                              |
| `has_file_version`                      | Whether PE product-version metadata exists.                                      |
| `is_signed` / `signature_known`         | Authenticode signature result and whether it could be checked.                   |
| `arch_native`                           | Whether candidate PE architecture matches the native system architecture.        |
| `arch_compatible` / `arch_known`        | Whether the PE can run on the system and whether its architecture could be read. |
| `file_size_log2`                        | Base-2 logarithm of file size plus one.                                          |
| `size_rank_percentile`                  | Candidate size rank within the current AppRoot.                                  |
| `candidate_count_log2`                  | Base-2 logarithm of AppRoot candidate count plus one.                            |
| `name_has_launcher`                     | File name contains a launcher/start token.                                       |
| `name_has_uninstaller`                  | File name contains an uninstall token.                                           |
| `name_has_installer`                    | File name contains a setup/installer token.                                      |
| `name_has_updater`                      | File name contains an update/upgrade token.                                      |
| `name_has_service`                      | File name contains a service/agent/guard token.                                  |
| `name_has_helper`                       | File name contains a helper/host/broker/worker token.                            |
| `name_has_diagnostic`                   | File name contains a diagnostic/test/repair token.                               |
| `name_has_plugin`                       | File name contains a plugin/developer-tool token.                                |
