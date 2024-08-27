use std::collections::BTreeSet;

use camino::Utf8PathBuf;
use petgraph::dot::{self, Dot};

use crate::{args::GraphArgs, clipboard, dir, graph::ModulesGraph, provider};

pub async fn print_graph(args: GraphArgs) {
    assert!(dir::current_dir_is_simpleinfra());

    let outdated_packages = if args.outdated {
        Some(get_packages_with_outdated_providers().await)
    } else {
        None
    };

    let graph = ModulesGraph::new(outdated_packages.as_ref());

    // Get `graphviz` format
    let output_str = format!(
        "{:?}",
        Dot::with_config(&graph.graph, &[dot::Config::EdgeNoLabel])
    );
    println!("{:?}", output_str);

    if args.clipboard {
        clipboard::copy_to_clipboard(output_str);
    }
}

async fn get_packages_with_outdated_providers() -> BTreeSet<Utf8PathBuf> {
    let lockfiles = provider::get_all_lockfiles();
    let providers = provider::get_all_providers(&lockfiles);
    let outdated_providers = provider::outdated_providers(providers).await.unwrap();

    let mut outdated_packages = BTreeSet::new();
    for (_provider, versions) in outdated_providers.providers {
        for (_version, lockfiles) in versions.versions {
            let parents = lockfiles
                .iter()
                .map(dir::get_stripped_parent)
                .collect::<Vec<_>>();
            outdated_packages.extend(parents);
        }
    }
    outdated_packages
}
