//! Path API - Node.js compatible path module
//!
//! High-performance path manipulation implemented in Rust.
//! Provides both platform-native and explicit posix/win32 APIs.
//!
//! Provides:
//! - path.join(...paths) - Join path segments
//! - path.resolve(...paths) - Resolve to absolute path
//! - path.dirname(path) - Get directory name
//! - path.basename(path, ext?) - Get base name
//! - path.extname(path) - Get extension
//! - path.normalize(path) - Normalize path
//! - path.isAbsolute(path) - Check if path is absolute
//! - path.relative(from, to) - Get relative path
//! - path.parse(path) - Parse path into components
//! - path.format(obj) - Format components into path
//! - path.sep - Path separator
//! - path.delimiter - Path delimiter
//! - path.posix - POSIX-specific functions
//! - path.win32 - Windows-specific functions

use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, property::Attribute,
};

/// Path separator for the current platform
#[cfg(windows)]
const SEP: char = '\\';
#[cfg(not(windows))]
const SEP: char = '/';

/// Path delimiter for the current platform
#[cfg(windows)]
const DELIMITER: char = ';';
#[cfg(not(windows))]
const DELIMITER: char = ':';

// ============================================================================
// POSIX Path Operations (maximum performance, zero-copy where possible)
// ============================================================================

/// Normalize a POSIX path - resolves . and .. segments
#[inline]
fn normalize_posix(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }

    let is_absolute = path.starts_with('/');
    let trailing_slash = path.ends_with('/') && path.len() > 1;

    let mut parts: Vec<&str> = Vec::with_capacity(path.matches('/').count() + 1);

    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                if !parts.is_empty() && parts.last() != Some(&"..") {
                    parts.pop();
                } else if !is_absolute {
                    parts.push("..");
                }
            }
            _ => parts.push(part),
        }
    }

    let mut result = if is_absolute {
        String::with_capacity(path.len())
    } else {
        String::with_capacity(path.len())
    };

    if is_absolute {
        result.push('/');
    }

    result.push_str(&parts.join("/"));

    if trailing_slash && !result.ends_with('/') {
        result.push('/');
    }

    if result.is_empty() {
        return ".".to_string();
    }

    result
}

/// Join POSIX paths
#[inline]
fn join_posix(paths: &[&str]) -> String {
    let mut joined = String::with_capacity(paths.iter().map(|p| p.len() + 1).sum());

    for (i, path) in paths.iter().enumerate() {
        if path.is_empty() {
            continue;
        }

        if !joined.is_empty() && !joined.ends_with('/') {
            joined.push('/');
        }

        // Handle absolute paths in the middle - they reset the path
        if path.starts_with('/') && i > 0 {
            joined.clear();
        }

        joined.push_str(path);
    }

    if joined.is_empty() {
        return ".".to_string();
    }

    normalize_posix(&joined)
}

/// Resolve POSIX paths to absolute
#[inline]
fn resolve_posix(paths: &[&str], cwd: &str) -> String {
    let mut resolved = String::with_capacity(256);

    // Process paths from right to left
    for path in paths.iter().rev() {
        if path.is_empty() {
            continue;
        }

        if resolved.is_empty() {
            resolved = (*path).to_string();
        } else {
            resolved = format!("{}/{}", path, resolved);
        }

        if path.starts_with('/') {
            break;
        }
    }

    // If still not absolute, prepend cwd
    if !resolved.starts_with('/') {
        resolved = format!("{}/{}", cwd, resolved);
    }

    normalize_posix(&resolved)
}

/// Get directory name (POSIX)
#[inline]
fn dirname_posix(path: &str) -> &str {
    if path.is_empty() {
        return ".";
    }

    // Remove trailing slashes
    let path = path.trim_end_matches('/');

    if path.is_empty() {
        return "/";
    }

    match path.rfind('/') {
        Some(0) => "/",
        Some(idx) => &path[..idx],
        None => ".",
    }
}

/// Get base name (POSIX)
#[inline]
fn basename_posix<'a>(path: &'a str, ext: Option<&str>) -> &'a str {
    if path.is_empty() {
        return "";
    }

    // Remove trailing slashes
    let path = path.trim_end_matches('/');

    if path.is_empty() {
        return "";
    }

    let base = match path.rfind('/') {
        Some(idx) => &path[idx + 1..],
        None => path,
    };

    // Remove extension if specified
    if let Some(ext) = ext {
        if base.ends_with(ext) {
            return &base[..base.len() - ext.len()];
        }
    }

    base
}

