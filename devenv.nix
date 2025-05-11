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
    gen-entities = {
      exec = ''
        sea-orm-cli generate entity \
          --database-url $DATABASE_URL \
          --output-dir ./iodine-state/iodine-persistence-pg/src/entities \
          --with-serde both \
          --date-time-crate chrono \
          --lib

          mv ./iodine-state/iodine-persistence-pg/src/entities/lib.rs \
             ./iodine-state/iodine-persistence-pg/src/entities/mod.rs
      '';
    };

    sanitize = {
      exec = ''
        RUSTFLAGS="-Z sanitizer=thread" cargo +nightly run --target x86_64-unknown-linux-gnu
      '';
    };
  };
}
