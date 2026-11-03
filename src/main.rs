use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use wheel::traits::IoResultExt as _;
use nixos_needsreboot::{Error, NeedsReboot};

pub static NIXOS_NEEDS_REBOOT: &str = "/var/run/reboot-required";

#[derive(clap::Parser)]
#[clap(version)]
struct Args {
    #[clap(long)]
    dry_run: bool,
}

#[wheel::main]
fn main(Args { dry_run }: Args) -> Result<(), Error> {
    let user = env::var_os("USER")
        .unwrap()
        .into_string()
        .expect("Cannot convert OsString into String");
    if user != "root" && !dry_run {
        println!("ERROR: please run this as root");
        println!("HINT: use the '--dry-run' option");
        std::process::exit(1);
    }

    if Path::new("/nix/var/nix/profiles/system").exists() {
        if Path::new(NIXOS_NEEDS_REBOOT).exists() {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let _ = handle.write_all(&fs::read(NIXOS_NEEDS_REBOOT).at(NIXOS_NEEDS_REBOOT)?);
            let _ = handle.flush();
            std::process::exit(2);
        } else {
            match nixos_needsreboot::needs_reboot_sync()? {
                NeedsReboot::IsLatest => eprintln!("DEBUG: you are using the latest NixOS generation, no need to reboot"),
                NeedsReboot::NoUpdates => eprintln!("DEBUG: no updates available, moar uptime!!!"),
                NeedsReboot::Updates(reason) => {
                    if dry_run {
                        println!("{reason}");
                    } else {
                        fs::write(NIXOS_NEEDS_REBOOT, reason).at(NIXOS_NEEDS_REBOOT)?;
                    }
                    std::process::exit(2);
                }
            }
        }
    } else {
        eprintln!("This binary is intedned to run only on NixOS.");
        std::process::exit(1);
    }

    Ok(())
}
