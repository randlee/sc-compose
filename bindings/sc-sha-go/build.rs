fn main() {
    uniffi_build::generate_scaffolding("src/sc_sha_go.udl")
        .expect("generate sc-sha Go UniFFI scaffolding");
}
