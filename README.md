# orf

A lightweight version control system, inspired by Git. This educational project aims to provide insights into how version control systems work internally

## Features

### Repository Management
- `orf init` - Initialize a new repository
- `orf status` - Show working tree status

### Staging Area (Index) Operations
- `orf add <paths>` - Add files to the staging area
- `orf rm [--cached] [--force] <paths>` - Remove files from working tree and/or index
- `orf ls-files [--stage]` - Show information about files in the index
- `orf write-tree [--path]` - Create a tree object from the current index

### Object Management
- `orf hash-object <file>` - Compute object ID and optionally creates a blob
- `orf cat-file [-p] [-t] [-s] <object>` - Inspect repository objects
- `orf show <commit>` - Show detailed object information

### Commit History
- `orf commit -m <message> [--author]` - Record changes to the repository
- `orf log [--count]` - Show commit history
- `orf diff [from] [to]` - Show changes between commits

### Branching
- `orf branch [name] [--delete] [--list]` - List, create or delete branches
- `orf checkout <target> [--force]` - Switch branches or restore working tree files

### Configuration
- `orf config [--global] <command>` - Manage configuration settings
- `orf remote <command>` - Manage remote repositories

## Installation

### From Source

1. Clone the repository:
```bash
git clone https://github.com/Niko256/orf.git
cd orf
```

2. Run the installation script:
```bash
./install.sh
```
