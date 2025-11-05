# AutoShortcut

[简体中文](README.zh.md) [English](README.md)

## 介绍

`AutoShortcut` 是一款自动化快捷方式创建工具。专为高效管理软件快捷方式而设计。它能够智能识别各类软件目录结构，自动分析并创建最优的快捷方式，极大提升桌面和开始菜单的整洁度与使用效率。

### 🎉 **AutoShortcut 的优势**

- 🔍 **智能识别**：自动识别绿色软件、单文件应用并创建对应快捷方式
- ⚡ **多维度评分**：通过多维度评分算法识别主程序
- 🛠 **丰富定制**：支持模板命名、忽略规则、批量创建等高级功能
- 📂 **双模式支持**：同时支持命令行和配置文件，满足不同使用场景
- 🌍 **多语言界面**：内置中文、英文支持，易于国际化扩展
- 🎯 **灵活配置**：提供详细的配置选项，满足个性化需求

### 🔍 **AutoShortcut 的逻辑**

**目录类型定义：**

- `分类目录`: 包含多个软件目录的目录，每个软件目录下可包含`单文件程序目录`、`绿色软件目录`
- `单文件程序目录`: 仅包含多个 exe 程序的目录，不包含其他文件（包括配置文件等）
- `绿色软件`: 有程序依赖文件，可能包含多个exe程序

> 提示
>
> - 若能保持规范的软件目录结构（遵循上述`分类目录`、`单文件程序目录`和`绿色软件`的划分原则），`AutoShortcut`
    可以直接智能识别并创建快捷方式，无需额外编写配置文件。
> - 特别提醒：请避免混合存放的情况（例如在单文件程序目录中包含绿色软件及其依赖文件），这类不规范的结构可能导致程序无法正确识别。如遇识别问题，请先检查并调整您的软件目录结构，必要时通过配置文件进行辅助定义。

`AutoShortcut`根据以下逻辑扫描目录：

1. 对指定的程序目录扫描每一个子目录
2. 识别子目录类型：

    - 识别到`分类目录`，则继续扫描下一个子目录
    - 识别到`单文件程序目录`，则对每个 exe 程序创建快捷方式
    - 识别到`绿色软件`，则对每个 exe 程序进行多维度综合评分：

        - 程序名是否与父目录名称相匹配
        - 程序是否有描述信息
        - 程序是否为界面程序
        - 程序是否有图标
        - 程序是否有数字签名
        - 程序是否与当前系统架构相匹配
        - 程序名是否有辅助关键字
        - （仅指定配置文件时）根据配置文件中指定的程序文件名加分

      以得分最高程序判断为主程序，并向上追溯绿色软件根目录，后续访问此软件子目录将跳过。如分数不满足阈值（默认阈值为最高分的
      30%）则判定为没有主程序，跳过此目录。

> 提示
>
> - 创建快捷方式默认将使用经过处理的产品名称（删除网址、删除特殊字符等）作为快捷方式名称，尽可能的还原程序名称
> - 如不为合理的产品名称（如`7z setup sfx`等）将回退至原始文件名作为快捷方式名称。

## 🚀 快速开始

> 本程序为命令行程序，故需要在其后面接参数运行，可通过`cmd`、`PowerShell`等终端来运行。

```bash
# 示例1: 扫描Program Files目录，在桌面创建所有程序的快捷方式
AutoShortcut.exe "C:\Program Files" "%Desktop%"

# 示例2: 扫描Program Files目录，在开始菜单创建快捷方式并按程序分类
AutoShortcut.exe -d "C:\Program Files" "%Programs%"

# 示例3: 扫描并自动运行指定软件（适用于绿色软件）
AutoShortcut.exe -s "C:\Program Files\Everything"
```

> 提示：
>
> - Windows 推荐以“管理员身份”运行，防止 UAC 拦截；
> - 路径含空格请用引号；
> - 环境变量（如 %USERPROFILE%、%Programs%）可直接使用，同时支持程序内置变量。

## 命令行用法

