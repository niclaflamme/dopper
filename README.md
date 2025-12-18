# 💧 Dopper

> **The zero-footprint environment variable manager.**

Dopper is a lightweight, Rust-based CLI tool designed to manage project secrets and environment variables without polluting your shell profile or risking accidental commits of `.env` files. It uses a secure local SQLite database to store secrets and injects them into your application processes on demand.

## ✨ Features

- **📂 Project-Based Isolation:** Secrets are scoped to specific projects, keeping your configurations clean and organized.
- **🔗 Smart Directory Linking:** Link your project directories once. Dopper automatically detects which project you are in.
- **🌍 Multi-Environment Support:** comes with `dev` and `prod` out of the box, with support for custom environments (e.g., `staging`, `yolo`).
- **🚀 Zero-Footprint Injection:** Run commands with `dopper run` to inject secrets directly into the process environment. No persistent shell exports.
- **🛡️ Integrity & Safety:** Built-in database integrity checks and safe-guards against deleting critical environments.

---

## 📦 Installation

### Prerequisites
- **Rust:** Ensure you have the latest stable Rust installed (`rustc` and `cargo`).

### Build from Source

You can compile and install Dopper directly from the source using the provided Makefile.

```bash
# Clone the repository
git clone https://github.com/yourusername/dopper.git
cd dopper

# Compile and install (defaults to ~/bin)
make compile

# Or install to a custom directory
make compile INSTALL_DIR=/usr/local/bin
```

Ensure the installation directory is in your `PATH`.

---

## 🚀 Getting Started

Follow this quick guide to get up and running with Dopper.

### 1. Initialize
Initialize the local Dopper database (located at `~/.dopper/dopper.db`).

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
Execute your application command. Dopper will inject the configured secrets into the process.

```bash
dopper run -- npm start
# or
dopper run -- cargo run
```

---

## 📖 Usage Reference

### 🏗️ Projects

Manage your project namespaces.

- **List Projects:** `dopper project list`
- **Create Project:** `dopper project create <name>`
- **Delete Project:** `dopper project delete <name>`

### 🌍 Environments

Every project starts with `dev` and `prod`. You can add more as needed.

- **List Environments:** `dopper env list`
- **Create Environment:** `dopper env create <slug>`
- **Delete Environment:** `dopper env delete <slug>` *(Note: `dev` and `prod` cannot be deleted)*
- **Switch Active Env:** `dopper env use <slug>` (Changes the default env for the current directory)

### 🔑 Secrets

Manage key-value pairs for the active environment.

- **Set Secret:** `dopper secrets set <KEY> <VALUE>`
- **Set for Specific Env:** `dopper secrets set <KEY> <VALUE> --env prod`
- **List Secrets:** `dopper secrets list`
- **Unset Secret:** `dopper secrets unset <KEY>`

### 🛠️ Core Commands

- **Link Directory:** `dopper link <project_name>`
- **Run Command:** `dopper run -- <command>`
- **Print Secrets (.env format):** `dopper print [--env <slug>]` *(Prompts for confirmation; use `--yes` to skip)*
- **Copy Secrets to Clipboard:** `dopper clip [--env <slug>]` *(No confirmation)*
- **Destroy Database:** `dopper destroy` *(Warning: Irrevocable! Wipes all data)*

---

## 🧠 Development

Dopper is written in Rust and uses `rusqlite` for database management.

### Running Tests
The project includes a comprehensive integration test suite.

```bash
make test
```

### Database Location
All data is stored locally in:
`~/.dopper/dopper.db`

If this file is corrupted, Dopper's integrity check will detect it upon the next initialization attempt and may require a hard reset (using `dopper destroy` or manual deletion) to recover, protecting you from inconsistent states.

---

## 📄 License

This project is open source. Feel free to contribute!
