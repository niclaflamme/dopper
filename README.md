# Dopper

> **Zero-footprint environment variable manager.**

Dopper is a lightweight, secure CLI tool designed to manage project secrets and environment variables without polluting your shell profile or risking accidental commits of `.env` files. It uses a secure, encrypted local SQLite database to store secrets and injects them into your application processes only when needed.

It was designed with macOS in mind, but it might just work cross platform. Who knows?

## Features

- **Encrypted Storage:** Your secrets are encrypted at rest using AES-256 (via SQLCipher) with keys managed securely by your system's keychain.
- **Project-Based Isolation:** Secrets are scoped to specific projects, keeping your configurations clean and organized.
- **Smart Directory Linking:** Link your project directories once. Dopper automatically detects which project you are in.
- **Multi-Environment Support:** Comes with `dev` and `prod` out of the box, with support for custom environments (e.g., `staging`, `ci`).
- **Zero-Footprint Injection:** Run commands with `dopper run` to inject secrets directly into the process environment. No persistent shell exports.

---

## Installation

### Option 1: Install via Cargo

If you have Rust installed, you can install Dopper directly from the source:

```bash
git clone https://github.com/niclaflamme/dopper.git
cd dopper
cargo install --path .
```

### Option 2: Build from Source using Make

You can also use the provided Makefile to compile and install to a specific directory.

```bash
# Compile and install (defaults to ~/bin)
make compile

# Or install to a custom directory
make compile INSTALL_DIR=/usr/local/bin
```

Ensure your installation directory is in your `PATH`.

---

## Getting Started

Follow this quick guide to get up and running.

### 1. Initialize

Initialize the secure local database (located at `~/.dopper/dopper.db`).

```bash
dopper init
```

### 2. Create a Project

Create a namespace for your application.

```bash
dopper project create my-awesome-app
```

### 3. Link Directory

Tell Dopper that the current directory belongs to this project.

```bash
cd /path/to/my-awesome-app
dopper link my-awesome-app
```

### 4. Manage Secrets

Set secrets for your `dev` environment (default).

```bash
dopper secrets set DATABASE_URL "postgres://localhost:5432/mydb"
dopper secrets set API_KEY "super_secret_key"
```

### 5. Run Your App

Execute your application command. Dopper will inject the configured secrets into the process environment.

```bash
dopper run -- npm start
# or
dopper run -- cargo run
```

---

## Usage Reference

### Projects

Manage your project namespaces.

- **List Projects:** `dopper project list`
- **Create Project:** `dopper project create <name>`
- **Delete Project:** `dopper project delete <name>`

### Environments

Every project starts with `dev` and `prod`. You can add more as needed.

- **List Environments:** `dopper env list`
- **Create Environment:** `dopper env create <slug>` (e.g., `staging`, `ci`)
- **Delete Environment:** `dopper env delete <slug>` _(Note: `dev` and `prod` cannot be deleted)_
- **Switch Active Env:** `dopper env use <slug>` (Changes the default env for the current directory)

### Secrets

Manage key-value pairs for the active environment.

- **Set Secret (Standard):** `dopper secrets set <KEY> <VALUE>`
- **Set Secret (Secure):** `dopper secrets set <KEY>` (Prompts for value; keeps secrets out of shell history)
- **Set for Specific Env:** `dopper secrets set <KEY> [VALUE] --env prod`
- **List Secrets:** `dopper secrets list`
- **Unset Secret:** `dopper secrets unset <KEY>`

### Core Commands

- **Link Directory:** `dopper link <project_name>`
- **Run Command:** `dopper run -- <command>`
- **Print Secrets (.env format):** `dopper print [--env <slug>]` _(Prompts for confirmation; use `--yes` to skip)_
- **Copy Secrets to Clipboard:** `dopper clip [--env <slug>]` _(No confirmation)_
- **Lock/Unlock DB:** `dopper lock` / `dopper unlock` _(Manually encrypt/decrypt the database file)_

---

## Security Architecture

Dopper takes security seriously. Unlike `.env` files which are plain text, Dopper stores all data in a local SQLite database that is encrypted using **SQLCipher**.

- **Encryption:** The database is encrypted using 256-bit AES.
- **Key Management:** The encryption key is generated locally and stored securely in your operating system's native keychain (e.g., macOS Keychain, Linux Secret Service, Windows Credential Manager).
- **Zero-Trace:** Secrets are injected directly into the environment of the child process (`dopper run -- <cmd>`) and are never written to disk or exposed to other processes.

---

## Development

Dopper is written in Rust.

### Running Tests

The project includes a comprehensive integration test suite.

```bash
make test
```

### Database Location

All data is stored locally in `~/.dopper/dopper.db`. If this file is corrupted, Dopper's integrity check will detect it upon the next initialization attempt.

---

## License

This project is open source, but not open contribution. Feel free to fork and modify for personal use, but pull requests will not be accepted.
