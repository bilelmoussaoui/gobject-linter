use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Raw meson introspection data
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct MesonData {
    #[serde(default)]
    pub benchmarks: Vec<Benchmark>,
    #[serde(default)]
    pub buildoptions: Vec<BuildOption>,
    #[serde(default)]
    pub buildsystem_files: Vec<String>,
    #[serde(default)]
    pub compilers: HashMap<String, HashMap<String, Compiler>>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub install_plan: HashMap<String, HashMap<String, InstallPlan>>,
    #[serde(default)]
    pub installed: HashMap<String, String>,
    #[serde(default)]
    pub machines: HashMap<String, MachineInfo>,
    #[serde(default)]
    pub projectinfo: Option<ProjectInfo>,
    #[serde(default)]
    pub targets: Vec<Target>,
    #[serde(default)]
    pub tests: Vec<Test>,
}

/// Represents the full output of `meson introspect --all` with pre-computed
/// caches
#[derive(Debug)]
pub struct MesonIntrospection {
    /// Path to the meson build directory
    build_dir: PathBuf,

    /// Raw meson data
    #[allow(dead_code)]
    data: MesonData,

    /// Pre-computed set of GIR-introspected headers
    introspected_headers: HashSet<PathBuf>,

    /// Pre-computed set of installed headers
    installed_headers: HashSet<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct Benchmark {
    pub name: String,
    pub suite: Vec<String>,
    pub cmd: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub depends: Vec<String>,
    pub workdir: Option<String>,
    pub timeout: u32,
    pub protocol: String,
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
pub struct BuildOption {
    pub name: String,
    #[serde(rename = "type")]
    pub option_type: String,
    pub value: serde_json::Value,
    pub section: String,
    pub machine: String,
    pub description: String,
    #[serde(default)]
    pub choices: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Compiler {
    pub id: String,
    #[serde(default)]
    pub exelist: Vec<String>,
    #[serde(default)]
    pub linker_exelist: Vec<String>,
    #[serde(default)]
    pub file_suffixes: Vec<String>,
    #[serde(default)]
    pub default_suffix: Option<String>,
    pub version: String,
    pub full_version: String,
    #[serde(default)]
    pub linker_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(rename = "type")]
    pub dep_type: String,
    pub version: String,
    #[serde(default)]
    pub compile_args: Vec<String>,
    #[serde(default)]
    pub link_args: Vec<String>,
    #[serde(default)]
    pub include_directories: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct InstallPlan {
    pub destination: String,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub subproject: Option<String>,
    #[serde(default)]
    pub install_rpath: Option<String>,
    #[serde(default)]
    pub exclude_dirs: Vec<String>,
    #[serde(default)]
    pub exclude_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MachineInfo {
    pub system: String,
    pub cpu_family: String,
    pub cpu: String,
    pub endian: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInfo {
    pub version: String,
    pub descriptive_name: String,
    #[serde(default)]
    pub license: Vec<String>,
    #[serde(default)]
    pub subprojects: Vec<SubprojectInfo>,
}

#[derive(Debug, Deserialize)]
pub struct SubprojectInfo {
    pub name: String,
    pub version: String,
    pub descriptive_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: String,
    pub id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub defined_in: String,
    pub filename: Vec<String>,
    pub build_by_default: bool,
    #[serde(default)]
    pub target_sources: Vec<TargetSource>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub subproject: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Deserialize)]
pub struct TargetSource {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub compiler: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub generated_sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Test {
    pub name: String,
    pub suite: Vec<String>,
    pub cmd: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub depends: Vec<String>,
    pub workdir: Option<String>,
    pub timeout: u32,
    pub protocol: String,
    pub priority: i32,
}

/// Find the meson build directory by looking for meson-info/meson-info.json
/// Searches in:
/// 1. User-specified build_dir from config
/// 2. Common build directory names (build/, builddir/, _build/)
/// 3. Any directory containing meson-info/
fn find_build_dir(project_root: &Path, config_build_dir: Option<&str>) -> Option<PathBuf> {
    // 1. Check user-specified build_dir
    if let Some(dir) = config_build_dir {
        let path = project_root.join(dir);
        if is_build_dir(&path) {
            return Some(path);
        }
    }

    // 2. Check common build directory names
    for name in &["build", "builddir", "_build"] {
        let path = project_root.join(name);
        if is_build_dir(&path) {
            return Some(path);
        }
    }

    // 3. Search for any directory containing meson-info/
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if path.is_dir() && is_build_dir(&path) {
                return Some(path);
            }
        }
    }

    None
}

/// Check if a directory is a valid meson build directory
fn is_build_dir(path: &Path) -> bool {
    path.join("meson-info").join("meson-info.json").exists()
}

impl MesonIntrospection {
    /// Create a mock MesonIntrospection for testing with specified headers
    #[doc(hidden)]
    pub fn mock(
        introspected_headers: HashSet<PathBuf>,
        installed_headers: HashSet<PathBuf>,
    ) -> Self {
        Self {
            build_dir: PathBuf::from("/mock/build"),
            data: MesonData::default(),
            introspected_headers,
            installed_headers,
        }
    }

    /// Create MesonIntrospection by finding the build directory and running
    /// `meson introspect --all` Returns None if no build directory is found
    pub fn new(project_root: &Path, config_build_dir: Option<&str>) -> Result<Option<Self>> {
        // Find build directory
        let Some(build_dir) = find_build_dir(project_root, config_build_dir) else {
            return Ok(None);
        };

        // Run meson introspect --all
        let output = Command::new("meson")
            .arg("introspect")
            .arg(&build_dir)
            .arg("--all")
            .output()
            .context("Failed to run meson introspect (is meson installed?)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("meson introspect --all failed: {}", stderr);
        }

        // Parse JSON output
        let stdout = String::from_utf8(output.stdout)
            .context("meson introspect output is not valid UTF-8")?;

        let data: MesonData = serde_json::from_str(&stdout)
            .context("Failed to parse meson introspect JSON output")?;

        // Pre-compute introspected headers
        let introspected_headers = Self::compute_introspected_headers(&data, &build_dir)?;

        // Pre-compute installed headers
        let installed_headers = Self::compute_installed_headers(&data);

        Ok(Some(Self {
            build_dir,
            data,
            introspected_headers,
            installed_headers,
        }))
    }

    /// Compute headers scanned by g-ir-scanner (GObject introspection headers)
    fn compute_introspected_headers(
        data: &MesonData,
        _build_dir: &Path,
    ) -> Result<HashSet<PathBuf>> {
        let mut headers = HashSet::new();

        // Find all targets that produce .gir files
        for target in &data.targets {
            // Check if this target produces a .gir file
            let produces_gir = target.filename.iter().any(|f| f.ends_with(".gir"));
            if !produces_gir {
                continue;
            }

            // Look for --filelist argument in g-ir-scanner command
            for source in &target.target_sources {
                for arg in &source.compiler {
                    if let Some(filelist_path) = arg.strip_prefix("--filelist=") {
                        // Read the filelist and extract .h files
                        let content = std::fs::read_to_string(filelist_path)
                            .context(format!("Failed to read GIR filelist: {}", filelist_path))?;

                        for line in content.lines() {
                            let line = line.trim();
                            if line.ends_with(".h") {
                                headers.insert(PathBuf::from(line));
                            }
                        }
                    }
                }
            }
        }

        Ok(headers)
    }

    /// Compute installed header files (all headers that will be installed)
    fn compute_installed_headers(data: &MesonData) -> HashSet<PathBuf> {
        let mut headers = HashSet::new();
        for source_path in data.installed.keys() {
            if source_path.ends_with(".h") {
                headers.insert(PathBuf::from(source_path));
            }
        }
        headers
    }

    /// Get headers scanned by g-ir-scanner (GObject introspection headers)
    pub fn get_introspected_headers(&self) -> &HashSet<PathBuf> {
        &self.introspected_headers
    }

    /// Get installed header files (all headers that will be installed)
    pub fn get_installed_headers(&self) -> &HashSet<PathBuf> {
        &self.installed_headers
    }
}

/// Compiler information extracted from compile_commands.json
#[derive(Debug, Clone)]
pub struct CompilerInfo {
    /// Compiler executable (e.g., "cc", "gcc", "/usr/bin/clang")
    pub compiler: String,
    /// Compilation flags (-I, -D, etc.)
    pub flags: Vec<String>,
}

/// Entry in compile_commands.json (standard format used by CMake, Meson, etc.)
#[derive(Debug, Deserialize)]
struct CompileCommand {
    /// Build directory
    directory: String,
    /// Full compilation command
    command: String,
    /// Source file path (relative to directory)
    file: String,
}

impl MesonIntrospection {
    /// Load compile_commands.json and build a map from source file →
    /// CompilerInfo
    pub fn load_compiler_map(&self) -> Result<HashMap<PathBuf, CompilerInfo>> {
        let compile_commands_path = self.build_dir.join("compile_commands.json");
        if !compile_commands_path.exists() {
            anyhow::bail!(
                "compile_commands.json not found in {}",
                self.build_dir.display()
            );
        }

        let content = std::fs::read_to_string(&compile_commands_path)
            .context("Failed to read compile_commands.json")?;

        let commands: Vec<CompileCommand> =
            serde_json::from_str(&content).context("Failed to parse compile_commands.json")?;

        let mut map = HashMap::new();

        for cmd in commands {
            let (compiler, flags) = Self::parse_compile_command(&cmd.command);

            // Resolve file path (may be relative to build directory)
            let file_path = if Path::new(&cmd.file).is_absolute() {
                PathBuf::from(&cmd.file)
            } else {
                PathBuf::from(&cmd.directory).join(&cmd.file)
            };

            // Canonicalize to get absolute path
            let file_path = file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.clone());

            map.insert(file_path, CompilerInfo { compiler, flags });
        }

        tracing::debug!("Loaded {} entries from compile_commands.json", map.len());

        Ok(map)
    }

    /// Parse a compilation command string to extract compiler and flags
    fn parse_compile_command(command: &str) -> (String, Vec<String>) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return ("cc".to_string(), Vec::new());
        }

        // First part is compiler (skip ccache if present)
        let (compiler_idx, compiler) = if parts[0] == "ccache" && parts.len() > 1 {
            (1, parts[1].to_string())
        } else {
            (0, parts[0].to_string())
        };

        // Extract flags (skip compiler, -o, -c, -MF, -MQ, -MD and their arguments, and
        // the input file)
        let mut flags = Vec::new();
        let mut i = compiler_idx + 1;

        while i < parts.len() {
            let flag = parts[i];

            // Skip output-related flags and their arguments
            if flag == "-o" || flag == "-MF" || flag == "-MQ" {
                i += 2; // Skip flag and its argument
                continue;
            }

            // Skip standalone flags
            if flag == "-c" || flag == "-MD" {
                i += 1;
                continue;
            }

            // Skip the input file (last .c file)
            if flag.ends_with(".c") && i == parts.len() - 1 {
                break;
            }

            flags.push(flag.to_string());
            i += 1;
        }

        (compiler, flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_compile_command ---

    #[test]
    fn parse_simple_command() {
        let (compiler, flags) =
            MesonIntrospection::parse_compile_command("cc -I/usr/include -DFOO=1 -c main.c");
        assert_eq!(compiler, "cc");
        assert_eq!(flags, vec!["-I/usr/include", "-DFOO=1"]);
    }

    #[test]
    fn parse_command_with_ccache() {
        let (compiler, flags) =
            MesonIntrospection::parse_compile_command("ccache gcc -Wall -c foo.c");
        assert_eq!(compiler, "gcc");
        assert_eq!(flags, vec!["-Wall"]);
    }

    #[test]
    fn parse_command_skips_output_flags() {
        let (compiler, flags) = MesonIntrospection::parse_compile_command(
            "gcc -I/inc -o output.o -MF deps.d -MQ target -MD -c src.c",
        );
        assert_eq!(compiler, "gcc");
        assert_eq!(flags, vec!["-I/inc"]);
    }

    #[test]
    fn parse_empty_command() {
        let (compiler, flags) = MesonIntrospection::parse_compile_command("");
        assert_eq!(compiler, "cc");
        assert!(flags.is_empty());
    }

    #[test]
    fn parse_command_with_multiple_includes() {
        let (compiler, flags) = MesonIntrospection::parse_compile_command(
            "/usr/bin/clang -I/a -I/b -DBAR -Wall -Werror -c widget.c",
        );
        assert_eq!(compiler, "/usr/bin/clang");
        assert!(flags.contains(&"-I/a".to_string()));
        assert!(flags.contains(&"-I/b".to_string()));
        assert!(flags.contains(&"-DBAR".to_string()));
        assert!(flags.contains(&"-Wall".to_string()));
        assert!(flags.contains(&"-Werror".to_string()));
    }

    // --- find_build_dir ---

    #[test]
    fn find_build_dir_none_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_build_dir(dir.path(), None).is_none());
    }

