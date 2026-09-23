# Setup

## 1. Install Claude Code

Follow https://code.claude.com/docs/en/overview. Verify:

```bash
claude --version
```

Job Hunter finds `claude` on your PATH and in the usual install locations (`~/.local/bin`, `~/.claude/local`, npm global directories, nvm). You can also set an explicit path in **Settings → Claude & Chrome**.

## 2. Authenticate Claude Code

```bash
claude auth login      # or just run `claude` and follow the login prompt
claude auth status     # must report loggedIn: true
```

Chrome integration requires a direct Anthropic plan login (Pro, Max, Team or Enterprise). API-key or third-party-provider setups are not used by Job Hunter.

## 3. Install Google Chrome

Download from https://www.google.com/chrome/. Job Hunter detects the standard install locations on Windows, macOS and Linux, or you can set the path in Settings.

## 4. Install Claude in Chrome

Install the **Claude** extension from the Chrome Web Store (id `fcoeoabgfenejglbffodgkkbkcdhcgfn`, version 1.0.36 or newer), keep it enabled, and sign in to the extension. Keep Chrome running while the agent works; Claude opens its own tab group in your existing browser session, so you stay logged in to LinkedIn, Naukri, etc.

You can verify the integration outside Job Hunter:

```bash
claude --chrome
```

## 5. Configure MongoDB Atlas (optional)

See [mongodb.md](mongodb.md). Paste the `mongodb+srv://` connection string in **Settings → MongoDB Atlas** or in the setup wizard. It is stored in your OS credential manager.

## 6. Start Job Hunter

Development:

```bash
pnpm install
pnpm tauri dev
```

Installed build: run the installer produced by `pnpm tauri build` (see [development.md](development.md)).

## 7. First run

The wizard walks through:

1. Check Claude CLI
2. Check Claude authentication
3. Check Chrome + Claude in Chrome
4. Configure MongoDB Atlas (skippable)
5. Import master resume (PDF or DOCX). The original file is copied to `resumes/master/` and never modified; its text is extracted for Claude. **Fill profile with Claude** parses it into structured experience, projects, skills and education.
6. Configure candidate profile: name, email, target roles and skills are required.

## 8. Run the first job hunt

Press **Start Job Hunt** on the dashboard. Watch the Agent Activity panel. When the run reaches **Waiting for your approval**, open **Applications**.

## 9. Approve an application

Open an application, review the job, match analysis, resume preview (edit it if needed) and cover letter, then press **Approve & Apply**. Claude opens the posting in Chrome and fills the form. If it needs an answer you have not given it, the application switches to **Needs your input**; answer in the app and press **Continue application**.

## 10. Handle manual applications

If Claude hits a CAPTCHA, login wall, unsupported flow or any error, the application becomes **Manual action** with the reason. Press **Open Application** (or **Apply Manually**) to continue in Chrome with the generated documents, then **Mark as Applied**.
