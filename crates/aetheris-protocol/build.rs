fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/auth.proto");
    println!("cargo:rerun-if-changed=proto/matchmaking.proto");
    println!("cargo:rerun-if-changed=proto/telemetry.proto");

    #[cfg(feature = "grpc")]
    {
        // Build scripts always run on the HOST, so cfg!() refers to the host.
        // To detect the compile TARGET, we must read the CARGO_CFG_TARGET_ARCH env var.
        let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let is_wasm = target_arch == "wasm32";

        if is_wasm {
            // For wasm32 builds, generate only prost message types — no service stubs.
            // The WASM crate provides its own hand-written gRPC-web client.
            tonic_build::configure()
                .build_client(false)
                .build_server(false)
                .compile_protos(
                    &[
                        "proto/auth.proto",
                        "proto/matchmaking.proto",
                        "proto/telemetry.proto",
                    ],
                    &["proto"],
                )?;
        } else {
            // For native targets, generate the full client + server stubs.
            tonic_build::configure().compile_protos(
                &[
                    "proto/auth.proto",
                    "proto/matchmaking.proto",
                    "proto/telemetry.proto",
                ],
                &["proto"],
            )?;
        }
    }
    Ok(())
}
