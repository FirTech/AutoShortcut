# AutoShortcut

## Introduction

`AutoShortcut` is an automated shortcut creation tool. Designed for efficient management of software shortcuts. It can
intelligently identify various software directory structures, automatically analyze and create optimal shortcuts,
greatly improving desktop and start menu cleanliness and efficiency.

### 🎉 **Advantages of AutoShortcut**

- 🔍 **Intelligent Recognition**: Automatically identifies portable software and single-file applications to create
  corresponding shortcuts
- ⚡ **Multi-dimensional Scoring**: Identifies main programs through multi-dimensional scoring algorithm
- 🛠 **Rich Customization**: Supports advanced features such as template naming, ignore rules, and batch creation
- 📂 **Dual Mode Support**: Supports both command-line and configuration file modes to meet different usage scenarios
- 🌍 **Multi-language Interface**: Built-in Chinese and English support, easy for internationalization
- 🎯 **Flexible Configuration**: Provides detailed configuration options to meet personalized needs

### 🔍 **Logic of AutoShortcut**

Due to software diversity, folders are divided into the following types:

- `Category Directory`: Contains multiple software directories, each software directory may contain
  `Single-file Program Directory` or `Portable Software Directory`
- `Single-file Program Directory`: Contains only multiple exe programs, no other files (including configuration files)
- `Portable Software`: Has program dependency files, may contain multiple exe programs

A good software directory structure requires no additional configuration files, just scan the directory according to the
above logic.
If the software directory structure does not conform to the above logic, you need to adjust the directory structure or
configuration file according to the actual situation.

`AutoShortcut` scans directories according to the following logic:

1. Scan each subdirectory in the specified program directory
2. Identify subdirectory types: `Category Directory`, `Single-file Program Directory`, `Portable Software`

    - If a `Category Directory` is identified, continue scanning the next subdirectory
    - If a `Single-file Program Directory` is identified, create shortcuts for each exe program
    - If `Portable Software` is identified, perform multi-dimensional comprehensive scoring for each exe program:

        - Whether the program name matches the parent directory name
        - Whether the program has description information
        - Whether the program is a GUI program
        - Whether the program has an icon
        - Whether the program has a digital signature
        - Whether the program matches the current system architecture
        - Whether the program name has auxiliary keywords
        - (Only when a configuration file is specified) Score based on the program file name specified in the
          configuration file

      The program with the highest score is determined as the main program, and trace back to the root directory of the
      portable software. Subsequent access to this software subdirectory will be skipped. If the score does not meet the
      threshold (default threshold is 30% of the highest score), it is determined that there is no main program, and
      this directory is skipped.

> Tips:
>
> - By default, shortcuts will use the processed product name (remove URLs, special characters, etc.) as the shortcut
    name, trying to restore the program name as much as possible
> - If it's not a reasonable product name (such as `7z setup sfx` etc.), it will fall back to the original file name as
    the shortcut name.

## 🚀 Quick Start

> This program is a command-line program, so you need to run it with parameters after it. You can run it through
> terminals such as `cmd`, `PowerShell`, etc.

```bash
# Scan directory and create shortcuts on desktop
AutoShortcut.exe "C:\Program Files" "%Desktop%"

# Scan directory and create shortcuts in start menu with a directory
AutoShortcut.exe -d "C:\Program Files" "%Programs%"

# Scan directory and automatically run after identifying the main program
AutoShortcut.exe -s "C:\Program Files\Everything"
```

> Tips:
>
> - Windows recommends running as "Administrator" to prevent UAC interception;
> - Use quotes for paths with spaces;
> - Environment variables (such as %USERPROFILE%, %Programs%) can be used directly, and built-in variables of the
    program are also supported.

## Command Line Usage

> Friendly reminder: Use quotes for paths with spaces

Usage: `AutoShortcut.exe [options] ["program_path"] ["shortcut_path"] [--config "config_file_path"]`

### Basic Usage

`AutoShortcut.exe "program_path" "shortcut_path"`

- Create desktop shortcut: `AutoShortcut.exe "C:\Program Files" %Desktop%`
- Create start menu shortcut: `AutoShortcut.exe "C:\Program Files" "%Programs%"`

