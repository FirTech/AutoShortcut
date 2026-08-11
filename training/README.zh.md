# 主可执行文件模型训练

1. 从应用程序导出已审阅的候选项：

```powershell
AutoShortcut.exe D:\Apps --export-features training\data\candidates.csv
```

2. 对每个要用于训练的软件，将所有候选项行标记为 `label_state=reviewed`。恰好标记一个候选项为 `label_is_main=1`，其余所有候选项均标记为 `label_is_main=0`。审阅不完整的软件分组将被排除。

3. 创建 Python 虚拟环境，安装 `requirements.txt` 中的依赖：

```powershell
cd training
python -m venv .venv
.venv\Scripts\Activate.ps1
python -m pip install -r requirements.txt
```

4. 训练模型并生成 Rust 代码：

```powershell
python train.py data\candidates.csv --model-output reports\model.joblib --report-output reports\metrics.json
python generate_model.py reports\model.joblib ..\src\model\generated_xgboost.rs --parity-output reports\parity.json
cargo fmt
cargo test
```

CSV 中的路径均相对于扫描根目录。生成的模型仅包含数值决策逻辑和模型元数据，不包含软件路径。

## CSV 元数据

| 列                     | 含义                                                         |
| ---------------------- | ------------------------------------------------------------ |
| `schema_version`       | 训练程序校验的特征与 CSV 架构版本。                          |
| `sample_id`            | 单个可执行文件候选项的相对稳定标识符。                       |
| `software_id`          | 同一 AppRoot 中所有候选项共用的分组标识符。                  |
| `app_root_relative`    | 相对于扫描目录的 AppRoot 路径。                              |
| `candidate_relative`   | 相对于扫描目录的候选项路径。                                 |
| `candidate_name`       | 候选 EXE 文件名。                                            |
| `directory_role`       | 目录分析器角色；导出的行均为 AppRoot 候选项。                |
| `directory_confidence` | 基于规则的边界分析器给出的置信度。                           |
| `candidate_count`      | 由 AppRoot 拥有的可执行文件候选项数量。                      |
| `product_name`         | PE 产品名称；不可用时为空。                                  |
| `file_description`     | PE 文件说明；不可用时为空。                                  |
| `company_name`         | PE 公司名称；不可用时为空。                                  |
| `file_version`         | PE 产品版本；不可用时为空。                                  |
| `rule_score`           | 回退规则选择器生成的分数。                                   |
| `rule_rank`            | 回退规则选择器生成的候选项排名。                             |
| `rule_selected`        | 回退选择器是否会选择该候选项。                               |
| `label_state`          | `unreviewed` 或 `reviewed`；训练仅使用已完整审阅的软件分组。 |
| `label_is_main`        | 人工标签：唯一的主可执行文件为 `1`，其余所有候选项为 `0`。   |
| `label_source`         | 可选的标签来源，例如 `manual`。                              |
| `label_note`           | 可选的注释说明。                                             |

## 数值特征

| 特征                                    | 含义                                             |
| --------------------------------------- | ------------------------------------------------ |
| `name_root_similarity`                  | EXE 文件主名与 AppRoot 目录名称之间的相似度。    |
| `name_parent_similarity`                | EXE 文件主名与其直接父目录之间的相似度。         |
| `is_root_executable`                    | EXE 是否直接位于 AppRoot 内。                    |
| `relative_depth`                        | 候选项父目录相对于 AppRoot 的层级深度。          |
| `is_launch_path`                        | 相对路径是否包含常见的启动目录。                 |
| `launch_path_depth`                     | 相对路径中常见启动目录组件的数量。               |
| `is_gui` / `gui_known`                  | PE GUI 子系统结果，以及是否能够读取该结果。      |
| `has_icon`                              | 可执行文件是否包含图标资源。                     |
| `has_description` / `description_known` | 是否存在 PE 文件说明，以及是否能够读取元数据。   |
| `has_product_name`                      | 是否存在 PE 产品名称元数据。                     |
| `has_company_name`                      | 是否存在 PE 公司名称元数据。                     |
| `has_file_version`                      | 是否存在 PE 产品版本元数据。                     |
| `is_signed` / `signature_known`         | Authenticode 签名结果，以及是否能够检查该签名。  |
| `arch_native`                           | 候选 PE 架构是否与系统原生架构匹配。             |
| `arch_compatible` / `arch_known`        | PE 是否可在系统上运行，以及是否能够读取其架构。  |
| `file_size_log2`                        | 文件大小加一后的以 2 为底对数。                  |
| `size_rank_percentile`                  | 候选项大小在当前 AppRoot 中的排名百分位。        |
| `candidate_count_log2`                  | AppRoot 候选项数量加一后的以 2 为底对数。        |
| `name_has_launcher`                     | 文件名包含启动器或启动标记。                     |
| `name_has_uninstaller`                  | 文件名包含卸载标记。                             |
| `name_has_installer`                    | 文件名包含安装程序或安装标记。                   |
| `name_has_updater`                      | 文件名包含更新或升级标记。                       |
| `name_has_service`                      | 文件名包含服务、代理或守护标记。                 |
| `name_has_helper`                       | 文件名包含 helper、host、broker 或 worker 标记。 |
| `name_has_diagnostic`                   | 文件名包含诊断、测试或修复标记。                 |
| `name_has_plugin`                       | 文件名包含插件或开发者工具标记。                 |
