use colored::{control, Colorize};
use ptree::{
    print_tree_with,
    style::{Color, Style},
    PrintConfig, TreeBuilder,
};
use rayon::prelude::*;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use walkdir::WalkDir;
use itertools::Itertools;

// Map with the directory as key, and as value
// (Vec<children_directories>, inode_count, updated)
type NodeMap = BTreeMap<PathBuf, (Vec<PathBuf>, usize, bool)>;

#[derive(Debug, StructOpt)]
#[structopt(name = "icounter", about = "Count inodes in a directory structure.")]
struct Opt {
    /// Count hidden files
    #[structopt(short, long)]
    show_hidden: bool,

    /// Show percentage of total inode count for each directory
    #[structopt(short = "p", long)]
    show_percent: bool,

    /// Do print with colored output
    #[structopt(short, long)]
    ignore_colors: bool,

    /// Max depth to display counts per directory
    #[structopt(short, long, default_value = "0")]
    depth: usize,

    /// Root to count inodes from
    root: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    if opt.ignore_colors {
        control::set_override(false);
    }

    // Enable parallelism on children regardless of chosen display depth
    let max_depth = opt.depth.max(1);

    let mut map: NodeMap = BTreeMap::new();
    map.insert(opt.root.clone(), (vec![], 1, false));

    let mut to_count = vec![];

    for entry in WalkDir::new(opt.root.clone())
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| opt.show_hidden || !is_hidden(e))
        .flatten()
    {
        if entry.path() == opt.root {
            continue;
        }
        if entry.path().is_dir() {
            map.insert(entry.path().to_owned(), (vec![], 1, false));
            if let Some(parent) = entry.path().parent() {
                map.get_mut(parent).unwrap().0.push(entry.path().to_owned())
            }
            if entry.depth() == max_depth {
                to_count.push(entry.path().to_owned())
            }
        } else {
            map.get_mut(entry.path().parent().unwrap()).unwrap().1 += 1;
        }
    }

    let counts: Vec<_> = to_count
        .par_iter()
        .map(move |entry| {
            let count = count_dir_inodes(entry, opt.show_hidden).unwrap();
            (entry, count)
        })
        .collect();

    for (entry, count) in counts {
        let child = map.get_mut(entry).unwrap();
        child.1 += count;
        child.2 = true;
    }

    update_node(&mut map, &opt.root);

    let root_name = opt.root.file_name().unwrap().to_str().unwrap();
    let root_node = map.get(&opt.root).unwrap().clone();
    let root_string = format_node(root_name, root_node.1, 100., opt.show_percent);

    let config = if opt.ignore_colors {
        PrintConfig::default()
    } else {
        let mut config = PrintConfig::from_env();
        config.branch = Style {
            foreground: Some(Color::Blue),
            ..Style::default()
        };
        config
    };

    if opt.depth == 0 {
        println!("{root_string}");
    } else {
        let mut tree = TreeBuilder::new(root_string);
        for child in root_node.0.iter().sorted_by(|a, b| {
            let count_a = map.get(*a).unwrap().1;
            let count_b = map.get(*b).unwrap().1;
            Ord::cmp(&count_b, &count_a)
        }) {
            print_node(&mut tree, child, &mut map, root_node.1, opt.show_percent);
        }
        print_tree_with(&tree.build(), &config)?;
        println!();
    }

    Ok(())
}

fn format_node(name: &str, count: usize, percent: f32, show_percent: bool) -> String {
    if show_percent {
        format!(
            "{} {} ({})",
            name.bold().blue().underline(),
            format!("{count}").bold().red(),
            format!("{percent:.0}%").yellow()
        )
    } else {
        format!("{} {}", name.bold().blue(), format!("{count}").bold().red())
    }
}

fn update_node(map: &mut NodeMap, root: &Path) -> usize {
    let mut node = map.get_mut(root).unwrap().clone();
    if !node.2 {
        let mut count = node.1;
        for child in node.0.clone() {
            count += update_node(map, &child)
        }
        node.1 = count;
        node.2 = true;
        map.insert(root.to_owned(), node);
        count
    } else {
        node.1
    }
}

fn print_node(
    tree: &mut TreeBuilder,
    root: &Path,
    map: &mut NodeMap,
    total: usize,
    show_percent: bool,
) {
    let count = update_node(map, root);
    let p: f32 = (count as f32 / total as f32) * 100.0;
    let display_name = root.file_name().unwrap().to_str().unwrap();
    tree.begin_child(format_node(display_name, count, p, show_percent));
    for child in map.get(root).unwrap().0.clone().iter().sorted_by(|a, b| {
        let count_a = map.get(*a).unwrap().1;
        let count_b = map.get(*b).unwrap().1;
        Ord::cmp(&count_b, &count_a)
    }) {
        print_node(tree, child, map, total, show_percent);
    }
    tree.end_child();
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

// Counts the number of inodes in a directory
fn count_dir_inodes<P: AsRef<Path>>(root: P, show_hidden: bool) -> io::Result<usize> {
    let mut count = 0;

    let entries: Box<dyn Iterator<Item = walkdir::Result<walkdir::DirEntry>>> = if show_hidden {
        Box::new(WalkDir::new(root).into_iter())
    } else {
        Box::new(
            WalkDir::new(root)
                .into_iter()
                .filter_entry(|e| !is_hidden(e)),
        )
    };

    for entry in entries {
        match entry {
            Ok(_) => {},
            Err(err) => {
                let path = err.path().unwrap_or_else(|| Path::new("")).display();
                if let Some(inner) = err.io_error() {
                    match inner.kind() {
                        io::ErrorKind::PermissionDenied => {
                            eprintln!("Permission denied for: {path}")
                        },
                        _ => return Err(err.into_io_error().unwrap())
                    }
                }
            },
        };
        count += 1;
    }

    Ok(count - 1)
}