### Create Program Folder

`AutoShortcut.exe -d "program_path" "shortcut_path"`

- `AutoShortcut.exe -d "C:\Program Files" "%Programs%"`

### Try to Install Software

Automatically run in the software directory: `install.cmd/bat`, `setup.cmd/bat`, `绿化.cmd/bat`

`AutoShortcut.exe -i "program_path" "shortcut_path"`

- `AutoShortcut.exe -i "C:\Program Files" "%Programs%"`

### Run Program

Automatically run the program after identifying the main program

`AutoShortcut.exe --start "program_path"`

### Use Original Filename

Set shortcut name to use the original exe filename

`AutoShortcut.exe --use-filename "program_path" "shortcut_path"`

### List Shortcuts Only

Only list program paths that need shortcuts, without performing shortcut creation operations.

`AutoShortcut.exe --list "program_path" "shortcut_path"`

### Match Configuration File Only

Only create shortcuts for shortcut information in the configuration file

`AutoShortcut.exe --only-match "program_path" "shortcut_path"`

### Score Threshold

Set the judgment threshold (percentage) when scoring exe programs for portable software. Programs below this score will
be determined as having no main program. Default value is `0.3`.

`AutoShortcut.exe --score-ratio 0.5`

## Configuration File (Optional)

`AutoShortcut.exe [options] ["program_path"] ["shortcut_path"] [--config "config_file_path"]`

### Introduction

To more effectively create shortcuts and meet personalized user needs, **optional** configuration files are introduced.
Configuration files include the following features:

- Global configuration tool options;
- Configure creation method information (name, command line, icon, etc.);
- Create shortcuts for programs not matched;

Usage:

1. Create a new text file and rename the extension to `.toml`
2. Open it with a text tool (such as Notepad, etc.) and fill in the corresponding configuration information

### Global Settings

- Ignore List

  Configure paths to ignore during scanning, subsequent scanning of directories/files will ignore them. Default is
  empty.

  ```toml
  ignore = ["temp"]
  ```

- Installation Scripts

  If set to `true`, automatically run in the software directory: `install.cmd/bat`, `setup.cmd/bat`, `绿化.cmd/bat`

  ```toml
  install = true
  ```

- Parallel Script Execution

  If set to `true`, do not wait for script execution to complete, but run all scripts in parallel. Default is `false`.

  ```toml
  install_parallel = true
  ```

- Script Rules

  Configure script file types for software installation (default supports `setup.cmd/bat`, `install.cmd/bat`,
  `绿化.cmd/bat`)

  ```toml
   # Example: support all cmd, bat scripts
   scripts = ["*.cmd", "*.bat"]
  ```

- Use Original Filename

  If set to `true`, all shortcut names will use the original exe filename.

  ```toml
  use_filename = true
  ```

- Match Configuration File Only

  If set to `true`, only create shortcuts for shortcut information in the configuration file. Shortcuts not specified in
  the configuration file will not be created.

  ```toml
  only_match = true
  ```

- Score Threshold

  Set the judgment threshold (percentage) when scoring exe programs for portable software. Programs below this score
  will be determined as having no main program. Default is `0.3`.

  ```toml
  score_ratio = 0.5
  ```

- Enable Escaping

  To facilitate Windows path representation, escaping is disabled by default. If escaping is needed, it can be enabled
  through configuration. Default is `false`.

  ```toml
  enable_escape = true
  ```

### Shortcut Definition

This program supports multiple shortcut attribute writing methods with the same functionality. You can choose freely
according to personal preference.

| Configuration Item | Description                                                                                                                                         |
|:------------------:|-----------------------------------------------------------------------------------------------------------------------------------------------------|
|       `exec`       | Program path (required), other configuration items are optional                                                                                     |
|       `name`       | Shortcut name                                                                                                                                       |
|       `icon`       | Icon path, supports relative paths (in order: program path, working path, system path), also supports specifying icon index such as `shell32.dll#1` |
|       `args`       | Command line                                                                                                                                        |
|     `work_dir`     | Working path (starting position)                                                                                                                    |
|       `dest`       | Shortcut storage path                                                                                                                               |
|   `window_state`   | Display mode: `normal` (activate and show) / `minimized` (minimize) / `maximized` (maximize)                                                        |
|     `comment`      | Comment                                                                                                                                             |

