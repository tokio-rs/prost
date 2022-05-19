use prost_build::{Config, CustomType};
use std::io::Write;

fn main() {
    let src = std::env::current_dir().unwrap().join("src");

    protobuf_strict::write_protos(&src);

    macro_rules! initialize_dir {
        ($name: expr) => {{
            // Maybe the dir doesn't exists yet
            let _ = std::fs::remove_dir_all(src.join($name));
            std::fs::create_dir(src.join($name)).unwrap();
            std::fs::File::create(src.join($name).join("mod.rs")).unwrap()
        }};
    }

    let mut b_generated = initialize_dir!("b_generated");
    let mut generated = initialize_dir!("generated");
    let mut protos = vec![];
    let paths = std::fs::read_dir("./src/protos").unwrap();

    for path in paths {
        let path = path.unwrap();
        let file_name = path.file_name();
        let s = file_name.to_str().unwrap();

        protos.push("src/protos/".to_string() + s);
    }

    macro_rules! go_generate {
        ($name: expr, $file: expr, $config: expr) => {
            std::env::set_var("OUT_DIR", src.join($name).to_str().unwrap());

            $config
                .compile_protos(protos.as_slice(), &["src/protos/".to_string()])
                .unwrap();

            let paths = std::fs::read_dir("./src/".to_string() + $name).unwrap();

            for path in paths {
                let path = path.unwrap();
                let file_name = path.file_name();
                let s = file_name.to_str().unwrap();

                if s == "mod.rs" {
                    continue;
                }

                let m = s.strip_suffix(".rs").unwrap();

                writeln!($file, "#[rustfmt::skip]\nmod {};\npub use {}::*;", m, m).unwrap();
            }
        };
    }

    let mut config = Config::new();

    config
        .add_types_mapping(protobuf_strict::get_uuids().to_vec(), CustomType::Uuid)
        .strict_messages()
        .inline_enums();

    go_generate!("b_generated", b_generated, config);
    go_generate!("generated", generated, Config::new());
}