    #[test]
    fn find_build_dir_user_specified() {
        let dir = tempfile::tempdir().unwrap();
        let build = dir.path().join("mybuild/meson-info");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("meson-info.json"), "{}").unwrap();
        let result = find_build_dir(dir.path(), Some("mybuild"));
        assert_eq!(result, Some(dir.path().join("mybuild")));
    }

    #[test]
    fn find_build_dir_common_names() {
        for name in &["build", "builddir", "_build"] {
            let dir = tempfile::tempdir().unwrap();
            let info = dir.path().join(name).join("meson-info");
            std::fs::create_dir_all(&info).unwrap();
            std::fs::write(info.join("meson-info.json"), "{}").unwrap();
            let result = find_build_dir(dir.path(), None);
            assert_eq!(result, Some(dir.path().join(name)));
        }
    }

    #[test]
    fn find_build_dir_fallback_search() {
        let dir = tempfile::tempdir().unwrap();
        let info = dir.path().join("custom-build/meson-info");
        std::fs::create_dir_all(&info).unwrap();
        std::fs::write(info.join("meson-info.json"), "{}").unwrap();
        let result = find_build_dir(dir.path(), None);
        assert!(result.is_some());
    }

    // --- is_build_dir ---

    #[test]
    fn is_build_dir_true() {
        let dir = tempfile::tempdir().unwrap();
        let info = dir.path().join("meson-info");
        std::fs::create_dir_all(&info).unwrap();
        std::fs::write(info.join("meson-info.json"), "{}").unwrap();
        assert!(is_build_dir(dir.path()));
    }

    #[test]
    fn is_build_dir_false() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_build_dir(dir.path()));
    }

    // --- MesonIntrospection::mock ---

    #[test]
    fn mock_returns_specified_headers() {
        let introspected: HashSet<PathBuf> = [PathBuf::from("/src/foo.h")].into_iter().collect();
        let installed: HashSet<PathBuf> = [PathBuf::from("/src/bar.h")].into_iter().collect();

        let meson = MesonIntrospection::mock(introspected.clone(), installed.clone());
        assert_eq!(meson.get_introspected_headers(), &introspected);
        assert_eq!(meson.get_installed_headers(), &installed);
    }

    // --- compute_installed_headers ---

    #[test]
    fn compute_installed_headers_filters_non_headers() {
        let mut installed = HashMap::new();
        installed.insert(
            "/src/widget.h".to_string(),
            "/usr/include/widget.h".to_string(),
        );
        installed.insert("/src/widget.c".to_string(), "/usr/lib/widget.o".to_string());
        installed.insert("/src/util.h".to_string(), "/usr/include/util.h".to_string());

        let data = MesonData {
            installed,
            ..MesonData::default()
        };

        let headers = MesonIntrospection::compute_installed_headers(&data);
        assert_eq!(headers.len(), 2);
        assert!(headers.contains(&PathBuf::from("/src/widget.h")));
        assert!(headers.contains(&PathBuf::from("/src/util.h")));
    }
}
