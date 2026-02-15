use aho_corasick::AhoCorasick;
use anyhow::{anyhow, bail, Result};
use goblin::pe::options::ParseOptions;
use goblin::pe::subsystem::IMAGE_SUBSYSTEM_WINDOWS_GUI;
use goblin::pe::PE;
use memmap2::Mmap;
use std::collections::HashMap;
use std::ffi::{c_void, OsStr, OsString};
use std::fs::File;
use std::io::ErrorKind;
use std::option::Option;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::{env, ptr, slice};
use windows::core::{Interface, BOOL, GUID, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
    IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, IsWow64Process};
use windows::Win32::UI::Shell::{
    ExtractIconExW, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Favorites,
    FOLDERID_Music, FOLDERID_Pictures, FOLDERID_ProgramFilesX86, FOLDERID_Programs,
    FOLDERID_PublicDesktop, FOLDERID_PublicDocuments, FOLDERID_PublicDownloads,
    FOLDERID_PublicMusic, FOLDERID_PublicPictures, FOLDERID_PublicVideos, FOLDERID_QuickLaunch,
    FOLDERID_SendTo, FOLDERID_StartMenu, FOLDERID_Startup, FOLDERID_System, FOLDERID_Videos,
    FOLDERID_Windows, IShellLinkW, SHGetKnownFolderPath, ShellLink, KNOWN_FOLDER_FLAG,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SW_SHOWMAXIMIZED, SW_SHOWMINNOACTIVE, SW_SHOWNORMAL,
};

/// 创建快捷方式
///
/// # 参数
/// - `target`: 目标路径
/// - `link`: 快捷方式路径
/// - `args`: 命令行参数
/// - `icon`: 图标路径+索引，格式为 `path#index`
/// - `working_dir`: 工作目录
/// - `window_state`: 窗口状态
/// - `description`: 描述
/// - `hotkey`: 快捷键 (u16 格式)
///
/// # 返回值
/// - `Ok(())`: 成功
/// - `Err(...)`：创建快捷方式失败
///
/// # 说明
/// - 图标路径可以是文件路径或系统图标路径（如 `shell32.dll,0`）
/// - 窗口状态可以是 `normal`、`maximized`、`minimized`
/// - 快捷键格式为 (modifiers << 8) | vk_code
///
/// [参考文档](https://learn.microsoft.com/en-us/windows/win32/shell/links)
pub fn create_shortcut(
    target: &Path,
    link: &Path,
    args: Option<String>,
    icon: Option<(String, i32)>,
    working_dir: Option<String>,
    window_state: Option<String>,
    description: Option<String>,
    hotkey: Option<u16>,
) -> Result<()> {
    unsafe {
        // 初始化 COM（STA 模式）
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|e| anyhow!("CoInitializeEx failed: {}", e))?;

        // 创建 ShellLink COM 对象
        let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| anyhow!("CoCreateInstance(IShellLink) failed: {}", e))?;

        // SetPath
        let wt = HSTRING::from(target);
        shell
            .SetPath(PCWSTR(wt.as_ptr()))
            .map_err(|e| anyhow!("IShellLink::SetPath failed: {}", e))?;

        // Set arguments
        if let Some(a) = args {
            if !a.is_empty() {
                let wa: Vec<u16> = a.encode_utf16().chain(std::iter::once(0)).collect();
                shell
                    .SetArguments(PCWSTR(wa.as_ptr()))
                    .map_err(|e| anyhow!("IShellLink::SetArguments failed: {}", e))?;
            }
        }

        // Set working directory
        if let Some(wd) = working_dir {
            if !wd.is_empty() {
                let wwd: Vec<u16> = wd.encode_utf16().chain(std::iter::once(0)).collect();
                shell
                    .SetWorkingDirectory(PCWSTR(wwd.as_ptr()))
                    .map_err(|e| anyhow!("IShellLink::SetWorkingDirectory failed: {}", e))?;
            }
        }

        // Set icon location
        if let Some((icon_str, icon_index)) = icon {
            // 解析传入字符串为 PathBuf（不做强制存在性检查）
            let icon_path = Path::new(&icon_str);
            let icon_wide = HSTRING::from(icon_path);
            shell
                .SetIconLocation(PCWSTR(icon_wide.as_ptr()), icon_index)
                .map_err(|e| anyhow!("IShellLink::SetIconLocation failed: {}", e))?;
        }

        // Set description
        if let Some(des) = description {
            if !des.is_empty() {
                let wdes: Vec<u16> = des.encode_utf16().chain(std::iter::once(0)).collect();
                shell
                    .SetDescription(PCWSTR(wdes.as_ptr()))
                    .map_err(|e| anyhow!("IShellLink::SetDescription failed: {}", e))?;
            }
        }

        if let Some(state) = window_state {
            shell.SetShowCmd(match state.as_str() {
                "normal" => SW_SHOWNORMAL,
                "minimized" => SW_SHOWMINNOACTIVE,
                "maximized" => SW_SHOWMAXIMIZED,
                _ => SW_SHOWNORMAL,
            })?;
        }

        // 设置快捷键
        if let Some(hk) = hotkey {
            shell
                .SetHotkey(hk)
                .map_err(|e| anyhow!("IShellLink::SetHotkey failed: {}", e))?;
        }

        // Query IPersistFile
        let persist: IPersistFile = shell
            .cast()
            .map_err(|e| anyhow!("Query IPersistFile failed: {}", e))?;

        // 保存到 .lnk（link 路径必须是绝对或可写的）
        let wlink = HSTRING::from(link);
        // 第二个参数 bForce? TRUE 表示强制写入
        persist
            .Save(PCWSTR(wlink.as_ptr()), BOOL(1).into())
            .map_err(|e| anyhow!("IPersistFile::Save failed: {}", e))?;

        // 释放 COM
        CoUninitialize();

        Ok(())
    }
}

