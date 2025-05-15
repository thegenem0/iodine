{ pkgs, ... }:

{
  languages.python = {
    enable = true;
    package = pkgs.python312;
    venv = { enable = true; };
  };

  packages = with pkgs; [ protobuf ];
}
