fn main() {
  cc::Build::new().cpp(true).file("src/lib.cpp").compile("lib");
  println!("cargo:rerun-if-changed=src/lib.cpp");
}