/// 获取 .lnk 快捷方式的目标路径
///
/// # 参数
/// - `path`: 快捷方式路径
///
/// # 返回值
/// - `Ok(PathBuf)`: 快捷方式目标路径
/// - `Err(...)`：获取快捷方式目标路径失败
pub fn get_shortcut_target(path: &Path) -> Result<PathBuf> {
    // 初始化 COM
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    // 创建 ShellLink 对象
    let shell_link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)? };

    // 加载 .lnk 文件
    let wide_path = HSTRING::from(path);
    unsafe {
        shell_link.cast::<IPersistFile>()?.Load(
            PCWSTR(wide_path.as_ptr()),
            windows::Win32::System::Com::STGM(0),
        )?;
    }

    // 获取目标路径
    let mut buffer: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
    unsafe {
        shell_link.GetPath(&mut buffer, ptr::null_mut(), 0)?;
    }

    // 清理 COM
    unsafe {
        CoUninitialize();
    }

    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    if len == 0 {
        // 没有解析出目标路径，把它视为“未找到目标”的错误返回
        return Err(std::io::Error::new(
            ErrorKind::NotFound,
            "shortcut target not found or target is empty",
        )
        .into());
    }

    // 转换并返回 PathBuf
    let os_str = OsString::from_wide(&buffer[..len]);
    Ok(PathBuf::from(os_str))
}

/// 解析快捷键字符串为 u16 值
///
/// # 参数
/// - `hotkey_str`: 快捷键字符串，例如 "Alt + L", "Ctrl+Shift+A", "Ctrl + Alt + Delete"
///
/// # 返回值
/// - `Ok(u16)`: 解析后的热键值 (格式: (modifiers << 8) | vk_code)
/// - `Err(...)`: 格式错误或不支持的键
///
/// # 支持的修饰符
/// - Ctrl/Control: 0x02
/// - Shift: 0x01
/// - Alt/Option: 0x04
/// - Ext: 0x08 (Windows 键)
///
/// # 支持的虚拟键码 (低字节)
/// - 字母 A-Z: 0x41-0x5A
/// - 数字 0-9: 0x30-0x39
/// - F1-F24: 0x70-0x87
/// - 方向键: Left(0x25), Up(0x26), Right(0x27), Down(0x28)
/// - 常用键: Space(0x20), Enter(0x0D), Esc(0x1B), Tab(0x09), Delete(0x2E), Insert(0x2D), Home(0x24), End(0x23), PageUp(0x21), PageDown(0x22)
/// - 其他: Backspace(0x08), CapsLock(0x14), ScrollLock(0x91), NumLock(0x90), PrintScreen(0x2A)
///
/// # 示例
/// - "Alt + L" -> 0x044C
/// - "Ctrl+Shift+A" -> 0x0341
/// - "Ctrl + Alt + Delete" -> 0x062E
pub fn parse_hotkey(hotkey_str: &str) -> Result<u16> {
    // Windows hotkey modifiers (高字节)
    const HOTKEYF_SHIFT: u16 = 0x01;
    const HOTKEYF_CONTROL: u16 = 0x02;
    const HOTKEYF_ALT: u16 = 0x04;
    const HOTKEYF_EXT: u16 = 0x08; // Windows 键

    // 虚拟键码映射表
    fn vk_code_from_str(key: &str) -> Option<u16> {
        let key = key.trim().to_ascii_uppercase();

        // 字母 A-Z
        if key.len() == 1 && key.as_bytes()[0].is_ascii_alphabetic() {
            return Some(key.as_bytes()[0] as u16);
        }

        // 数字 0-9
        if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
            return Some(key.as_bytes()[0] as u16);
        }

        match key.as_str() {
            // 功能键 F1-F24
            "F1" => Some(0x70),
            "F2" => Some(0x71),
            "F3" => Some(0x72),
            "F4" => Some(0x73),
            "F5" => Some(0x74),
            "F6" => Some(0x75),
            "F7" => Some(0x76),
            "F8" => Some(0x77),
            "F9" => Some(0x78),
            "F10" => Some(0x79),
            "F11" => Some(0x7A),
            "F12" => Some(0x7B),
            "F13" => Some(0x7C),
            "F14" => Some(0x7D),
            "F15" => Some(0x7E),
            "F16" => Some(0x7F),
            "F17" => Some(0x80),
            "F18" => Some(0x81),
            "F19" => Some(0x82),
            "F20" => Some(0x83),
            "F21" => Some(0x84),
            "F22" => Some(0x85),
            "F23" => Some(0x86),
            "F24" => Some(0x87),

            // 方向键
            "LEFT" | "ARROWLEFT" => Some(0x25),
            "UP" | "ARROWUP" => Some(0x26),
            "RIGHT" | "ARROWRIGHT" => Some(0x27),
            "DOWN" | "ARROWDOWN" => Some(0x28),

            // 特殊键
            "SPACE" => Some(0x20),
            "ENTER" | "RETURN" => Some(0x0D),
            "ESC" | "ESCAPE" => Some(0x1B),
            "TAB" => Some(0x09),
            "DELETE" | "DEL" => Some(0x2E),
            "INSERT" | "INS" => Some(0x2D),
            "HOME" => Some(0x24),
            "END" => Some(0x23),
            "PAGEUP" | "PGUP" => Some(0x21),
            "PAGEDOWN" | "PGDN" => Some(0x22),
            "BACKSPACE" | "BS" | "BKSP" => Some(0x08),

            // 小键盘
            "NUMPAD0" => Some(0x60),
            "NUMPAD1" => Some(0x61),
            "NUMPAD2" => Some(0x62),
            "NUMPAD3" => Some(0x63),
            "NUMPAD4" => Some(0x64),
            "NUMPAD5" => Some(0x65),
            "NUMPAD6" => Some(0x66),
            "NUMPAD7" => Some(0x67),
            "NUMPAD8" => Some(0x68),
            "NUMPAD9" => Some(0x69),
            "MULTIPLY" => Some(0x6A),
            "ADD" => Some(0x6B),
            "SUBTRACT" => Some(0x6D),
            "DECIMAL" => Some(0x6E),
            "DIVIDE" => Some(0x6F),

            // 其他
            "CAPSLOCK" => Some(0x14),
            "SCROLLLOCK" => Some(0x91),
            "NUMLOCK" => Some(0x90),
            "PRINTSCREEN" | "PRTSC" => Some(0x2A),
            "PAUSE" => Some(0x13),

            _ => None,
        }
    }

    // 分割输入字符串 (支持 +, 空格, 或组合)
    let parts: Vec<&str> = hotkey_str
        .split(&['+', ' ', '\t'][..])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        bail!("empty hotkey string");
    }

    let mut modifiers: u16 = 0;
    let mut main_key: Option<u16> = None;

    for part in &parts {
        let part_upper = part.to_ascii_uppercase();

        match part_upper.as_str() {
            "CTRL" | "CONTROL" => modifiers |= HOTKEYF_CONTROL,
            "SHIFT" => modifiers |= HOTKEYF_SHIFT,
            "ALT" | "OPTION" => modifiers |= HOTKEYF_ALT,
            "WIN" | "WINDOWS" | "META" | "CMD" | "COMMAND" => modifiers |= HOTKEYF_EXT,
            _ => {
                if main_key.is_some() {
                    bail!("multiple main keys detected: {}", hotkey_str);
                }
                main_key = vk_code_from_str(part);
                if main_key.is_none() {
                    bail!("unsupported key: {}", part);
                }
            }
        }
    }

    let main_key =
        main_key.ok_or_else(|| anyhow!("no main key found in hotkey: {}", hotkey_str))?;

    Ok((modifiers << 8) | main_key)
}