- Configuration Item Mode

  ```toml
  # This configuration is for field description only
  [[shortcut]]
  name = "Shortcut Name"
  exec = "Program Path"
  work_dir="Starting Position"
  args = "Command Line Parameters"
  icon = "Icon Path"
  dest = "Shortcut Location"
  window_state = "Display Mode"
  comment = "Comment"

  # Example
  [[shortcut]]
  name = "Everything"
  exec = "Everything.exe"
  ```

- Inline Mode

  > Tip: In inline mode, each shortcut information must be on one line and cannot be wrapped.

  ```toml
  shortcut = [
    # This configuration is for field description only
    { name = "Shortcut Name", exec = "Program Path", work_dir="Starting Position", args = "Command Line Parameters", icon = "Icon Path", dest = "Shortcut Location" },
    # Example
    { name = "Everything", exec = "Everything.exe" },
  ]
  ```

- Mapping Table Mode

  ```toml
  # Configure shortcut name
  [name]
  "Program Path" = "Shortcut Name"
  # Example
  "Everything.exe" = "Everything"

  # Configure shortcut parameters
  [args]
  "Program Path" = "Command Line Parameters"
  # Example
  "Everything.exe" = "-s example.txt"

  # Configure shortcut starting path
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
  ```

### Supported Built-in Variables

**Environment variables** and the following built-in variables are supported in the configuration file:

|       Variable        | Description                      |
|:---------------------:|----------------------------------|
|      `%CurDir%`       | Configuration file directory     |
|      `%CurFile%`      | Configuration file name          |
|      `%CurDrv%`       | Configuration file drive         |
|     `%Personal%`      | My Documents directory           |
|     `%Programs%`      | Programs menu directory          |
|      `%SendTo%`       | Send to directory                |
|     `%StartMenu%`     | Start menu directory             |
|      `%Startup%`      | Startup menu directory           |
|    `%QuickLaunch%`    | Quick launch bar                 |
|      `%Desktop%`      | Desktop directory                |
|     `%Downloads%`     | Downloads directory              |
|      `%Videos%`       | Videos directory                 |
|     `%Pictures%`      | Pictures directory               |
|       `%Music%`       | Music directory                  |
|     `%Favorites%`     | Favorites directory              |
|   `%PublicDesktop%`   | Public desktop directory         |
|  `%PublicDownloads%`  | Public downloads directory       |
|   `%PublicVideos%`    | Public videos directory          |
|  `%PublicPictures%`   | Public pictures directory        |
|    `%PublicMusic%`    | Public music directory           |
|  `%PublicPersonal%`   | Public documents directory       |
|   `%ProgramFiles%`    | Program files directory          |
| `%ProgramFiles(x86)%` | Program files directory (32-bit) |

### Pure Configuration Mode

To better meet personalized needs, in addition to scanning mode, a pure configuration mode is provided. The pure
configuration mode will not scan directories, only create shortcuts based on the information provided in the
configuration file.

`AutoShortcut.exe --config config_file_path`

> Tips:
>
> - In pure configuration mode, **`exec` (program path) and `dest` (shortcut path) must both be filled in**;
> - No need to pass `<TARGET_PATH>` (program path) and `<LNK_PATH>` (shortcut path) in the command line;
> - When `name` (shortcut name) is omitted, the program description will be used as the shortcut name;
> - Similarly, three writing methods are supported: configuration item mode, inline mode, and mapping table mode

```toml
shortcut = [
    # This configuration is for field description only
    { name = "Shortcut Name", exec = "Program Path", args = "Command Line Parameters", icon = "Icon Path", dest = "Shortcut Path" },
    # Example: create desktop shortcut
    { name = "Everything", exec = "C:\Program Files\Everything\Everything.exe", dest = "%Desktop%" },
    # Example: use program name as shortcut name
    { exec = "C:\Program Files\Everything\Everything.exe", dest = "%Desktop%" },
]
```

## Shortcut Attribute Template

