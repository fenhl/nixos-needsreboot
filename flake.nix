{
    inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/*.tar.gz";
    outputs = attrs: let
        # helpers for producing system-specific outputs
        supportedSystems = [
            "aarch64-linux"
            "riscv64-linux"
            "x86_64-linux"
        ];
        forEachSupportedSystem = f: attrs.nixpkgs.lib.genAttrs supportedSystems (system: f {
            pkgs = import attrs.nixpkgs {
                inherit system;
            };
        });
    in {
        devShells = forEachSupportedSystem ({ pkgs, ... }: {
            default = pkgs.mkShell {
                packages = with pkgs; [
                    cargo
                    gdb
                    pkg-config
                    nixpkgs-fmt # formatting this flake
                ];
            };
        });
        packages = forEachSupportedSystem ({ pkgs, ... }: {
            default = pkgs.rustPlatform.buildRustPackage {
                pname = "nixos-needsreboot";
                version = "0.1.10";
                src = ./.;
                cargoLock = {
                    allowBuiltinFetchGit = true; # allows omitting cargoLock.outputHashes
                    lockFile = ./Cargo.lock;
                };
            };
        });
    };
}