> 温馨提示：路径含空格请用引号

Usage: `AutoShortcut.exe [选项] ["程序路径"] ["快捷方式路径"] [--config "配置文件路径"]`

### 基本使用

```bash
AutoShortcut.exe "程序路径" "快捷方式路径"
```

- 创建桌面快捷方式: `AutoShortcut.exe "C:\Program Files" %Desktop%`
- 创建开始菜单快捷方式: `AutoShortcut.exe "C:\Program Files" "%Programs%"`

### 创建程序文件夹

```bash
`AutoShortcut.exe -d "程序路径" "快捷方式路径"`
```

- `AutoShortcut.exe -d "C:\Program Files" "%Programs%"`

### 尝试安装软件

将自动运行软件目录内的: `install.cmd/bat`、`setup.cmd/bat`、`绿化.cmd/bat`

```bash
AutoShortcut.exe -i "程序路径" "快捷方式路径"
```

- `AutoShortcut.exe -i "C:\Program Files" "%Programs%"`

### 运行程序

识别主程序后自动运行程序

```bash
AutoShortcut.exe --start "程序路径"
```

### 使用原始文件名

设置快捷方式名称使用原始 exe 文件名

```bash
AutoShortcut.exe --use-filename "程序路径" "快捷方式路径"
```

### 仅列出快捷方式

仅列出需要创建快捷方式的程序路径，不进行创建快捷方式操作。

```bash
AutoShortcut.exe --list "程序路径" "快捷方式路径"
```

### 仅匹配配置文件

仅将配置文件中的快捷方式信息创建快捷方式

```bash
AutoShortcut.exe --only-match "程序路径" "快捷方式路径"
```

### 评分阈值

设置绿色软件 exe 程序评分时的判定阈值（百分比），低于此分数则判定为没有主程序。默认值为`0.3`。

```bash
AutoShortcut.exe --score-ratio 0.5
```

## 配置文件（可选）

```bash
AutoShortcut.exe [选项] ["程序路径"] ["快捷方式路径"] [--config "配置文件路径"]
```

### 简介

为了更有效的创建快捷方式，满足用户个性化需求，**可选的**引入配置文件。配置文件包括以下功能：

- 全局配置工具选项；
- 配置创建方式信息（名称、命令行、图标等）；
- 为未匹配的程序创建快捷方式;

使用方法:

1. 新建文本文件，将后缀重命名为`.toml`
2. 使用文本工具打开（如记事本等），填写相应的配置信息

### 全局设置

- 忽略列表

  配置扫描中需要忽略的文件或目录，后续扫描目录/文件将忽略。默认值为系统特殊目录（如回收站目录等）。

  ```toml
  ignore = ["temp", "Everything.exe", "D:\Everything"]
  ```

- 安装脚本

  如设置为`true`则自动运行软件目录内的: `install.cmd/bat`、`setup.cmd/bat`、`绿化.cmd/bat`

  ```toml
  install = true
  ```

- 并行运行脚本

  如设置为`true`则不等待运行脚本执行完成，而是并行运行所有脚本。默认值为`false`。

  ```toml
  install_parallel = true
  ```

- 脚本规则

  配置安装软件的脚本文件类型（默认支持`setup.cmd/bat`、`install.cmd/bat`、`绿化.cmd/bat`）

  ```toml
   # 示例：支持全部cmd、bat脚本
   scripts = ["*.cmd", "*.bat"]
  ```

- 使用原始文件名

  如设置为`true`则所有快捷方式名称将使用原始 exe 文件名。

  ```toml
  use_filename = true
  ```

- 仅匹配配置文件

  如设置为`true`则仅将配置文件中的快捷方式信息创建快捷方式，配置文件中未指定的快捷方式将不会创建。

  ```toml
  only_match = true
  ```

- 评分阈值

  设置为绿色软件 exe 程序评分时的判定阈值（百分比），低于此分数则判定为没有主程序，设置为`0`表示无阈值。默认值为`0.3`。

  ```toml
  score_ratio = 0.5
  ```

