{ pkgs, ... }: {
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
  };

  packages = with pkgs; [
    clang-tools_19
    cargo-expand
    valgrind
    sqlitebrowser
    protobuf
    firecracker
  ];

  enterShell = ''
    cargo install --locked cargo-autoinherit
    cargo install sea-orm-cli --no-default-features --features "cli,codegen,runtime-tokio-rustls,async-std/default,async-std/attributes"
  '';

  dotenv.enable = true;

  scripts = {
    sanitize = {
      exec = ''
        RUSTFLAGS="-Z sanitizer=thread" cargo +nightly run --target x86_64-unknown-linux-gnu
      '';
    };
  };
}
