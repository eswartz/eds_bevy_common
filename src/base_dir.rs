use std::path::Path;
use std::path::PathBuf;

/// Find the runtime directory for the running program, by checking first
/// based on the executable's location (for deployment), then falling back to
/// CARGO_MANIFEST_DIR for dev-time runs.
///
/// match_dir: a function which is passed a candidate Path
/// and returns true if this looks like the base directory.
///
#[cfg(not(target_arch = "wasm32"))]
pub fn find_runtime_base_directory(match_dir: impl Fn(&Path) -> bool) -> Result<PathBuf, &'static str> {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // We're building. This file is likely in the source tree. Step up.
        let mut comps = Path::new(&manifest_dir).ancestors();
        while let Some(test) = comps.next() {
            // if end.file_name().is_some_and(|f| f == "crates") {
            if match_dir(test) {
                return Ok(test.to_path_buf());
            }
        }
        // eep, nothing found
    }

    let Some(exe_path) = std::env::args_os().next() else {
        return Err("no executable path detected in argv!");
    };

    let Ok(cwd) = std::env::current_dir() else {
        return Err("failed to find the current directory");
    };

    // Running from source dir under debugger?
    if match_dir(&cwd) {
        return Ok(cwd.to_path_buf());
    }

    let exe_dir = cwd.join(Path::new(Path::new(&exe_path).parent().expect("no directory for argv[0]")));
    let mut comps = exe_dir.ancestors();
    while let Some(test) = comps.next() {
        if match_dir(test) {
            return Ok(test.to_path_buf());
        }
    }
    Err("no base directory detected")
}

#[cfg(target_arch = "wasm32")]
pub fn find_runtime_base_directory(_match_dir: impl Fn(&Path) -> bool) -> Result<PathBuf, &'static str> {
    // No possibility of running "outside" the build on web.
    Ok(Path::new(".").to_path_buf())
}

/// Detect the base directory which contains the given folder.
pub fn find_runtime_base_directory_by_folder(folder: &str) -> Result<PathBuf, &'static str> {
    find_runtime_base_directory(|path| path.join(folder).is_dir())
}
