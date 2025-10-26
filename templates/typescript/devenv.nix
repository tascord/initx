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
    pkgs.nodejs
  ];

  enterShell = ''
    git --version
  '';
  
}
