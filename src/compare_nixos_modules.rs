use {
    std::{
        fmt,
        path::Path,
        pin::pin,
    },
    futures::stream::TryStreamExt as _,
    strum::IntoEnumIterator as _,
    strum_macros::EnumIter,
    wheel::traits::IoResultExt as _,
};

use crate::{NEW_SYSTEM_PATH, OLD_SYSTEM_PATH};

#[derive(EnumIter)]
enum ModuleType {
    LinuxKernel,
    Systemd,
}

// for printing messages
impl fmt::Display for ModuleType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::LinuxKernel => write!(f, "Linux Kernel"),
            Self::Systemd => write!(f, "Systemd"),
        }
    }
}

impl ModuleType {
    fn get_nix_store_path_sync(&self, use_old_path: bool) -> Result<String, crate::Error> {
        let suffix = match self {
            Self::LinuxKernel => "/kernel-modules",
            Self::Systemd => "/systemd",
        };

        let system_path = if use_old_path {
            OLD_SYSTEM_PATH.to_string()
        } else {
            NEW_SYSTEM_PATH.to_string()
        };

        match std::fs::read_link(system_path.clone() + suffix).at(system_path.clone() + suffix) {
            Ok(path) => Ok(path
                .into_os_string()
                .into_string()
                .expect("Cannot convert PathBuf to String")),
            Err(_) if matches!(self, Self::LinuxKernel) => {
                // Older generations predate the kernel/modules split so
                // use the kernel derivation root.
                let kernel_link = std::fs::read_link(system_path.clone() + "/kernel").at(system_path + "/kernel")?
                    .into_os_string()
                    .into_string()
                    .expect("Cannot convert PathBuf to String");
                let mut module_dir = kernel_link
                    .split('/')
                    .collect::<Vec<&str>>()
                    .get(1..4)
                    .ok_or(crate::Error::MissingModuleDir)?
                    .join("/");
                module_dir.insert(0, '/');
                Ok(module_dir)
            }
            Err(err) => Err(err.into()),
        }
    }

