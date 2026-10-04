{
  description = "Nix flake for enum-assoc";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        formatter = pkgs.nixfmt-tree;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Nix
            nixd
            deadnix
            statix
            self.formatter.${system}
            # Rust
            rustc
            rustfmt
            rust-analyzer
            cargo
            cargo-watch
            clippy
          ];
        };
      }
    );
}
