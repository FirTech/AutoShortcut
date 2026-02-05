# AutoShortcut

[简体中文](README.zh.md) | English

## Introduction

AutoShortcut is an automated shortcut creation tool designed for efficient software shortcut management. It
intelligently identifies various software directory structures, automatically analyzes and creates optimal shortcuts,
significantly improving the cleanliness and efficiency of your desktop and Start menu.

### 🎉 **AutoShortcut Advantages**

- 🔍 **Intelligent Identification**: Automatically identifies portable software and single-file applications and creates
  corresponding shortcuts
- ⚡ **Multi-dimensional Scoring**: Identifies the main program using a multi-dimensional scoring algorithm
- 🛠 **Rich Customization**: Supports advanced features such as template naming, ignore rules, and batch creation
- 📂 **Dual-Mode Support**: Supports both command-line and configuration files to meet different usage scenarios
- 🌍 **Multi-language Interface**: Built-in Chinese and English support for easy internationalization
- 🎯 **Flexible Configuration**: Provides detailed configuration options to meet personalized needs

### 🔍 **AutoShortcut Logic**

**Directory Type Definition:**

- `Category Directory`: A directory containing multiple software directories. Each software directory can
  contain `Single-File Program Directories` and `Portable Software Directories`
- `Single-File Program Directories`: A directory containing only multiple EXE programs, excluding other files (including
  configuration files).
- Green Software: Contains program dependencies and may contain multiple EXE programs.