/// Get extension (POSIX)
#[inline]
fn extname_posix(path: &str) -> &str {
    let base = basename_posix(path, None);

    if base.is_empty() || base == "." || base == ".." {
        return "";
    }

    // Find the last dot that isn't at position 0
    if let Some(dot_idx) = base.rfind('.') {
        if dot_idx > 0 {
            return &base[dot_idx..];
        }
    }

    ""
}

/// Check if path is absolute (POSIX)
#[inline]
fn is_absolute_posix(path: &str) -> bool {
    path.starts_with('/')
}

/// Get relative path (POSIX)
fn relative_posix(from: &str, to: &str, cwd: &str) -> String {
    let from_abs = resolve_posix(&[from], cwd);
    let to_abs = resolve_posix(&[to], cwd);

    if from_abs == to_abs {
        return String::new();
    }

    let from_parts: Vec<&str> = from_abs.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to_abs.split('/').filter(|s| !s.is_empty()).collect();

    // Find common prefix length
    let mut common = 0;
    for (a, b) in from_parts.iter().zip(to_parts.iter()) {
        if a != b {
            break;
        }
        common += 1;
    }

    // Build relative path
    let up_count = from_parts.len() - common;
    let mut result = Vec::with_capacity(up_count + to_parts.len() - common);

    for _ in 0..up_count {
        result.push("..");
    }

    for part in &to_parts[common..] {
        result.push(*part);
    }

    if result.is_empty() {
        ".".to_string()
    } else {
        result.join("/")
    }
}

// ============================================================================
// Windows Path Operations
// ============================================================================

/// Normalize a Windows path
#[inline]
fn normalize_win32(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }

    // Check for drive letter or UNC
    let (prefix, rest) = extract_win32_prefix(path);
    let is_absolute = !prefix.is_empty() || path.starts_with('\\') || path.starts_with('/');
    let trailing_slash =
        (path.ends_with('\\') || path.ends_with('/')) && path.len() > prefix.len() + 1;

    let mut parts: Vec<&str> = Vec::with_capacity(16);

    for part in rest.split(|c| c == '\\' || c == '/') {
        match part {
            "" | "." => continue,
            ".." => {
                if !parts.is_empty() && parts.last() != Some(&"..") {
                    parts.pop();
                } else if !is_absolute {
                    parts.push("..");
                }
            }
            _ => parts.push(part),
        }
    }

    let mut result = String::with_capacity(path.len());
    result.push_str(prefix);

    if is_absolute && !prefix.is_empty() && !prefix.ends_with('\\') {
        result.push('\\');
    } else if is_absolute && prefix.is_empty() {
        result.push('\\');
    }

    result.push_str(&parts.join("\\"));

    if trailing_slash && !result.ends_with('\\') {
        result.push('\\');
    }

    if result.is_empty() {
        return ".".to_string();
    }

    result
}

/// Extract Windows path prefix (drive letter or UNC)
#[inline]
fn extract_win32_prefix(path: &str) -> (&str, &str) {
    let bytes = path.as_bytes();

    // Check for drive letter (C:)
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return (&path[..2], &path[2..]);
    }

    // Check for UNC (\\server\share)
    if bytes.len() >= 2 && (bytes[0] == b'\\' || bytes[0] == b'/') && bytes[0] == bytes[1] {
        // Find end of server name
        if let Some(server_end) = path[2..].find(|c| c == '\\' || c == '/') {
            let server_end = server_end + 2;
            // Find end of share name
            if let Some(share_end) = path[server_end + 1..].find(|c| c == '\\' || c == '/') {
                let share_end = share_end + server_end + 1;
                return (&path[..share_end], &path[share_end..]);
            }
            return (&path[..], "");
        }
    }

    ("", path)
}

/// Join Windows paths
#[inline]
fn join_win32(paths: &[&str]) -> String {
    let mut joined = String::with_capacity(paths.iter().map(|p| p.len() + 1).sum());

    for path in paths.iter() {
        if path.is_empty() {
            continue;
        }

        // Handle absolute paths - they reset
        if is_absolute_win32(path) {
            joined.clear();
            joined.push_str(path);
        } else {
            if !joined.is_empty()
                && !joined.ends_with('\\')
                && !joined.ends_with('/')
                && !joined.ends_with(':')
            {
                joined.push('\\');
            }
            joined.push_str(path);
        }
    }

    if joined.is_empty() {
        return ".".to_string();
    }

    normalize_win32(&joined)
}

