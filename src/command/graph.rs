use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use petgraph::{
    dot::{self, Dot},
    graph::NodeIndex,
    Graph,
};

use crate::{args::GraphArgs, clipboard, dir};

pub fn print_graph(args: GraphArgs) {
    let graph = get_graph();

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

fn get_parent(path: &Utf8PathBuf) -> Utf8PathBuf {
    let curr_dir = dir::current_dir();
    let parent = path.parent().unwrap();
    parent.strip_prefix(&curr_dir).unwrap().to_path_buf()
}

pub fn get_graph() -> Graph<Utf8PathBuf, i32> {
    let mut graph: Graph<Utf8PathBuf, i32> = Graph::new();
    // Collection of `file` - `graph index`.
    let mut indices = HashMap::<Utf8PathBuf, NodeIndex>::new();
    let files = get_all_files_tf_and_hcl_files();
    for f in files {
        let f_parent = get_parent(&f);
        let node_index = indices
            .get(&f_parent)
            .cloned()
            .unwrap_or_else(|| add_node(&mut graph, f_parent, &mut indices));
        let dependencies = get_dependencies(&f);
        for d in dependencies {
            let existing_index = indices.get(&d);

            if let Some(&existing_index) = existing_index {
                graph.add_edge(node_index, existing_index, 0);
            } else {
                let d_index = add_node(&mut graph, d, &mut indices);
                graph.add_edge(node_index, d_index, 0);
            }
        }
    }
    graph
}

fn add_node(
    graph: &mut Graph<Utf8PathBuf, i32>,
    dir: Utf8PathBuf,
    indices: &mut HashMap<Utf8PathBuf, NodeIndex>,
) -> NodeIndex {
    let node_index = graph.add_node(dir.clone());
    indices.insert(dir.to_path_buf(), node_index);
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
            dependencies.push(module_path);
        }
    }
    dependencies
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
    Some(tokens[2].trim_matches('"'))
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