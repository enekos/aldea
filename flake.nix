{
  description = "Monochrome diff viewer for e-ink terminals: font weight instead of color";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
    in
    {
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "aldea";
          version = manifest.version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          meta = {
            description = manifest.description;
            homepage = "https://github.com/enekos/aldea";
            license = pkgs.lib.licenses.mit;
            mainProgram = "aldea";
          };
        };
      });
    };
}
