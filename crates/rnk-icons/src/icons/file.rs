//! File type icons (Nerd Font)

/// Rust file icon
pub const fn rust() -> &'static str {
    ""
}

/// Python file icon
pub const fn python() -> &'static str {
    ""
}

/// JavaScript file icon
pub const fn javascript() -> &'static str {
    ""
}

/// TypeScript file icon
pub const fn typescript() -> &'static str {
    ""
}

/// Go file icon
pub const fn go() -> &'static str {
    ""
}

/// C file icon
pub const fn c() -> &'static str {
    ""
}

/// C++ file icon
pub const fn cpp() -> &'static str {
    ""
}

/// Java file icon
pub const fn java() -> &'static str {
    ""
}

/// Ruby file icon
pub const fn ruby() -> &'static str {
    ""
}

/// PHP file icon
pub const fn php() -> &'static str {
    ""
}

/// Swift file icon
pub const fn swift() -> &'static str {
    ""
}

/// Kotlin file icon
pub const fn kotlin() -> &'static str {
    ""
}

/// Lua file icon
pub const fn lua() -> &'static str {
    ""
}

/// Vim file icon
pub const fn vim() -> &'static str {
    ""
}

/// Shell/Bash file icon
pub const fn shell() -> &'static str {
    ""
}

/// HTML file icon
pub const fn html() -> &'static str {
    ""
}

/// CSS file icon
pub const fn css() -> &'static str {
    ""
}

/// Sass/SCSS file icon
pub const fn sass() -> &'static str {
    ""
}

/// JSON file icon
pub const fn json() -> &'static str {
    ""
}

/// YAML file icon
pub const fn yaml() -> &'static str {
    ""
}

/// TOML file icon
pub const fn toml() -> &'static str {
    ""
}

/// XML file icon
pub const fn xml() -> &'static str {
    "謹"
}

/// Markdown file icon
pub const fn markdown() -> &'static str {
    ""
}

/// Text file icon
pub const fn text() -> &'static str {
    ""
}

/// PDF file icon
pub const fn pdf() -> &'static str {
    ""
}

/// Image file icon
pub const fn image() -> &'static str {
    ""
}

/// Video file icon
pub const fn video() -> &'static str {
    ""
}

/// Audio file icon
pub const fn audio() -> &'static str {
    ""
}

/// Archive/Zip file icon
pub const fn archive() -> &'static str {
    ""
}

/// Binary file icon
pub const fn binary() -> &'static str {
    ""
}

/// Lock file icon
pub const fn lock() -> &'static str {
    ""
}

/// Config file icon
pub const fn config() -> &'static str {
    ""
}

/// Database file icon
pub const fn database() -> &'static str {
    ""
}

/// Docker file icon
pub const fn docker() -> &'static str {
    ""
}

/// Makefile icon
pub const fn makefile() -> &'static str {
    ""
}

/// License file icon
pub const fn license() -> &'static str {
    ""
}

/// Readme file icon
pub const fn readme() -> &'static str {
    ""
}

/// Git file icon
pub const fn gitfile() -> &'static str {
    ""
}

/// NPM file icon
pub const fn npm() -> &'static str {
    ""
}

/// Cargo.toml icon
pub const fn cargo() -> &'static str {
    ""
}

/// Default/unknown file icon
pub const fn default() -> &'static str {
    ""
}

/// Get icon for a file extension
pub fn for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "rs" => rust(),
        "py" | "pyw" | "pyi" => python(),
        "js" | "mjs" | "cjs" => javascript(),
        "ts" | "mts" | "cts" => typescript(),
        "go" => go(),
        "c" | "h" => c(),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => cpp(),
        "java" => java(),
        "rb" => ruby(),
        "php" => php(),
        "swift" => swift(),
        "kt" | "kts" => kotlin(),
        "lua" => lua(),
        "vim" => vim(),
        "sh" | "bash" | "zsh" | "fish" => shell(),
        "html" | "htm" => html(),
        "css" => css(),
        "scss" | "sass" => sass(),
        "json" => json(),
        "yaml" | "yml" => yaml(),
        "toml" => toml(),
        "xml" => xml(),
        "md" | "markdown" => markdown(),
        "txt" => text(),
        "pdf" => pdf(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" => image(),
        "mp4" | "mkv" | "avi" | "mov" | "webm" => video(),
        "mp3" | "wav" | "flac" | "ogg" | "m4a" => audio(),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => archive(),
        "exe" | "dll" | "so" | "dylib" | "bin" => binary(),
        "lock" => lock(),
        "db" | "sqlite" | "sqlite3" => database(),
        _ => default(),
    }
}

/// Get icon for a filename
pub fn for_filename(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    match lower.as_str() {
        "cargo.toml" | "cargo.lock" => cargo(),
        "dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => docker(),
        "makefile" | "gnumakefile" => makefile(),
        "license" | "license.md" | "license.txt" => license(),
        "readme" | "readme.md" | "readme.txt" => readme(),
        ".gitignore" | ".gitattributes" | ".gitmodules" => gitfile(),
        "package.json" | "package-lock.json" => npm(),
        _ => {
            // Try extension
            if let Some(ext) = name.rsplit('.').next() {
                if ext != name {
                    return for_extension(ext);
                }
            }
            default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_icons() {
        assert_eq!(rust(), "");
        assert_eq!(python(), "");
        assert_eq!(javascript(), "");
    }

    #[test]
    fn test_for_extension() {
        assert_eq!(for_extension("rs"), "");
        assert_eq!(for_extension("py"), "");
        assert_eq!(for_extension("unknown"), default());
    }

    #[test]
    fn test_for_filename() {
        assert_eq!(for_filename("Cargo.toml"), cargo());
        assert_eq!(for_filename("main.rs"), rust());
        assert_eq!(for_filename("Dockerfile"), docker());
    }
}