/// Resolve Windows paths to absolute
#[inline]
fn resolve_win32(paths: &[&str], cwd: &str) -> String {
    let mut resolved = String::with_capacity(256);

    for path in paths.iter().rev() {
        if path.is_empty() {
            continue;
        }

        if resolved.is_empty() {
            resolved = (*path).to_string();
        } else if is_absolute_win32(path) {
            resolved = (*path).to_string();
            break;
        } else {
            resolved = format!("{}\\{}", path, resolved);
        }

        if is_absolute_win32(&resolved) {
            break;
        }
    }

    if !is_absolute_win32(&resolved) {
        resolved = format!("{}\\{}", cwd, resolved);
    }

    normalize_win32(&resolved)
}

/// Get directory name (Windows)
#[inline]
fn dirname_win32(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }

    let (prefix, rest) = extract_win32_prefix(path);

    // Remove trailing slashes
    let rest = rest.trim_end_matches(|c| c == '\\' || c == '/');

    if rest.is_empty() {
        if !prefix.is_empty() {
            return prefix.to_string();
        }
        return ".".to_string();
    }

    match rest.rfind(|c| c == '\\' || c == '/') {
        Some(idx) if idx == 0 => format!("{}\\", prefix),
        Some(idx) => format!("{}{}", prefix, &rest[..idx]),
        None => {
            if !prefix.is_empty() {
                prefix.to_string()
            } else {
                ".".to_string()
            }
        }
    }
}

/// Get base name (Windows)
#[inline]
fn basename_win32<'a>(path: &'a str, ext: Option<&str>) -> String {
    if path.is_empty() {
        return String::new();
    }

    let (_, rest) = extract_win32_prefix(path);

    // Remove trailing slashes
    let rest = rest.trim_end_matches(|c| c == '\\' || c == '/');

    if rest.is_empty() {
        return String::new();
    }

    let base = match rest.rfind(|c| c == '\\' || c == '/') {
        Some(idx) => &rest[idx + 1..],
        None => rest,
    };

    // Remove extension if specified
    if let Some(ext) = ext {
        if base.to_lowercase().ends_with(&ext.to_lowercase()) {
            return base[..base.len() - ext.len()].to_string();
        }
    }

    base.to_string()
}

/// Get extension (Windows)
#[inline]
fn extname_win32(path: &str) -> String {
    let base = basename_win32(path, None);

    if base.is_empty() || base == "." || base == ".." {
        return String::new();
    }

    if let Some(dot_idx) = base.rfind('.') {
        if dot_idx > 0 {
            return base[dot_idx..].to_string();
        }
    }

    String::new()
}

/// Check if path is absolute (Windows)
#[inline]
fn is_absolute_win32(path: &str) -> bool {
    let bytes = path.as_bytes();

    // Drive letter with slash (C:\)
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }

    // UNC path (\\server)
    if bytes.len() >= 2 && (bytes[0] == b'\\' || bytes[0] == b'/') && bytes[0] == bytes[1] {
        return true;
    }

    false
}

// ============================================================================
// Glob Matching (shared between POSIX and Windows)
// ============================================================================

/// Match a path against a glob pattern
/// Supports: * (any chars except separator), ** (any chars including separator), ? (single char)
#[inline]
fn matches_glob(path: &str, pattern: &str) -> bool {
    matches_glob_recursive(path.as_bytes(), pattern.as_bytes())
}

