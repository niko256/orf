#!/bin/bash

cd "$(dirname "$0")"

if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
    echo "Rust and Cargo are required but not installed. Please install them first."
    exit 1
fi

# install the project
install_project() {
    echo "Installing orf..."
    cargo install --path .
    if [ $? -eq 0 ]; then
        echo "orf installed successfully."
        add_to_path
    else
        echo "Failed to install orf."
        exit 1
    fi
}

# update the project
update_project() {
    echo "Updating orf..."
    cargo install --path . --force
    if [ $? -eq 0 ]; then
        echo "orf updated successfully."
        add_to_path
    else
        echo "Failed to update orf."
        exit 1
    fi
}

# add orf to the PATH
add_to_path() {
    echo "Adding orf to PATH..."
    
    local shell_rc
    
    if [ -n "$ZSH_VERSION" ]; then
        shell_rc=~/.zshrc
    elif [ -n "$BASH_VERSION" ]; then
        shell_rc=~/.bashrc
    else
        shell_rc=~/.profile
    fi

    if [[ ":PATH:" != *":HOME/.cargo/bin:"* ]]; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$shell_rc"
        echo "Please restart your shell or run 'source $shell_rc' to apply changes"
    else
        echo "orf is already in PATH."
    fi
}

echo "Welcome to the orf installer!"
echo "Please select an option:"
echo "1. Install orf"
echo "2. Update orf"
echo "3. Exit"

read -p "Enter your choice (1/2/3): " choice

case $choice in
    1)
        install_project
        ;;
    2)
        update_project
        ;;
    3)
        echo "Exiting..."
        exit 0
        ;;
    *)
        echo "Invalid choice. Exiting..."
        exit 1
        ;;
esac
