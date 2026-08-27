use std::{
    env,
    error::Error,
    fmt::{self, Write as _},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use clap::{Parser, Subcommand, ValueEnum};

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
struct SetupArgs {
    /// Print the setup plan without changing the machine.
    #[arg(long)]
    dry_run: bool,

    /// Accept Builder Kit prompts. Does not bypass upstream installer prompts.
    #[arg(long)]
    yes: bool,

    /// Release channel to install.
    #[arg(long, value_enum, default_value_t = Channel::Stable)]
    channel: Channel,

    /// Local verified macOS payload directory containing flake.nix and flake.lock.
    #[arg(long)]
    payload_dir: Option<PathBuf>,

    /// Local pinned upstream nix-installer binary to use when Nix is absent.
    #[arg(long)]
    nix_installer: Option<PathBuf>,

    /// Write and validate configuration but do not run nix-darwin switch.
    #[arg(long)]
    no_activate: bool,
}

#[derive(clap::Args, Debug)]
struct UpdateArgs {
    /// Check for updates without changing the machine.
    #[arg(long)]
    check: bool,

    /// Accept Builder Kit prompts.
    #[arg(long)]
    yes: bool,
}

#[derive(clap::Args, Debug)]
struct EjectArgs {
    /// Accept Builder Kit prompts.
    #[arg(long)]
    yes: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Channel {
    Stable,
    Edge,
}

impl fmt::Display for Channel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stable => formatter.write_str("stable"),
            Self::Edge => formatter.write_str("edge"),
        }
    }
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
            let plan = setup_plan(platform, args.channel, args.yes || args.dry_run);
            print!("{}", plan.render());
            Err("Builder Kit setup is supported only on Windows and macOS".into())
        }
    }
}