/// 替换变量
pub fn process_env(content: String, config_path: Option<&Path>) -> String {
    let mut vars = HashMap::new();

    // 配置文件相关变量
    if let Some(path) = config_path {
        // 配置文件目录
        vars.insert(
            "CurDir".into(),
            path.parent().unwrap().to_string_lossy().to_string(),
        );
        // 配置文件名称
        vars.insert("CurFile".into(), path.to_string_lossy().to_string());
        // 配置文件驱动器
        vars.insert("CurDrv".into(), path.to_string_lossy()[..2].to_string());
    }

    // 程序目录(64位)
    if let Some(p) = get_known_folder(&FOLDERID_Windows) {
        vars.insert(
            "ProgramFiles".into(),
            p.parent()
                .unwrap()
                .join("Program Files")
                .to_string_lossy()
                .to_string(),
        );
    }
    // 程序目录(32位)
    if let Some(p) = get_known_folder(&FOLDERID_ProgramFilesX86) {
        vars.insert("ProgramFiles(x86)".into(), p.to_string_lossy().to_string());
    }

    // 桌面目录
    if let Some(p) = get_known_folder(&FOLDERID_Desktop) {
        vars.insert("Desktop".into(), p.to_string_lossy().to_string());
    }
    // 下载目录
    if let Some(p) = get_known_folder(&FOLDERID_Downloads) {
        vars.insert("Downloads".into(), p.to_string_lossy().to_string());
    }
    // 视频目录
    if let Some(p) = get_known_folder(&FOLDERID_Videos) {
        vars.insert("Videos".into(), p.to_string_lossy().to_string());
    }
    // 图片目录
    if let Some(p) = get_known_folder(&FOLDERID_Pictures) {
        vars.insert("Pictures".into(), p.to_string_lossy().to_string());
    }
    // 音乐目录
    if let Some(p) = get_known_folder(&FOLDERID_Music) {
        vars.insert("Music".into(), p.to_string_lossy().to_string());
    }
    // 收藏夹目录
    if let Some(p) = get_known_folder(&FOLDERID_Favorites) {
        vars.insert("Favorites".into(), p.to_string_lossy().to_string());
    }
    // 文档目录
    if let Some(p) = get_known_folder(&FOLDERID_Documents) {
        vars.insert("Personal".into(), p.to_string_lossy().to_string());
    }

    // 公共桌面目录
    if let Some(p) = get_known_folder(&FOLDERID_PublicDesktop) {
        vars.insert("PublicDesktop".into(), p.to_string_lossy().to_string());
    }
    // 公共下载目录
    if let Some(p) = get_known_folder(&FOLDERID_PublicDownloads) {
        vars.insert("PublicDownloads".into(), p.to_string_lossy().to_string());
    }
    // 公共视频目录
    if let Some(p) = get_known_folder(&FOLDERID_PublicVideos) {
        vars.insert("PublicVideos".into(), p.to_string_lossy().to_string());
    }
    // 公共图片目录
    if let Some(p) = get_known_folder(&FOLDERID_PublicPictures) {
        vars.insert("PublicPictures".into(), p.to_string_lossy().to_string());
    }
    // 公共音乐目录
    if let Some(p) = get_known_folder(&FOLDERID_PublicMusic) {
        vars.insert("PublicMusic".into(), p.to_string_lossy().to_string());
    }
    // 公共文档目录
    if let Some(p) = get_known_folder(&FOLDERID_PublicDocuments) {
        vars.insert("PublicPersonal".into(), p.to_string_lossy().to_string());
    }

    // 程序菜单目录
    if let Some(p) = get_known_folder(&FOLDERID_Programs) {
        vars.insert("Programs".into(), p.to_string_lossy().to_string());
    }
    // 发送到目录
    if let Some(p) = get_known_folder(&FOLDERID_SendTo) {
        vars.insert("SendTo".into(), p.to_string_lossy().to_string());
    }
    // 开始菜单目录
    if let Some(p) = get_known_folder(&FOLDERID_StartMenu) {
        vars.insert("StartMenu".into(), p.to_string_lossy().to_string());
    }
    // 启动菜单目录
    if let Some(p) = get_known_folder(&FOLDERID_Startup) {
        vars.insert("Startup".into(), p.to_string_lossy().to_string());
    }
    // 快速启动栏
    if let Some(p) = get_known_folder(&FOLDERID_QuickLaunch) {
        vars.insert("QuickLaunch".into(), p.to_string_lossy().to_string());
    }

    // 处理系统环境变量
    for (key, value) in env::vars() {
        // 以内置变量优先
        vars.entry(key.to_string()).or_insert(value);
    }

    // 替换全部变量
    let patterns: Vec<String> = vars
        .keys()
        .map(|key: &String| format!("%{}%", key))
        .collect();
    let replacements: Vec<&str> = vars.values().map(String::as_str).collect();

    let ac = AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(patterns)
        .unwrap();
    ac.replace_all(&content, &replacements)
}