- 开启转义

  为了方便表示 Windows 路径，默认关闭转义功能，如需转义可通过配置开启。默认值为`false`。

  ```toml
  enable_escape = true
  ```

### 快捷方式定义

本程序支持多种快捷方式属性写法，功能一致，可根据个人喜好自由选择。

|      配置项       | 说明                                                           |
|:--------------:|--------------------------------------------------------------|
|     `exec`     | 程序路径（必填），其它配置项均为可选项                                          |
|     `name`     | 快捷方式名称                                                       |
|     `icon`     | 图标路径，支持相对路径（顺序为：程序所在路径、工作路径、系统路径），也支持指定图标索引如 `shell32.dll#1` |
|     `args`     | 命令行                                                          |
|   `work_dir`   | 工作路径（起始位置）                                                   |
|     `dest`     | 快捷方式存放路径                                                     |
| `window_state` | 显示模式：`normal`（激活并显示） / `minimized`（最大化） / `maximized`（最小化）   |
|   `comment`    | 备注                                                           |

- 配置项模式

  ```toml
  # 此处配置仅为字段说明
  [[shortcut]]
  name = "快捷方式名称"
  exec = "程序路径"
  work_dir="起始位置"
  args = "命令行参数"
  icon = "图标路径"
  dest = "快捷方式位置"
  window_state = "显示模式"
  comment = "备注"

  # 示例
  [[shortcut]]
  name = "Everything"
  exec = "Everything.exe"
  ```

- 内联模式

  > 提示：内联模式中每一项快捷方式信息都必须在一行内，不可换行。

  ```toml
  shortcut = [
    # 此处配置仅为字段说明
    { name = "快捷方式名称", exec = "程序路径", work_dir="起始位置", args = "命令行参数", icon = "图标路径", dest = "快捷方式位置" },
    # 示例
    { name = "Everything", exec = "Everything.exe" },
  ]
  ```

- 映射表模式

  ```toml
  # 配置快捷方式名称
  [name]
  "程序路径" = "快捷方式名称"
  # 示例
  "Everything.exe" = "Everything"

  # 配置快捷方式参数
  [args]
  "程序路径" = "命令行参数"
  # 示例
  "Everything.exe" = "-s example.txt"

  # 配置快捷方式起始路径
  [work_dir]
  "程序路径" = "起始路径"
  # 示例
  "Everything.exe" = "D:\Everything"

  # 配置快捷方式图标
  [icon]
  "程序路径" = "图标路径"
  # 示例
  "Everything.exe" = "D:\Everything\Everything.ico"

  # 配置快捷方式备注
  [comment]
  "程序路径" = "备注"
  # 示例
  "Everything.exe" = "文件搜索"
  ```

### 支持内置变量

在配置文件中支持使用**环境变量**和以下内置变量：

|          变量           | 说明         |
|:---------------------:|------------|
|      `%CurDir%`       | 配置文件目录     |
|      `%CurFile%`      | 配置文件名称     |
|      `%CurDrv%`       | 配置文件驱动器    |
|     `%Personal%`      | 我的文档目录     |
|     `%Programs%`      | 程序菜单目录     |
|      `%SendTo%`       | 发送到目录      |
|     `%StartMenu%`     | 开始菜单目录     |
|      `%Startup%`      | 启动菜单目录     |
|    `%QuickLaunch%`    | 快速启动栏      |
|      `%Desktop%`      | 桌面目录       |
|     `%Downloads%`     | 下载目录       |
|      `%Videos%`       | 视频目录       |
|     `%Pictures%`      | 图片目录       |
|       `%Music%`       | 音乐目录       |
|     `%Favorites%`     | 收藏夹目录      |
|   `%PublicDesktop%`   | 公共桌面目录     |
|  `%PublicDownloads%`  | 公共下载目录     |
|   `%PublicVideos%`    | 公共视频目录     |
|  `%PublicPictures%`   | 公共图片目录     |
|    `%PublicMusic%`    | 公共音乐目录     |
|  `%PublicPersonal%`   | 公共文档目录     |
|   `%ProgramFiles%`    | 程序目录       |
| `%ProgramFiles(x86)%` | 程序目录（32 位） |

