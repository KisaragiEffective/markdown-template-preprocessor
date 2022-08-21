use std::borrow::{BorrowMut, Cow};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::ops::Index;
use std::path::PathBuf;
use std::str::FromStr;
use clap::Parser;
use regex::{Captures, Regex};
use strum::EnumString;
use itertools::Itertools;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    input_file: PathBuf,
    #[clap(short = 'm', long)]
    build_mode: BuildMode,
    #[clap(short, long)]
    output_file: PathBuf,
}

impl Args {
    fn validate(self) -> Result<Self, String> {
        if self.input_file.is_dir() {
            return Err("The input path must point to file".to_string())
        }

        if !self.input_file.exists() {
            return Err("The input file must be exist".to_string())
        }

        if self.output_file.is_dir() {
            return Err("The output path must point to file".to_string())
        }

        if !self.output_file.exists() {
            return Err("The output file must be exist".to_string())
        }

        Ok(self)
    }
}

#[derive(EnumString, Copy, Clone, Eq, PartialEq, Debug)]
#[strum(serialize_all = "camelCase")]
enum BuildMode {
    Dynamic,
    Static,
}

fn main() {
    let args: Args = Args::parse().validate().unwrap();
    println!("{args:?}", args = &args);
    let mut input = BufReader::new(File::open(&args.input_file).expect("failed to open input file"));
    let input_content = {
        let mut buf = String::new();
        input.read_to_string(&mut buf).unwrap();
        buf
    };

    let input_content = {
        let mut pattern = Regex::from_str(r#"\{\{link or include\|./((?:\w+/)+)(\w+\.md)\}\}"#).unwrap();
        pattern.replace_all(input_content.as_str(), |captures: &Captures| {
            let file_path = captures.index(1);
            let file_name = captures.index(2);
            println!("including: {file_path} {file_name}");
            let full_file_path = format!("{file_path}{file_name}");
            match args.build_mode {
                BuildMode::Dynamic => format!("This section is migrated. Please see [{file_name}](./{full_file_path})"),
                BuildMode::Static => {
                    let mut cloned_path = args.input_file.clone();
                    cloned_path.pop();
                    let target_path = cloned_path.join(file_path).join(file_name);
                    println!("{target_path}", target_path = &target_path.to_str().unwrap());
                    let target_path = target_path.canonicalize().unwrap();
                    println!("{path}", path = &target_path.to_str().unwrap());
                    let mut fd = BufReader::new(File::open(target_path).expect("Inclusion target file open error"));
                    let mut buf = String::new();
                    fd.read_to_string(&mut buf).unwrap();

                    let including_text = {
                        let include_pat = Regex::from_str(r#"<!-- START -->\n?((.|\n)*)<!-- END -->"#).unwrap();
                        let cap_iter = include_pat.captures_iter(buf.as_str());
                        let ret = cap_iter.map(|a| {
                            let to_include = a.index(1).to_string();
                            // lower header level by one
                            let header_pat = Regex::from_str("(?m)^(#{1,5})(.*)$").unwrap();
                            let to_include = header_pat.replace_all(to_include.as_str(), |cap: &Captures| {
                                format!("#{header}{headline_text}", header = cap.index(1), headline_text = cap.index(2))
                            }).to_string();

                            to_include
                        }).join("");
                        ret
                    };

                    including_text
                }
            }
        }).to_string()
    };

    let mut output = BufWriter::new(File::options().write(true).create(true).truncate(true).open(args.output_file).unwrap());
    output.write_all(input_content.as_bytes()).expect("could not write output to destination");
}