/// 尝试把相对路径解析为实际存在的候选路径
pub fn resolve_relative_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(PathBuf::from(path));
    }

    // 当前工作目录
    if let Ok(current_dir) = env::current_dir() {
        let full_path = current_dir.join(path);
        if full_path.exists() {
            return Some(full_path);
        }
    }

    // 程序所在目录
    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let full_path = parent.join(path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }

    // 系统目录
    if let Some(sysroot) = get_known_folder(&FOLDERID_System) {
        let full_path = &sysroot
            .join(match is_running_under_wow64() {
                Ok(true) => "SysNative",
                _ => "System32",
            })
            .join(path);
        if full_path.exists() {
            return Some(sysroot.join("System32"));
        }
    }

    // Windows 目录
    if let Some(sysroot) = get_known_folder(&FOLDERID_System) {
        let full_path = &sysroot.join(path);
        if full_path.exists() {
            return Some(PathBuf::from(full_path));
        }
    }

    None
}

/// glob匹配函数
pub fn matches_glob(pattern: &str, filename: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let filename = filename.to_lowercase();

    // 处理通配符逻辑
    match (pattern.starts_with('*'), pattern.ends_with('*')) {
        // 后缀匹配 *.cmd
        (true, false) => filename.ends_with(&pattern[1..]),
        // 前缀匹配 setup*
        (false, true) => filename.starts_with(&pattern[..pattern.len() - 1]),
        // 全匹配 * 或包含匹配 *test*
        _ => filename.contains(pattern.trim_matches('*')),
    }
}

/// 不区分大小写替换字符串（仅适用于 ASCII 模式的模式与替换）
/// 简单实现：通过 lower-case 查找匹配位置并构建新字符串
///
/// # 参数
/// - `input`: 输入字符串
/// - `pattern`: 模式字符串
/// - `value`: 替换字符串
///
/// # 返回值
/// - `String`: 替换后的字符串
pub fn replace_ignore_case(input: &str, pattern: &str, value: &str) -> String {
    if pattern.is_empty() {
        return input.to_string();
    }
    let hs_lower = input.to_ascii_lowercase();
    let pat_lower = pattern.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while let Some(pos) = hs_lower[i..].find(&pat_lower) {
        let found = i + pos;
        out.push_str(&input[i..found]); // push unmatched prefix preserving original case
        out.push_str(value); // push replacement (preserve rep case)
        i = found + pattern.len(); // advance by pattern length (assumes ASCII len OK)
    }
    out.push_str(&input[i..]);
    out
}

/// 判断是否运行在64位系统中
///
/// # 返回值
/// - `Ok(true)`: 运行在64位系统中
/// - `Ok(false)`: 运行在32位系统中
/// - `Err(...)`：获取系统信息失败
///
/// # 说明
/// - 此函数通过查询当前进程是否为WOW64进程来判断是否运行在64位系统中。
/// - 仅当本进程为 32-bit 编译时才需要判断（64-bit 编译的进程上 IsWow64Process 返回 false）
pub fn is_running_under_wow64() -> Result<bool, windows::core::Error> {
    // 仅当本进程为 32-bit 编译时才需要判断（64-bit 编译的进程上 IsWow64Process 返回 false）
    unsafe {
        let mut wow64: BOOL = BOOL(0);
        IsWow64Process(GetCurrentProcess(), &mut wow64)?;
        Ok(wow64.as_bool())
    }
}

/// 获取系统特定目录
///
/// # 备注
/// - `SHGetKnownFolderPath`最低支持平台为`Windows Vista`，可通过`YY-Thunks`进行兼容NT5平台
pub fn get_known_folder(rfid: &GUID) -> Option<PathBuf> {
    unsafe {
        let ptr: PWSTR = match SHGetKnownFolderPath(rfid as *const GUID, KNOWN_FOLDER_FLAG(0), None)
        {
            Ok(p) if !p.is_null() => p,
            _ => return None,
        };

        let mut len = 0;
        while *ptr.0.offset(len) != 0 {
            len += 1;
        }
        let wide = slice::from_raw_parts(ptr.0, len as usize);
        let path = String::from_utf16_lossy(wide);

        CoTaskMemFree(Some(ptr.0 as *mut c_void));
        Some(PathBuf::from(path))
    }
}