> Tips:
>
> - Shortcut attribute templates only take effect in the `[Template]` configuration item.

`AutoShortcut` provides a powerful template system that uses rule templates to globally define shortcut attributes such
as name and icon. Template processing is divided into two stages:

1. **Control Flow Stage**: Parse and execute expressions first.
2. **Replacement Stage**: Replace remaining placeholder variables with corresponding values.

### 1. Variable Replacement

> Tips:
>
> - All variables in the template need to be wrapped in curly braces `{}`.
> - Variable names are case-sensitive.

Multiple built-in variables are available for direct reference in templates:

#### 📄 File Information Variables

|    Variable     | Description                                  |
|:---------------:|----------------------------------------------|
|    `{exec}`     | Full path, e.g., `D:\Apps\WeChat\WeChat.exe` |
|    `{stem}`     | Filename (without extension)                 |
|     `{ext}`     | File extension, e.g., `exe`                  |
|   `{parent}`    | Parent directory path of the program         |
| `{parent_name}` | Parent directory name of the program         |
|    `{size}`     | File size (bytes)                            |
|   `{size_kb}`   | File size (kilobytes)                        |
|   `{size_mb}`   | File size (megabytes)                        |
|   `{size_gb}`   | File size (gigabytes)                        |
|   `{size_tb}`   | File size (terabytes)                        |
| `{create_time}` | Creation time                                |
| `{modify_time}` | Modification time                            |
| `{access_time}` | Access time                                  |

#### 📝 Metadata Variables (from file version information)

|     Variable      | Description                                      |
|:-----------------:|--------------------------------------------------|
|    `{product}`    | Product name (ProductName)                       |
|  `{product_raw}`  | Original product name                            |
|     `{desc}`      | Program description (illegal characters cleaned) |
|   `{desc_raw}`    | Original program description                     |
|     `{arch}`      | Architecture (x86/x64/arm64)                     |
|   `{arch_num}`    | Architecture number (32/64/arm64)                |
|    `{version}`    | File version                                     |
|    `{company}`    | Company name (CompanyName)                       |
| `{orig_filename}` | Original filename                                |
|   `{copyright}`   | Copyright statement (LegalCopyright)             |

### 2. Conditional Syntax

- **Conditional (ternary) syntax**: `{cond ? then : else}`,
  `{condition variable ? content when condition is met : content when condition is not met}`

    - `cond` is a condition identifier (without curly braces). When the variable corresponding to `cond` is not empty,
      `then` is rendered; otherwise, `else` is rendered. Negation is supported: `!cond`.
    - `then` and `else` are template fragments, allowing placeholders (e.g., `desc`, `stem`).
    - `else` can be omitted (nothing is output when omitted).
    - `cond` / `then` / `else` can contain other templates (nesting supported).

  Examples:

    - `{arch_num ? arch_num + 'bit' : 'unknown bits'}`: If `arch_num` exists, use `arch_num` plus "bit", otherwise use "
      unknown bits".
    - `{product ? product : desc}`: If `product` exists, use `product`, otherwise use `desc`.

- **Default value syntax**: `{var ?? default}`, `{variable ?? default value}`

    - When variable `var` is not empty, use its value; when empty, render `default` (`default` can be a nested
      template).
    - Equivalent to `{var ? var : default}`.

  Examples:

    - `{product ?? desc}`: If `product` exists, use `product`, otherwise use `desc`.
    - `{desc ?? stem}`: If `desc` exists, use `desc`, otherwise use `stem`.

### 3. Filter Syntax

Filters are used to process variables, such as converting to lowercase, uppercase, title case, etc. Syntax:
`{var | filter: arg}`

Supported filters:

- Convert to lowercase: `{var | lower}`  
  Convert string to lowercase.  
  For example, `{desc | lower}` means "convert `desc` to lowercase".
- Convert to uppercase: `{var | upper}`  
  Convert string to uppercase.  
  For example, `{desc | upper}` means "convert `desc` to uppercase".
- Capitalize first letter: `{var | capitalize}`  
  Convert the first character of the string to uppercase, other characters to lowercase.  
  For example, `{desc | capitalize}` means "convert the first character of `desc` to uppercase, other characters to
  lowercase".
