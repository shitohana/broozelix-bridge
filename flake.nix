{
  description = "Thin Helix socket bridge for broozelix";

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
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          broozelixBridge = pkgs.rustPlatform.buildRustPackage {
            pname = "broozelix-bridge";
            version = "0.1.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            meta = with pkgs.lib; {
              description = "Thin Helix socket bridge for broozelix";
              homepage = "https://github.com/shitohana/broozelix-bridge";
              license = licenses.mit;
              mainProgram = "broozelix-bridge";
            };
          };
        in
        {
          inherit broozelixBridge;
          default = broozelixBridge;
          broozelix-bridge = broozelixBridge;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
              rustfmt
              clippy
            ];
          };
        }
      );
    };
}