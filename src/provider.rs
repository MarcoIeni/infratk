use std::collections::BTreeMap;

use semver::Version;

use crate::command::upgrade_provider::{ProviderVersions, Providers};

async fn get_latest_version(provider: &str) -> anyhow::Result<Version> {
    #[derive(serde::Deserialize)]
    struct ProviderJson {
        version: String,
        published_at: String,
    }

    let url = format!("https://registry.terraform.io/v1/providers/{}", provider);
    let response: ProviderJson = reqwest::get(&url).await?.json().await?;

    let version = semver::Version::parse(&response.version)?;
    println!("- {} - `{provider}`: {version} \t", response.published_at);
    Ok(version)
}

pub async fn outdated_providers(providers: Providers) -> anyhow::Result<Providers> {
    let mut outdated = BTreeMap::new();
    println!("latest providers versions:");
    for (provider_name, provider_versions) in providers.providers {
        let latest_version = get_latest_version(&provider_name).await?;
        let mut outdated_versions = BTreeMap::new();
        for (version, lockfiles) in provider_versions.versions {
            if version != latest_version {
                outdated_versions.insert(version.clone(), lockfiles);
            }
        }
        if !outdated_versions.is_empty() {
            outdated.insert(
                provider_name,
                ProviderVersions {
                    versions: outdated_versions,
                },
            );
        }
    }
    Ok(Providers {
        providers: outdated,
    })
}