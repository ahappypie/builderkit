use std::{
    env,
    error::Error,
    fmt::{self, Write as _},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

mod release;

#[derive(Parser, Debug)]
#[command(version, about = "Builder Kit host setup and update tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Plan and set up a Builder Kit environment.
    Setup(SetupArgs),

    /// Check for or apply a Builder Kit update.
    Update(UpdateArgs),

    /// Stop Builder Kit from managing this environment.
    Eject(EjectArgs),
}

#[derive(clap::Args, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "these fields mirror CLI flags"
)]
struct SetupArgs {
    /// Print the setup plan without changing the machine.
    #[arg(long)]
    dry_run: bool,

    /// Accept Builder Kit prompts. Does not bypass upstream installer prompts.
    #[arg(long)]
    yes: bool,

    /// Exact release to install (YYYY.MM.PATCH). No latest resolution.
    #[arg(long, value_parser = release::validate_version)]
    version: String,

    /// Download setup files without installing Nix or changing configuration.
    #[arg(long, conflicts_with_all = ["dry_run", "no_activate"])]
    download_only: bool,

    /// Write and validate configuration but do not run nix-darwin switch.
    #[arg(long)]
    no_activate: bool,
}

#[derive(clap::Args, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "these fields mirror CLI flags"
)]
struct UpdateArgs {
    /// Check for updates without changing the machine.
    #[arg(long)]
    check: bool,

    /// Accept Builder Kit prompts.
    #[arg(long)]
    yes: bool,

    /// Exact release to install (YYYY.MM.PATCH). No latest resolution.
    #[arg(long, value_parser = release::validate_version)]
    version: String,

    /// Print the update plan without downloads or changes.
    #[arg(long, conflicts_with = "check")]
    dry_run: bool,

    /// Write and validate configuration but do not run nix-darwin switch.
    #[arg(long)]
    no_activate: bool,

    /// Do not run nix-store --gc after a successful activation.
    #[arg(long)]
    no_gc: bool,
}

#[derive(clap::Args, Debug)]
struct EjectArgs {
    /// Accept Builder Kit prompts.
    #[arg(long)]
    yes: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostPlatform {
    Windows,
    Macos,
    Unsupported,
}

impl HostPlatform {
    const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Unsupported
        }
    }
}

impl fmt::Display for HostPlatform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows => formatter.write_str("Windows"),
            Self::Macos => formatter.write_str("macOS"),
            Self::Unsupported => formatter.write_str("unsupported host"),
        }
    }
}

