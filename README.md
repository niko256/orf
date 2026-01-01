# Vox

A lightweight version control system, inspired by Git for the some part, built as an educational project to understand how version control systems work under the hood.

The project started after watching the Jon Gjengset's [video](https://youtu.be/u0VotuGzD_w?si=m0YOhTSavxuy6vIU), which sparked the curiosity to build a vcs from scratch. Let's see how far this goes!

## What it can do

### Basic Repository Operations

- `vox init` - Initialize a new repository
- `vox status` - Show working tree status

### Working with Files (Staging Area) / (Index)

- `vox add <paths>` - Add files to the staging area
- `vox rm [--cached] [--force] <paths>` - Remove files from working tree and/or index
- `vox ls-files [--stage]` - Show information about files in the index
- `vox write-tree [--path]` - Generate a tree object from staged files

### Object Management

- `vox hash-object <file>` - Generate an object ID and optionally creates a blob
- `vox cat-file [-p] [-t] [-s] <object>` - Inspect stored objects
- `vox show <commit>` - View commit details

### Commit History

- `vox commit -m <message> [--author]` - Record changes to the repository
- `vox log [--count]` - Show commit history
- `vox diff [from] [to]` - Show changes between commits

### Branching

- `vox branch [name] [--delete] [--list]` - List, create or delete branches
- `vox checkout <target> [--force]` - Switch branches or restore working tree files

### Configuration

- `vox config [--global] <command>` - Manage configuration settings
- `vox remote <command>` - Manage remote repositories
