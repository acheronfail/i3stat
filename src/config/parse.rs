use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use figment::error::Kind;
use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use wordexp::{wordexp, Wordexp};

use crate::cli::Cli;
use crate::config::AppConfig;
use crate::error::Result;

fn expand_include_path(s: impl AsRef<str>, cfg_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let cfg_dir = cfg_dir.as_ref();
    // perform expansion, see: man 3 wordexp
    Ok(wordexp(s.as_ref(), Wordexp::new(0), 0)?
        .map(|path| -> Result<_> {
            // convert expansion to path
            let path = PathBuf::from(path);
            if path.is_absolute() {
                // if it's already absolute, keep it
                Ok(path)
            } else {
                // if it's not absolute, assume relative to `cfg_dir` and attempt to resolve
                let joined = cfg_dir.join(path);
                match joined.canonicalize() {
                    Ok(p) => Ok(p),
                    Err(e) => bail!("failed to resolve {}: {}", joined.display(), e),
                }
            }
        })
        .collect::<Result<_>>()?)
}

pub fn parse(args: &Cli) -> Result<AppConfig> {
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
        let mut seen = HashSet::new();
        seen.insert(cfg_file.clone());

        // as long as we get more include paths, keep parsing
        loop {
            let include_paths = match figment.extract_inner::<Vec<String>>("include") {
                // we got some include paths
                Ok(user_paths) => {
                    let mut paths = vec![];
                    for unexpanded in user_paths {
                        paths.extend(expand_include_path(unexpanded, &cfg_dir)?);
                    }

                    paths
                }
                // ignore if "include" wasn't specified at all
                Err(e) if matches!(e.kind, Kind::MissingField(_)) => vec![],
                // some other error occurred
                Err(e) => bail!(e),
            };

            if include_paths.iter().all(|p| seen.contains(p)) {
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

                seen.insert(include);
            }
        }
    };

    Ok(figment.extract::<AppConfig>()?)
}