fn setup_windows(args: &SetupArgs) -> Result<(), Box<dyn Error>> {
    let plan = setup_plan(
        HostPlatform::Windows,
        args.channel,
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
    let inspection = MacosSetupInspection::inspect()?;
    let plan = macos_setup_plan(&inspection, args.channel, args.yes || args.dry_run);
    print!("{}", plan.render());

    if args.dry_run {
        return Ok(());
    }

    if !args.yes && !confirm_standard()? {
        return Err("setup cancelled".into());
    }

    inspection.ensure_supported()?;
    execute_macos_setup(&inspection, args)
}

fn update(platform: HostPlatform, args: &UpdateArgs) -> Result<(), Box<dyn Error>> {
    let plan = update_plan(platform, args.check, args.yes || args.check);
    print!("{}", plan.render());

    if args.check {
        return Ok(());
    }

    Err("update execution is not implemented yet".into())
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

fn execute_macos_setup(
    inspection: &MacosSetupInspection,
    args: &SetupArgs,
) -> Result<(), Box<dyn Error>> {
    if inspection.nix_state == NixState::Absent {
        let installer = args
            .nix_installer
            .as_ref()
            .ok_or("Nix is absent; pass --nix-installer <path> for this test setup path")?;
        std::fs::create_dir_all(&inspection.installer_dir)?;
        let installer = retain_installer(installer, &inspection.installer_dir)?;
        prepare_local_executable(&installer)?;
        run_command(
            ProcessCommand::new(&installer)
                .arg("install")
                .arg("--enable-flakes"),
        )?;
    }

    let nix = resolve_nix_executable()?;

    let payload_dir = args
        .payload_dir
        .as_ref()
        .ok_or("pass --payload-dir <dir> containing flake.nix and flake.lock")?;

    let payload = MacosPayload::from_dir(payload_dir)?;
    let settings = prompt_user_settings(&inspection.current_user, &inspection.home_dir)?;
    let flake = replace_user_settings_block(&payload.flake, &settings.to_nix_block())?;

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

    if !args.no_activate {
        run_darwin_activation(&nix, &inspection.config_dir)?;
    }

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
    fn from_dir(directory: &Path) -> Result<Self, Box<dyn Error>> {
        let flake = std::fs::read_to_string(directory.join("flake.nix"))?;
        let lock = std::fs::read_to_string(directory.join("flake.lock"))?;
        Ok(Self { flake, lock })
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
        io::stdin().read_line(&mut line)?;
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
    let end = end_marker_start + END.len();

    if flake[end..].contains(BEGIN) || flake[end..].contains(END) || flake[..begin].contains(BEGIN)
    {
        return Err("duplicate Builder Kit user settings marker".into());
    }

    let mut output = String::with_capacity(flake.len() + replacement.len());
    output.push_str(&flake[..begin]);
    output.push_str(replacement);
    output.push_str(&flake[end..]);
    Ok(output)
}

fn escape_nix_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

fn retain_installer(source: &Path, installer_dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("installer path has no file name: {}", source.display()))?;
    let destination = installer_dir.join(file_name);

    if source != destination {
        std::fs::copy(source, &destination)?;
    }

    Ok(destination)
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

    #[cfg(target_os = "macos")]
    {
        let _ = ProcessCommand::new("xattr")
            .arg("-d")
            .arg("com.apple.quarantine")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
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

fn run_darwin_activation(nix: &Path, config_dir: &Path) -> Result<(), Box<dyn Error>> {
    let flake = format!("{}#aarch64-darwin", config_dir.display());

    run_command(
        ProcessCommand::new("sudo")
            .arg("-H")
            .arg(nix)
            .arg("run")
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

fn macos_setup_plan(inspection: &MacosSetupInspection, channel: Channel, confirmed: bool) -> Plan {
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
            "Use installer and manifest cache directory {}.",
            inspection.installer_dir.display()
        ),
        format!(
            "Install the kit command to {}.",
            inspection.local_bin.display()
        ),
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
        format!("Fetch and verify the signed {channel} release manifest."),
        "Download verified macOS flake and lock artifacts into the local release cache.".to_owned(),
    ];

    match inspection.nix_state {
        NixState::Absent => {
            lines.push(
                "Download, verify, retain, and run the pinned upstream Nix installer.".to_owned(),
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
        "Validate and activate nix-darwin/Home Manager for the current macOS user.".to_owned(),
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

fn setup_plan(platform: HostPlatform, channel: Channel, confirmed: bool) -> Plan {
    let mut lines = match platform {
        HostPlatform::Windows => vec![
            "Detect WSL 2 prerequisites and existing Builder Kit state.".to_owned(),
            format!("Fetch and verify the signed {channel} release manifest."),
            "Download and verify the Builder Kit WSL image.".to_owned(),
            "Install the image as the Builder Kit WSL distribution.".to_owned(),
            "Configure the Linux user declaratively through the WSL agent.".to_owned(),
        ],
        HostPlatform::Macos => vec![
            "Detect the current macOS account and existing Nix state.".to_owned(),
            format!("Fetch and verify the signed {channel} release manifest."),
            "Install upstream Nix only if no supported Nix installation exists.".to_owned(),
            "Create ~/.kit/nix/flake.nix and flake.lock from verified release artifacts."
                .to_owned(),
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

fn update_plan(platform: HostPlatform, check_only: bool, confirmed: bool) -> Plan {
    let mut lines = vec![
        "Read local non-secret Builder Kit installation state.".to_owned(),
        "Fetch and verify the signed release manifest for the installed channel.".to_owned(),
        "Report whether a dependency or feature update is available.".to_owned(),
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
    fn setup_plan_mentions_macos_nix_darwin() {
        let rendered = setup_plan(HostPlatform::Macos, Channel::Stable, false).render();
        assert!(rendered.contains("nix-darwin"));
        assert!(rendered.contains("Proceed? [y/N]"));
    }

    #[test]
    fn setup_dry_run_plan_needs_no_confirmation() {
        let rendered = setup_plan(HostPlatform::Windows, Channel::Edge, true).render();
        assert!(rendered.contains("edge"));
        assert!(!rendered.contains("Proceed? [y/N]"));
    }

    #[test]
    fn update_check_plan_does_not_activate() {
        let rendered = update_plan(HostPlatform::Macos, true, true).render();
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