#[derive(Debug)]
struct Plan {
    title: &'static str,
    platform: HostPlatform,
    lines: Vec<String>,
    confirmation: Confirmation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Confirmation {
    Standard,
    Eject,
    NotRequired,
}

impl Plan {
    fn render(&self) -> String {
        let mut output = String::new();
        output.push_str(self.title);
        output.push('\n');
        output.push_str("────────────────────────────────────────\n");
        let _ = write!(output, "Platform: {}\n\n", self.platform);
        output.push_str("This command will:\n");

        for (index, line) in self.lines.iter().enumerate() {
            let _ = writeln!(output, "  {}. {line}", index + 1);
        }

        match self.confirmation {
            Confirmation::Standard => output.push_str("\nProceed? [y/N]\n"),
            Confirmation::Eject => output.push_str("\nType EJECT to continue.\n"),
            Confirmation::NotRequired => {}
        }

        output
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("kit: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let platform = HostPlatform::current();

    match cli.command {
        Command::Setup(args) => setup(platform, &args),
        Command::Update(args) => update(platform, &args),
        Command::Eject(args) => eject(platform, &args),
    }
}

fn setup(platform: HostPlatform, args: &SetupArgs) -> Result<(), Box<dyn Error>> {
    match platform {
        HostPlatform::Macos => setup_macos(args),
        HostPlatform::Windows => setup_windows(args),
        HostPlatform::Unsupported => {
            let plan = setup_plan(platform, &args.version, args.yes || args.dry_run);
            print!("{}", plan.render());
            Err("Builder Kit setup is supported only on Windows and macOS".into())
        }
    }
}

fn setup_windows(args: &SetupArgs) -> Result<(), Box<dyn Error>> {
    let plan = setup_plan(
        HostPlatform::Windows,
        &args.version,
        args.yes || args.dry_run,
    );
    print!("{}", plan.render());

    if args.dry_run {
        Ok(())
    } else {
        Err("Windows setup execution is not implemented yet".into())
    }
}

fn setup_macos(args: &SetupArgs) -> Result<(), Box<dyn Error>> {
    ensure_macos_architecture(env::consts::ARCH)?;
    let inspection = MacosSetupInspection::inspect()?;
    let plan = macos_setup_plan(&inspection, args);
    print!("{}", plan.render());

    if args.dry_run {
        return Ok(());
    }

    inspection.ensure_supported()?;

    if !args.yes && !confirm_standard()? {
        return Err("setup cancelled".into());
    }

    execute_macos_setup(&inspection, args)
}

fn update(platform: HostPlatform, args: &UpdateArgs) -> Result<(), Box<dyn Error>> {
    match platform {
        HostPlatform::Macos => update_macos(args),
        HostPlatform::Windows => {
            let plan = update_plan(
                platform,
                &args.version,
                args.check,
                args.yes || args.check || args.dry_run,
            );
            print!("{}", plan.render());
            if args.dry_run {
                Ok(())
            } else {
                Err("Windows update execution is not implemented yet".into())
            }
        }
        HostPlatform::Unsupported => {
            let plan = update_plan(
                platform,
                &args.version,
                args.check,
                args.yes || args.check || args.dry_run,
            );
            print!("{}", plan.render());
            Err("Builder Kit update is supported only on Windows and macOS".into())
        }
    }
}

fn update_macos(args: &UpdateArgs) -> Result<(), Box<dyn Error>> {
    ensure_macos_architecture(env::consts::ARCH)?;
    let inspection = MacosSetupInspection::inspect()?;
    let installed = InstallationState::load(&inspection.kit_root)?
        .ok_or("Builder Kit is not installed; expected ~/.kit/state/installation.json")?;
    let update = ResolvedUpdate {
        version: args.version.clone(),
    };
    let plan = macos_update_plan(&inspection, &installed, &update, args);
    print!("{}", plan.render());
    if args.dry_run {
        return Ok(());
    }
    if installed.version == args.version {
        println!("Builder Kit {} is already installed.", args.version);
        return Ok(());
    }
    if !calver_gt(&args.version, &installed.version) {
        return Err("requested release is not newer than the installed version; preview downgrades are not supported".into());
    }
    if args.check {
        release::check_available(&args.version)?;
        println!("Builder Kit {} is available.", args.version);
        return Ok(());
    }
    if !args.yes && !confirm_standard()? {
        return Err("update cancelled".into());
    }
    apply_macos_update(&inspection, args, &update)
}

fn eject(platform: HostPlatform, args: &EjectArgs) -> Result<(), Box<dyn Error>> {
    let plan = eject_plan(platform, args.yes);
    print!("{}", plan.render());
    Err("eject execution is not implemented yet".into())
}

#[derive(Debug)]
struct MacosSetupInspection {
    executable_dir: PathBuf,
    kit_root: PathBuf,
    installer_dir: PathBuf,
    config_dir: PathBuf,
    local_bin: PathBuf,
    nix_state: NixState,
    config_state: ConfigState,
    current_user: String,
    home_dir: PathBuf,
}

impl MacosSetupInspection {
    fn inspect() -> Result<Self, Box<dyn Error>> {
        let executable_dir = canonical_executable_dir()?;
        let home_dir = home_dir()?;
        let kit_root = home_dir.join(".kit");
        let installer_dir = kit_root.join("installer");
        let config_dir = kit_root.join("nix");
        let local_bin = home_dir.join(".local").join("bin");
        let current_user = current_username()?;
        let nix_state = detect_nix_state();
        let config_state = detect_config_state(&config_dir);

        Ok(Self {
            executable_dir,
            kit_root,
            installer_dir,
            config_dir,
            local_bin,
            nix_state,
            config_state,
            current_user,
            home_dir,
        })
    }

    fn ensure_supported(&self) -> Result<(), Box<dyn Error>> {
        match self.config_state {
            ConfigState::Absent => {}
            ConfigState::Managed => {
                return Err("Builder Kit already appears to be set up on this machine".into());
            }
            ConfigState::Unrelated | ConfigState::Incomplete => {
                return Err(format!(
                    "unsupported existing Builder Kit configuration state: {}",
                    self.config_state
                )
                .into());
            }
        }

        match self.nix_state {
            NixState::Absent | NixState::Present => Ok(()),
            NixState::Unhealthy => Err("nix is present but did not respond successfully".into()),
        }
    }
}

#[derive(Debug)]
struct ResolvedUpdate {
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationState {
    schema_version: u32,
    platform: String,
    version: String,
    config_dir: PathBuf,
}

impl InstallationState {
    fn load(kit_root: &Path) -> Result<Option<Self>, Box<dyn Error>> {
        let path = Self::path(kit_root);
        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&contents)?))
    }

    fn save(&self, kit_root: &Path) -> Result<(), Box<dyn Error>> {
        let path = Self::path(kit_root);
        let parent = path
            .parent()
            .ok_or("installation state path has no parent directory")?;
        std::fs::create_dir_all(parent)?;
        let json = serde_json::to_vec_pretty(self)?;
        write_atomically(&path, &json)
    }

