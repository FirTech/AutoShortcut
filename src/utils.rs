use mslnk::{MSLinkError, ShellLink};
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

/// 创建快捷方式
pub fn create_shortcut(target: &Path, link: &Path, args: Option<String>, icon: Option<String>) -> Result<(), MSLinkError> {
    let mut sl = ShellLink::new(target)?;
    sl.set_arguments(args);
    sl.set_icon_location(icon);
    sl.create_lnk(link)
}

/// 处理特殊环境变量
pub fn process_env(path: &Path) -> PathBuf {
    let mut output = path.display().to_string().to_lowercase();
    // 处理 %Desktop% 和 %Programs%
    if let Ok(user_profile) = env::var("USERPROFILE") {
        let desktop = PathBuf::from(&user_profile).join("Desktop");
        output = output.replace("%desktop%", &desktop.to_string_lossy());

        let programs = PathBuf::from(&user_profile).join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs");
        output = output.replace("%programs%", &programs.to_string_lossy());
    }

    // 处理系统环境变量（如 %APPDATA%）
    for (key, value) in env::vars() {
        let placeholder = format!("%{}%", key.to_lowercase());
        output = output.replace(&placeholder, &value);
    }

    PathBuf::from(output)
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
        _ => filename.contains(pattern.trim_matches('*'))
    }
}

/// 获取程序描述
pub fn get_exe_description(path: &Path) -> Result<Option<String>, Box<dyn Error>> {
    // Convert Path to wide string (UTF-16) for Windows API
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    // Get the size of the version information block
    let mut handle = 0u32;
    let info_size = unsafe { GetFileVersionInfoSizeW(PCWSTR(path_wide.as_ptr()), Some(&mut handle)) };

    if info_size == 0 {
        return Ok(None);
    }

    // Allocate buffer for version information
    let mut info_buffer = vec![0u8; info_size as usize];

    // Get the version information
    if unsafe {
        GetFileVersionInfoW(
            PCWSTR(path_wide.as_ptr()),
            Some(handle),
            info_size,
            info_buffer.as_mut_ptr() as *mut std::ffi::c_void,
        )
    }.is_err() {
        return Ok(None);
    }

    // Query for "FileDescription" string
    let mut fixed_file_info_ptr = std::ptr::null_mut();
    let mut fixed_file_info_len = 0u32;

    // 使用 OsStr::new() 转换字符串字面量
    let fixed_info_query_wide: Vec<u16> = OsStr::new("\\").encode_wide().chain(std::iter::once(0)).collect();

    let success_fixed = unsafe {
        VerQueryValueW(
            info_buffer.as_ptr() as *mut std::ffi::c_void,
            PCWSTR(fixed_info_query_wide.as_ptr()),
            &mut fixed_file_info_ptr,
            &mut fixed_file_info_len,
        )
    };

    if success_fixed == BOOL(0) || fixed_file_info_len == 0 {
        return Ok(None);
    }

    let mut translation_ptr = std::ptr::null_mut();
    let mut translation_len = 0u32;

    // 使用 OsStr::new() 转换字符串字面量
    let translation_query_wide: Vec<u16> = OsStr::new("\\VarFileInfo\\Translation").encode_wide().chain(std::iter::once(0)).collect();

    let success_translation = unsafe {
        VerQueryValueW(
            info_buffer.as_ptr() as *mut std::ffi::c_void,
            PCWSTR(translation_query_wide.as_ptr()),
            &mut translation_ptr,
            &mut translation_len,
        )
    };

    if success_translation == BOOL(0) || translation_len == 0 {
        return Ok(None);
    }

    let translation_pair_ptr = translation_ptr as *mut u32;
    let lang_codepage = unsafe { *translation_pair_ptr };

    let lang_id = (lang_codepage & 0xFFFF) as u16;
    let codepage = ((lang_codepage >> 16) & 0xFFFF) as u16;

    let lang_charset_str = format!("{:04x}{:04x}", lang_id, codepage);

    let query_string_template = format!("\\StringFileInfo\\{}\\{}", lang_charset_str, "FileDescription");
    // 使用 OsStr::new() 转换 String 变量
    let query_wide: Vec<u16> = OsStr::new(&query_string_template).encode_wide().chain(std::iter::once(0)).collect();

    let mut value_ptr = std::ptr::null_mut();
    let mut value_len = 0u32;

    let success_query = unsafe {
        VerQueryValueW(
            info_buffer.as_ptr() as *mut std::ffi::c_void,
            PCWSTR(query_wide.as_ptr()),
            &mut value_ptr,
            &mut value_len,
        )
    };

    if success_query == BOOL(0) || value_len == 0 {
        return Ok(None);
    }

    let num_chars = value_len as usize;
    let wide_chars = unsafe { std::slice::from_raw_parts(value_ptr as *const u16, num_chars) };
    let description = String::from_utf16_lossy(wide_chars).trim_end_matches('\0').to_string();

    Ok(Some(description))
}
