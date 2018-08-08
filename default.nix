let
    pkgs = import <nixpkgs> {};
    frameworks = pkgs.darwin.apple_sdk.frameworks;
in pkgs.stdenv.mkDerivation {
    name = "vault-thing";

    buildInputs = [
      pkgs.rustc
      pkgs.cargo
      pkgs.rustfmt
      frameworks.Security
      frameworks.CoreFoundation
      frameworks.CoreServices
    ];

    shellHook = ''
        export NIX_LDFLAGS="-F${frameworks.CoreFoundation}/Library/Frameworks -framework CoreFoundation $NIX_LDFLAGS";
    '';
}
