#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::collection_is_never_read)]
#![warn(clippy::pedantic, clippy::nursery)]

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::ops::Index;
use std::path::{Path, PathBuf};
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

        Ok(self)
    }
}

#[derive(EnumString, Copy, Clone, Eq, PartialEq, Debug)]
#[strum(serialize_all = "camelCase")]
enum BuildMode {
    Dynamic,
    Static,
    Spoiler
}

struct BuildContext<'ctx> {
    mode: BuildMode,
    input_file: &'ctx Path,
}

trait PreProcessor {
    fn transform(&self, build_context: &BuildContext<'_>, content: String) -> String;
}

/**
* insert inter-link or file content directly
* tag syntax: {{link or include|&lt;relative path of markdown from root document&gt;}}
*/
struct LinkOrInclude;

impl PreProcessor for LinkOrInclude {
    fn transform(&self, build_context: &BuildContext<'_>, input_content: String) -> String {
        let pattern = Regex::from_str(r#"\{\{link or include\|./((?:\w+/)+)(\w+\.md)\}\}"#).unwrap();
        pattern.replace_all(input_content.as_str(), |captures: &Captures| {
            let file_path = captures.index(1);
            let file_name = captures.index(2);
            println!("including: {file_path}/{file_name}");
            let full_file_path = format!("{file_path}{file_name}");
            match build_context.mode {
                BuildMode::Dynamic => format!("This section is migrated. Please see [{file_name}](./{full_file_path})"),
                BuildMode::Spoiler => {
                    let mut cloned_path = build_context.input_file.to_path_buf();
                    cloned_path.pop();
                    let target_path = cloned_path.join(file_path).join(file_name);
                    let mut pasting_text = String::from("<details><summary>");
                    pasting_text.push_str("content of ");
                    pasting_text.push_str(target_path.to_str().unwrap_or("included file"));
                    pasting_text.push_str("</summary>\n\n");

                    println!("{target_path}", target_path = &target_path.to_str().unwrap());
                    let target_path = target_path.canonicalize().unwrap();
                    println!("{path}", path = &target_path.to_str().unwrap());
                    let mut fd = BufReader::new(File::open(target_path).expect("Inclusion target file open error"));
                    let mut buf = String::new();
                    fd.read_to_string(&mut buf).unwrap();

                    let including_text = {
                        let include_pat = Regex::from_str(r#"<!-- START -->\n?((.|\n)*)<!-- END -->"#).unwrap();
                        let ret = include_pat.captures_iter(buf.as_str()).map(|a| {
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

                    pasting_text.push_str(&including_text);
                    pasting_text.push_str("\n</details>");

                    pasting_text
                }
                BuildMode::Static => {
                    let mut cloned_path = build_context.input_file.to_path_buf();
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
                        let ret = include_pat.captures_iter(buf.as_str()).map(|a| {
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
    }
}

/**
 * insert arbitrary file content directly
 * tag syntax: {{include|&lt;relative path from root document&gt;}}
 */
struct AlwaysInclude;

impl PreProcessor for AlwaysInclude {
    fn transform(&self, build_context: &BuildContext<'_>, content: String) -> String {
        let pattern = Regex::from_str(r#"\{\{include\|./((?:\w+/)+)([\w.]+)\}\}"#).unwrap();
        pattern.replace_all(content.as_str(), |captures: &Captures| {
            let file_path = captures.index(1);
            let file_name = captures.index(2);
            println!("including: {file_path}/{file_name}");
            let mut cloned_path = build_context.input_file.to_path_buf();
            cloned_path.pop();
            let target_path = cloned_path.join(file_path).join(file_name);
            println!("{target_path}", target_path = &target_path.to_str().unwrap());
            let target_path = target_path.canonicalize().unwrap();
            println!("{path}", path = &target_path.to_str().unwrap());
            let mut fd = BufReader::new(File::open(target_path).expect("Inclusion target file open error"));
            let mut including_text = String::new();
            fd.read_to_string(&mut including_text).unwrap();

            including_text
        }).to_string()
    }
}

fn main() {
    let mut args: Args = Args::parse().validate().unwrap();
    println!("{args:?}", args = &args);
    let mut input = BufReader::new(File::open(&args.input_file).expect("failed to open input file"));
    let input_content = {
        let mut buf = String::new();
        input.read_to_string(&mut buf).unwrap();
        buf
    };

    let build_context = BuildContext {
        mode: args.build_mode,
        input_file: &mut args.input_file,
    };

    let input_content = LinkOrInclude.transform(&build_context, input_content);
    let input_content = AlwaysInclude.transform(&build_context, input_content);

    let mut output = BufWriter::new(File::options().write(true).create(true).truncate(true).open(args.output_file).unwrap());
    output.write_all(input_content.as_bytes()).expect("could not write output to destination");
}