fn matches_glob_recursive(path: &[u8], pattern: &[u8]) -> bool {
    let mut p_idx = 0;
    let mut s_idx = 0;
    let mut star_idx: Option<usize> = None;
    let mut match_idx = 0;

    while s_idx < path.len() {
        if p_idx < pattern.len() {
            let p_char = pattern[p_idx];
            let s_char = path[s_idx];

            // Handle **
            if p_idx + 1 < pattern.len() && p_char == b'*' && pattern[p_idx + 1] == b'*' {
                // ** matches anything including path separators
                // Skip the **
                p_idx += 2;
                // Skip optional trailing separator after **
                if p_idx < pattern.len() && (pattern[p_idx] == b'/' || pattern[p_idx] == b'\\') {
                    p_idx += 1;
                }

                // Try to match the rest of the pattern against every suffix of path
                if p_idx >= pattern.len() {
                    return true; // ** at end matches everything
                }

                // Try matching from each position
                for i in s_idx..=path.len() {
                    if matches_glob_recursive(&path[i..], &pattern[p_idx..]) {
                        return true;
                    }
                }
                return false;
            }

            // Handle single *
            if p_char == b'*' {
                star_idx = Some(p_idx);
                match_idx = s_idx;
                p_idx += 1;
                continue;
            }

            // Handle ?
            if p_char == b'?' {
                // ? matches any single character except path separator
                if s_char != b'/' && s_char != b'\\' {
                    p_idx += 1;
                    s_idx += 1;
                    continue;
                }
            }

            // Handle character class [abc] or [a-z]
            if p_char == b'[' {
                if let Some((matched, new_p_idx)) = match_char_class(&pattern[p_idx..], s_char) {
                    if matched {
                        p_idx += new_p_idx;
                        s_idx += 1;
                        continue;
                    }
                }
            }

            // Exact match (case-insensitive on Windows could be added)
            if p_char == s_char
                || (p_char == b'/' && s_char == b'\\')
                || (p_char == b'\\' && s_char == b'/')
            {
                p_idx += 1;
                s_idx += 1;
                continue;
            }
        }

        // No match, but we have a * to fall back to
        if let Some(star) = star_idx {
            // * doesn't match path separators
            if path[match_idx] == b'/' || path[match_idx] == b'\\' {
                return false;
            }
            p_idx = star + 1;
            match_idx += 1;
            s_idx = match_idx;
            continue;
        }

        return false;
    }

    // Skip trailing *s and **s in pattern
    while p_idx < pattern.len() {
        if pattern[p_idx] == b'*' {
            p_idx += 1;
        } else {
            break;
        }
    }

    p_idx >= pattern.len()
}

/// Match a character class like [abc] or [a-z] or [!abc]
fn match_char_class(pattern: &[u8], ch: u8) -> Option<(bool, usize)> {
    if pattern.is_empty() || pattern[0] != b'[' {
        return None;
    }

    let mut idx = 1;
    let negated = if idx < pattern.len() && pattern[idx] == b'!' {
        idx += 1;
        true
    } else {
        false
    };

    let mut matched = false;

    while idx < pattern.len() && pattern[idx] != b']' {
        let start = pattern[idx];
        idx += 1;

        // Check for range like a-z
        if idx + 1 < pattern.len() && pattern[idx] == b'-' && pattern[idx + 1] != b']' {
            let end = pattern[idx + 1];
            idx += 2;
            if ch >= start && ch <= end {
                matched = true;
            }
        } else if ch == start {
            matched = true;
        }
    }

    if idx < pattern.len() && pattern[idx] == b']' {
        Some((if negated { !matched } else { matched }, idx + 1))
    } else {
        None // Unclosed bracket
    }
}

/// Get relative path (Windows)
fn relative_win32(from: &str, to: &str, cwd: &str) -> String {
    let from_abs = resolve_win32(&[from], cwd).to_lowercase();
    let to_abs = resolve_win32(&[to], cwd);
    let to_abs_lower = to_abs.to_lowercase();

    if from_abs == to_abs_lower {
        return String::new();
    }

    // Check if they're on different drives
    let (from_prefix, _) = extract_win32_prefix(&from_abs);
    let (to_prefix, _) = extract_win32_prefix(&to_abs_lower);

    if !from_prefix.is_empty()
        && !to_prefix.is_empty()
        && from_prefix.to_lowercase() != to_prefix.to_lowercase()
    {
        return to_abs;
    }

    let from_parts: Vec<&str> = from_abs
        .split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .collect();
    let to_parts: Vec<&str> = to_abs_lower
        .split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .collect();

    let mut common = 0;
    for (a, b) in from_parts.iter().zip(to_parts.iter()) {
        if a.to_lowercase() != b.to_lowercase() {
            break;
        }
        common += 1;
    }

    let up_count = from_parts.len() - common;
    let mut result = Vec::with_capacity(up_count + to_parts.len() - common);

    for _ in 0..up_count {
        result.push("..");
    }

    // Use original case from to_abs for the remaining parts
    let to_parts_original: Vec<&str> = to_abs
        .split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .collect();

    for part in &to_parts_original[common..] {
        result.push(*part);
    }

    if result.is_empty() {
        ".".to_string()
    } else {
        result.join("\\")
    }
}