/// 返回值当前进程的父进程 PID
fn get_parent_pid(pid: u32) -> windows::core::Result<u32> {
    unsafe {
        // 全进程快照
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        if h.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        // 枚举第一个
        Process32FirstW(h, &mut entry)?;
        loop {
            if entry.th32ProcessID == pid {
                let _ = CloseHandle(h);
                return Ok(entry.th32ParentProcessID);
            }
            if Process32NextW(h, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(h);
        Err(windows::core::Error::from_thread())
    }
}

/// 给定 PID，返回进程名（不含路径），如 "explorer.exe"
fn get_process_name(pid: u32) -> windows::core::Result<String> {
    unsafe {
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        if h.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        Process32FirstW(h, &mut entry)?;
        loop {
            if entry.th32ProcessID == pid {
                // 找到第一个 NUL 终止符
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(MAX_PATH as usize);
                let name = OsString::from_wide(&entry.szExeFile[..len])
                    .into_string()
                    .map_err(|_| windows::core::Error::from_thread())?;
                let _ = CloseHandle(h);
                return Ok(name);
            }
            if Process32NextW(h, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(h);
        Err(windows::core::Error::from_thread())
    }
}

/// 检查父进程名是否为 explorer.exe
pub fn launched_from_explorer() -> bool {
    let self_pid = unsafe { GetCurrentProcessId() };
    if let Ok(ppid) = get_parent_pid(self_pid) {
        if let Ok(name) = get_process_name(ppid) {
            return name.eq_ignore_ascii_case("explorer.exe");
        }
    }
    false
}

/// 标准化应用程序名称函数，用于比较
pub fn normalize_app_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .replace("_x86", "")
        .replace("_x32", "")
        .replace("_x64", "")
        .replace("-win64-shipping", "")
        .replace(" ", "")
        .replace("-", "")
        .replace("_", "")
        .replace(".exe", "") // 移除扩展名
}

/// 读取文件的 version resource 到 Vec<u8>
fn get_version_info_data(path: &Path) -> Result<Vec<u8>> {
    // Convert file path to wide string (UTF-16) null-terminated
    let wide_path = HSTRING::from(path);

    // Get size of version info
    let mut dummy: u32 = 0;
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(wide_path.as_ptr()), Some(&mut dummy)) };
    if size == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    // Allocate buffer for version info
    let mut data: Vec<u8> = vec![0; size as usize];
    unsafe {
        GetFileVersionInfoW(
            PCWSTR(wide_path.as_ptr()),
            None,
            size,
            data.as_mut_ptr() as *mut _,
        )?;
    }

    Ok(data)
}

/// 在 version resource data 中查询指定的字符串字段（例如 "FileDescription", "ProductName", "OriginalFilename"）
fn query_string_from_version(data: &[u8], field: &str) -> Result<Option<String>> {
    // 先查询 Translation 列表 (\VarFileInfo\Translation)，取第一个 lang/codepage
    let trans_key: Vec<u16> = OsStr::new("\\VarFileInfo\\Translation")
        .encode_wide()
        .chain(Some(0))
        .collect();

    let mut trans_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut trans_len: u32 = 0;

    let ok = unsafe {
        VerQueryValueW(
            data.as_ptr() as *const _,
            PCWSTR(trans_key.as_ptr()),
            &mut trans_ptr,
            &mut trans_len,
        )
    };

    if !ok.as_bool() || trans_ptr.is_null() || trans_len < 4 {
        // 没有 Translation 信息，返回 None
        return Ok(None);
    }

    // trans_ptr 指向 WORD[2] 或多个，取第一个（word = u16）
    let trans_words =
        unsafe { std::slice::from_raw_parts(trans_ptr as *const u16, (trans_len / 2) as usize) };
    if trans_words.len() < 2 {
        return Ok(None);
    }
    let lang_id = trans_words[0];
    let codepage = trans_words[1];

    // 构造查询字符串，例如: \StringFileInfo\040904E4\FileDescription
    let subblock = format!(
        "\\StringFileInfo\\{:04x}{:04x}\\{}",
        lang_id, codepage, field
    );
    let wide_subblock = HSTRING::from(subblock);

    let mut value_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut value_len: u32 = 0;

    let ok2 = unsafe {
        VerQueryValueW(
            data.as_ptr() as *const _,
            PCWSTR(wide_subblock.as_ptr()),
            &mut value_ptr,
            &mut value_len,
        )
    };

    if ok2.as_bool() && !value_ptr.is_null() && value_len > 0 {
        // value_len 是宽字符数（u16），把它转成 Rust String
        let slice =
            unsafe { std::slice::from_raw_parts(value_ptr as *const u16, value_len as usize) };
        // 寻找 null 终止符（有时 value_len 包含末尾的 0，有时不包含）
        let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
        let s = String::from_utf16(&slice[..end])?;
        return Ok(Some(s));
    }

    Ok(None)
}

/// 获取exe产品名称
pub fn get_exe_product_name(path: &Path) -> Result<Option<String>> {
    let data = get_version_info_data(path)?;
    query_string_from_version(&data, "ProductName")
}

/// 获取程序描述
pub fn get_exe_description(path: &Path) -> Result<Option<String>> {
    let data = get_version_info_data(path)?;
    query_string_from_version(&data, "FileDescription")
}

/// 获取exe公司名称
pub fn get_exe_company_name(path: &Path) -> Result<Option<String>> {
    let data = get_version_info_data(path)?;
    query_string_from_version(&data, "CompanyName")
}

/// 获取exe版权信息
pub fn get_exe_copyright(path: &Path) -> Result<Option<String>> {
    let data = get_version_info_data(path)?;
    query_string_from_version(&data, "LegalCopyright")
}

/// 获取exe原始文件名
pub fn get_exe_original_filename(path: &Path) -> Result<Option<String>> {
    let data = get_version_info_data(path)?;
    query_string_from_version(&data, "OriginalFilename")
}

/// 获取exe产品版本
///
/// # 参数
/// - `path`: 可执行文件的路径。
///
/// # 返回值
/// - `Ok(Some(version_string))`: 如果成功获取到版本号，返回格式为 "Major.Minor.Patch.Build" 的字符串。
/// - `Ok(None)`: 如果文件没有版本信息，或者无法获取。
/// - `Err(error)`: 如果在读取或解析过程中发生错误。
///
/// # 说明
/// - 此函数通过查询 PE 文件的版本资源，获取产品版本信息。
/// - 版本信息通常包含在文件的资源部分，格式为 "Major.Minor.Patch.Build"。
pub fn get_exe_product_version(path: &Path) -> Result<Option<String>> {
    let data = get_version_info_data(path)?;
    query_string_from_version(&data, "ProductVersion")
}

/// 获取EXE程序的文件版本。
///
/// 此函数通过 Windows API 查询 PE 文件的版本信息，提取其中的数字版本号。
///
/// # 参数:
/// - `path`: 可执行文件的路径。
///
/// # 返回值:
/// - `Ok(Some(version_string))`: 如果成功获取到版本号，返回格式为 "Major.Minor.Patch.Build" 的字符串。
/// - `Ok(None)`: 如果文件没有版本信息，或者无法获取。
/// - `Err(error)`: 如果在读取或解析过程中发生错误。
pub fn get_exe_file_version(path: &Path) -> Result<Option<String>> {
    // 将 Rust Path 转换为 Windows API 所需的宽字符串 (UTF-16)
    let path_wide = HSTRING::from(path);

    // 获取版本信息块的大小
    let mut handle = 0u32;
    let info_size =
        unsafe { GetFileVersionInfoSizeW(PCWSTR(path_wide.as_ptr()), Some(&mut handle)) };

    if info_size == 0 {
        // 文件没有版本信息，或者文件不存在/无法访问
        return Ok(None);
    }

    // 分配缓冲区来存储版本信息
    let mut info_buffer = vec![0u8; info_size as usize];

    // 获取版本信息到缓冲区
    if unsafe {
        GetFileVersionInfoW(
            PCWSTR(path_wide.as_ptr()),
            Some(handle),
            info_size,
            info_buffer.as_mut_ptr() as *mut std::ffi::c_void,
        )
    }
    .is_err()
    {
        // 无法获取版本信息
        return Ok(None);
    }

    // 查询固定文件信息结构体 (VS_FIXEDFILEINFO)，它位于版本信息块的根路径 "\\"
    let mut value_ptr = std::ptr::null_mut();
    let mut value_len = 0u32;

    // 查询字符串为 "\\"，表示根信息
    let query_wide: Vec<u16> = OsStr::new("\\")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let success_query = unsafe {
        VerQueryValueW(
            info_buffer.as_ptr() as *mut std::ffi::c_void,
            PCWSTR(query_wide.as_ptr()),
            &mut value_ptr,
            &mut value_len,
        )
    };

    if success_query == BOOL(0) || value_len == 0 {
        // 未能查询到固定文件信息
        return Ok(None);
    }

    // 将获取到的指针转换为 VS_FIXEDFILEINFO 结构体
    // 确保返回的长度足够容纳 VS_FIXEDFILEINFO 结构体
    if value_len < std::mem::size_of::<VS_FIXEDFILEINFO>() as u32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Returned data too small for VS_FIXEDFILEINFO",
        )
        .into());
    }

    let fixed_file_info = unsafe { *(value_ptr as *const VS_FIXEDFILEINFO) };

    // 从 VS_FIXEDFILEINFO 结构体中提取版本组件
    // dwFileVersionMS 包含主版本和次版本 (高 16 位是主版本，低 16 位是次版本)
    let major = (fixed_file_info.dwFileVersionMS >> 16) as u16;
    let minor = (fixed_file_info.dwFileVersionMS & 0xFFFF) as u16;
    // dwFileVersionLS 包含修订版本和构建版本 (高 16 位是修订版本，低 16 位是构建版本)
    let patch = (fixed_file_info.dwFileVersionLS >> 16) as u16;
    let build = (fixed_file_info.dwFileVersionLS & 0xFFFF) as u16;

    // 格式化版本字符串
    let version_string = format!("{}.{}.{}.{}", major, minor, patch, build);

    Ok(Some(version_string))
}

