use clang::{Clang, Entity, Index};
use clap::{App, Arg, SubCommand};
use regex::Regex;
use std::{
    fs::OpenOptions,
    io::{stdout, Result, Write},
};
use tap::TapOps;

mod ast;
mod classifier;
mod dump_entity;
mod ruby;
mod utils;
use ast::{dump_ast, SourceFile};
use classifier::{dump_classifier, Class, Classifier};
use dump_entity::dump_entity;
use ruby::GenerateBindings as GenerateRubyBindings;

fn main() -> Result<()> {
    let matches = App::new("Qiniu C Translator")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS").split(':').last().unwrap())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("header-file")
                .long("header-file")
                .required(true)
                .value_name("FILE")
                .help("To generate bindings")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("generate-ruby-bindings")
                .about("Generate Ruby bindings code")
                .arg(
                    Arg::with_name("output")
                        .long("output")
                        .value_name("FILE")
                        .help("Output ruby code to file")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("dump-entity")
                .about("Show clang entities, only for debug")
                .arg(
                    Arg::with_name("pretty-print")
                        .long("pretty")
                        .help("Pretty-printed output"),
                ),
        )
        .subcommand(SubCommand::with_name("dump-classifier").about("Show classifier, only for debug"))
        .subcommand(
            SubCommand::with_name("dump-ast").about("Show ast, only for debug").arg(
                Arg::with_name("pretty-print")
                    .long("pretty")
                    .help("Pretty-printed output"),
            ),
        )
        .get_matches();
    let cl = Clang::new().unwrap();
    let idx = Index::new(&cl, true, false);
    let tu = {
        let header_file_path = matches.value_of_os("header-file").unwrap();
        if let Err(err) = OpenOptions::new().read(true).open(&header_file_path) {
            panic!("Failed to open header file: {}", err);
        }
        idx.parser(header_file_path).parse().unwrap()
    };
    let entity = tu.get_entity();
    match matches.subcommand() {
        ("generate-ruby-bindings", args) => GenerateRubyBindings::default()
            // 这里目前是写死的模块路径，如果有需要可以改为参数配置
            .module_names(["QiniuNg".into(), "Bindings".into()])
            .version_constant("QiniuNg::VERSION")
            .build(
                &entity,
                make_classifier(&entity),
                &mut args
                    .and_then(|args| args.value_of_os("output"))
                    .and_then(|path| if path == "-" { None } else { Some(path) })
                    .map(|file_path| {
                        let output: Box<dyn Write> = Box::new(
                            OpenOptions::new()
                                .write(true)
                                .truncate(true)
                                .create(true)
                                .open(file_path)?,
                        );
                        Ok(output) as Result<Box<dyn Write>>
                    })
                    .unwrap_or_else(|| Ok(Box::new(stdout())))?,
            )?,
        ("dump-entity", args) => dump_entity(
            &entity,
            args.map(|args| args.is_present("pretty-print")).unwrap_or(false),
        )?,

        ("dump-ast", args) => dump_ast(
            &entity,
            args.map(|args| args.is_present("pretty-print")).unwrap_or(false),
        ),
        ("dump-classifier", _) => dump_classifier(&make_classifier(&entity))?,
        ("", _) => {}
        (subcommand, _) => panic!("Unrecognized subcommand: {}", subcommand),
    }

    Ok(())
}

fn make_classifier(entity: &Entity) -> Classifier {
    let source_file = SourceFile::parse(&entity);
    Classifier::default().tap(|classifier| {
        classifier.add_class(Class::new(
            "Str",
            Regex::new("^qiniu_ng_str_(\\w+)").unwrap(),
            Some(Regex::new("^qiniu_ng_str_(list|map)_").unwrap()),
            source_file.function_declarations().iter(),
            None,
            None,
        ))
    })
}
