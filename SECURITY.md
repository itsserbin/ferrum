# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| Latest  | :white_check_mark: |
| < Latest | :x:               |

Only the latest release receives security fixes.

## Reporting a Vulnerability

Please report security vulnerabilities through [GitHub Security Advisories](https://github.com/itsserbin/ferrum/security/advisories/new).

**Do not open a public issue for security vulnerabilities.**

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response timeline

- **Acknowledgment**: within 7 days
- **Initial assessment**: within 14 days
- **Fix release**: depends on severity, critical issues are prioritized

### What happens next

1. Your report is acknowledged within 7 days
2. We investigate and confirm the vulnerability
3. A fix is developed and tested
4. A new release is published with the fix
5. You are credited in the release notes (unless you prefer anonymity)

## Scope

The following are considered security vulnerabilities:

- Arbitrary code execution via terminal escape sequences
- Sandbox escape or privilege escalation
- Data exfiltration through malicious escape sequences
- Denial of service via crafted input that crashes the application
- Vulnerabilities in dependencies that affect Ferrum

The following are **not** in scope:

- Issues requiring physical access to the machine
- Social engineering attacks
- Vulnerabilities in the operating system or shell
