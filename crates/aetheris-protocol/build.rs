const PROTO_FILES: &[&str] = &[
    "proto/auth.proto",
    "proto/matchmaking.proto",
    "proto/telemetry.proto",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for proto in PROTO_FILES {
        println!("cargo:rerun-if-changed={proto}");
    }

    #[cfg(feature = "grpc")]
    {
        // Build scripts always run on the HOST, so cfg!() refers to the host.
        // To detect the compile TARGET, we must read the CARGO_CFG_TARGET_ARCH env var.
        let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let is_wasm = target_arch == "wasm32";

        if is_wasm {
            // For wasm32 builds, generate only prost message types — no service stubs.
            // The WASM crate provides its own hand-written gRPC-web client.
            tonic_prost_build::configure()
                .build_client(false)
                .build_server(false)
                .compile_protos(PROTO_FILES, &["proto"])?;
        } else {
            // For native targets, generate the full client + server stubs.
            tonic_prost_build::configure().compile_protos(PROTO_FILES, &["proto"])?;
        }
    }
    Ok(())
}