// ============================================================================
// Native Function Registrations
// ============================================================================

/// Create native functions for a path module (posix or win32)
fn create_path_object(context: &mut Context, is_win32: bool) -> JsResult<JsObject> {
    let sep = if is_win32 { "\\" } else { "/" };
    let delimiter = if is_win32 { ";" } else { ":" };

    let mut builder = ObjectInitializer::new(context);

    // path.sep
    builder.property(js_string!("sep"), js_string!(sep), Attribute::all());

    // path.delimiter
    builder.property(
        js_string!("delimiter"),
        js_string!(delimiter),
        Attribute::all(),
    );

    let path_obj = builder.build();

    // Register native functions on global, then reference them
    let prefix = if is_win32 { "win32" } else { "posix" };

    // join
    let join_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let paths: Vec<String> = args
                .iter()
                .filter_map(|v| v.to_string(context).ok())
                .map(|s| s.to_std_string_escaped())
                .collect();
            let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            Ok(JsValue::from(js_string!(join_win32(&path_refs))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let paths: Vec<String> = args
                .iter()
                .filter_map(|v| v.to_string(context).ok())
                .map(|s| s.to_std_string_escaped())
                .collect();
            let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            Ok(JsValue::from(js_string!(join_posix(&path_refs))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_join", prefix)),
        join_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // resolve
    let resolve_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let paths: Vec<String> = args
                .iter()
                .filter_map(|v| v.to_string(context).ok())
                .map(|s| s.to_std_string_escaped())
                .collect();
            let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            Ok(JsValue::from(js_string!(resolve_win32(&path_refs, &cwd))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let cwd = std::env::current_dir()
                .map(|p| {
                    let s = p.to_string_lossy().to_string();
                    // Convert Windows paths to POSIX for posix.resolve
                    s.replace('\\', "/")
                })
                .unwrap_or_else(|_| ".".to_string());
            let paths: Vec<String> = args
                .iter()
                .filter_map(|v| v.to_string(context).ok())
                .map(|s| s.to_std_string_escaped())
                .collect();
            let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            Ok(JsValue::from(js_string!(resolve_posix(&path_refs, &cwd))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_resolve", prefix)),
        resolve_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // normalize
    let normalize_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(normalize_win32(&path))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(normalize_posix(&path))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_normalize", prefix)),
        normalize_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // dirname
    let dirname_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(dirname_win32(&path))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(dirname_posix(&path))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_dirname", prefix)),
        dirname_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // basename
    let basename_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let ext = args
                .get(1)
                .filter(|v| !v.is_undefined())
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped());
            Ok(JsValue::from(js_string!(basename_win32(
                &path,
                ext.as_deref()
            ))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let ext = args
                .get(1)
                .filter(|v| !v.is_undefined())
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped());
            Ok(JsValue::from(js_string!(basename_posix(
                &path,
                ext.as_deref()
            ))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_basename", prefix)),
        basename_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // extname
    let extname_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(extname_win32(&path))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(extname_posix(&path))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_extname", prefix)),
        extname_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // isAbsolute
    let is_absolute_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(is_absolute_win32(&path)))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(is_absolute_posix(&path)))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_isAbsolute", prefix)),
        is_absolute_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // relative
    let relative_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let from = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let to = args
                .get(1)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(relative_win32(&from, &to, &cwd))))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let cwd = std::env::current_dir()
                .map(|p| {
                    let s = p.to_string_lossy().to_string();
                    s.replace('\\', "/")
                })
                .unwrap_or_else(|_| ".".to_string());
            let from = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let to = args
                .get(1)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(relative_posix(&from, &to, &cwd))))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_relative", prefix)),
        relative_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // parse
    let parse_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let (root, _) = extract_win32_prefix(&path);
            let dir = dirname_win32(&path);
            let base = basename_win32(&path, None);
            let ext = extname_win32(&path);
            let name = if !ext.is_empty() && base.ends_with(&ext) {
                base[..base.len() - ext.len()].to_string()
            } else {
                base.clone()
            };

            let obj = ObjectInitializer::new(context)
                .property(
                    js_string!("root"),
                    js_string!(if is_absolute_win32(&path) {
                        if root.is_empty() { "\\" } else { root }
                    } else {
                        ""
                    }),
                    Attribute::all(),
                )
                .property(js_string!("dir"), js_string!(dir), Attribute::all())
                .property(js_string!("base"), js_string!(base), Attribute::all())
                .property(js_string!("ext"), js_string!(ext), Attribute::all())
                .property(js_string!("name"), js_string!(name), Attribute::all())
                .build();

            Ok(JsValue::from(obj))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            let dir = dirname_posix(&path);
            let base = basename_posix(&path, None);
            let ext = extname_posix(&path);
            let name = if !ext.is_empty() && base.ends_with(ext) {
                &base[..base.len() - ext.len()]
            } else {
                base
            };

            let obj = ObjectInitializer::new(context)
                .property(
                    js_string!("root"),
                    js_string!(if is_absolute_posix(&path) { "/" } else { "" }),
                    Attribute::all(),
                )
                .property(js_string!("dir"), js_string!(dir), Attribute::all())
                .property(js_string!("base"), js_string!(base), Attribute::all())
                .property(js_string!("ext"), js_string!(ext), Attribute::all())
                .property(js_string!("name"), js_string!(name), Attribute::all())
                .build();

            Ok(JsValue::from(obj))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_parse", prefix)),
        parse_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // format
    // Helper to get string property, returning empty string for undefined/null
    fn get_string_prop(obj: &JsObject, key: &str, context: &mut Context) -> String {
        let val = match obj.get(js_string!(key), context) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        if val.is_undefined() || val.is_null() {
            return String::new();
        }
        val.to_string(context)
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default()
    }

    let format_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let obj = args.get(0).ok_or_else(|| {
                JsNativeError::typ().with_message("path.format requires an object argument")
            })?;
            let obj = obj.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("path.format requires an object argument")
            })?;

            let dir = get_string_prop(&obj, "dir", context);
            let root = get_string_prop(&obj, "root", context);
            let base = get_string_prop(&obj, "base", context);
            let name = get_string_prop(&obj, "name", context);
            let ext = get_string_prop(&obj, "ext", context);

            let filename = if !base.is_empty() {
                base
            } else {
                let ext_with_dot = if !ext.is_empty() && !ext.starts_with('.') {
                    format!(".{}", ext)
                } else {
                    ext
                };
                format!("{}{}", name, ext_with_dot)
            };

            let result = if !dir.is_empty() {
                if dir.ends_with('\\') || dir.ends_with('/') {
                    format!("{}{}", dir, filename)
                } else {
                    format!("{}\\{}", dir, filename)
                }
            } else if !root.is_empty() {
                format!("{}{}", root, filename)
            } else {
                filename
            };

            Ok(JsValue::from(js_string!(result)))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let obj = args.get(0).ok_or_else(|| {
                JsNativeError::typ().with_message("path.format requires an object argument")
            })?;
            let obj = obj.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("path.format requires an object argument")
            })?;

            let dir = get_string_prop(&obj, "dir", context);
            let root = get_string_prop(&obj, "root", context);
            let base = get_string_prop(&obj, "base", context);
            let name = get_string_prop(&obj, "name", context);
            let ext = get_string_prop(&obj, "ext", context);

            let filename = if !base.is_empty() {
                base
            } else {
                let ext_with_dot = if !ext.is_empty() && !ext.starts_with('.') {
                    format!(".{}", ext)
                } else {
                    ext
                };
                format!("{}{}", name, ext_with_dot)
            };

            let result = if !dir.is_empty() {
                if dir.ends_with('/') {
                    format!("{}{}", dir, filename)
                } else {
                    format!("{}/{}", dir, filename)
                }
            } else if !root.is_empty() {
                format!("{}{}", root, filename)
            } else {
                filename
            };

            Ok(JsValue::from(js_string!(result)))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_format", prefix)),
        format_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // toNamespacedPath (Windows-only, returns input on POSIX)
    let to_namespaced_fn = if is_win32 {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            // Convert to extended-length path if absolute
            if is_absolute_win32(&path) {
                let normalized = normalize_win32(&path);
                // Check if already namespaced
                if normalized.starts_with("\\\\?\\") || normalized.starts_with("\\\\.\\") {
                    return Ok(JsValue::from(js_string!(normalized)));
                }
                // UNC paths: \\server\share -> \\?\UNC\server\share
                if normalized.starts_with("\\\\") {
                    let unc_path = format!("\\\\?\\UNC\\{}", &normalized[2..]);
                    return Ok(JsValue::from(js_string!(unc_path)));
                }
                // Regular paths: C:\foo -> \\?\C:\foo
                let ns_path = format!("\\\\?\\{}", normalized);
                return Ok(JsValue::from(js_string!(ns_path)));
            }

            Ok(JsValue::from(js_string!(path)))
        })
    } else {
        NativeFunction::from_fn_ptr(|_this, args, context| {
            // On POSIX, just return the path as-is
            let path = args
                .get(0)
                .map(|v| v.to_string(context))
                .transpose()?
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            Ok(JsValue::from(js_string!(path)))
        })
    };
    context.global_object().set(
        js_string!(format!("__viper_path_{}_toNamespacedPath", prefix)),
        to_namespaced_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // matchesGlob - check if path matches a glob pattern
    let matches_glob_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let pattern = args
            .get(1)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        Ok(JsValue::from(matches_glob(&path, &pattern)))
    });
    context.global_object().set(
        js_string!(format!("__viper_path_{}_matchesGlob", prefix)),
        matches_glob_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(path_obj)
}

