fn main() -> shadow_rs::SdResult<()> {
  tonic_build::configure()
    .build_client(true)
    .build_server(false)
    .compile(&["proto/sandbox.proto"], &["proto/"])
    .unwrap();
  return shadow_rs::new();
}
