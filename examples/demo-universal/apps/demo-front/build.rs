#[path = "../../build_support/cms_hello.rs"]
mod cms_hello;

fn main() {
    cms_hello::compile("../demo-back/templates/cms-hello");
}