    fn path(kit_root: &Path) -> PathBuf {
        kit_root.join("state").join("installation.json")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NixState {
    Absent,
    Present,
    Unhealthy,
}

impl fmt::Display for NixState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => formatter.write_str("not installed"),
            Self::Present => formatter.write_str("present and responding"),
            Self::Unhealthy => formatter.write_str("present but unhealthy"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfigState {
    Absent,
    Managed,
    Incomplete,
    Unrelated,
}

impl fmt::Display for ConfigState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => formatter.write_str("absent"),
            Self::Managed => formatter.write_str("managed Builder Kit configuration"),
            Self::Incomplete => formatter.write_str("incomplete Builder Kit configuration"),
            Self::Unrelated => formatter.write_str("unrelated existing files"),
        }
    }
}

fn canonical_executable_dir() -> Result<PathBuf, Box<dyn Error>> {
    let executable = env::current_exe()?;
    let executable = executable.canonicalize()?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "executable path has no parent directory".into())
}

fn home_dir() -> Result<PathBuf, Box<dyn Error>> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| "HOME is not set".into())
}

fn current_username() -> Result<String, Box<dyn Error>> {
    env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .map_err(|_| "USER and LOGNAME are not set".into())
        .and_then(|username| {
            if username.is_empty() {
                Err("current username is empty".into())
            } else {
                Ok(username)
            }
        })
}

fn detect_nix_state() -> NixState {
    match ProcessCommand::new("nix").arg("--version").output() {
        Ok(output) if output.status.success() => NixState::Present,
        Err(error) if error.kind() == io::ErrorKind::NotFound => NixState::Absent,
        Ok(_) | Err(_) => NixState::Unhealthy,
    }
}

fn detect_config_state(config_dir: &Path) -> ConfigState {
    if !config_dir.exists() {
        return ConfigState::Absent;
    }

    let flake = config_dir.join("flake.nix");
    let lock = config_dir.join("flake.lock");

    match (flake.exists(), lock.exists()) {
        (true, true) => {
            let Ok(contents) = std::fs::read_to_string(flake) else {
                return ConfigState::Unrelated;
            };

            if contents.contains("BEGIN BUILDER-KIT USER SETTINGS")
                && contents.contains("BEGIN BUILDER-KIT MANAGED IMPLEMENTATION")
            {
                ConfigState::Managed
            } else {
                ConfigState::Unrelated
            }
        }
        (false, false) => ConfigState::Absent,
        _ => ConfigState::Incomplete,
    }
}

fn confirm_standard() -> Result<bool, Box<dyn Error>> {
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(matches!(line.trim(), "y" | "Y" | "yes" | "YES" | "Yes"))
}

fn ensure_macos_architecture(architecture: &str) -> Result<(), Box<dyn Error>> {
    if architecture != "aarch64" {
        return Err("this preview supports only native Apple Silicon (arm64) macOS".into());
    }
    Ok(())
}

fn execute_macos_setup(
    inspection: &MacosSetupInspection,
    args: &SetupArgs,
) -> Result<(), Box<dyn Error>> {
    let payload = release::prepare_payload(&inspection.kit_root, &args.version)?;
    let installer = if inspection.nix_state == NixState::Absent {
        Some(release::prepare_installer(&inspection.installer_dir)?)
    } else {
        None
    };

    if args.download_only {
        println!(
            "Downloads completed for {}. Nix was not installed; no configuration, command, or installation state was written.",
            args.version
        );
        return Ok(());
    }

    println!(
        "Ready to install Builder Kit {}. System changes may request administrator authentication.",
        args.version
    );
    if !args.yes {
        print!("Install and configure this release? [y/N] ");
        if !confirm_standard()? {
            return Err("setup cancelled; downloads have been retained".into());
        }
    }

    let settings = prompt_user_settings(&inspection.current_user, &inspection.home_dir)?;
    let flake = replace_user_settings_block(&payload.flake, &settings.to_nix_block())?;

    if inspection.nix_state == NixState::Absent {
        let installer = installer.as_ref().ok_or("missing Nix installer")?;
        prepare_local_executable(installer)?;
        run_command(
            ProcessCommand::new("sudo")
                .arg(installer)
                .arg("install")
                .arg("--enable-flakes"),
        )?;
    }

    let nix = resolve_nix_executable()?;

    install_self_to_local_bin(&inspection.local_bin)?;

    std::fs::create_dir_all(&inspection.installer_dir)?;
    std::fs::create_dir_all(&inspection.config_dir)?;
    write_atomically(&inspection.config_dir.join("flake.nix"), flake.as_bytes())?;
    write_atomically(
        &inspection.config_dir.join("flake.lock"),
        payload.lock.as_bytes(),
    )?;

    run_command(
        ProcessCommand::new(&nix)
            .arg("flake")
            .arg("check")
            .arg(format!("path:{}", inspection.config_dir.display())),
    )?;

    if args.no_activate {
        println!(
            "Configuration validated but not activated; installation has not been marked complete."
        );
        return Ok(());
    }
    run_darwin_activation(&nix, &inspection.config_dir)?;

    InstallationState {
        schema_version: 1,
        platform: "darwin".to_owned(),
        version: args.version.clone(),
        config_dir: inspection.config_dir.clone(),
    }
    .save(&inspection.kit_root)?;

    println!(
        "Builder Kit macOS configuration written to {}",
        inspection.config_dir.display()
    );
    Ok(())
}