/// 获取当前系统的处理器架构。
///
/// 此函数通过调用 Windows API `GetNativeSystemInfo` 来检索有关当前系统体系结构的信息。
/// 它返回 `wProcessorArchitecture` 字段的值，该值标识处理器架构。
///
/// # 返回值
/// - `u16`: 代表系统处理器架构的数值。常见的值包括：
///   - `0` (PROCESSOR_ARCHITECTURE_INTEL): Intel 或兼容的 x86 架构。
///   - `9` (PROCESSOR_ARCHITECTURE_AMD64): x64 (AMD64) 架构。
///   - `12` (PROCESSOR_ARCHITECTURE_ARM64): ARM64 架构。
///   - 其他值表示其他或未知的架构类型。
pub fn get_native_arch() -> u16 {
    let mut sys_info = SYSTEM_INFO::default();
    unsafe {
        GetNativeSystemInfo(&mut sys_info);
        sys_info.Anonymous.Anonymous.wProcessorArchitecture.0
    }
}

/// 获取程序架构
///
/// # 参数
/// - `program`: 程序路径
///
/// # 返回值
/// - `Ok(u16)`: PE 文件 Machine 字段
///   - 0x014c → x86
///   - 0x8664 → x64
///   - 0xAA64 → ARM64
/// - `Err(...)`：读取或解析失败
pub fn get_program_arch(program: impl AsRef<Path>) -> Result<u16> {
    // open file and mmap
    let file = File::open(program)?;
    let mmap = unsafe { Mmap::map(&file)? };

    let mut options = ParseOptions::default();
    options.parse_attribute_certificates = false;
    options.parse_tls_data = false;

    if let Ok(pe) = PE::parse_with_opts(&mmap, &options) {
        return Ok(pe.header.coff_header.machine);
    }

    // 解析基本头部
    if mmap.len() < 0x40 {
        return Err(anyhow!("file too small to be a valid PE"));
    }
    let e_lfanew = u32::from_le_bytes(mmap[0x3C..0x40].try_into()?) as usize;
    if mmap.len() < e_lfanew + 4 + 20 {
        return Err(anyhow!("invalid PE header offset or file truncated"));
    }
    if &mmap[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return Err(anyhow!("invalid PE signature"));
    }

    let coff_off = e_lfanew + 4;
    // machine (u16)
    if mmap.len() < coff_off + 2 {
        return Err(anyhow!("truncated COFF header"));
    }
    let machine = u16::from_le_bytes(mmap[coff_off..coff_off + 2].try_into()?);
    Ok(machine)
}

