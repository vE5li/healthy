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
    in rec {
      formatter = pkgs.alejandra;

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          pkg-config
          openssl
        ];

        # For any tools that need to see the rust toolchain src
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
      };

      packages.healthy = pkgs.rustPlatform.buildRustPackage rec {
        pname = "healthy";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ];

        meta.mainProgram = pname;
      };

      packages.default = packages.healthy;

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
            description = "Path to the devices configuration file";
          };

          port = lib.mkOption {
            type = lib.types.port;
            default = 4901;
            description = "Port to listen on";
          };

          openFirewall = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Whether to open the port in the firewall";
          };
        };

        config = lib.mkIf configuration.enable {
          systemd.services.healthy = {
            description = "Healthy Service";
            wantedBy = ["multi-user.target"];
            after = ["network.target"];

            serviceConfig = {
              ExecStart = "${lib.getExe packages.healthy} --config ${configuration.configFile} --port ${toString configuration.port}";
              Restart = "always";
              DynamicUser = true;
              AmbientCapabilities = "CAP_NET_RAW";
            };
          };

          networking.firewall.allowedTCPPorts = lib.mkIf configuration.openFirewall [
            configuration.port
          ];
        };
      };
    });
}