#[derive(Debug)]
struct MacosPayload {
    flake: String,
    lock: String,
}

impl MacosPayload {
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        let settings = extract_user_settings_block(&self.flake)?;
        replace_user_settings_block(&self.flake, &settings)?;
        if !self
            .flake
            .contains("# BEGIN BUILDER-KIT MANAGED IMPLEMENTATION")
        {
            return Err(
                "configuration is missing the Builder Kit managed implementation marker".into(),
            );
        }
        let _: serde_json::Value = serde_json::from_str(&self.lock)?;
        Ok(())
    }
}

#[derive(Debug)]
struct UserSettings {
    username: String,
    home_directory: PathBuf,
    full_name: String,
    email: String,
}

impl UserSettings {
    fn to_nix_block(&self) -> String {
        format!(
            "# BEGIN BUILDER-KIT USER SETTINGS\n      builderKit = {{\n        settingsSchemaVersion = 1;\n        hostName = \"builderkit-mac\";\n\n        user = {{\n          username = \"{}\";\n          homeDirectory = \"{}\";\n          fullName = \"{}\";\n          email = \"{}\";\n        }};\n\n        git.defaultBranch = \"main\";\n      }};\n      # END BUILDER-KIT USER SETTINGS",
            escape_nix_string(&self.username),
            escape_nix_string(&self.home_directory.display().to_string()),
            escape_nix_string(&self.full_name),
            escape_nix_string(&self.email),
        )
    }
}

fn prompt_user_settings(
    username: &str,
    home_directory: &Path,
) -> Result<UserSettings, Box<dyn Error>> {
    let full_name = prompt_required("Full name")?;
    let email = prompt_required("Git email")?;

    Ok(UserSettings {
        username: username.to_owned(),
        home_directory: home_directory.to_path_buf(),
        full_name,
        email,
    })
}

fn prompt_required(label: &str) -> Result<String, Box<dyn Error>> {
    loop {
        print!("{label}: ");
        io::stdout().flush()?;

        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            return Err(format!("input ended while reading {label}").into());
        }
        let value = line.trim().to_owned();

        if !value.is_empty() {
            return Ok(value);
        }

        eprintln!("{label} is required.");
    }
}

fn replace_user_settings_block(flake: &str, replacement: &str) -> Result<String, Box<dyn Error>> {
    const BEGIN: &str = "# BEGIN BUILDER-KIT USER SETTINGS";
    const END: &str = "# END BUILDER-KIT USER SETTINGS";

    let begin = flake
        .find(BEGIN)
        .ok_or("missing Builder Kit user settings begin marker")?;
    let end_marker_start = flake
        .find(END)
        .ok_or("missing Builder Kit user settings end marker")?;
    if end_marker_start < begin
        || flake.matches(BEGIN).count() != 1
        || flake.matches(END).count() != 1
    {
        return Err("invalid or duplicate Builder Kit user settings markers".into());
    }
    let end = end_marker_start + END.len();

    let mut output = String::with_capacity(flake.len() + replacement.len());
    output.push_str(&flake[..begin]);
    output.push_str(replacement);
    output.push_str(&flake[end..]);
    Ok(output)
}

fn escape_nix_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${")
}

fn install_self_to_local_bin(local_bin: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(local_bin)?;

    let source = env::current_exe()?.canonicalize()?;
    let destination = local_bin.join("kit");

    if source != destination {
        std::fs::copy(&source, &destination)?;
    }

    prepare_local_executable(&destination)?;

    Ok(())
}

