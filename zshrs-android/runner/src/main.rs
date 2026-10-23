//! Apply Android and dependency patches to the current checkout.
use semver::Version;
use serde::Deserialize;
use std::{
    collections::HashSet,
    env, fs,
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
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
    patch: String,
}
impl Default for Tools {
    fn default() -> Self {
        Self {
            patch: "patch".into(),
        }
    }
}
struct Options {
    repo: PathBuf,
    overlay: PathBuf,
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
        return Err("usage: zshrs-android apply [--repo PATH] [--overlay PATH]".into());
    }
    let mut repo = env::current_dir()?;
    let mut overlay: Option<PathBuf> = None;
    while let Some(flag) = args.next() {
        if flag == "--repo" {
            repo = args.next().ok_or("--repo requires a path")?.into();
        } else if flag == "--overlay" {
            overlay = Some(args.next().ok_or("--overlay requires a path")?.into());
        } else {
            return Err(format!("unknown option {flag}").into());
        }
    }
    let repo = repo.canonicalize()?;
    let overlay = match overlay {
        Some(p) => p.canonicalize()?,
        None => repo.join("zshrs-android"),
    };
    Ok(Options { repo, overlay })
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
        if p.hold
            .as_ref()
            .is_some_and(|reason| reason.trim().is_empty())
        {
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
            return Err("Android patches cannot have dependency upgrade metadata".into());
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
fn run() -> Result<()> {
    let options = options()?;
    let directory = options.overlay;
    let tools: Tools = toml::from_str(&fs::read_to_string(directory.join("tools.toml"))?)?;
    let series_path = safe(&directory, Path::new("series.toml"))?;
    let series: Series = toml::from_str(&fs::read_to_string(series_path)?)?;
    let patches = safe(&directory, Path::new("patches"))?;
    validate(&series, &patches)?;
    for p in &series.patch {
        apply(p, &tools, &options.repo, &patches)?;
    }
    Ok(())
}
