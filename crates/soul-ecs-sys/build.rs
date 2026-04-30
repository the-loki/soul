fn main() {
    println!("cargo:rerun-if-changed=vendor/flecs/flecs.c");
    println!("cargo:rerun-if-changed=vendor/flecs/flecs.h");
    println!("cargo:rerun-if-changed=src/shim.c");

    cc::Build::new()
        .file("vendor/flecs/flecs.c")
        .file("src/shim.c")
        .include("vendor/flecs")
        .define("FLECS_NO_CPP", None)
        .compile("soul_ecs_flecs");
}
