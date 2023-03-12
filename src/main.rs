mod cli;
mod xml_to_json;

use std::fs::{read_dir, File};
use std::io::BufReader;

use anyhow::Context;
use rust_stemmers::{Algorithm, Stemmer};

use cli::get_args;
use xml_to_json::{parse_xml, Config};

fn main() -> anyhow::Result<()> {
    let cli = get_args();
    let input_directory = read_dir(&cli.input)?;
    let config = Config {
        stemmer: Some(Stemmer::create(Algorithm::English)),
        ignore_attributes: vec!["schemaLocation".to_owned()],
    };

    for file in input_directory {
        let file = file?;
        if !file.file_type()?.is_file() {
            continue;
        }

        let path = file.path();
        let Some(stem) = path.file_stem() else { continue; };
        if !matches!(path.extension(), Some(ext) if ext == "xml") {
            continue;
        }

        let mut reader = BufReader::new(File::open(&path)?);
        let json = parse_xml(&mut reader, &config)
            .with_context(|| format!("Failed while parsing file: {}", path.display()))?;

        let output_path = cli.output.join(stem).with_extension("json");
        let mut output_file = File::create(&output_path)?;

        serde_json::to_writer_pretty(&mut output_file, &json["tsResponse"])?;
        println!("File written to: {}", output_path.display());
    }

    Ok(())
}