    async fn get_nix_store_path_async(&self, use_old_path: bool) -> Result<String, crate::Error> {
        let suffix = match self {
            Self::LinuxKernel => "kernel-modules",
            Self::Systemd => "systemd",
        };

        let system_path = if use_old_path {
            Path::new(OLD_SYSTEM_PATH)
        } else {
            Path::new(NEW_SYSTEM_PATH)
        };

        match wheel::fs::read_link(system_path.join(suffix)).await {
            Ok(path) => Ok(path
                .into_os_string()
                .into_string()
                .expect("Cannot convert PathBuf to String")),
            Err(_) if matches!(self, Self::LinuxKernel) => {
                // Older generations predate the kernel/modules split so
                // use the kernel derivation root.
                let kernel_link = wheel::fs::read_link(system_path.join("kernel")).await?
                    .into_os_string()
                    .into_string()
                    .expect("Cannot convert PathBuf to String");
                let mut module_dir = kernel_link
                    .split('/')
                    .collect::<Vec<&str>>()
                    .get(1..4)
                    .ok_or(crate::Error::MissingModuleDir)?
                    .join("/");
                module_dir.insert(0, '/');
                Ok(module_dir)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn get_linux_version_sync(linux_path: &str) -> Result<String, crate::Error> {
        let lib_modules_path = std::fs::read_dir(linux_path).at(linux_path)?
            .nth(0)
            .ok_or_else(|| crate::Error::MissingLinuxVersionDir(linux_path.to_owned()))?.at(linux_path)?
            .path()
            .into_os_string()
            .into_string()
            .expect("Cannot convert PathBuf to String");
        let linux_version = lib_modules_path.split('/').nth(6).ok_or_else(|| crate::Error::KernelVersion(lib_modules_path.clone()))?;

        Ok(linux_version.to_string())
    }

    async fn get_linux_version_async(linux_path: &str) -> Result<String, crate::Error> {
        let lib_modules_path = pin!(wheel::fs::read_dir(linux_path))
            .try_next().await?
            .ok_or_else(|| crate::Error::MissingLinuxVersionDir(linux_path.to_owned()))?
            .path()
            .into_os_string()
            .into_string()
            .expect("Cannot convert PathBuf to String");
        let linux_version = lib_modules_path.split('/').nth(6).ok_or_else(|| crate::Error::KernelVersion(lib_modules_path.clone()))?;

        Ok(linux_version.to_string())
    }

    fn get_systemd_version(systemd_path: &str) -> Result<String, crate::Error> {
        let split_systemd_path = systemd_path.split('-').collect::<Vec<&str>>();
        let systemd_version = split_systemd_path
            .get(2..)
            .ok_or_else(|| crate::Error::SystemdVersion(systemd_path.to_owned()))?
            .join("-");
        Ok(systemd_version)
    }

    fn get_version_sync(&self) -> Result<(String, String), crate::Error> {
        let old_module_root_path = self.get_nix_store_path_sync(true)?;
        let new_module_root_path = self.get_nix_store_path_sync(false)?;

        let old_module_version: String;
        let new_module_version: String;

        match self {
            Self::LinuxKernel => {
                let old_linux_path = old_module_root_path + "/lib/modules";
                let new_linux_path = new_module_root_path + "/lib/modules";
                old_module_version = Self::get_linux_version_sync(&old_linux_path)?;
                new_module_version = Self::get_linux_version_sync(&new_linux_path)?;
            }
            Self::Systemd => {
                old_module_version = Self::get_systemd_version(&old_module_root_path)?;
                new_module_version = Self::get_systemd_version(&new_module_root_path)?;
            }
        }

        Ok((old_module_version, new_module_version))
    }

    async fn get_version_async(&self) -> Result<(String, String), crate::Error> {
        let old_module_root_path = self.get_nix_store_path_async(true).await?;
        let new_module_root_path = self.get_nix_store_path_async(false).await?;

        let old_module_version: String;
        let new_module_version: String;

        match self {
            Self::LinuxKernel => {
                let old_linux_path = old_module_root_path + "/lib/modules";
                let new_linux_path = new_module_root_path + "/lib/modules";
                old_module_version = Self::get_linux_version_async(&old_linux_path).await?;
                new_module_version = Self::get_linux_version_async(&new_linux_path).await?;
            }
            Self::Systemd => {
                old_module_version = Self::get_systemd_version(&old_module_root_path)?;
                new_module_version = Self::get_systemd_version(&new_module_root_path)?;
            }
        }

        Ok((old_module_version, new_module_version))
    }
}

pub fn upgrades_available_sync() -> Result<String, crate::Error> {
    let mut reason = String::new();
    'x: for module in ModuleType::iter() {
        let (mut old_module_version, mut new_module_version) = module.get_version_sync()?;

        if old_module_version != new_module_version {
            if old_module_version.len() != new_module_version.len() {
                if old_module_version.contains("-rc") && !new_module_version.contains("-rc") {
                    old_module_version = old_module_version.replace("-rc", ".");
                    new_module_version.push_str(".0");
                } else if new_module_version.contains("-rc") && !old_module_version.contains("-rc")
                {
                    new_module_version = new_module_version.replace("-rc", ".");
                    old_module_version.push_str(".0");
                } else if new_module_version.contains("-rc") && old_module_version.contains("-rc") {
                    new_module_version = new_module_version.replace("-rc", ".");
                    old_module_version = old_module_version.replace("-rc", ".");
                }
            }

            let old_version = &old_module_version.split('.').collect::<Vec<&str>>();
            let new_version = &new_module_version.split('.').collect::<Vec<&str>>();

            for (old, new) in old_version.iter().zip(new_version.iter()) {
                if new > old {
                    reason = format!("{module} ({old_module_version} -> {new_module_version})\n");
                    break 'x;
                }
            }
        }
    }

    Ok(reason)
}

pub async fn upgrades_available_async() -> Result<String, crate::Error> {
    let mut reason = String::default();
    'x: for module in ModuleType::iter() {
        let (mut old_module_version, mut new_module_version) = module.get_version_async().await?;

        if old_module_version != new_module_version {
            if old_module_version.len() != new_module_version.len() {
                if old_module_version.contains("-rc") && !new_module_version.contains("-rc") {
                    old_module_version = old_module_version.replace("-rc", ".");
                    new_module_version.push_str(".0");
                } else if new_module_version.contains("-rc") && !old_module_version.contains("-rc")
                {
                    new_module_version = new_module_version.replace("-rc", ".");
                    old_module_version.push_str(".0");
                } else if new_module_version.contains("-rc") && old_module_version.contains("-rc") {
                    new_module_version = new_module_version.replace("-rc", ".");
                    old_module_version = old_module_version.replace("-rc", ".");
                }
            }

            let old_version = &old_module_version.split('.').collect::<Vec<&str>>();
            let new_version = &new_module_version.split('.').collect::<Vec<&str>>();

            for (old, new) in old_version.iter().zip(new_version.iter()) {
                if new > old {
                    reason = format!("{module} ({old_module_version} -> {new_module_version})\n");
                    break 'x;
                }
            }
        }
    }

    Ok(reason)
}
