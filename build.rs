/*
Davenstein - by David Petnick
*/

fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/Davenstein.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("packaging/windows/Davenstein.ico")
            .compile()
            .expect("failed to embed the Davenstein Windows icon");
    }
}
