# Works After Reinstall

> Your Windows setup, rebuilt after every reinstall.

Because reinstalling Windows is easy. Recreating your environment isn't.

**Works After Reinstall** is a Windows machine provisioning tool that lets you define your preferred applications, Windows settings, environment variables, developer configuration, and files — then automatically rebuilds that setup after a fresh Windows installation.

```text
Fresh Windows
     ↓
Works After Reinstall
     ↓
┌─────────────────────────┐
│ Applications            │
│ Windows Settings        │
│ Environment Variables   │
│ Developer Tools         │
│ Configuration Files     │
└─────────────────────────┘
     ↓
Your machine, again.
```

## What it manages

* 📦 **Applications** — Install software through WinGet
* ⚙️ **Windows Settings** — Reapply your preferred Windows configuration
* 🌱 **Environment** — Restore environment variables and PATH entries
* 🛠️ **Developer Setup** — Configure tools and development environments
* 📁 **Files & Config** — Restore dotfiles and application configuration
* 🔄 **Reboots** — Resume setup when Windows requires a restart
* 📋 **Declarative Configuration** — Describe what your machine should look like instead of writing installation scripts

## Example

```yaml
apps:
  - Google.Chrome
  - Microsoft.VisualStudioCode
  - Git.Git
  - OpenJS.NodeJS
  - 7zip.7zip

windows:
  dark_mode: true
  show_file_extensions: true
  show_hidden_files: true
  developer_mode: true

environment:
  variables:
    EDITOR: code
    NODE_ENV: development

files:
  copy:
    - source: ".gitconfig"
      destination: "%USERPROFILE%\\.gitconfig"
```

Then:

```powershell
works-after-reinstall setup
```

And the boring part takes care of itself.

## Status

🚧 **Early development**

The goal is simple:

**Reinstall Windows. Run one command. Get your machine back.**

## License

MIT