/// 判断程序是否为界面程序
///
/// # 参数
/// - `program`: 程序路径
///
/// # 返回值
/// - `Ok(bool)`: 是否为界面程序
/// - `Err(...)`：读取或解析失败
pub fn is_gui_program(program: impl AsRef<Path>) -> Result<bool> {
    let file = File::open(program)?;
    let mmap = unsafe { Mmap::map(&file)? };

    let mut options = ParseOptions::default();
    options.parse_attribute_certificates = false;
    options.parse_tls_data = false;
    if let Ok(pe) = PE::parse_with_opts(&mmap, &options) {
        return Ok(pe.header.optional_header.unwrap().windows_fields.subsystem
            == IMAGE_SUBSYSTEM_WINDOWS_GUI);
    }

    // 解析基本头部
    if mmap.len() < 0x40 {
        return Err(anyhow!("file too small to be a valid PE"));
    }
    let e_lfanew = u32::from_le_bytes(mmap[0x3C..0x40].try_into()?) as usize;
    if mmap.len() < e_lfanew + 4 + 20 {
        return Err(anyhow!("invalid PE header offset or file truncated"));
    }
    if &mmap[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return Err(anyhow!("invalid PE signature"));
    }
    let coff_off = e_lfanew + 4;
    let optional_off = coff_off + 20;
    // Subsystem is at optional_off + 68 (0x44) for PE32/PE32+
    if mmap.len() >= optional_off + 68 + 2 {
        let subsystem = u16::from_le_bytes(mmap[optional_off + 68..optional_off + 70].try_into()?);
        Ok(subsystem == IMAGE_SUBSYSTEM_WINDOWS_GUI)
    } else {
        // optional header truncated: we can't determine subsystem
        Err(anyhow!(
            "PE optional header too small; cannot determine subsystem"
        ))
    }
}

/// 判断程序是否有数字签名
///
/// # 参数
/// - `program`: 程序路径
///
/// # 返回值
/// - `Ok(bool)`: 是否有数字签名
/// - `Err(...)`：读取或解析失败
pub fn exe_has_signature(program: impl AsRef<Path>) -> Result<bool> {
    const IMAGE_DIRECTORY_ENTRY_SECURITY: usize = 4;

    let file = File::open(program)?;
    let mmap = unsafe { Mmap::map(&file)? };

    let pe = PE::parse(&mmap)?;
    if let Some(optional_header) = pe.header.optional_header.as_ref() {
        if let Some(entry) = optional_header
            .data_directories
            .data_directories
            .get(IMAGE_DIRECTORY_ENTRY_SECURITY)
        {
            // entry: &Option<(usize, DataDirectory)>
            if let Some((_rva, data_dir)) = entry.as_ref() {
                return Ok(data_dir.size > 0);
            }
        }
    }

    Ok(false)
}

/// 判断程序是否有图标
///
/// # 参数
/// - `program`: 程序路径
///
/// # 返回值
/// - `bool`: 是否有图标
pub fn has_icon_in_program(program: &Path) -> bool {
    // 将 Path 转换为 Windows API 所需的宽字符串 (UTF-16)
    let wide_path = HSTRING::from(program);

    // 获取文件中包含的图标总数
    let icon_count = unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), -1, None, None, 0) };

    // 如果返回的图标数量大于 0，则表示存在图标
    icon_count > 0
}

/// 从字符串解析图标路径、图标索引
///
/// # 参数
/// - `raw`: 图标路径+索引，格式为 `path#index`
///
/// # 返回值
/// - `(String, i32)`: 图标路径、图标索引
///
/// # 说明
/// - 如果未指定索引，默认索引为 0
pub fn parse_icon_spec(raw: &str) -> (String, i32) {
    if let Some((file, idx)) = raw.split_once('#') {
        (
            file.trim().to_string(),
            idx.trim().parse::<i32>().unwrap_or(0),
        )
    } else {
        (raw.trim().to_string(), 0)
    }
}

/// 校验配置文件中提供的 shortcut.name 是否合法
///
/// # 参数
/// - `name`: 快捷方式名称
///
/// # 返回值
/// - `bool`: 是否合法
pub fn validate_shortcut_name_for_config(name: &str) -> bool {
    /// 禁止在文件名中出现的字符（Windows）
    const FORBIDDEN_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    /// Windows 保留设备名（不区分大小写）
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    // 不能为空或全空白
    if name.trim().is_empty() {
        return false;
    }

    // 不能包含控制字符或 NUL
    if name.chars().any(|c| c.is_control() || c == '\0') {
        return false;
    }

    // 不能包含禁止字符
    if name.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) {
        return false;
    }

    // 不能以空格或点结尾（Windows 不允许）
    if name.ends_with(' ') || name.ends_with('.') {
        return false;
    }

    // 不能是保留设备名（忽略扩展名），比较时不区分大小写
    // 例如 "CON", "con.txt" 都视为保留名
    let upper = name.to_ascii_uppercase();
    let base = upper.split('.').next().unwrap_or(&upper);
    if RESERVED_NAMES.contains(&base) {
        return false;
    }

    true
}

/// 清洗可执行文件的描述，返回 `Some(clean)` 或 `None`（表示不可用或空）
/// - 去除 NUL / 控制字符
/// - 去掉 URL（http/https/www）
/// - 去掉 “by XXX” / "-by XXX" / "—by XXX" 等常见后缀
/// - 去掉尾部架构词（x86/x64/32位/64位 等）和常见版本 token（v1.2.3、1.2.3.4）/日期样式
/// - 替换文件名不允许的字符（<>:"/\\|?* 等 -> '-'），把 & 替为空格
/// - 合并多空格并 Trim
///
/// # 参数
/// - `raw`: 原始描述
///
/// # 返回值
/// - `Option<String>`: 清洗后的描述或 `None`（表示不可用或空）
pub fn sanitize_description(raw: &str) -> Option<String> {
    if raw.trim().is_empty() {
        return None;
    }

    // 初步过滤控制字符与 NUL
    let mut s: String = raw
        .chars()
        .filter(|c| *c != '\0' && !c.is_control())
        .collect();
    if s.trim().is_empty() {
        return None;
    }

    // 小写用于若干快速检测
    let low = s.to_ascii_lowercase();
    // 明确判定为不可用的几类
    if low.trim_matches('_').is_empty()
        || low.contains("todo:")
        || low.contains("7z setup sfx")
        || low.contains("7-zip sfx")
        || low.contains("7zS.sfx")
        || low.starts_with("http://")
        || low.starts_with("https://")
        || low.starts_with("www.")
        || low.starts_with("易语言程序")
        || low.starts_with('@')
        || low.starts_with("QQ：")
    {
        return None;
    }

    // 先删除 URL（会同步更新 s）
    s = strip_urls(&s);

    // 删除 by/ -by / —by 等尾部作者标记（删除到末尾）
    s = strip_by_like(&s);

    // 从尾部剥离版本/位数/日期等噪声（最多剥离 3 段，可根据需要调整）
    s = strip_trailing_version_like(s, 3);

    // 替换非法文件名字符，保留中文/字母/数字/空格/括号
    let mut mapped: String = s
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | ';' => '-',
            '&' => ' ',
            other => other,
        })
        .collect();

    // 删除商标/注册符号
    for ch in &['®', '™', '©'] {
        mapped.retain(|c| c != *ch);
    }

    // 去掉首尾多余标点（常见垃圾）
    mapped = mapped
        .trim()
        .trim_matches(|c: char| ".-–—:;、，。.\\/([{_".contains(c))
        .to_string();

    // 合并空白并修剪
    let final_s = mapped.split_whitespace().collect::<Vec<_>>().join(" ");
    let final_s = final_s.trim().to_string();

    if final_s.is_empty() {
        None
    } else {
        Some(final_s)
    }
}