/// Register the path module
pub fn register_path(context: &mut Context) -> JsResult<()> {
    // Create posix and win32 objects
    let _posix_obj = create_path_object(context, false)?;
    let _win32_obj = create_path_object(context, true)?;

    // Determine which is the native platform
    let is_windows = cfg!(windows);
    let native_prefix = if is_windows { "win32" } else { "posix" };

    // Create the path module in JavaScript that delegates to native functions
    let path_code = format!(
        r#"
        // Create path module with native Rust functions
        const __path_posix = {{
            sep: '/',
            delimiter: ':',
            join: (...paths) => __viper_path_posix_join(...paths),
            resolve: (...paths) => __viper_path_posix_resolve(...paths),
            normalize: (p) => __viper_path_posix_normalize(p),
            dirname: (p) => __viper_path_posix_dirname(p),
            basename: (p, ext) => __viper_path_posix_basename(p, ext),
            extname: (p) => __viper_path_posix_extname(p),
            isAbsolute: (p) => __viper_path_posix_isAbsolute(p),
            relative: (from, to) => __viper_path_posix_relative(from, to),
            parse: (p) => __viper_path_posix_parse(p),
            format: (obj) => __viper_path_posix_format(obj),
            toNamespacedPath: (p) => __viper_path_posix_toNamespacedPath(p),
            matchesGlob: (path, pattern) => __viper_path_posix_matchesGlob(path, pattern),
        }};

        const __path_win32 = {{
            sep: '\\',
            delimiter: ';',
            join: (...paths) => __viper_path_win32_join(...paths),
            resolve: (...paths) => __viper_path_win32_resolve(...paths),
            normalize: (p) => __viper_path_win32_normalize(p),
            dirname: (p) => __viper_path_win32_dirname(p),
            basename: (p, ext) => __viper_path_win32_basename(p, ext),
            extname: (p) => __viper_path_win32_extname(p),
            isAbsolute: (p) => __viper_path_win32_isAbsolute(p),
            relative: (from, to) => __viper_path_win32_relative(from, to),
            parse: (p) => __viper_path_win32_parse(p),
            format: (obj) => __viper_path_win32_format(obj),
            toNamespacedPath: (p) => __viper_path_win32_toNamespacedPath(p),
            matchesGlob: (path, pattern) => __viper_path_win32_matchesGlob(path, pattern),
        }};

        // Add cross-references
        __path_posix.posix = __path_posix;
        __path_posix.win32 = __path_win32;
        __path_win32.posix = __path_posix;
        __path_win32.win32 = __path_win32;

        // Create main path object (platform-native)
        globalThis.path = {{
            sep: '{sep}',
            delimiter: '{delimiter}',
            join: (...paths) => __viper_path_{native}_join(...paths),
            resolve: (...paths) => __viper_path_{native}_resolve(...paths),
            normalize: (p) => __viper_path_{native}_normalize(p),
            dirname: (p) => __viper_path_{native}_dirname(p),
            basename: (p, ext) => __viper_path_{native}_basename(p, ext),
            extname: (p) => __viper_path_{native}_extname(p),
            isAbsolute: (p) => __viper_path_{native}_isAbsolute(p),
            relative: (from, to) => __viper_path_{native}_relative(from, to),
            parse: (p) => __viper_path_{native}_parse(p),
            format: (obj) => __viper_path_{native}_format(obj),
            toNamespacedPath: (p) => __viper_path_{native}_toNamespacedPath(p),
            matchesGlob: (path, pattern) => __viper_path_{native}_matchesGlob(path, pattern),
            posix: __path_posix,
            win32: __path_win32,
        }};

        // Also make it available as a module-like export
        globalThis.__viper_modules = globalThis.__viper_modules || {{}};
        globalThis.__viper_modules['path'] = globalThis.path;
        globalThis.__viper_modules['node:path'] = globalThis.path;
    "#,
        sep = if is_windows { "\\\\" } else { "/" },
        delimiter = if is_windows { ";" } else { ":" },
        native = native_prefix,
    );

    let source = Source::from_bytes(path_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_posix() {
        assert_eq!(
            normalize_posix("/foo/bar//baz/asdf/quux/.."),
            "/foo/bar/baz/asdf"
        );
        assert_eq!(normalize_posix("foo/bar/../baz"), "foo/baz");
        assert_eq!(normalize_posix("./foo/bar"), "foo/bar");
        assert_eq!(normalize_posix(""), ".");
        assert_eq!(normalize_posix("/"), "/");
        assert_eq!(normalize_posix(".."), "..");
        assert_eq!(normalize_posix("../.."), "../..");
    }

    #[test]
    fn test_join_posix() {
        assert_eq!(
            join_posix(&["/foo", "bar", "baz/asdf", "quux", ".."]),
            "/foo/bar/baz/asdf"
        );
        assert_eq!(join_posix(&["foo", "bar", "baz"]), "foo/bar/baz");
        assert_eq!(join_posix(&["", ""]), ".");
    }

    #[test]
    fn test_dirname_posix() {
        assert_eq!(dirname_posix("/foo/bar/baz/asdf/quux"), "/foo/bar/baz/asdf");
        assert_eq!(dirname_posix("/foo/bar"), "/foo");
        assert_eq!(dirname_posix("/foo"), "/");
        assert_eq!(dirname_posix("foo"), ".");
        assert_eq!(dirname_posix(""), ".");
    }

    #[test]
    fn test_basename_posix() {
        assert_eq!(
            basename_posix("/foo/bar/baz/asdf/quux.html", None),
            "quux.html"
        );
        assert_eq!(
            basename_posix("/foo/bar/baz/asdf/quux.html", Some(".html")),
            "quux"
        );
        assert_eq!(
            basename_posix("/foo/bar/baz/asdf/quux.html", Some(".htm")),
            "quux.html"
        );
    }

    #[test]
    fn test_extname_posix() {
        assert_eq!(extname_posix("index.html"), ".html");
        assert_eq!(extname_posix("index.coffee.md"), ".md");
        assert_eq!(extname_posix("index."), ".");
        assert_eq!(extname_posix("index"), "");
        assert_eq!(extname_posix(".index"), "");
        assert_eq!(extname_posix(".index.md"), ".md");
    }

    #[test]
    fn test_is_absolute_posix() {
        assert!(is_absolute_posix("/foo/bar"));
        assert!(is_absolute_posix("/baz/.."));
        assert!(!is_absolute_posix("qux/"));
        assert!(!is_absolute_posix("."));
    }

    #[test]
    fn test_normalize_win32() {
        assert_eq!(normalize_win32("C:\\foo\\bar\\..\\baz"), "C:\\foo\\baz");
        assert_eq!(normalize_win32("C:\\foo\\bar\\.\\baz"), "C:\\foo\\bar\\baz");
        assert_eq!(normalize_win32("C:/foo/bar"), "C:\\foo\\bar");
    }

    #[test]
    fn test_is_absolute_win32() {
        assert!(is_absolute_win32("C:\\foo\\bar"));
        assert!(is_absolute_win32("C:/foo/bar"));
        assert!(is_absolute_win32("\\\\server\\share"));
        assert!(!is_absolute_win32("foo\\bar"));
        assert!(!is_absolute_win32("C:foo"));
    }
}
