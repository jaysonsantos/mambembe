{ pkgs, lib, ... }:

{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [
    pkgs.cairo
    pkgs.gdk-pixbuf
    pkgs.git
    pkgs.glib
    pkgs.gtk4
    pkgs.graphene
    pkgs.harfbuzz
    pkgs.libadwaita
    pkgs.pango
  ] ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk; [
    frameworks.Security
    frameworks.CoreFoundation
    frameworks.CoreServices
    frameworks.SystemConfiguration
    frameworks.IOKit
  ]);

  # https://devenv.sh/scripts/
  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
    hello
    git --version
  '';

  dotenv.enable = true;

  # https://devenv.sh/languages/
  languages.nix.enable = true;
  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks = {
    editorconfig-checker.enable = true;
    nixfmt.enable = true;
    rustfmt.enable = true;
  };

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
}
