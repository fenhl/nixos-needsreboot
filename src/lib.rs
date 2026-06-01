use wheel::traits::IoResultExt as _;

mod compare_nixos_modules;

pub static OLD_SYSTEM_PATH: &str = "/run/booted-system";
pub static NEW_SYSTEM_PATH: &str = "/nix/var/nix/profiles/system";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("Could not determine Linux kernel version from path: {0}")]
    KernelVersion(String),
    #[error("Expected one directory in {0}")]
    MissingLinuxVersionDir(String),
    #[error("Cannot find the module's directory in /nix/store")]
    MissingModuleDir,
    #[error("Could not determine Systemd version from path: {0}")]
    SystemdVersion(String),
}

#[derive(Debug)]
pub enum NeedsReboot {
    /// No reboot needed, running the latest NixOS generation
    IsLatest,
    /// No reboot needed, no updates available
    NoUpdates,
    /// Reboot needed, updates available
    Updates(String),
}

pub fn needs_reboot_sync() -> Result<NeedsReboot, Error> {
    let old_system_id = std::fs::read_to_string(OLD_SYSTEM_PATH.to_string() + "/nixos-version").at(OLD_SYSTEM_PATH.to_string() + "/nixos-version")?;
    let new_system_id = std::fs::read_to_string(NEW_SYSTEM_PATH.to_string() + "/nixos-version").at(NEW_SYSTEM_PATH.to_string() + "/nixos-version")?;

    Ok(if old_system_id == new_system_id {
        NeedsReboot::IsLatest
    } else {
        let reason = compare_nixos_modules::upgrades_available()?;
        if reason.is_empty() {
            NeedsReboot::NoUpdates
        } else {
            NeedsReboot::Updates(reason)
        }
    })
}

pub async fn needs_reboot_async() -> Result<NeedsReboot, Error> {
    let old_system_id = wheel::fs::read_to_string(OLD_SYSTEM_PATH.to_string() + "/nixos-version").await?;
    let new_system_id = wheel::fs::read_to_string(NEW_SYSTEM_PATH.to_string() + "/nixos-version").await?;

    Ok(if old_system_id == new_system_id {
        NeedsReboot::IsLatest
    } else {
        let reason = compare_nixos_modules::upgrades_available()?;
        if reason.is_empty() {
            NeedsReboot::NoUpdates
        } else {
            NeedsReboot::Updates(reason)
        }
    })
}
