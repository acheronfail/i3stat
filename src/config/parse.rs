use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;

use figment::error::Kind;
use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;

use crate::cli::Cli;
use crate::config::AppConfig;

pub fn parse(args: &Cli) -> Result<AppConfig, Box<dyn Error>> {
    let cfg_file = args
        .config
        .as_ref()
        .map(|p| p.to_owned())
        .or_else(|| dirs::config_dir().map(|d| d.join("istat/config")))
        .ok_or_else(|| "failed to find config file")?;

    let cfg_dir = cfg_file
        .parent()
        .ok_or_else(|| "failed to find config dir")?;

    // main configuration file
    let mut figment = Figment::new()
        .merge(Toml::file(cfg_file.with_extension("toml")))
        .merge(Json::file(cfg_file.with_extension("json")))
        .merge(Yaml::file(cfg_file.with_extension("yaml")))
        .merge(Yaml::file(cfg_file.with_extension("yml")));

    // parse any additional config files
    let figment = {
        let mut seen_config_files = HashSet::new();
        seen_config_files.insert(cfg_file.clone());
        loop {
            let include_paths = match figment.extract_inner::<Vec<PathBuf>>("include") {
                // we got some include paths, make them relative to the main config file
                Ok(paths) => paths
                    .into_iter()
                    .map(|p| cfg_dir.join(p).canonicalize())
                    .collect::<Result<Vec<_>, _>>()?,
                // ignore if "include" wasn't specified at all
                Err(e) if matches!(e.kind, Kind::MissingField(_)) => vec![],
                // some other error occurred
                Err(e) => bail!(e),
            };

            if include_paths.iter().all(|p| seen_config_files.contains(p)) {
                break figment;
            }

            for include in include_paths {
                match include.extension().and_then(OsStr::to_str) {
                    Some("toml") => figment = figment.admerge(Toml::file(&include)),
                    Some("json") => figment = figment.admerge(Json::file(&include)),
                    Some("yaml") | Some("yml") => figment = figment.admerge(Yaml::file(&include)),
                    Some(e) => bail!("Unsupported file extension: {}", e),
                    None => bail!("No file extension, cannot infer file format"),
                }

                seen_config_files.insert(include);
            }
        }
    };

    Ok(figment.extract::<AppConfig>()?)
}
