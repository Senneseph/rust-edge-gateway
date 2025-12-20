$env:RUST_LOG="info,rust_edge_gateway=debug"
$env:DEFAULT_ADMIN_PASSWORD="testpassword123!"
cargo run --release --manifest-path crates/rust-edge-gateway/Cargo.toml