> Tips
>
> - If you maintain a standard software directory structure (following the aforementioned principles for "classified

    directories," "single-file program directories," and "green software"), AutoShortcut can intelligently identify and
    create shortcuts without the need for additional configuration files.

> - Special reminder: Avoid mixed storage (for example, including green software and its dependencies in a single-file

    program directory). This irregular structure may prevent the program from correctly identifying the software. If you
    encounter recognition issues, first check and adjust your software directory structure, and use a configuration file
    to assist with definition if necessary.

AutoShortcut scans directories based on the following logic:

1. Scans each subdirectory of the specified program directory.
2. Identifies the subdirectory type:

- If a "classified directory" is identified, it continues scanning the next subdirectory.
- If a "single-file program directory" is identified, a shortcut is created for each EXE program.
- If a "green software" is identified, each EXE program is evaluated across multiple dimensions:

- Whether the program name matches the parent directory name.
- Whether the program has a description.
- Whether the program is a user interface program.
- Whether the program has an icon.
- Whether the program has a digital signature.
- Whether the program matches the current system architecture.
- Whether the program name contains auxiliary keywords.
- (When a configuration file is specified only) Scores are assigned based on the program file name specified in the
  configuration file.

The highest-scoring program is identified as the main program, and subsequent accesses to the green software root
directory are skipped. If the score does not meet the threshold (the default threshold is 30% of the maximum score), it
is determined to have no main program and the directory is skipped.

Tips

- By default, the shortcut name will be processed (removing URLs and special characters), and the program name will be
  restored as much as possible.
- If the product name is not a valid one (such as `7z setup sfx`), the shortcut name will be restored to its original
  file name.

## 🚀 Quick Start

This program is a command-line program, so it requires parameters to run. You can run it through terminals such as `cmd`
and `PowerShell`.

```bash
# Example 1: Scan the Program Files directory and create shortcuts to all programs on the desktop
AutoShortcut.exe "C:\Program Files" "%Desktop%"

# Example 2: Scan the Program Files directory and create shortcuts in the Start menu, categorizing them by program
AutoShortcut.exe -d "C:\Program Files" "%Programs%"

# Example 3: Scan and automatically run specific software (suitable for portable software)
AutoShortcut.exe -s "C:\Program Files\Everything"
```

> Tips:
>
> - Windows recommends running as an administrator to prevent UAC blocking;
> - Use quotes for paths containing spaces;
> - Environment variables (such as %USERPROFILE% and %Programs%) can be used directly, and built-in program variables

    are also supported.

## Command Line Usage

> Tip: Use quotes if the path contains spaces.

Usage: `AutoShortcut.exe [options] ["Program Path"] ["Shortcut Path"] [--config "Configuration File Path"]`

### Basic Usage

```bash
AutoShortcut.exe "Program Path" "Shortcut Path"
```

- Create a desktop shortcut: `AutoShortcut.exe "C:\Program Files" %Desktop%`
- Create a Start Menu shortcut: `AutoShortcut.exe "C:\Program Files" "%Programs%"`

### Create a program folder

```bash
`AutoShortcut.exe -d "Program Path" "Shortcut Path"`
```

- `AutoShortcut.exe -d "C:\Program Files" "%Programs%"`

### Try installing the software.

This will automatically run the following commands in the software directory: `install.cmd/bat`, `setup.cmd/bat`,
and `greening.cmd/bat`.

```bash
AutoShortcut.exe -i "Program path" "Shortcut path"
```

- `AutoShortcut.exe -i "C:\Program Files" "%Programs%"`

### Run the program

Automatically run the program after identifying the main program.

```bash
AutoShortcut.exe --start "Program path"
```

### Use the original file name

Set the shortcut name to use the original exe file name.

```bash
AutoShortcut.exe --use-filename "Program path" "Shortcut path"
```

### List only shortcuts

Only lists the program paths for which shortcuts need to be created; shortcut creation will not occur.

```bash
AutoShortcut.exe --list "Program Path" "Shortcut Path"
```

### Match Configuration File Only

Create shortcuts based on only the shortcut information in the configuration file

```bash
AutoShortcut.exe --only-match "Program Path" "Shortcut Path"
```

### Scoring Threshold

Set the scoring threshold (percentage) for green software EXE programs. Scores below this threshold are considered to
have no main program. The default value is 0.3.

```bash
AutoShortcut.exe --score-ratio 0.5
```

### Configuration File (Optional)

```bash
AutoShortcut.exe [options] ["Program Path"] ["Shortcut Path"] [--config "Configuration File Path"]
```

### Introduction

To create shortcuts more efficiently and meet user needs, a configuration file is optionally introduced. The
configuration file includes the following features:

- Global configuration tool options;
- Configure creation method information (name, command line, icon, etc.);
- Create shortcuts for unmatched programs;

Instructions:

1. Create a new text file and rename it to `.toml`.
2. Open it with a text tool (such as Notepad) and fill in the corresponding configuration information.

### Global Settings

- Ignore List

Configure files or directories to be ignored during scanning. Subsequent scans will ignore these directories/files. The
default value is system special directories (such as the Recycle Bin).

```toml
ignore = ["temp", "Everything.exe", "D:\Everything"]
```

- Installation Scripts

If set to `true`, automatically runs `install.cmd/bat`, `setup.cmd/bat`, and `greening.cmd/bat` in the software
directory.

```toml
install = true
```

- Parallel Script Execution

If set to `true`, all scripts will be run in parallel without waiting for the script to complete. Default value
is `false`.

```toml
install_parallel = true
```

- Script rules

Configure the script file type for installing the software (default supports `setup.cmd/bat`, `install.cmd/bat`,
and `green.cmd/bat`)

```toml
# Example: Supports all cmd and bat scripts
scripts = ["*.cmd", "*.bat"]
```

- Use original filename

If set to `true`, all shortcut names will use the original exe file name.

```toml
use_filename = true
```

- Match configuration file only

If set to `true`, only shortcut information in the configuration file will be created. Shortcuts not specified in the
configuration file will not be created.

```toml
only_match = true
```

- Scoring threshold

Set the threshold (in percentage) for scoring green software exe programs. Scores below this threshold are considered to
have no main program. Setting `0` indicates no threshold. The default value is 0.3.

```toml
score_ratio = 0.5
```

- Enable escaping

To facilitate Windows path representation, escaping is disabled by default. If required, it can be enabled via
configuration. The default value is false.

```toml
enable_escape = true
```

### Shortcut Definition

This program supports multiple shortcut attribute writing methods, all with consistent functionality. You can choose
according to your preference.

| Configuration Item | Description                                                                                                                                                 |
| :----------------: | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
|       `exec`       | Program path (required), all other configuration items are optional                                                                                         |
|       `name`       | Shortcut name                                                                                                                                               |
|       `icon`       | Icon path, supports relative paths (in the order: program path, work path, system path), and also supports specifying an icon index such as `shell32.dll#1` |
|       `args`       | Command line                                                                                                                                                |
|     `work_dir`     | Working path (starting location)                                                                                                                            |
|       `dest`       | Shortcut storage path                                                                                                                                       |
|   `window_state`   | Display mode: `normal` (activated and displayed) / `minimized` (maximized) / `maximized` (minimized)                                                        |
|     `comment`      | Comment                                                                                                                                                     |
|      `hotkey`      | Hotkey, format like "Ctrl + Alt + N"                                                                                                                        |

- Configuration Item Mode

```toml
# The configuration here is only for field descriptions
[[shortcut]]
name = "shortcut name"
exec = "program path"
work_dir = "starting location""
args = "Command-line arguments"
icon = "Icon path"
dest = "Shortcut location"
window_state = "Display mode"
comment = "Remarks"
hotkey = "Hotkey"

# Example
[[shortcut]]
name = "Everything"
exec = "Everything.exe"
hotkey = "Ctrl + Alt + E"
```

- Inline mode

> Tip: Each shortcut information in inline mode must be on a single line, without line breaks.

```toml
shortcut = [
    # The configuration here is just a field description
    { name = "Shortcut name", exec = "Program path", work_dir = "Starting location", args = "Command-line arguments", icon = "Icon path", dest = "Shortcut location", hotkey = "Hotkey" },
    # Example
    { name = "Everything", exec = "Everything.exe", hotkey = "Ctrl + Alt + E" },
]
```

- Mapping table mode

````toml
# Configure shortcut name
[name]
"Program path" = "Shortcut name"
# Example
"Everything.exe" = "Everything"

# Configure shortcut parameters
[args]
"Program Path" = "Command Line Arguments"
# Example
"Everything.exe" = "-s example.txt"

# Configure shortcut starting directory
[work_dir]
"Program Path" = "Starting Path"
# Example
"Everything.exe" = "D:\Everything"

# Configure shortcut icon
[icon]
"Program Path" = "Icon Path"
# Example
"Everything.exe" = "D:\Everything\Everything.ico"

# Configure shortcut comment
[comment]
"Program Path" = "Comment"
# Example
"Everything.exe" = "File Search"

# Configure shortcut hotkey
[hotkey]
"Program Path" = "Hotkey"
# Example
"Everything.exe" = "Ctrl + Alt + E"

### Support for Built-in Variables

Environment variables and the following built-in variables are supported in the configuration file:

| Variable | Description |
|:---------------------:|------------|
| `%CurDir%` | Profile directory |
| `%CurFile%` | Profile name |
| `%CurDrv%` | Profile drive |
| `%Favorites%` | Full path to Favorites folder |
| `%Personal%` | My Documents directory |
| `%Programs%` | Program menu directory |
| `%SendTo%` | Send to directory |
| `%StartMenu%` | Start menu directory |
| `%Startup%` | Start menu directory |
| `%QuickLaunch%` | Quick Launch bar |
| `%Desktop%` | Desktop directory name |
| `%ProgramFiles%` | Program directory |
| `%ProgramFiles(x86)%` | Program directory (32-bit) |

### Pure Configuration Mode

To better meet personalized needs, in addition to scanning mode, pure configuration mode is provided. Pure configuration mode does not perform directory scanning and creates shortcuts based solely on the information provided in the configuration file.

`AutoShortcut.exe --config configuration file path`

> Tips:
>
> - In pure configuration mode, both `exec` (program path) and `dest` (shortcut path) are required.
> - `<TARGET_PATH>` (program path) and `<LNK_PATH>` (shortcut path) are not required in the command line.
> - If `name` (shortcut name) is omitted, the program description will be used as the shortcut name.
> - Similarly, configuration item mode, inline mode, and mapping table mode are supported.

```toml
shortcut = [
    # The configuration here is only for field descriptions
    { name = "shortcut name", exec = "program path", args = "command line arguments", icon = "icon path", dest = "shortcut path" },
    # Example: Create a desktop shortcut
    { name = "Everything", exec = "C:\Program Files\Everything\Everything.exe", dest = "%Desktop%" },
    # Example: Using the program name as the shortcut name
    { exec = "C:\Program Files\Everything\Everything.exe", dest = "%Desktop%" },
]
````

## Shortcut Property Template

> Tips:
>
> - Shortcut property templates are only valid in the `[Template]` configuration option.

`AutoShortcut` provides a powerful template system that uses rule templates to globally define shortcut properties such
as the name and icon. Template processing is divided into two phases:

1. **Control Flow Phase**: First, the expression is parsed and executed.

2. **Replacement Phase**: Remaining placeholder variables are replaced with corresponding values.

### 1. Variable Substitution

> Tips:
>
> - All variables in the template must be enclosed in curly braces `{}`.
> - Variable names are case-sensitive.

A variety of built-in variables are available, which can be referenced directly in templates:

#### 📄 File Information Variables

|    Variable     | Description                                    |
| :-------------: | ---------------------------------------------- |
|    `{exec}`     | Full path, such as `D:\Apps\WeChat\WeChat.exe` |
|    `{stem}`     | File name (excluding extension)                |
|     `{ext}`     | File extension, such as `exe`                  |
|   `{parent}`    | Program parent directory path                  |
| `{parent_name}` | Program parent directory name                  |
|    `{size}`     | File size (bytes)                              |
|   `{size_kb}`   | File size (kilobytes)                          |
|   `{size_mb}`   | File size (megabytes)                          |
|   `{size_gb}`   | File size (gigabytes)                          |
|   `{size_tb}`   | File size (terabytes)                          |
| `{create_time}` | Creation time                                  |
| `{modify_time}` | Modification time                              |
| `{access_time}` | Access time                                    |

#### 📝 Metadata variables (from file version information)

|     Variable      | Description                                         |
| :---------------: | --------------------------------------------------- |
|    `{product}`    | Product name (ProductName)                          |
|  `{product_raw}`  | Raw product name                                    |
|     `{desc}`      | Program description (cleaned of illegal characters) |
|   `{desc_raw}`    | Raw program description                             |
|     `{arch}`      | Architecture (x86/x64/arm64)                        |
|   `{arch_num}`    | Architecture number (32/64/arm64)                   |
|    `{version}`    | File version                                        |
|    `{company}`    | Company name (CompanyName)                          |
| `{orig_filename}` | Original file name                                  |
|   `{copyright}`   | Copyright Notice (LegalCopyright)                   |

### 2. Conditional Syntax

- **Conditional (Ternary) Syntax
  **: `{cond ? then : else}`, `{Condition variable ? Content if condition is true : Content if condition is false}`

- `cond` is a conditional identifier (without curly braces). `then` is rendered if the variable corresponding to `cond`
  is not null, otherwise `else` is rendered. Negation is supported: `!cond`.
- `then` and `else` are template fragments, allowing placeholders (such as `desc` and `stem`).
- `else` can be omitted (no content is output if omitted).
- `cond` / `then` / `else` can contain other templates (nesting is supported).

Example:

- `{arch_num ? arch_num + 'bits' : 'unknown bit number'}`: If `arch_num` exists, `then` is rendered. `arch_num` adds "
  digits", otherwise uses "unknown digits".
- `{product ? product : desc}`: If `product` exists, use `product`; otherwise, use `desc`.

- **Default value syntax**: `{var ?? default}`, `{variable ?? default}`

- Use the value of `var` if it's not empty; render `default` if it's empty (`default` can be a nested template).
- Equivalent to `{var ? var : default}`.

Example:

- `{product ?? desc}`: If `product` exists, use `product`; otherwise, use `desc`.
- `{desc ?? stem}`: If `desc` exists, use `desc`; otherwise, use `stem`.

### 3. Filter Syntax

Filters are used to process variables, such as converting to lowercase, uppercase, or titlecase.
Syntax: `{var | filter: arg}`

Supported filters:

- Lowercase: `{var | lower}`
  Converts a string to lowercase.

For example, `{desc | lower}` means "Convert `desc` to lowercase."

- Uppercase: `{var | upper}`
  Converts a string to uppercase.

For example, `{desc | upper}` means "Convert `desc` to uppercase."

- Capitalize: `{var | capitalize}`
  Converts the first character of a string to uppercase and all other characters to lowercase.

For example, `{desc | capitalize}` means "Convert the first character of `desc` to uppercase and all other characters to
lowercase."

- Title: `{var | title}`
  Converts a string to title case, i.e., **the first letter of each word is capitalized, and the rest of the characters
  are lowercase**. This tag does not convert "insignificant words" to lowercase.
  For example, `{desc | title}` means "Convert `desc` to title case."
- Trim spaces: `{var | trim}`
  Trims leading and trailing spaces from a string.
  For example, `{desc | trim}` means "Trims leading and trailing spaces from `desc`."
- Calculate string length: `{var | length}`
  Calculates the length (number of characters) of a string.
  For example, `{desc | length}` means "Calculates the length of `desc`."
- Delete text: `{var | cut:"str"}`
  Deletes specified text from a string.
  For example, `{desc | cut:"微信"}` means "Delete "微信" from `desc`." - Replace Text: `{var | replace:"old","new"}`
  Replaces the specified text in a string.
  For example, `{desc | replace:"微信","微信"}` means "Replace "微信" in `desc` with "微信"."
- Truncate String: `{var | truncate:10}`
  Truncates a string to a specified length, omitting any excess characters.
  For example, `{desc | truncate:10}` means "If `desc` is longer than 10 characters, truncate the first 10 characters."
  If `desc` is less than 10 characters, leave it unchanged.
- Format Date: `{var | date:"format"}`
  Formats a date variable. `format` is a date format string, for example, `"{create_time | date:'%Y-%m-%d %H:%M:%S'}"`.
  Supported date format placeholders:
- `%Y`: Four-digit year (e.g., 2023)
- `%m`: Month (01-12)
- `%d`: Day (01-31)
- `%H`: Hour in 24-hour format (00-23)
- `%M`: Minute (00-59)
- `%S`: Second (00-59)

#### Common Examples

- Use product name (if available, use product name; otherwise, use file name):

```toml
[template]
name = "{product ?? stem}"
```

- Display version and bit number (only if the corresponding variable exists):

```toml
[template]
name = "{desc ?? stem} {version? 'v' + version : ''} {arch_num? arch_num + 'bit' : ''}"
```

Note: `{version? 'v' + version : ''}` means `'v' + version` will be output if `version` is present (note that the
leading space is controlled by the template).

- Unified icon file paths and comments

```toml
[template]
icon = "D:\Icons\{stem}.ico"
comment = "{exec}"
```

> Choosing between `{cond ? then : else}` and `{var ?? default}`
>
> - `{var ?? default}` is generally shorter and more intuitive, suitable for "fallback if variable is empty" scenarios (

    e.g., using `stem` if `product` is missing).

> - `{cond ? then : else}` is more flexible, allowing for more complex logic and including `!` negation. For "fallback

    if missing", `{var ?? default}` is preferred.

## Example Configuration

### Basic Configuration Example

- Configure shortcut names and hotkeys

```toml
shortcut = [
    { name = "File Search", exec = "Everything.exe", hotkey = "Ctrl + Alt + E" },
    { name = "File Copy", exec = "FastCopy.exe", hotkey = "Ctrl + Shift + F" },
]
```

- Create shortcuts only for programs in the configuration file, using a template

```toml
only_match = true

shortcut = [
    { exec = "Everything.exe" },
    { exec = "FastCopy.exe" },
]

[template]
name = "{desc ?? stem} {version? 'v' + version : ''} {arch_num? arch_num + 'bit' : ''}"
```

### Advanced Configuration Example

<details> <summary>Click to view: Complete configuration including templates and installation scripts</summary>

config.toml

```toml
ignore = ["temp"]
scripts = ["*.cmd", "*.bat"]
shortcut = [
    { exec = "WeChat.exe", name = "微信" },
    { exec = "QQ.exe", name = "Tencent QQ" },
    { exec = "DingTalk.exe", name = "DingTalk" },
    { exec = "Feishu.exe", name = "Feishu" },
    { exec = "TIM.exe", name = "TIM Office" },
    { exec = "wemeetapp.exe", name = "Tencent Meeting" },
    { exec = "wps.exe", name = "WPS Office" },
    { exec = "FoxitReader.exe", name = "Foxit Reader" },
    { exec = "WinRAR.exe", name = "WinRAR Compression" },
    { exec = "360zip.exe", name = "360 Compression" },
    { exec = "Notepad++.exe", name = "Notepad++" },
    { exec = "DrvCeo.exe", name = "Driver President" },
    { exec = "UltraISO.exe", name = "Softdisk" },
    { exec = "chrome.exe", name = "Google Chrome" },
    { exec = "msedge.exe", name = "Edge" },
    { exec = "360se.exe", name = "360 Security Browser" },
    { exec = "QQBrowser.exe", name = "QQ Browser" },
    { exec = "SogouExplorer.exe", name = "Sogou Browser" },
    { exec = "360Safe.exe", name = "360 Security Guard" },
    { exec = "Huorong.exe", name = "Huorong Security" },
    { exec = "QQPCMgr.exe", name = "Tencent PC Manager" },
    { exec = "BaiduSd.exe", name = "Baidu Antivirus" },
    { exec = "Thunder.exe", name = "Thunder" },
    { exec = "BaiduNetdisk.exe", name = "Baidu Netdisk" },
    { exec = "MicroCloud.exe", name = "Tencent MicroCloud" },
    { exec = "alist.exe", name = "Ali Cloud Disk" },
    { exec = "eCloud.exe", name = "Tianyi Cloud Disk" },
    { exec = "QuarkCloudDrive.exe", name = "Quark Cloud Disk" },
    { exec = "123pan.exe", name = "123 Cloud Disk" },
    { exec = "mCloud.exe", name = "Mobile Cloud Disk" },
    { exec = "Kanbox.exe", name = "Nut Cloud" },
    { exec = "Dropbox.exe", name = "Dropbox" },
    { exec = "XunleiDownload.exe", name = "Xunlei Download" },
    { exec = "IDMan.exe", name = "IDM Downloader" },
    { exec = "BitComet.exe", name = "Bit Comet" },
    { exec = "cloudmusic.exe", name = "NetEase Cloud Music" },
    { exec = "QQMusic.exe", name = "QQ Music" },
    { exec = "Kugou.exe", name = "Kugou Music" },
    { exec = "douyin_launcher.exe", name = "TikTok" },
    { exec = "QQLive.exe", name = "Tencent Video" },
    { exec = "QyClient.exe", name = "iQiyi" },
    { exec = "Youku.exe", name = "Youku" },
    { exec = "PotPlayerMini32.exe", name = "PotPlayer" },
    { exec = "PotPlayerMini64.exe", name = "PotPlayer" },
    { exec = "StormPlayer.exe", name = "Storm Player" },
    { exec = "WeGame.exe", name = "Tencent WeGame" },
    { exec = "steam.exe", name = "Steam Platform" },
    { exec = "mihoyo.exe", name = "mihoyo Launcher" },
    { exec = "Honeyview.exe", name = "Honey Viewer" },
    { exec = "2345Pic.exe", name = "2345 Picture King" },
    { exec = "JianyingPro.exe", name = "Jianying" },
    { exec = "Code.exe", name = "Visual Studio Code" },
    { exec = "e.exe", name = "Easy Language" },
    { exec = "Eclipse.exe", name = "Eclipse Development Tools" },
    { exec = "VMware.exe", name = "VMware Virtual Machine" },
    { exec = "MuMuPlayer.exe", name = "MUMU Simulator" },
    { exec = "Navicat.exe", name = "Navicat Database" },
    { exec = "TeamViewer.exe", name = "TeamViewer" },
    { exec = "SunloginClient.exe", name = "Sunflower Remote" },
    { exec = "AnyDesk.exe", name = "AnyDesk Remote" },
    { exec = "ToDesk.exe", name = "ToDesk Remote" }
]
```

</details>

### Pure Configuration Mode Example

```toml
# Pure Configuration Mode: Do not scan directories, only create the specified shortcut
[[shortcut]]
name = "WeChat"
exec = "D:\Software\WeChat\WeChat.exe"
dest = "%Desktop%"

[[shortcut]]
name = "Chrome Browser"
exec = "C:\Program Files\Google\Chrome\chrome.exe"
args = "--start-maximized"
dest = "%Programs%\Browser"
```

### FAQ

### Q: Why aren't shortcuts created for some programs?

**A:** Possible causes include:

- The program directory structure is not reasonable
- The program directory has been added to the `ignore list` list
- The green software score is below the configured threshold (default 0.3)
- The `--only-match` mode is used but the program is not in the configuration list
- The program may be a background service rather than a GUI application

### Q: How do I customize the shortcut name?

**A:** There are two methods:

1. Specify a single program in `[[shortcut]]`:

```toml
[[shortcut]]
name = "WeChat"
```

2. Use a template to set the name uniformly:

```toml
[template]
name = "{stem}"
```

### Q: Unwanted shortcuts are created

**A:** Add the file name or directory name in `ignore = ["dirname"]`.

### Q: What is the difference between pure configuration mode and normal mode?

**A:**

- Normal mode: Automatically scans specified directories and intelligently identifies programs.
- Configuration-only mode (--config): Creates shortcuts based solely on the configuration file without scanning
  directories.

- Hybrid mode: Scans directories while overriding or enhancing certain settings using the configuration file.

### Q: How do I use environment variables in command lines and configuration files?

**A:** Supports standard Windows environment variables and built-in program variables, using the `%variablename%`
format, for example: `%Desktop%`, `%ProgramFiles%`, `%CurDir%`, etc.

## Open Source License

`AutoShortcut` is open source under the MPL v2.0 license. Please adhere to the license.

## Acknowledgements

- slore
- qq826773297
- liangnijian

## Contributing

1. Fork this repository
2. Create a new branch called `Feat_xxx`
3. Submit code
4. Create a pull request