- Title case: `{var | title}`  
  Convert string to title case, i.e., **capitalize the first letter of each word, other letters lowercase**. This tag
  will not convert "unimportant words" to lowercase.  
  For example, `{desc | title}` means "convert `desc` to title case".
- Trim spaces: `{var | trim}`  
  Remove spaces at the beginning and end of the string.  
  For example, `{desc | trim}` means "remove spaces at the beginning and end of `desc`".
- Calculate string length: `{var | length}`  
  Calculate the length of the string (number of characters).  
  For example, `{desc | length}` means "calculate the length of `desc`".
- Delete text: `{var | cut:"str"}`  
  Delete specified text in the string.  
  For example, `{desc | cut:"WeChat"}` means "delete "WeChat" in `desc`".
- Replace text: `{var | replace:"old","new"}`  
  Replace specified text in the string.  
  For example, `{desc | replace:"WeChat","WeChat"}` means "replace "WeChat" in `desc` with "WeChat"".
- Truncate string: `{var | truncate:10}`  
  Truncate string to specified length, omitting excess parts.  
  For example, `{desc | truncate:10}` means "if `desc` length exceeds 10 characters, take the first 10 characters".  
  If `desc` length does not exceed 10 characters, it remains unchanged.
- Date formatting: `{var | date:"format"}`  
  Format date variable. `format` is a date format string, for example `"{create_time | date:'%Y-%m-%d %H:%M:%S'}"`.
  Supported date format placeholders:
    - `%Y`: Four-digit year (e.g., 2023)
    - `%m`: Month (01-12)
    - `%d`: Day (01-31)
    - `%H`: 24-hour format hour (00-23)
    - `%M`: Minute (00-59)
    - `%S`: Second (00-59)

#### Common Examples

- Use product name (use product name if available, otherwise use filename):

  ```toml
  [template]
  name = "{product ?? stem}"
  ```

- Display version and architecture (only when corresponding variables exist):

  ```toml
  [template]
  name = "{desc ?? stem} {version? 'v' + version : ''} {arch_num? arch_num + 'bit' : ''}"
  ```

  Note: `{version? 'v' + version : ''}` means output `'v' + version` when `version` exists (note that the preceding
  space is controlled by the template)

- Unified icon file path, comment

  ```toml
  [template]
  icon = "D:\Icons\{stem}.ico"
  comment = "{exec}"
  ```

> Regarding the choice between `{cond ? then : else}` and `{var ?? default}`
>
> - `{var ?? default}` is usually shorter and more intuitive, suitable for "if variable is empty, fall back" scenarios (
    e.g., when `product` is missing, use `stem`).
> - `{cond ? then : else}` is more flexible, can write more complex logic and include `!` negation. If only doing "
    missing fallback", prioritize using `{var ?? default}`.

## Example Configurations

- Configure shortcut names

  ```toml
  shortcut = [
    { name = "File Search", exec = "Everything.exe"},
    { name = "File Copy", exec = "FastCopy.exe"},
  ]
  ```

- Create shortcuts only for programs in the configuration file and use templates

  ```toml
  only_match = true

  shortcut = [
    { exec = "Everything.exe"},
    { exec = "FastCopy.exe"},
  ]

  [template]
  name = "{desc ?? stem} {version? 'v' + version : ''} {arch_num? arch_num + 'bit' : ''}"
  ```

