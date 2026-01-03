fn main() {
  let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  println!("cargo:rerun-if-changed=prebuild");
  println!("cargo:rustc-link-search=native={manifest_dir}/prebuild");
}