fn prepare_local_executable(path: &Path) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        return Err(format!("executable does not exist: {}", path.display()).into());
    }

    #[cfg(unix)]
    {
        let metadata = std::fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        std::fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

fn resolve_nix_executable() -> Result<PathBuf, Box<dyn Error>> {
    if ProcessCommand::new("nix")
        .arg("--version")
        .status()
        .is_ok_and(|status| status.success())
    {
        return Ok(PathBuf::from("nix"));
    }

    let profile_nix = PathBuf::from("/nix/var/nix/profiles/default/bin/nix");
    if profile_nix.exists() {
        return Ok(profile_nix);
    }

    Err("nix was installed, but no nix executable was found on PATH or at /nix/var/nix/profiles/default/bin/nix; open a new shell or source /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh".into())
}

fn run_nix_store_gc(nix: &Path) -> Result<(), Box<dyn Error>> {
    let nix_store = nix
        .parent()
        .map(|parent| parent.join("nix-store"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("nix-store"));

    run_command(ProcessCommand::new(nix_store).arg("--gc"))
}

fn run_darwin_activation(nix: &Path, config_dir: &Path) -> Result<(), Box<dyn Error>> {
    let flake = format!("{}#aarch64-darwin", config_dir.display());

    run_command(
        ProcessCommand::new("sudo")
            .arg("-H")
            .arg(nix)
            .arg("run")
            .arg("--inputs-from")
            .arg(format!("path:{}", config_dir.display()))
            .arg("nix-darwin")
            .arg("--")
            .arg("switch")
            .arg("--flake")
            .arg(flake),
    )
}

fn run_command(command: &mut ProcessCommand) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status {status}: {command:?}").into())
    }
}

fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, contents)?;
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

fn apply_macos_update(
    inspection: &MacosSetupInspection,
    args: &UpdateArgs,
    update: &ResolvedUpdate,
) -> Result<(), Box<dyn Error>> {
    let payload = release::prepare_payload(&inspection.kit_root, &update.version)?;
    let current_flake_path = inspection.config_dir.join("flake.nix");
    let current_lock_path = inspection.config_dir.join("flake.lock");
    let current_flake = std::fs::read_to_string(&current_flake_path)?;
    let user_settings = extract_user_settings_block(&current_flake)?;
    let candidate_flake = replace_user_settings_block(&payload.flake, &user_settings)?;

    let backup_dir = create_backup(inspection)?;
    write_atomically(&current_flake_path, candidate_flake.as_bytes())?;
    write_atomically(&current_lock_path, payload.lock.as_bytes())?;

    let nix = resolve_nix_executable()?;
    let result = run_command(
        ProcessCommand::new(&nix)
            .arg("flake")
            .arg("check")
            .arg(format!("path:{}", inspection.config_dir.display())),
    )
    .and_then(|()| {
        if args.no_activate {
            Ok(())
        } else {
            run_darwin_activation(&nix, &inspection.config_dir)
        }
    });

    if let Err(error) = result {
        restore_backup(inspection, &backup_dir)?;
        return Err(format!(
            "update failed; restored flake files from {}: {error}",
            backup_dir.display()
        )
        .into());
    }

    if !args.no_activate
        && !args.no_gc
        && let Err(error) = run_nix_store_gc(&nix)
    {
        eprintln!("warning: nix-store --gc failed after successful activation: {error}");
    }

    if args.no_activate {
        println!(
            "Update configuration validated but not activated; installed version is unchanged."
        );
        return Ok(());
    }

    InstallationState {
        schema_version: 1,
        platform: "darwin".to_owned(),
        version: update.version.clone(),
        config_dir: inspection.config_dir.clone(),
    }
    .save(&inspection.kit_root)?;

    println!("Builder Kit updated to {}", update.version);
    Ok(())
}

fn create_backup(inspection: &MacosSetupInspection) -> Result<PathBuf, Box<dyn Error>> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let backup_dir = inspection
        .kit_root
        .join("backups")
        .join(format!("update-{timestamp}"));
    std::fs::create_dir_all(&backup_dir)?;
    std::fs::copy(
        inspection.config_dir.join("flake.nix"),
        backup_dir.join("flake.nix"),
    )?;
    std::fs::copy(
        inspection.config_dir.join("flake.lock"),
        backup_dir.join("flake.lock"),
    )?;
    Ok(backup_dir)
}

fn restore_backup(
    inspection: &MacosSetupInspection,
    backup_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    write_atomically(
        &inspection.config_dir.join("flake.nix"),
        &std::fs::read(backup_dir.join("flake.nix"))?,
    )?;
    write_atomically(
        &inspection.config_dir.join("flake.lock"),
        &std::fs::read(backup_dir.join("flake.lock"))?,
    )?;
    Ok(())
}

fn extract_user_settings_block(flake: &str) -> Result<String, Box<dyn Error>> {
    const BEGIN: &str = "# BEGIN BUILDER-KIT USER SETTINGS";
    const END: &str = "# END BUILDER-KIT USER SETTINGS";

    let begin = flake
        .find(BEGIN)
        .ok_or("missing Builder Kit user settings begin marker")?;
    let end_marker_start = flake
        .find(END)
        .ok_or("missing Builder Kit user settings end marker")?;
    if end_marker_start < begin
        || flake.matches(BEGIN).count() != 1
        || flake.matches(END).count() != 1
    {
        return Err("invalid or duplicate Builder Kit user settings markers".into());
    }
    let end = end_marker_start + END.len();
    Ok(flake[begin..end].to_owned())
}

