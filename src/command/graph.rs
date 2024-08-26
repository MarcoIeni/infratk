use std::collections::{BTreeSet, HashMap};

use camino::{Utf8Path, Utf8PathBuf};
use petgraph::{
    dot::{self, Dot},
    graph::NodeIndex,
    Graph,
};
use tracing::warn;

use crate::{args::GraphArgs, clipboard, dir, provider};

pub async fn print_graph(args: GraphArgs) {
    assert!(dir::current_dir_is_simpleinfra());

    let outdated_packages = if args.outdated {
        Some(get_packages_with_outdated_providers().await)
    } else {
        None
    };

    let graph = get_graph(outdated_packages.as_ref());

    // Get `graphviz` format
    let output_str = format!(
        "{:?}",
        Dot::with_config(&graph, &[dot::Config::EdgeNoLabel])
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
            let parents = lockfiles.iter().map(get_parent).collect::<Vec<_>>();
            outdated_packages.extend(parents);
        }
    }
    outdated_packages
}

fn get_parent(path: &Utf8PathBuf) -> Utf8PathBuf {
    let parent = path.parent().unwrap();
    dir::strip_current_dir(parent)
}

pub fn get_graph(outdated_packages: Option<&BTreeSet<Utf8PathBuf>>) -> Graph<Utf8PathBuf, i32> {
    let mut graph: Graph<Utf8PathBuf, i32> = Graph::new();
    // Collection of `file` - `graph index`.
    let mut indices = HashMap::<Utf8PathBuf, NodeIndex>::new();
    let files = get_all_files_tf_and_hcl_files();
    for f in files {
        let f_parent = get_parent(&f);
        let node_index = indices
            .get(&f_parent)
            .cloned()
            .unwrap_or_else(|| add_node(&mut graph, f_parent, &mut indices, outdated_packages));
        let dependencies = get_dependencies(&f);
        for d in dependencies {
            let d_index = indices
                .get(&d)
                .cloned()
                .unwrap_or_else(|| add_node(&mut graph, d, &mut indices, outdated_packages));

            graph.update_edge(node_index, d_index, 0);
        }
    }
    graph
}

fn add_node(
    graph: &mut Graph<Utf8PathBuf, i32>,
    dir: Utf8PathBuf,
    indices: &mut HashMap<Utf8PathBuf, NodeIndex>,
    outdated_packages: Option<&BTreeSet<Utf8PathBuf>>,
) -> NodeIndex {
    let label = if let Some(outdated_packages) = outdated_packages {
        // add an emoji to the path just for the graph visualization.
        if outdated_packages.contains(&dir) {
            dir.join(" ✅")
        } else {
            dir.join(" ❌")
        }
    } else {
        dir.clone()
    };
    let node_index = graph.add_node(label.clone());
    indices.insert(dir, node_index);
    node_index
}

/// Get the dependencies of a file
/// Dependencies are anything in the file like `source = "path"` or `config_path = "path"`.
fn get_dependencies(file: &Utf8Path) -> Vec<Utf8PathBuf> {
    let content = std::fs::read_to_string(file).expect("could not read file");
    let mut dependencies = vec![];
    for line in content.lines() {
        if let Some(dependency) = get_dependency_from_line(line) {
            let module_path = file.parent().unwrap().join(dependency);
            let relative_path = get_relative_path(&module_path);
            dependencies.push(relative_path);
        }
    }
    dependencies
}

fn get_relative_path(path: &Utf8Path) -> Utf8PathBuf {
    // canonicalize to convert `a/b/../c` to `a/c`
    let canonicalized = match path.canonicalize_utf8() {
        Ok(c) => c,
        Err(err) => {
            warn!("Could not canonicalize path {path}: {err:?}");
            path.to_path_buf()
        }
    };
    dir::strip_current_dir(&canonicalized)
}

fn get_dependency_from_line(line: &str) -> Option<&str> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let first_token = *tokens.first()?;
    if first_token != "source" && first_token != "config_path" {
        return None;
    }
    let second_token = *tokens.get(1)?;
    if second_token != "=" {
        return None;
    }
    let third_token = tokens[2].trim_matches('"');
    if !third_token.starts_with(".") {
        // it's not a directory. E.g. it's `source  = "hashicorp/aws"`.
        return None;
    }

    Some(third_token)
}

/// Get all the files that might contain a dependency
pub fn get_all_files_tf_and_hcl_files() -> Vec<Utf8PathBuf> {
    let mut files = vec![];
    let current_dir = dir::current_dir();
    let walker = ignore::WalkBuilder::new(current_dir)
        // Read hidden files
        .hidden(false)
        .build();

    for entry in walker {
        let entry = entry.expect("invalid entry");
        let file_type = entry.file_type().expect("unknown file type");
        if !file_type.is_dir()
            && (entry.path().extension() == Some("tf".as_ref())
                || entry.path().extension() == Some("hcl".as_ref()))
        {
            let path = entry.path().to_path_buf();
            let utf8path = Utf8PathBuf::from_path_buf(path).unwrap();
            files.push(utf8path);
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino_tempfile::NamedUtf8TempFile;

    #[test]
    fn dependencies_are_read() {
        let file = NamedUtf8TempFile::new().unwrap();
        let content = r#"
                        source = "../aaaa"
                "#;
        fs_err::write(file.path(), content).unwrap();
        let dependencies = get_dependencies(file.path());
        assert_eq!(dependencies.len(), 1);
    }
}
