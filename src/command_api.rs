use crate::config_file::load_config_db;
use crate::config_file::JuliaupConfigChannel;
use crate::utils::{get_juliaupconfig_path, parse_versionstring};
use anyhow::{anyhow, Context, Result};
use normpath::PathExt;
use serde::{Deserialize, Serialize};
use semver::Version;

#[derive(Serialize, Deserialize, Clone)]
pub struct JuliaupChannelInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "File")]
    pub file: String,
    #[serde(rename = "Args")]
    pub args: Vec<String>,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Arch")]
    pub arch: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JuliaupApiGetinfoReturn {
    #[serde(rename = "DefaultChannel")]
    pub default: Option<JuliaupChannelInfo>,
    #[serde(rename = "OtherChannels")]
    pub other_versions: Vec<JuliaupChannelInfo>,
}

pub fn run_command_api(command: String) -> Result<()> {
    if command != "getconfig1" {
        return Err(anyhow!("Wrong API command."));
    }

    let foo = get_juliaupconfig_path()?;

    let julia_bin_root_path = foo.parent().unwrap();

    let mut ret_value = JuliaupApiGetinfoReturn {
        default: None,
        other_versions: Vec::new(),
    };

    let config_data =
        load_config_db().with_context(|| "`status` command failed to load configuration file.")?;

    for (key, value) in config_data.installed_channels {
        let curr = match value {
            JuliaupConfigChannel::SystemChannel {
                version: fullversion,
            } => {
                let (platform, mut version) = parse_versionstring(&fullversion).unwrap();

                version.build = semver::BuildMetadata::EMPTY;

                match config_data.installed_versions.get(&fullversion) {
                    Some(channel) => JuliaupChannelInfo {
                        name: key.clone(),
                        file: julia_bin_root_path.join(&channel.path).join("bin").join(format!("julia{}", std::env::consts::EXE_SUFFIX)).normalize()?.as_path().to_str().unwrap().to_string().clone(),
                        args: Vec::new(),
                        version: version.to_string(),
                        arch: platform
                    },
                    None => return Err(anyhow!("The channel '{}' is configured as a system channel, but no such channel exists in the versions database.", key))
                }
            }
            JuliaupConfigChannel::LinkedChannel { command, args } => 
            {
                let mut new_args: Vec<String> = Vec::new();

                for i in args.as_ref().unwrap() {
                    new_args.push(i.to_string());
                }

                new_args.push("--version".to_string());

                let res = std::process::Command::new(&command).args(&new_args).output();

                match res {
                    Ok(output) => {
                        let expected_version_prefix = "julia version ";

                        let trimmed_string = std::str::from_utf8(&output.stdout).unwrap().trim();

                        if !trimmed_string.starts_with(expected_version_prefix) {
                            continue
                        }

                        let version = Version::parse(&trimmed_string[expected_version_prefix.len()..])?;

                        JuliaupChannelInfo {
                            name: key.clone(),
                            file: command.clone(),
                            args: args.unwrap_or(Vec::new()),
                            version: version.to_string(),
                            arch: "".to_string(),
                        }
                    },
                    Err(_) => continue
                }
            },
        };

        match config_data.default {
            Some(ref default_value) => {
                if &key == default_value {
                    ret_value.default = Some(curr.clone());
                } else {
                    ret_value.other_versions.push(curr);
                }
                ()
            }
            None => {
                ret_value.other_versions.push(curr);
                ()
            }
        }
    }

    // Serialize it to a JSON string.
    let j = serde_json::to_string(&ret_value)?;

    // Print, write to a file, or send to an HTTP server.
    println!("{}", j);

    Ok(())
}