fn parse_calver(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((year, month, patch))
}

fn compare_calver(left: &str, right: &str) -> std::cmp::Ordering {
    parse_calver(left).cmp(&parse_calver(right))
}

fn calver_gt(left: &str, right: &str) -> bool {
    compare_calver(left, right).is_gt()
}

fn macos_update_plan(
    inspection: &MacosSetupInspection,
    installed: &InstallationState,
    update: &ResolvedUpdate,
    args: &UpdateArgs,
) -> Plan {
    let confirmed = args.yes || args.check || args.dry_run;
    let mut lines = vec![
        format!("Current Builder Kit version: {}.", installed.version),
        format!("Selected Builder Kit version: {}.", update.version),
        format!(
            "Use configuration URL {}.",
            release::configuration_url(&update.version).expect("validated release version")
        ),
        "Preview: trust GitHub/HTTPS; no signatures or hash checks are performed.".to_owned(),
    ];
    if args.check {
        lines.push("Check whether the requested release is available WITHOUT downloading or modifying files.".to_owned());
    } else {
        lines.extend([
            "Download the requested release configuration.".to_owned(),
            format!(
                "Back up current flake files under {}.",
                inspection.kit_root.join("backups").display()
            ),
            "Preserve the existing Builder Kit user settings block.".to_owned(),
            "Replace flake.nix and flake.lock atomically and validate the candidate flake."
                .to_owned(),
            if args.no_activate {
                "Stop without activation or changing the installed version.".to_owned()
            } else {
                "Activate nix-darwin/Home Manager for the current macOS user.".to_owned()
            },
            "Restore previous flake files if validation or activation fails.".to_owned(),
        ]);
        if !args.no_gc && !args.no_activate {
            lines.push("Run nix-store --gc after successful activation.".to_owned());
        }
    }
    Plan {
        title: "Builder Kit update plan",
        platform: HostPlatform::Macos,
        lines,
        confirmation: if confirmed {
            Confirmation::NotRequired
        } else {
            Confirmation::Standard
        },
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "keep the ordered setup plan together"
)]
fn macos_setup_plan(inspection: &MacosSetupInspection, args: &SetupArgs) -> Plan {
    let confirmed = args.yes || args.dry_run;
    let mut lines = vec![
        format!(
            "Use executable directory {}.",
            inspection.executable_dir.display()
        ),
        format!(
            "Use Builder Kit directory {}.",
            inspection.kit_root.display()
        ),
        format!(
            "Use installer directory {}.",
            inspection.installer_dir.display()
        ),
        if args.download_only {
            "Leave the installed kit command unchanged.".to_owned()
        } else {
            format!(
                "Install the kit command to {}.",
                inspection.local_bin.display()
            )
        },
        format!(
            "Inspect current macOS user {} with home directory {}.",
            inspection.current_user,
            inspection.home_dir.display()
        ),
        format!(
            "Inspect Builder Kit Nix configuration at {}.",
            inspection.config_dir.display()
        ),
        format!("Detected Nix state: {}.", inspection.nix_state),
        format!("Detected configuration state: {}.", inspection.config_state),
        format!(
            "Download release {} from {}.",
            args.version,
            release::configuration_url(&args.version).expect("validated release version")
        ),
        "Preview: trust GitHub/HTTPS; no signatures or hash checks are performed.".to_owned(),
    ];

    if args.download_only {
        lines.push(format!(
            "Download {} if Nix is absent; retain download files only.",
            release::NIX_INSTALLER_URL
        ));
        lines.push("Stop WITHOUT installing Nix, activating configuration, collecting identity, or writing installation state.".to_owned());
        return Plan {
            title: "Builder Kit download plan",
            platform: HostPlatform::Macos,
            lines,
            confirmation: if confirmed {
                Confirmation::NotRequired
            } else {
                Confirmation::Standard
            },
        };
    }

    match inspection.nix_state {
        NixState::Absent => {
            lines.push(
                format!("Download and run upstream Nix installer {} from {}; administrator authentication may be required.", release::NIX_INSTALLER_VERSION, release::NIX_INSTALLER_URL),
            );
        }
        NixState::Present => {
            lines.push(
                "Reuse the existing responding Nix installation without reinstalling it."
                    .to_owned(),
            );
        }
        NixState::Unhealthy => {
            lines.push("Stop before making changes because Nix appears unhealthy.".to_owned());
        }
    }

    match inspection.config_state {
        ConfigState::Absent => {
            lines.push("Create ~/.kit/nix/flake.nix and flake.lock.".to_owned());
        }
        ConfigState::Managed => {
            lines.push(
                "Stop setup because this machine already appears managed by Builder Kit."
                    .to_owned(),
            );
        }
        ConfigState::Incomplete | ConfigState::Unrelated => {
            lines.push(
                "Stop setup because ~/.kit/nix is not a clean Builder Kit target.".to_owned(),
            );
        }
    }

    lines.extend([
        "Collect student Git identity and tool preferences without storing secrets.".to_owned(),
        "Write those values only into the marked Builder Kit user settings block.".to_owned(),
        if args.no_activate {
            "Validate configuration WITHOUT activation or marking installation complete.".to_owned()
        } else {
            "Validate and activate nix-darwin/Home Manager for the current macOS user; administrator authentication may be required.".to_owned()
        },
    ]);

    if confirmed {
        lines.push(
            "Builder Kit confirmation has already been accepted for this command.".to_owned(),
        );
    }

    Plan {
        title: "Builder Kit setup plan",
        platform: HostPlatform::Macos,
        lines,
        confirmation: if confirmed {
            Confirmation::NotRequired
        } else {
            Confirmation::Standard
        },
    }
}

