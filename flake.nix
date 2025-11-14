{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    flake-utils,
    rust-overlay,
    nixpkgs,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = (import nixpkgs) {inherit system overlays;};
    in {
      formatter = pkgs.alejandra;

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          pkgs.nodejs
        ];

        # For any tools that need to see the rust toolchain src
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
      };

      packages.backend = pkgs.rustPlatform.buildRustPackage {
        pname = "healthy-backend";
        version = "0.1.0";
        src = ./backend;
        cargoLock.lockFile = ./backend/Cargo.lock;
      };

      packages.frontend = pkgs.buildNpmPackage {
        pname = "healthy-frontend";
        version = "0.1.0";
        src = ./frontend;
        npmDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        installPhase = ''
          mkdir -p $out
          cp -r dist/* $out/
        '';
      };

      nixosModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        configuration = config.services.healthy;
      in {
        options.services.healthy = {
          enable = lib.mkEnableOption "healthy service";

          configFile = lib.mkOption {
            type = lib.types.path;
            default = pkgs.writeText "devices.json" (
              builtins.toJSON {
                devices = [
                  "8.8.8.8"
                  "1.1.1.1"
                ];
              }
            );
            description = "Path to the devices configuration file";
          };

          port = lib.mkOption {
            type = lib.types.port;
            default = 5173;
            description = "Port to listen on";
          };

          openFirewall = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Whether to open the port in the firewall";
          };
        };

        config = lib.mkIf configuration.enable {
          systemd.services = {
            healthy-backend = {
              description = "Healthy Backend Service";
              wantedBy = ["multi-user.target"];
              after = ["network.target"];

              serviceConfig = {
                ExecStart = "${
                  self.packages.${pkgs.system}.backend
                }/bin/backend --config ${configuration.configFile}";
                Restart = "always";
                DynamicUser = true;
                AmbientCapabilities = "CAP_NET_RAW";
              };
            };

            healthy-frontend = {
              description = "Healthy Frontend Service";
              wantedBy = ["multi-user.target"];
              after = ["network.target"];

              serviceConfig = {
                ExecStart = "${pkgs.python3}/bin/python3 -m http.server ${toString configuration.port} --directory ${
                  self.packages.${pkgs.system}.frontend
                }";
                Restart = "always";
                DynamicUser = true;
              };
            };
          };

          networking.firewall.allowedTCPPorts = lib.mkIf configuration.openFirewall [
            configuration.port
          ];
        };
      };
    });
}
