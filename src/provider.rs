use std::collections::BTreeMap;

use semver::Version;

use crate::command::upgrade_provider::Provider;

async fn get_latest_version(provider: &str) -> anyhow::Result<Version> {
    #[derive(serde::Deserialize)]
    struct ProviderJson {
        version: String,
    }

    let url = format!("https://registry.terraform.io/v1/providers/{}", provider);
    let response: ProviderJson = reqwest::get(&url).await?.json().await?;

    let version = semver::Version::parse(&response.version)?;
    Ok(version)
}

pub async fn outdated_providers(
    providers: BTreeMap<String, Vec<Provider>>,
) -> anyhow::Result<BTreeMap<String, Vec<Provider>>> {
    let mut outdated = BTreeMap::new();
    for (provider_name, providers) in providers {
        let provider_name = provider_name
            .strip_prefix("registry.terraform.io/")
            .expect("invalid provider name")
            .to_string();
        println!("Checking latest version for provider: {}", provider_name);
        let latest_version = get_latest_version(&provider_name).await?;
        let outdated_lockfiles: Vec<Provider> = providers
            .into_iter()
            .filter(|p| p.version != latest_version)
            .collect();
        if !outdated_lockfiles.is_empty() {
            outdated.insert(provider_name, outdated_lockfiles);
        }
    }
    Ok(outdated)
}