fn setup_plan(platform: HostPlatform, version: &str, confirmed: bool) -> Plan {
    let mut lines = match platform {
        HostPlatform::Windows => vec![
            "Detect WSL 2 prerequisites and existing Builder Kit state.".to_owned(),
            format!("Use explicitly requested release {version}."),
            "Windows setup execution is not implemented yet.".to_owned(),
            "Install the image as the Builder Kit WSL distribution.".to_owned(),
            "Configure the Linux user declaratively through the WSL agent.".to_owned(),
        ],
        HostPlatform::Macos => vec![
            "Detect the current macOS account and existing Nix state.".to_owned(),
            format!("Download explicitly requested release {version} from GitHub over HTTPS."),
            "Install upstream Nix only if no supported Nix installation exists.".to_owned(),
            "Create ~/.kit/nix/flake.nix and flake.lock from the requested release.".to_owned(),
            "Activate nix-darwin and Home Manager for the current macOS user.".to_owned(),
        ],
        HostPlatform::Unsupported => vec![
            "Stop without changing this machine because the host platform is unsupported."
                .to_owned(),
        ],
    };

    if confirmed {
        lines.push(
            "Builder Kit confirmation has already been accepted for this command.".to_owned(),
        );
    }

    Plan {
        title: "Builder Kit setup plan",
        platform,
        lines,
        confirmation: if confirmed {
            Confirmation::NotRequired
        } else {
            Confirmation::Standard
        },
    }
}

fn update_plan(platform: HostPlatform, version: &str, check_only: bool, confirmed: bool) -> Plan {
    let mut lines = vec![
        "Read local non-secret Builder Kit installation state.".to_owned(),
        format!("Use explicitly requested release {version}; no latest resolution."),
        "Report whether the requested release is available.".to_owned(),
    ];

    if !check_only {
        lines.extend([
            "Back up the current flake and lock file before replacement.".to_owned(),
            "Validate the candidate configuration before activation.".to_owned(),
            match platform {
                HostPlatform::Windows => "Apply the update through the WSL agent.".to_owned(),
                HostPlatform::Macos => "Apply the update through nix-darwin.".to_owned(),
                HostPlatform::Unsupported => {
                    "Stop without changing this machine because the host platform is unsupported."
                        .to_owned()
                }
            },
        ]);
    }

    if confirmed {
        lines.push(
            "Builder Kit confirmation has already been accepted for this command.".to_owned(),
        );
    }

    Plan {
        title: "Builder Kit update plan",
        platform,
        lines,
        confirmation: if confirmed {
            Confirmation::NotRequired
        } else {
            Confirmation::Standard
        },
    }
}

