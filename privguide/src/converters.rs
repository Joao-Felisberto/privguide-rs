use std::{fs::File, io::BufReader, path::Path};
use crate::error::{ConversionError, ConversionResult};

pub enum Format {
    JSON,
    // #[cfg(feature="yaml")]
    YAML,
}

pub fn convert_file(path: &Path) -> ConversionResult {
    if let Some(ext) = path.extension() {
        match ext.to_str() {
            Some("json") => from_json(path),
            Some("yaml") | Some("yml") => from_yaml(path),
            Some(other) => ConversionResult::Err(ConversionError::UnsupportedExtensionError(other.to_string())),
            None => ConversionResult::Err(ConversionError::UnsupportedExtensionError("".to_string())),
        }
    } else {
        ConversionResult::Err(ConversionError::UnsupportedExtensionError("".to_string()))
    }
}

pub fn convert_file_from_format(path: &Path, format: Format) -> ConversionResult {
    match format {
        Format::JSON => from_json(path),
        Format::YAML=> from_yaml(path),
    }
}

fn from_json(path: &Path) -> ConversionResult {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let res = serde_json::from_reader(reader)?;
    Ok(res)
}

fn from_yaml(path: &Path) -> ConversionResult {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let res = serde_yaml_ng::from_reader(reader)?;
    Ok(res)
}