### 纯配置模式

为了更好的满足个性化需求，除扫描模式外提供纯配置模式，纯配置模式将不进行目录扫描，仅根据配置文件所提供的信息来创建快捷方式。

`AutoShortcut.exe --config 配置文件路径`

> 提示：
>
> - 纯配置模式下，**`exec`（程序路径）、`dest`（快捷方式路径） 两项都必须填写**；
> - 命令行中无需传 `<TARGET_PATH>`（程序路径）、`<LNK_PATH>`（快捷方式路径）；
> - `name`（快捷方式名称）省略时将使用程序描述作为快捷方式名称；
> - 同理支持配置项模式、内联模式、映射表模式三种写法

```toml
shortcut = [
    # 此处配置仅为字段说明
    { name = "快捷方式名称", exec = "程序路径", args = "命令行参数", icon = "图标路径", dest = "快捷方式路径" },
    # 示例：创建桌面快捷方式
    { name = "Everything", exec = "C:\Program Files\Everything\Everything.exe", dest = "%Desktop%" },
    # 示例：使用程序名称作为快捷方式名称
    { exec = "C:\Program Files\Everything\Everything.exe", dest = "%Desktop%" },
]
```

## 快捷方式属性模板

> 提示：
>
> - 快捷方式属性模板仅在`[Template]`配置项中生效。

`AutoShortcut` 提供强大的模板系统，使用规则模板来全局定义快捷方式的名称、图标等属性。模板处理分为两个阶段：

1. **控制流阶段**：先解析并执行表达式。
2. **替换阶段**：将剩余的占位变量替换为对应值。

### 1. 变量替换

> 提示：
>
> - 模板中所有变量均需要用大括号 `{}` 包裹。
> - 变量名区分大小写。

内置多种变量，可在模板中直接引用：

#### 📄 文件信息变量

|       变量        | 说明                                 |
|:---------------:|------------------------------------|
|    `{exec}`     | 完整路径，如 `D:\Apps\WeChat\WeChat.exe` |
|    `{stem}`     | 文件名（不含扩展名）                         |
|     `{ext}`     | 文件扩展名，如 `exe`                      |
|   `{parent}`    | 程序父目录路径                            |
| `{parent_name}` | 程序父目录名称                            |
|    `{size}`     | 文件大小（字节）                           |
|   `{size_kb}`   | 文件大小（千字节）                          |
|   `{size_mb}`   | 文件大小（兆字节）                          |
|   `{size_gb}`   | 文件大小（千兆字节）                         |
|   `{size_tb}`   | 文件大小（太字节）                          |
| `{create_time}` | 创建时间                               |
| `{modify_time}` | 修改时间                               |
| `{access_time}` | 访问时间                               |

#### 📝 元数据变量（来自文件版本信息）

|        变量         | 说明                   |
|:-----------------:|----------------------|
|    `{product}`    | 产品名称（ProductName）    |
|  `{product_raw}`  | 原始产品名称               |
|     `{desc}`      | 程序描述（已清理非法字符）        |
|   `{desc_raw}`    | 原始程序描述               |
|     `{arch}`      | 架构（x86/x64/arm64）    |
|   `{arch_num}`    | 架构数字（32/64/arm64）    |
|    `{version}`    | 文件版本                 |
|    `{company}`    | 公司名称（CompanyName）    |
| `{orig_filename}` | 原始文件名                |
|   `{copyright}`   | 版权声明（LegalCopyright） |

### 2. 条件语法

