fn main() {
    println!("cargo::rustc-check-cfg=cfg(warehouse)");
    println!("cargo::rustc-cfg=warehouse");
}
