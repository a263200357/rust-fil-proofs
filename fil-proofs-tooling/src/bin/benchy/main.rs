//requires nightly, or later stable version
//#![warn(clippy::unwrap_used)]

use std::io::{stdin, stdout};
use std::str::FromStr;

use anyhow::Result;
use byte_unit::Byte;
use clap::{value_t, App, AppSettings, Arg, SubCommand};

use storage_proofs_core::api_version::ApiVersion;

use crate::prodbench::ProdbenchInputs;

mod hash_fns;
mod merkleproofs;
mod prodbench;
mod window_post;
mod winning_post;
mod aggregate_proof;

fn main() -> Result<()> {
    fil_logger::init();

    let window_post_cmd = SubCommand::with_name("window-post")
        .about("Benchmark Window PoST")
        .arg(
            Arg::with_name("preserve-cache")
                .long("preserve-cache")
                .required(false)
                .help("Preserve the directory where cached files are persisted")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("skip-precommit-phase1")
                .long("skip-precommit-phase1")
                .required(false)
                .help("Skip precommit phase 1")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("skip-precommit-phase2")
                .long("skip-precommit-phase2")
                .required(false)
                .help("Skip precommit phase 2")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("skip-commit-phase1")
                .long("skip-commit-phase1")
                .required(false)
                .help("Skip commit phase 1")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("skip-commit-phase2")
                .long("skip-commit-phase2")
                .required(false)
                .help("Skip commit phase 2")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("test-resume")
                .long("test-resume")
                .required(false)
                .help("Test replication resume")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("cache")
                .long("cache")
                .required(false)
                .help("The directory where cached files are persisted")
                .default_value("")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("size")
                .long("size")
                .required(true)
                .help("The data size (e.g. 2KiB)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("api_version")
                .long("api-version")
                .required(true)
                .help("The api_version to use (default: 1.0.0)")
                .default_value("1.0.0")
                .takes_value(true),
        );

    let winning_post_cmd = SubCommand::with_name("winning-post")
        .about("Benchmark Winning PoST")
        .arg(
            Arg::with_name("size")
                .long("size")
                .required(true)
                .help("The data size (e.g. 2KiB)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("api_version")
                .long("api-version")
                .required(true)
                .help("The api_version to use (default: 1.0.0)")
                .default_value("1.0.0")
                .takes_value(true),
        );

    let hash_cmd = SubCommand::with_name("hash-constraints")
        .about("Benchmark hash function inside of a circuit");

    let prodbench_cmd = SubCommand::with_name("prodbench")
        .about("Benchmark prodbench")
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .required(false)
                .help("path to config.json"),
        )
        .arg(
            Arg::with_name("skip-seal-proof")
                .long("skip-seal-proof")
                .takes_value(false)
                .help("skip generation (and verification) of seal proof"),
        )
        .arg(
            Arg::with_name("skip-post-proof")
                .long("skip-post-proof")
                .takes_value(false)
                .help("skip generation (and verification) of PoSt proof"),
        )
        .arg(
            Arg::with_name("only-replicate")
                .long("only-replicate")
                .takes_value(false)
                .help("only run replication"),
        )
        .arg(
            Arg::with_name("only-add-piece")
                .long("only-add-piece")
                .takes_value(false)
                .help("only run piece addition"),
        );

    let merkleproof_cmd = SubCommand::with_name("merkleproofs")
        .about("Benchmark merkle proof generation")
        .arg(
            Arg::with_name("size")
                .long("size")
                .required(true)
                .help("The data size (e.g. 2KiB)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("proofs")
                .long("proofs")
                .default_value("1024")
                .required(false)
                .help("How many proofs to generate (default is 1024)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("validate")
                .long("validate")
                .required(false)
                .default_value("true")
                .help("Validate proofs if specified")
                .takes_value(false),
        );

    let agg_proof_cmd = SubCommand::with_name("aggregate-proof")
        .about("Benchmark Aggregate Window PoST Proofs")
        .arg(
            Arg::with_name("num_agg")
                .long("num_agg")
                .required(true)
                .default_value("128")
                .help("How many window-post proofs to aggregate (default is 128)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("size")
                .long("size")
                .required(true)
                .help("The data size (e.g. 2KiB)")
                .takes_value(true),
        );

    let matches = App::new("benchy")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version("0.1")
        .subcommand(window_post_cmd)
        .subcommand(winning_post_cmd)
        .subcommand(hash_cmd)
        .subcommand(prodbench_cmd)
        .subcommand(merkleproof_cmd)
        .subcommand(agg_proof_cmd)
        .get_matches();

    match matches.subcommand() {
        ("window-post", Some(m)) => {
            let preserve_cache = m.is_present("preserve-cache");
            // For now these options are combined.
            let skip_precommit_phase1 = m.is_present("skip-precommit-phase1");
            let skip_precommit_phase2 = m.is_present("skip-precommit-phase2");
            let skip_commit_phase1 = m.is_present("skip-commit-phase1");
            let skip_commit_phase2 = m.is_present("skip-commit-phase2");
            let test_resume = m.is_present("test-resume");
            let cache_dir = value_t!(m, "cache", String)?;
            let sector_size = Byte::from_str(value_t!(m, "size", String)?)?.get_bytes() as usize;
            let api_version = ApiVersion::from_str(&value_t!(m, "api_version", String)?)?;
            window_post::run(
                sector_size,
                api_version,
                cache_dir,
                preserve_cache,
                skip_precommit_phase1,
                skip_precommit_phase2,
                skip_commit_phase1,
                skip_commit_phase2,
                test_resume,
            )?;
        }
        ("winning-post", Some(m)) => {
            let sector_size = Byte::from_str(value_t!(m, "size", String)?)?.get_bytes() as usize;
            let api_version = ApiVersion::from_str(&value_t!(m, "api_version", String)?)?;
            winning_post::run(sector_size, api_version)?;
        }
        ("hash-constraints", Some(_m)) => {
            hash_fns::run()?;
        }
        ("merkleproofs", Some(m)) => {
            let size = Byte::from_str(value_t!(m, "size", String)?)?.get_bytes() as usize;

            let proofs = value_t!(m, "proofs", usize)?;
            merkleproofs::run(size, proofs, m.is_present("validate"))?;
        }
        ("prodbench", Some(m)) => {
            let inputs: ProdbenchInputs = if m.is_present("config") {
                let file = value_t!(m, "config", String).expect("failed to get config");
                serde_json::from_reader(
                    std::fs::File::open(&file)
                        .unwrap_or_else(|_| panic!("invalid file {:?}", file)),
                )
            } else {
                serde_json::from_reader(stdin())
            }
            .expect("failed to deserialize stdin to ProdbenchInputs");

            let outputs = prodbench::run(
                inputs,
                m.is_present("skip-seal-proof"),
                m.is_present("skip-post-proof"),
                m.is_present("only-replicate"),
                m.is_present("only-add-piece"),
            );

            serde_json::to_writer(stdout(), &outputs)
                .expect("failed to write ProdbenchOutput to stdout")
        }
        ("aggregate-proof", Some(m)) => {
            let nums = Byte::from_str(value_t!(m, "num_agg", String)?)?.get_bytes() as usize;
            let sector_size = Byte::from_str(value_t!(m, "size", String)?)?.get_bytes() as usize;
            aggregate_proof::run(sector_size, nums)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
