# NaAnSentinels roadmap

**Languages:** English · [Français](../fr/feuille-de-route.md) · [Español](../es/hoja-de-ruta.md)

## Done

- Static analysis engine: PE structure, sections, imported functions, entropy, SHA-256, x86/x64 disassembly.
- Explained verdict, no destructive action.
- Offline anti-phishing (pasted email or `.eml` file).
- Editable rules and config, reloadable from within the application.
- Animated simple view and expert view with diagrams.
- Optional, anonymous presence signal; optional account.
- Server: accounts, roles (user, analyst, admin; in French `utilisateur`, `analyste`, `admin`), dashboard,
  security log.
- Signing of official releases (Ed25519).

## Next

### Anti-phishing connected to the mailbox

- Optional IMAP connection, app password kept outside the repository and never displayed.
- Reading of the authentication headers: SPF, DKIM, DMARC.
- Detection of lookalike domains.

### Deeper reverse engineering

- Call tracing from the entry point, and call tree.
- Detection of known packers, delayed imports, encrypted strings.
- YARA rules in addition to the in-house format.
- Verification of the Authenticode signature of analyzed files.
- Lookup in a reputation database by hash, without sending the file.

### Monitoring

- Watching a folder (Downloads): analysis of each new program before it is opened.
- Reversible local quarantine, always on the user's decision.
- Analysis of a whole folder with a summary.

### Server

- Persistent rate limits and support for a trusted proxy.
- Two-factor authentication, password reset.
- History of connected devices over time.

## Out of scope (by choice)

- No kernel driver, no system-level interception of execution, no administrator action.
  NaAnSentinels remains a user-space tool that **analyzes and explains**.
- It does not replace the system's antivirus: it complements it.
