use std::env;
use std::process::ExitCode;

const NODE_2_HELP: &str = include_str!("../fixtures/node-oracle/help.txt");

fn main() -> ExitCode {
    match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [argument] if argument == "--help" || argument == "-h" => {
            print!("{NODE_2_HELP}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("当前构建仅实现 create-yss-spec --help oracle");
            ExitCode::from(2)
        }
    }
}