- **条件（三元）语法**：`{cond ? then : else}`、`{条件变量 ? 条件成立时的内容 : 条件不成立时的内容}`

    - `cond` 为条件标识（不带花括号），当 `cond` 对应变量非空时渲染 `then`，否则渲染 `else`。支持取反：`!cond`。
    - `then` 和 `else` 为模板片段，允许使用占位符（例如 `desc`、`stem`）。
    - `else` 可省略（省略时不输出任何内容）。
    - `cond` / `then` / `else` 内可包含其他模板（支持嵌套）。

  示例：

    - `{arch_num ? arch_num + '位' : '未知位数'}`: 如果 `arch_num` 存在用 `arch_num` 加上“位”，否则用“未知位数”。
    - `{product ? product : desc}`: 如果 `product` 存在用 `product`，否则用 `desc`。

- **默认值语法**: `{var ?? default}`、`{变量 ?? 默认值}`

    - 当变量 `var` 非空时使用其值；为空时渲染 `default`（`default` 可为嵌套模板）。
    - 等价于 `{var ? var : default}`。

  示例：

    - `{product ?? desc}`: 如果 `product` 存在用 `product`，否则用 `desc`。
    - `{desc ?? stem}`: 如果 `desc` 存在用 `desc`，否则用 `stem`。

### 3. 过滤器语法

过滤器用于对变量进行处理，例如转换为小写、大写、标题等。语法: `{var | filter: arg}`

支持的过滤器：

- 转小写: `{var | lower}`  
  将字符串转换为小写。  
  例如 `{desc | lower}` 表示“将 `desc` 转换为小写”。
- 转大写: `{var | upper}`  
  将字符串转换为大写。  
  例如 `{desc | upper}` 表示“将 `desc` 转换为大写”。
- 首字母大写: `{var | capitalize}`  
  将字符串的第一个字符转换为大写，其他字符转换为小写。  
  例如 `{desc | capitalize}` 表示“将 `desc` 的第一个字符转换为大写，其他字符转换为小写。
- 标题: `{var | title}`  
  将字符串转换为标题大小写，即**每个单词的首字母大写，其余字符小写**。此标签不会将“无关紧要的单词”转换为小写。  
  例如 `{desc | title}` 表示“将 `desc` 转换为标题大小写”。
- 去除空格: `{var | trim}`  
  去除字符串首尾的空格。  
  例如 `{desc | trim}` 表示“将 `desc` 首尾的空格去除”。
- 计算字符串长度: `{var | length}`  
  计算字符串的长度（字符数）。  
  例如 `{desc | length}` 表示“计算 `desc` 的长度”。
- 删除文本: `{var | cut:"str"}`  
  删除字符串中的指定文本。  
  例如 `{desc | cut:"微信"}` 表示“将 `desc` 中的“微信”删除”。
- 替换文本: `{var | replace:"old","new"}`  
  替换字符串中的指定文本。  
  例如 `{desc | replace:"微信","WeChat"}` 表示“将 `desc` 中的“微信”替换为“WeChat””。
- 截断字符串: `{var | truncate:10}`  
  截断字符串为指定长度，超出部分省略。  
  例如 `{desc | truncate:10}` 表示“如果 `desc` 长度超过 10 个字符，截取前 10 个字符”。  
  若 `desc` 长度不超过 10 个字符，保持不变。
- 日期格式化: `{var | date:"format"}`  
  格式化日期变量。`format` 为日期格式字符串，例如 `"{create_time | date:'%Y-%m-%d %H:%M:%S'}"`。
  支持的日期格式占位符：
    - `%Y`：四位数年份（例如 2023）
    - `%m`：月份（01-12）
    - `%d`：日期（01-31）
    - `%H`：24 小时格式小时（00-23）
    - `%M`：分钟（00-59）
    - `%S`：秒（00-59）

#### 常用示例

- 使用产品名称（有则用产品名称，否则用文件名）：

  ```toml
  [template]
  name = "{product ?? stem}"
  ```

- 显示版本与位数（仅当对应变量存在时）：

  ```toml
  [template]
  name = "{desc ?? stem} {version? 'v' + version : ''} {arch_num? arch_num + '位' : ''}"
  ```

  说明：`{version? 'v' + version : ''}` 表示有 `version` 就输出 `'v' + version`（注意前面的空格由模板控制）