fn eject_plan(platform: HostPlatform, confirmed: bool) -> Plan {
    let mut lines = vec![
        "Identify the managed Builder Kit configuration.".to_owned(),
        "Create a timestamped backup of Builder Kit-managed state.".to_owned(),
        "Mark the environment detached so managed updates stop.".to_owned(),
        "Leave Nix, WSL/nix-darwin, Home Manager, user accounts, and user data intact.".to_owned(),
    ];

    if platform == HostPlatform::Unsupported {
        lines.push(
            "Stop without changing this machine because the host platform is unsupported."
                .to_owned(),
        );
    }

    if confirmed {
        lines.push(
            "Builder Kit confirmation has already been accepted for this command.".to_owned(),
        );
    }

    Plan {
        title: "Builder Kit eject plan",
        platform,
        lines,
        confirmation: if confirmed {
            Confirmation::NotRequired
        } else {
            Confirmation::Eject
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_only_cli_conflicts_with_mutating_modes() {
        for flag in ["--dry-run", "--no-activate"] {
            assert!(
                Cli::try_parse_from([
                    "kit",
                    "setup",
                    "--version",
                    "2026.09.0",
                    "--download-only",
                    flag
                ])
                .is_err()
            );
        }
    }

    #[test]
    fn setup_and_update_require_an_explicit_version() {
        for command in ["setup", "update"] {
            assert!(Cli::try_parse_from(["kit", command]).is_err());
            assert!(Cli::try_parse_from(["kit", command, "--version", "latest"]).is_err());
            assert!(Cli::try_parse_from(["kit", command, "--version", "2026.09.0"]).is_ok());
            for removed in [
                "--channel",
                "--manifest-url",
                "--release-index",
                "--artifact",
                "--payload-dir",
                "--nix-installer",
            ] {
                assert!(
                    Cli::try_parse_from([
                        "kit",
                        command,
                        "--version",
                        "2026.09.0",
                        removed,
                        "value"
                    ])
                    .is_err()
                );
            }
        }
        assert!(
            Cli::try_parse_from(["kit", "--version"])
                .unwrap_err()
                .kind()
                == clap::error::ErrorKind::DisplayVersion
        );
    }

    #[test]
    fn download_plan_does_not_claim_it_will_install_or_activate() {
        let args =
            Cli::try_parse_from(["kit", "setup", "--version", "2026.09.0", "--download-only"])
                .unwrap();
        let Command::Setup(args) = args.command else {
            panic!("expected setup")
        };
        let inspection = MacosSetupInspection {
            executable_dir: "/tmp/downloads".into(),
            kit_root: "/tmp/home/.kit".into(),
            installer_dir: "/tmp/home/.kit/installer".into(),
            config_dir: "/tmp/home/.kit/nix".into(),
            local_bin: "/tmp/home/.local/bin".into(),
            nix_state: NixState::Absent,
            config_state: ConfigState::Absent,
            current_user: "tester".into(),
            home_dir: "/tmp/home".into(),
        };
        let plan = macos_setup_plan(&inspection, &args).render();
        assert!(plan.contains("download plan"));
        assert!(plan.contains("Stop WITHOUT installing Nix"));
        assert!(!plan.contains("Install the kit command"));
        assert!(!plan.contains("Validate and activate"));
    }

    #[test]
    fn only_native_arm_macos_is_qualified() {
        assert!(ensure_macos_architecture("aarch64").is_ok());
        assert!(ensure_macos_architecture("x86_64").is_err());
    }

    #[test]
    fn malformed_settings_markers_are_errors_not_panics() {
        let reversed = "# END BUILDER-KIT USER SETTINGS\n# BEGIN BUILDER-KIT USER SETTINGS";
        let duplicate = "# BEGIN BUILDER-KIT USER SETTINGS\n# BEGIN BUILDER-KIT USER SETTINGS\n# END BUILDER-KIT USER SETTINGS";
        for flake in [reversed, duplicate] {
            assert!(replace_user_settings_block(flake, "replacement").is_err());
            assert!(extract_user_settings_block(flake).is_err());
        }
    }

    #[test]
    fn user_values_cannot_interpolate_nix_code() {
        assert_eq!(
            escape_nix_string("${builtins.abort \"bad\"}"),
            "\\${builtins.abort \\\"bad\\\"}"
        );
    }

    #[test]
    fn setup_plan_mentions_macos_nix_darwin() {
        let rendered = setup_plan(HostPlatform::Macos, "2026.09.0", false).render();
        assert!(rendered.contains("nix-darwin"));
        assert!(rendered.contains("Proceed? [y/N]"));
    }

    #[test]
    fn setup_dry_run_plan_needs_no_confirmation() {
        let rendered = setup_plan(HostPlatform::Windows, "2026.09.0", true).render();
        assert!(rendered.contains("2026.09.0"));
        assert!(!rendered.contains("Proceed? [y/N]"));
    }

    #[test]
    fn update_check_plan_does_not_activate() {
        let rendered = update_plan(HostPlatform::Macos, "2026.09.0", true, true).render();
        assert!(rendered.contains("Report whether"));
        assert!(!rendered.contains("Apply the update"));
    }

    #[test]
    fn eject_requires_literal_confirmation_when_not_confirmed() {
        let rendered = eject_plan(HostPlatform::Windows, false).render();
        assert!(rendered.contains("Type EJECT"));
        assert!(rendered.contains("Leave Nix"));
    }

    #[test]
    fn existing_config_directory_without_flake_files_is_available_for_setup() {
        let unique = format!(
            "builderkit-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        );
        let directory = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&directory).expect("create temp config directory");

        let state = detect_config_state(&directory);
        std::fs::remove_dir_all(&directory).expect("remove temp config directory");

        assert_eq!(state, ConfigState::Absent);
    }
}
