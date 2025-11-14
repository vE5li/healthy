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

      packages.healthy-backend = pkgs.rustPlatform.buildRustPackage {
        pname = "healthy-backend";
        version = "0.1.0";
        src = ./backend;
        cargoLock.lockFile = ./backend/Cargo.lock;
      };

      packages.healthy-frontend = backendUrl:
        pkgs.buildNpmPackage {
          pname = "healthy-frontend";
          version = "0.1.0";
          src = ./frontend;
          npmDepsHash = "sha256-0+Y7RfnDnwItVLWOOySMNErIVAoyBkz2D9NIoQL3eKo=";
          BACKEND_URL = backendUrl;
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

          backend-port = lib.mkOption {
            type = lib.types.port;
            default = 4901;
            description = "Port to listen on";
          };

          frontend-port = lib.mkOption {
            type = lib.types.port;
            default = 5173;
            description = "Port to listen on";
          };

          backendUrl = lib.mkOption {
            type = lib.types.str;
            default = "http://127.0.0.1:${toString configuration.backend-port}";
            description = "URL of the backend service for the frontend to connect to";
          };

          openFirewall = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Whether to open the backend and frontend ports in the firewall";
          };
        };

        config = lib.mkIf configuration.enable {
          systemd.services = {
            healthy-backend = {
              description = "Healthy Backend Service";
              wantedBy = ["multi-user.target"];
              after = ["network.target"];

              serviceConfig = {
                ExecStart = "${lib.getExe' self.packages.${pkgs.system}.healthy-backend "backend"} --config ${configuration.configFile} --port ${toString configuration.backend-port}";
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
                ExecStart = "${pkgs.python3}/bin/python3 -m http.server ${toString configuration.frontend-port} --directory ${self.packages.${pkgs.system}.healthy-frontend configuration.backendUrl}";
                Restart = "always";
                DynamicUser = true;
              };
            };
          };

          networking.firewall.allowedTCPPorts = lib.mkIf configuration.openFirewall [
            configuration.backend-port
            configuration.frontend-port
          ];
        };
      };
    });
}
