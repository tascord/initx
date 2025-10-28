{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  env.GREET = "$name";
  packages = [
    pkgs.git
    pkgs.lld
    pkgs.mold
    pkgs.rust-analyzer
    pkgs.pre-commit
  ];

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  enterShell = ''
    git --version
  '';
  
}
