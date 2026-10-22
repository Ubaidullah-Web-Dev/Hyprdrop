# Hyprdrop — AUR Publishing & Automation Guide

This guide walks you through setting up and maintaining the official **AUR (Arch User Repository)** packages for **Hyprdrop**:
- `hyprdrop-bin`: Fast, pre-compiled binary release (recommended for most users).
- `hyprdrop-git`: Direct VCS release compiled from the latest `main` branch commit.

---

## 1. Prerequisites (One-time Setup)

### Step 1: Register on the AUR
If you don't already have an AUR account:
1. Visit [https://aur.archlinux.org/register](https://aur.archlinux.org/register).
2. Choose your username (e.g. `Ubaid` or your preferred handle) and email.

### Step 2: Generate an SSH Key Pair for AUR
On your terminal, generate a dedicated SSH key for AUR:
```bash
ssh-keygen -t ed25519 -C "aur-hyprdrop" -f ~/.ssh/aur_hyprdrop
```
*(Press Enter when prompted for passphrase, or enter a passphrase if using an agent).*

This creates:
- `~/.ssh/aur_hyprdrop` (Private Key — keep secret!)
- `~/.ssh/aur_hyprdrop.pub` (Public Key)

### Step 3: Add Public Key to Your AUR Account
1. Copy your public key:
   ```bash
   cat ~/.ssh/aur_hyprdrop.pub
   ```
2. Go to [https://aur.archlinux.org/account/](https://aur.archlinux.org/account/).
3. Paste the contents into the **SSH Public Key** field and click **Save**.

### Step 4: Configure GitHub Repository Secret
To enable GitHub Actions to push package updates to the AUR automatically:
1. Go to your GitHub repository: `https://github.com/Ubaidullah-Web-Dev/Hyprdrop`.
2. Navigate to **Settings** → **Secrets and variables** → **Actions**.
3. Click **New repository secret**.
4. Name: `AUR_SSH_PRIVATE_KEY`
5. Value: Copy and paste the entire private key:
   ```bash
   cat ~/.ssh/aur_hyprdrop
   ```
6. Click **Add secret**.

---

## 2. Initializing the Packages on AUR

AUR repositories are created automatically the first time you push to `ssh://aur@aur.archlinux.org/<pkgname>.git`.

You can initialize both repositories now using the included script:
```bash
# Ensure your SSH client knows to use your key for aur.archlinux.org:
cat << 'EOF' >> ~/.ssh/config

Host aur.archlinux.org
  User aur
  IdentityFile ~/.ssh/aur_hyprdrop
  StrictHostKeyChecking accept-new
EOF

# Sync both packages to the AUR
./packaging/scripts/sync-aur.sh all
```

---

## 3. How the Automated CI/CD Works

The GitHub Actions workflow (`.github/workflows/ci-and-aur.yml`) handles everything automatically:

### 1. On Every Push to `main` (`hyprdrop-git`)
- Runs test suites on Ubuntu.
- Spins up an Arch Linux container with `makepkg`.
- Clones `ssh://aur@aur.archlinux.org/hyprdrop-git.git`.
- Updates `PKGBUILD`, generates `.SRCINFO` with the latest commit rev-count.
- Commits and pushes directly to AUR.

### 2. On New Version Tags (`v*`, e.g. `git tag v0.1.1 && git push origin v0.1.1`)
- Compiles the optimized release binary with LTO.
- Creates `hyprdrop-v0.1.1-x86_64-unknown-linux-gnu.tar.gz` and computes SHA256.
- Publishes the release and assets to GitHub Releases.
- Clones `ssh://aur@aur.archlinux.org/hyprdrop-bin.git`.
- Automatically bumps `pkgver` and injects the SHA256 checksum into `PKGBUILD`.
- Generates `.SRCINFO` and pushes to AUR.

---

## 4. How Users Install Hyprdrop from AUR

Once pushed, any Arch Linux / Hyprland user can install Hyprdrop in seconds:

```bash
# Pre-compiled binary (Instant install, no compilation required):
yay -S hyprdrop-bin
# or
paru -S hyprdrop-bin

# Development VCS version (Compiles latest git commit from source):
yay -S hyprdrop-git
# or
paru -S hyprdrop-git
```
