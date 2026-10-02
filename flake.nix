{
  description = "Ask your Linux system why";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "why-linux";
            version = "0.1.4";
            src = ./.;

            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "Explain why things exist on a Linux system";
              homepage = "https://github.com/aethctl/why";
              license = pkgs.lib.licenses.mit;
              mainProgram = "why";
              platforms = pkgs.lib.platforms.linux;
            };
          };
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/why";
          meta.description = "Ask your Linux system why";
        };
      });

      devShells = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              rustc
              rustfmt
            ];
          };
        });

      formatter = forAllSystems (system:
        (import nixpkgs { inherit system; }).nixfmt
      );
    };
}