- The following configuration file configures shortcut names for common software and sets installation scripts to any
  batch and PECMD scripts

    <details> <summary>Click to view</summary>

  config.toml

  ```toml
  ignore = ["temp"]
  scripts = ["*.cmd", "*.bat", "*.wcs"]
  shortcut = [
      { exec = "WeChat.exe", name = "WeChat" },
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
      { exec = "DrvCeo.exe", name = "Driver CEO" },
      { exec = "UltraISO.exe", name = "UltraISO" },
      { exec = "chrome.exe", name = "Google Chrome" },
      { exec = "msedge.exe", name = "Edge" },
      { exec = "360se.exe", name = "360 Secure Browser" },
      { exec = "QQBrowser.exe", name = "QQ Browser" },
      { exec = "SogouExplorer.exe", name = "Sogou Browser" },
      { exec = "UCBrowser.exe", name = "UC Browser" },
      { exec = "360Safe.exe", name = "360 Security Guard" },
      { exec = "Huorong.exe", name = "Huorong Security" },
      { exec = "QQPCMgr.exe", name = "Tencent PC Manager" },
      { exec = "BaiduSd.exe", name = "Baidu Antivirus" },
      { exec = "Thunder.exe", name = "Thunder" },
      { exec = "BaiduNetdisk.exe", name = "Baidu Netdisk" },
      { exec = "MicroCloud.exe", name = "Tencent Weiyun" },
      { exec = "alist.exe", name = "Aliyun Drive" },
      { exec = "eCloud.exe", name = "eCloud Drive" },
      { exec = "QuarkCloudDrive.exe", name = "Quark Drive" },
      { exec = "123pan.exe", name = "123 Cloud Drive" },
      { exec = "mCloud.exe", name = "Mobile Cloud Drive" },
      { exec = "Kanbox.exe", name = "Kanbox" },
      { exec = "Dropbox.exe", name = "Dropbox" },
      { exec = "XunleiDownload.exe", name = "Xunlei Download" },
      { exec = "IDMan.exe", name = "IDM Downloader" },
      { exec = "BitComet.exe", name = "BitComet" },
      { exec = "cloudmusic.exe", name = "NetEase Cloud Music" },
      { exec = "QQMusic.exe", name = "QQ Music" },
      { exec = "Kugou.exe", name = "Kugou Music" },
      { exec = "douyin_launcher.exe", name = "Douyin" },
      { exec = "QQLive.exe", name = "Tencent Video" },
      { exec = "QyClient.exe", name = "iQIYI" },
      { exec = "Youku.exe", name = "Youku" },
      { exec = "PotPlayerMini32.exe", name = "PotPlayer" },
      { exec = "PotPlayerMini64.exe", name = "PotPlayer" },
      { exec = "StormPlayer.exe", name = "Storm Player" },
      { exec = "WeGame.exe", name = "Tencent WeGame" },
      { exec = "steam.exe", name = "Steam Platform" },
      { exec = "mihoyo.exe", name = "miHoYo Launcher" },
      { exec = "Honeyview.exe", name = "Honeyview" },
      { exec = "2345Pic.exe", name = "2345 Picture Viewer" },
      { exec = "JianyingPro.exe", name = "Jianying" },
      { exec = "Code.exe", name = "Visual Studio Code" },
      { exec = "e.exe", name = "E Language" },
      { exec = "Eclipse.exe", name = "Eclipse Development Tool" },
      { exec = "VMware.exe", name = "VMware Virtual Machine" },
      { exec = "MuMuPlayer.exe", name = "MuMu Player" },
      { exec = "Navicat.exe", name = "Navicat Database" },
      { exec = "TeamViewer.exe", name = "TeamViewer" },
      { exec = "SunloginClient.exe", name = "Sunlogin Remote" },
      { exec = "AnyDesk.exe", name = "AnyDesk Remote" },
      { exec = "ToDesk.exe", name = "ToDesk Remote" }
  ]
  ```

    </details>

## Frequently Asked Questions

- How to customize shortcut names?
  Add the `name` field in [[shortcut]].
    - For example, `{exec = "WeChat.exe", name = "WeChat"}` means "create `WeChat.exe` program as `WeChat.lnk`
      shortcut".
    - For example, `{exec = "QQ.exe", name = "Tencent QQ"}` means "create `QQ.exe` program as `Tencent QQ.lnk`
      shortcut".
- Why wasn't a shortcut created for a certain program?

    1. May be ignored by ignore;
    2. Score below threshold in scanning mode;
    3. Not in the [[shortcut]] list in `--only-match` mode.
- How to prevent a directory from being scanned?
  Add the directory name in ignore = ["dirname"].

## Open Source License

`AutoShortcut` is open source under the GPL V3.0 license. Please try to comply with the open source agreement.

## Acknowledgments

- slore
- qq826773297
- liangnijian

## Contributing

1. Fork this repository
2. Create a new Feat_xxx branch
3. Submit your code
4. Create a new Pull Request