- 统一图标文件路径、备注

  ```toml
  [template]
  icon = "D:\Icons\{stem}.ico"
  comment = "{exec}"
  ```

> 关于 `{cond ? then : else}` 与 `{var ?? default}` 的选择
>
> - `{var ?? default}` 通常更短、更直观，适合“如果变量为空则回退”场景（例如 `product` 缺失用 `stem`）。
> - `{cond ? then : else}` 更灵活，可写更复杂的逻辑并包含 `!` 取反。若只做“缺失回退”，优先使用 `{var ?? default}`。

## 示例配置

### 基础配置示例

- 配置快捷方式名称

  ```toml
  shortcut = [
    { name = "文件搜索", exec = "Everything.exe"},
    { name = "文件复制", exec = "FastCopy.exe"},
  ]
  ```

- 仅为配置文件中的程序创建快捷方式，并使用模板

  ```toml
  only_match = true

  shortcut = [
    { exec = "Everything.exe"},
    { exec = "FastCopy.exe"},
  ]

  [template]
  name = "{desc ?? stem} {version? 'v' + version : ''} {arch_num? arch_num + '位' : ''}"
  ```

### 高级配置示例

<details> <summary>点击查看：包含模板和安装脚本的完整配置</summary>

config.toml

```toml
ignore = ["Common Files", "Internet Explorer", "Windows Defender", "Windows Mail", "Windows Media Player", "Windows NT", "Windows Photo Viewer", "Windows Security", "EdgeCore", "EdgeUpdate", "WindowsApps", "dotnet"]
scripts = ["*.cmd", "*.bat"]
shortcut = [
    { exec = "WeChat.exe", name = "微信" },
    { exec = "QQ.exe", name = "腾讯QQ" },
    { exec = "DingTalk.exe", name = "钉钉" },
    { exec = "Feishu.exe", name = "飞书" },
    { exec = "TIM.exe", name = "TIM办公" },
    { exec = "wemeetapp.exe", name = "腾讯会议" },
    { exec = "wps.exe", name = "WPS办公" },
    { exec = "FoxitReader.exe", name = "福昕阅读器" },
    { exec = "WinRAR.exe", name = "WinRAR压缩" },
    { exec = "360zip.exe", name = "360压缩" },
    { exec = "Notepad++.exe", name = "记事本++" },
    { exec = "DrvCeo.exe", name = "驱动总裁" },
    { exec = "UltraISO.exe", name = "软碟通" },
    { exec = "chrome.exe", name = "谷歌浏览器" },
    { exec = "msedge.exe", name = "Edge" },
    { exec = "360se.exe", name = "360安全浏览器" },
    { exec = "QQBrowser.exe", name = "QQ浏览器" },
    { exec = "SogouExplorer.exe", name = "搜狗浏览器" },
    { exec = "UCBrowser.exe", name = "UC浏览器" },
    { exec = "360Safe.exe", name = "360安全卫士" },
    { exec = "Huorong.exe", name = "火绒安全" },
    { exec = "QQPCMgr.exe", name = "腾讯电脑管家" },
    { exec = "BaiduSd.exe", name = "百度杀毒" },
    { exec = "Thunder.exe", name = "迅雷" },
    { exec = "BaiduNetdisk.exe", name = "百度网盘" },
    { exec = "MicroCloud.exe", name = "腾讯微云" },
    { exec = "alist.exe", name = "阿里云盘" },
    { exec = "eCloud.exe", name = "天翼云盘" },
    { exec = "QuarkCloudDrive.exe", name = "夸克网盘" },
    { exec = "123pan.exe", name = "123云盘" },
    { exec = "mCloud.exe", name = "移动云盘" },
    { exec = "Kanbox.exe", name = "坚果云" },
    { exec = "Dropbox.exe", name = "Dropbox" },
    { exec = "XunleiDownload.exe", name = "迅雷下载" },
    { exec = "IDMan.exe", name = "IDM下载器" },
    { exec = "BitComet.exe", name = "比特彗星" },
    { exec = "cloudmusic.exe", name = "网易云音乐" },
    { exec = "QQMusic.exe", name = "QQ音乐" },
    { exec = "Kugou.exe", name = "酷狗音乐" },
    { exec = "douyin_launcher.exe", name = "抖音" },
    { exec = "QQLive.exe", name = "腾讯视频" },
    { exec = "QyClient.exe", name = "爱奇艺" },
    { exec = "Youku.exe", name = "优酷" },
    { exec = "PotPlayerMini32.exe", name = "PotPlayer" },
    { exec = "PotPlayerMini64.exe", name = "PotPlayer" },
    { exec = "StormPlayer.exe", name = "暴风影音" },
    { exec = "WeGame.exe", name = "腾讯WeGame" },
    { exec = "steam.exe", name = "Steam平台" },
    { exec = "mihoyo.exe", name = "米哈游启动器" },
    { exec = "Honeyview.exe", name = "蜂蜜看图" },
    { exec = "2345Pic.exe", name = "2345看图王" },
    { exec = "JianyingPro.exe", name = "剪映" },
    { exec = "Code.exe", name = "Visual Studio Code" },
    { exec = "e.exe", name = "易语言" },
    { exec = "Eclipse.exe", name = "Eclipse开发工具" },
    { exec = "VMware.exe", name = "VMware虚拟机" },
    { exec = "MuMuPlayer.exe", name = "MUMU模拟器" },
    { exec = "Navicat.exe", name = "Navicat数据库" },
    { exec = "TeamViewer.exe", name = "TeamViewer" },
    { exec = "SunloginClient.exe", name = "向日葵远程" },
    { exec = "AnyDesk.exe", name = "AnyDesk远程" },
    { exec = "ToDesk.exe", name = "ToDesk远程" }
]
```