/// 删除字符串中的 URL（简单策略：定位常见前缀并删除到下一个空白或结尾）
fn strip_urls(src: &str) -> String {
    let mut s = src.to_string();
    let mut lower = s.to_ascii_lowercase();
    for &prefix in &["http://", "https://", "www."] {
        while let Some(start) = lower.find(prefix) {
            // 从 start 向后找第一个空白字符
            let rest = &lower[start..];
            let end_rel = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let end = start + end_rel;
            s.replace_range(start..end, "");
            lower = s.to_ascii_lowercase();
        }
    }
    s
}

/// 删除尾部的 "by ..." 或 "-by ..." 等（大小写不敏感）
/// 这里策略较保守：一旦发现匹配，会把匹配到的位置一直删到字符串末尾（避免作者名残留）
fn strip_by_like(src: &str) -> String {
    let mut s = src.to_string();
    let lower = s.to_ascii_lowercase();
    // 常见的 "by" 前置符形式
    let patterns = [
        " by ", " by:", "-by ", "_by ", "—by ", "–by ", " -by ", " - by ", "·by ",
    ];
    if let Some(pos) = patterns
        .iter()
        .filter_map(|pat| lower.find(pat).map(|p| (p, *pat)))
        .min_by_key(|(p, _)| *p)
    {
        let (p, _pat) = pos;
        s.truncate(p);
        return s.trim_end().to_string();
    }
    s
}

/// 从尾部按分隔符逐段剥离版本 / 架构 / 日期等“像版本”的片段
fn strip_trailing_version_like(mut s: String, max_segments: usize) -> String {
    if s.is_empty() {
        return s;
    }

    // 常用分隔符（包括长横线）
    const SEPS: &[char] = &[' ', '-', '_', '.', '–', '—'];

    for _ in 0..max_segments {
        // 先 trim 末尾的空白和多余标点
        s = s
            .trim_end_matches(|c: char| ".:;、，。.\\/([{_".contains(c) || c.is_whitespace())
            .to_string();
        if s.is_empty() {
            break;
        }

        // 找到最后一个分隔符位置（按 char 边界）
        if let Some((sep_idx, _sep_char)) =
            s.char_indices().rev().find(|&(_, ch)| SEPS.contains(&ch))
        {
            // 计算 last_segment 开始的字节索引
            let after_sep_start = sep_idx + s[sep_idx..].chars().next().unwrap().len_utf8();
            let last_segment = s[after_sep_start..].trim();

            if last_segment.is_empty() {
                // 如果分隔符在末尾，去掉它继续
                s.truncate(sep_idx);
                continue;
            }

            if is_version_like(last_segment)
                || is_bitness_like(last_segment)
                || is_date_like(last_segment)
            {
                // 去掉从分隔符到末尾
                s.truncate(sep_idx);
                s = s.trim_end().to_string();
                continue;
            } else {
                // 如果不是 version-like，就停止（避免误删）
                break;
            }
        } else {
            // 没有分隔符：不在无分隔符的末尾进行自动删除，避免误删（例如 Tool2020 可能是有效名字）
            break;
        }
    }

    s
}

/// 判断 token 是否像版本号（v1.2.3, 1.2, 1.2.3.4 等）
/// 允许前导 'v' 或 'V'，允许后缀如 rc/alpha/beta，但总体需包含数字与点
fn is_version_like(tok: &str) -> bool {
    let t = tok
        .trim()
        .trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c == ',');
    if t.is_empty() {
        return false;
    }

    let mut s = t;
    if s.len() > 1 && (s.starts_with('v') || s.starts_with('V')) {
        s = &s[1..];
    }

    // 主规则：包含 '.' 且至少有一个数字
    if s.contains('.') {
        // 检查每个小段是否主要包含数字（允许字母尾缀）
        let parts: Vec<&str> = s.split('.').collect();
        let mut numish = 0;
        for p in &parts {
            if p.chars().any(|c| c.is_ascii_digit()) {
                numish += 1;
            }
        }
        return numish >= 1 && parts.len() <= 6; // 限制过长的异常串
    }

    // 如果没有 '.'，但全部是数字且长度>=6（可能是日期）也认为是版本/日期
    if s.chars().all(|c| c.is_ascii_digit()) && s.len() >= 6 {
        return true;
    }

    // 如果形如 "1.23" / "123" 等也可能被当作版本，但更保守，返回 false
    false
}

/// 判断 token 是否像位数/架构标注（x64, x86, 64bit, 64 位, arm64 等）
fn is_bitness_like(tok: &str) -> bool {
    let t = tok.to_ascii_lowercase();
    let clean = t
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();
    matches!(
        clean.as_str(),
        "x64"
            | "x86"
            | "x32"
            | "arm64"
            | "arm"
            | "arm32"
            | "arm64ec"
            | "64bit"
            | "32bit"
            | "64bitx"
            | "32bitx"
    ) || clean.ends_with("bit")
        || clean.ends_with("位")
}

/// 判断 token 是否像日期（纯数字且长度在6~8）
fn is_date_like(tok: &str) -> bool {
    let s = tok
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    (s.len() >= 6 && s.len() <= 8) && s.len() == tok.chars().filter(|c| c.is_ascii_digit()).count()
}
