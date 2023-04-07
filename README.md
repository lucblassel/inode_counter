# `inode_counter`

## goal
This program was made to be able to quickly tell which directories use up a lot of inodes on your system. 
This is particularly useful in environments where the number of available inodes is limited like on a shared HPC cluster. 

This program will output directories up to a certain depth with the number of inodes they contain. 

To speed up counting the directory traversal is parallelized. 

## example
Let us assume the following file structure:
```
root
├── .is_hidden
│   └── f4.txt
├── a
│   ├── .hidden_file
│   ├── b
│   │   └── d
│   └── e
│       ├── 1
│       └── f3.txt
├── f
│   └── g
├── f1.txt
└── f2.txt
```

Here are a few usage examples: 
```shell
# Count the number of visible inodes in `root`
$> ./inode_counter root
root 11

# Include inodes of hidden files and directories
$> ./inode_counter root --show-hidden
root 14

# Count number of inodes, display as tree with depth 2 and 
# show percentages of total inode count next to directory names
$> ./inode_counter root --show-hidden --depth 2 --show-percent
root 14 (100%)
├─ a 7 (50%)
│  ├─ e 3 (21%)
│  └─ b 2 (14%)
├─ f 2 (14%)
│  └─ g 1 (7%)
└─ .is_hidden 2 (14%)
```

## usage
```
Count inodes in a directory structure.

USAGE:
    inode_counter [FLAGS] [OPTIONS] <root>

FLAGS:
    -h, --help             Prints help information
    -i, --ignore-colors    Do print with colored output
    -s, --show-hidden      Count hidden files
    -p, --show-percent     Show percentage of total inode count for each directory
    -V, --version          Prints version information

OPTIONS:
    -d, --depth <depth>    Max depth to display counts per directory [default: 0]

ARGS:
    <root>    Root to count inodes from
```