</details>

### 纯配置模式示例

```toml
# 纯配置模式：不扫描目录，只创建指定的快捷方式
[[shortcut]]
name = "微信"
exec = "D:\Software\WeChat\WeChat.exe"
dest = "%Desktop%"

[[shortcut]]
name = "Chrome浏览器"
exec = "C:\Program Files\Google\Chrome\chrome.exe"
args = "--start-maximized"
dest = "%Programs%\浏览器"
```

## 常见问题

### Q: 为什么某些程序没有创建快捷方式？

**A:** 可能的原因包括：

- 程序目录结构不合理
- 程序目录被添加到`ignore列`表中
- 绿色软件评分低于设定阈值（默认0.3）
- 使用`--only-match`模式但程序不在配置列表中
- 程序可能是后台服务而非GUI应用

### Q: 如何自定义快捷方式的名称？

**A:** 有两种方法：

1. 在`[[shortcut]]`中为单个程序指定：
    ```toml
    [[shortcut]] 
    name = "微信"
    ```
2. 使用模板统一设置：

    ```toml
    [template] 
    name = "{stem}"
    ```

### Q: 创建了不需要的快捷方式

**A:** 在 ignore = ["dirname"] 中添加文件名或目录名。

### Q: 纯配置模式和普通模式有什么区别？

**A:**

- 普通模式：会自动扫描指定目录并智能识别程序
- 纯配置模式（`--config`）：不进行目录扫描，仅根据配置文件创建快捷方式
- 混合模式：扫描目录的同时，使用配置文件覆盖或增强某些设置

### Q: 如何在命令行和配置文件中使用环境变量？

**A:** 支持Windows标准环境变量和程序内置变量，使用`%变量名%`格式，例如：`%Desktop%`、`%ProgramFiles%`、`%CurDir%`等。

## 开源许可

`AutoShortcut` 使用 MPL V2.0 协议开源，请尽量遵守开源协议。

## 致谢

- slore
- qq826773297
- liangnijian

## 参与贡献

1. Fork 本仓库
2. 新建 Feat_xxx 分支
3. 提交代码
4. 新建 Pull Request
