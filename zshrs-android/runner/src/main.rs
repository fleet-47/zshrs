//! Apply Android and dependency patches to the current checkout.
use semver::{Version, VersionReq};
use serde::Deserialize;
use std::{
    collections::HashSet,
    env, fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode, Output},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Series {
    #[serde(default)]
    patch: Vec<Patch>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    id: String,
    group: String,
    file: PathBuf,
    dependency: Option<String>,
    version: Option<String>,
    #[serde(default)]
    manifests: Vec<PathBuf>,
    hold: Option<String>,
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Tools {
    jj: String,
    patch: String,
}
impl Default for Tools {
    fn default() -> Self {
        Self {
            jj: "jj".into(),
            patch: "patch".into(),
        }
    }
}
struct Options {
    repo: PathBuf,
    ci: bool,
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("zshrs-android: {e}");
            ExitCode::FAILURE
        }
    }
}
fn options() -> Result<Options> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("apply") {
        return Err("usage: zshrs-android apply [--repo PATH] [--ci]".into());
    }
    let mut repo = env::current_dir()?;
    let mut ci = false;
    while let Some(flag) = args.next() {
        if flag == "--ci" {
            ci = true;
            continue;
        }
        if flag != "--repo" {
            return Err(format!("unknown option {flag}").into());
        }
        repo = args.next().ok_or("--repo requires a path")?.into();
    }
    Ok(Options {
        repo: repo.canonicalize()?,
        ci,
    })
}
fn checked(command: &mut Command) -> Result<Output> {
    let output = command.output()?;
    if !output.status.success() {
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        return Err(format!(
            "{} failed: {}",
            command.get_program().to_string_lossy(),
            output.status
        )
        .into());
    }
    Ok(output)
}
fn streamed(command: &mut Command) -> Result<()> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!(
            "{} failed: {status}",
            command.get_program().to_string_lossy()
        )
        .into());
    }
    Ok(())
}
fn jj(tools: &Tools, repo: &Path, args: &[&str]) -> Result<String> {
    let output = checked(
        Command::new(&tools.jj)
            .current_dir(repo)
            .args(["--no-pager", "--color", "never"])
            .args(args),
    )?;
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
fn identity(tools: &Tools, repo: &Path, revision: &str) -> Result<String> {
    let value = jj(
        tools,
        repo,
        &[
            "log",
            "-r",
            revision,
            "--no-graph",
            "-T",
            "commit_id ++ \"\\n\"",
        ],
    )?;
    if value.len() != 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("revision must resolve to exactly one commit".into());
    }
    Ok(value)
}
fn relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || !path.components().all(|p| matches!(p, Component::Normal(_)))
        || path
            .components()
            .any(|p| p.as_os_str() == ".jj" || p.as_os_str() == ".git")
    {
        return Err(format!("invalid relative path: {}", path.display()).into());
    }
    Ok(())
}
fn safe(root: &Path, path: &Path) -> Result<PathBuf> {
    relative(path)?;
    safe_absolute(&root.join(path))
}
fn safe_absolute(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err("path validation requires an absolute path".into());
    }
    let mut full = PathBuf::new();
    for part in path.components() {
        full.push(part);
        match fs::symlink_metadata(&full) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("symlink refused: {}", full.display()).into());
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(full)
}
fn validate(series: &Series, patches: &Path) -> Result<()> {
    let mut ids = HashSet::new();
    let mut files = HashSet::new();
    for p in &series.patch {
        if p.id.is_empty() || !ids.insert(&p.id) || !files.insert(&p.file) {
            return Err("patch IDs/files must be unique".into());
        }
        if p.hold.as_ref().is_some_and(|reason| reason.trim().is_empty()) {
            return Err("held patch requires a nonempty reason".into());
        }
        if !["android", "upgrades"].contains(&p.group.as_str())
            || p.file.components().next().map(Component::as_os_str) != Some(p.group.as_ref())
        {
            return Err("patch group must match android/ or upgrades/ path".into());
        }
        if p.group == "upgrades" {
            if p.dependency.as_deref().is_none_or(str::is_empty) || p.manifests.is_empty() {
                return Err("upgrade requires dependency and manifests".into());
            }
            Version::parse(p.version.as_deref().ok_or("upgrade requires version")?)?;
        } else if p.dependency.is_some() || p.version.is_some() || !p.manifests.is_empty() {
            return Err("Android patches cannot have automatic retirement conditions".into());
        }
        for m in &p.manifests {
            relative(m)?;
        }
        if !safe(patches, &p.file)?.is_file() {
            return Err(format!("missing patch {}", p.id).into());
        }
    }
    Ok(())
}
fn requirements(value: &toml::Value, name: &str, result: &mut Vec<String>) -> Result<()> {
    let Some(table) = value.as_table() else {
        return Ok(());
    };
    for (key, child) in table {
        if ["dependencies", "dev-dependencies", "build-dependencies"].contains(&key.as_str()) {
            if let Some(deps) = child.as_table() {
                for (alias, declaration) in deps {
                    let package = declaration
                        .get("package")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(alias);
                    if package == name {
                        let req = declaration
                            .as_str()
                            .or_else(|| declaration.get("version").and_then(toml::Value::as_str));
                        result.push(req.ok_or("retirement needs explicit dependency version; inherited/path dependency requires manual review")?.to_owned());
                    }
                }
            }
        } else {
            requirements(child, name, result)?;
        }
    }
    Ok(())
}
fn satisfied(p: &Patch, tools: &Tools, repo: &Path, baseline: &str) -> Result<bool> {
    if p.group != "upgrades" || p.hold.is_some() {
        return Ok(false);
    }
    let name = p.dependency.as_deref().ok_or("missing dependency")?;
    let target = Version::parse(p.version.as_deref().ok_or("missing version")?)?;
    let lock: toml::Value = toml::from_str(&jj(
        tools,
        repo,
        &["file", "show", "-r", baseline, "Cargo.lock"],
    )?)?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("lock has no packages")?;
    for manifest in &p.manifests {
        let value: toml::Value = toml::from_str(&jj(
            tools,
            repo,
            &[
                "file",
                "show",
                "-r",
                baseline,
                manifest.to_str().ok_or("non-UTF8 manifest path")?,
            ],
        )?)?;
        let mut reqs = Vec::new();
        requirements(&value, name, &mut reqs)?;
        if reqs.is_empty() {
            return Ok(false);
        }
        let owner = value
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
            .ok_or("manifest missing package name")?;
        let owners: Vec<_> = packages
            .iter()
            .filter(|v| {
                v.get("name").and_then(toml::Value::as_str) == Some(owner)
                    && v.get("source").is_none()
            })
            .collect();
        if owners.len() != 1 {
            return Err("ambiguous manifest owner in baseline lockfile".into());
        }
        let edges = owners[0]
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .ok_or("lock owner missing dependency edges")?;
        for req in reqs {
            let req = VersionReq::parse(&req)?;
            let mut resolved = Vec::new();
            for edge in edges {
                let edge = edge.as_str().ok_or("invalid lock edge")?;
                let mut parts = edge.split_whitespace();
                if parts.next() != Some(name) {
                    continue;
                }
                let pinned = parts.next();
                for package in packages {
                    if package.get("name").and_then(toml::Value::as_str) != Some(name) {
                        continue;
                    }
                    let version = package
                        .get("version")
                        .and_then(toml::Value::as_str)
                        .ok_or("package missing version")?;
                    if pinned.is_none_or(|pin| pin == version) {
                        let version = Version::parse(version)?;
                        if req.matches(&version) {
                            resolved.push(version);
                        }
                    }
                }
            }
            if resolved.len() != 1 {
                return Ok(false);
            }
            if resolved[0] < target {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn patch_command(tools: &Tools, checkout: &Path, file: &Path, reverse: bool, dry: bool) -> Command {
    let mut command = Command::new(&tools.patch);
    command
        .current_dir(checkout)
        .args(["--force", "--fuzz=0", "-p1", "--no-backup-if-mismatch"]);
    if reverse {
        command.arg("--reverse");
    }
    if dry {
        command.arg("--dry-run");
    }
    command.arg("--input").arg(file);
    command
}
fn apply(p: &Patch, tools: &Tools, checkout: &Path, patches: &Path) -> Result<()> {
    if let Some(reason) = &p.hold {
        println!("held: {}: {reason}", p.id);
        return Ok(());
    }
    let file = safe(patches, &p.file)?;
    let text = fs::read_to_string(&file)?;
    let mut headers = 0;
    for line in text.lines() {
        if let Some(header) = line
            .strip_prefix("--- ")
            .or_else(|| line.strip_prefix("+++ "))
        {
            let path = header.split('\t').next().ok_or("missing patch path")?;
            if path == "/dev/null" {
                continue;
            }
            let (prefix, stripped) = path
                .split_once('/')
                .ok_or("expected a/ or b/ patch paths")?;
            if !["a", "b"].contains(&prefix) {
                return Err("expected a/ or b/ patch paths".into());
            }
            safe(checkout, Path::new(stripped))?;
            headers += 1;
        }
    }
    if headers == 0 {
        return Err("patch has no valid file headers".into());
    }
    println!("{}: {}", p.group, p.id);
    let dry = patch_command(tools, checkout, &file, false, true).output()?;
    if !dry.status.success() {
        if patch_command(tools, checkout, &file, true, true)
            .output()?
            .status
            .success()
        {
            println!("  exact reverse check confirms already satisfied; keeping patch");
            return Ok(());
        }
        io::stdout().write_all(&dry.stdout)?;
        io::stderr().write_all(&dry.stderr)?;
        return Err(format!("{} no longer applies exactly", p.id).into());
    }
    streamed(&mut patch_command(tools, checkout, &file, false, false))
}
fn verify_holds(series: &Series, tools: &Tools, checkout: &Path, patches: &Path) -> Result<()> {
    for p in &series.patch {
        if p.hold.is_some() {
            let file = safe(patches, &p.file)?;
            let dry = patch_command(tools, checkout, &file, false, true).output()?;
            if !dry.status.success() {
                io::stdout().write_all(&dry.stdout)?;
                io::stderr().write_all(&dry.stderr)?;
                return Err(format!(
                    "held patch {} does not match its upstream source; use a clean upstream checkout or review the hold before applying any patches",
                    p.id
                ).into());
            }
        }
    }
    Ok(())
}
fn run() -> Result<()> {
    let options = options()?;
    let directory = safe(&options.repo, Path::new("zshrs-android"))?;
    let tools: Tools = toml::from_str(&fs::read_to_string(directory.join("tools.toml"))?)?;
    let series_path = safe(&directory, Path::new("series.toml"))?;
    let original = fs::read_to_string(&series_path)?;
    let series: Series = toml::from_str(&original)?;
    let patches = safe(&directory, Path::new("patches"))?;
    validate(&series, &patches)?;
    // A hold must not silently leave a previously applied upgrade in place.
    // Check every held patch before making any source changes.
    verify_holds(&series, &tools, &options.repo, &patches)?;
    // CI tests enabled patches in its disposable checkout. It neither needs
    // a jj workspace nor makes retirement decisions against a moving main.
    if options.ci {
        for p in &series.patch {
            apply(p, &tools, &options.repo, &patches)?;
        }
        return Ok(());
    }
    let baseline = identity(&tools, &options.repo, "main")?;
    if identity(&tools, &options.repo, "main & ::@")? != baseline {
        return Err("current checkout must descend from main".into());
    }
    // Read the unpatched main commit, never the overlay, for retirement decisions.
    let mut candidates = HashSet::new();
    for p in &series.patch {
        if satisfied(p, &tools, &options.repo, &baseline)? {
            candidates.insert(p.id.clone());
        }
    }
    for p in &series.patch {
        if candidates.contains(&p.id) {
            println!("upgrades: {} already supplied by main", p.id);
        } else {
            apply(p, &tools, &options.repo, &patches)?;
        }
    }
    if !candidates.is_empty() {
        if identity(&tools, &options.repo, "main")? != baseline
            || fs::read_to_string(&series_path)? != original
        {
            return Err("main or series changed; refusing retirement".into());
        }
        let mut document: toml::Value = toml::from_str(&original)?;
        document
            .get_mut("patch")
            .and_then(toml::Value::as_array_mut)
            .ok_or("missing patches")?
            .retain(|entry| {
                !entry
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .is_some_and(|id| candidates.contains(id))
            });
        let replacement = toml::to_string_pretty(&document)?;
        fs::write(series_path, replacement)?;
        for p in &series.patch {
            if candidates.contains(&p.id) {
                fs::remove_file(safe(&patches, &p.file)?)?;
                println!("Retired {} (recoverable through jj)", p.id);
            }
        }
    }
    Ok(())
}
