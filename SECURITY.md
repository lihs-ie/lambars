# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of lambars seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. Send an email to the maintainers with:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes (optional)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt within 48 hours
- **Initial Assessment**: We will provide an initial assessment within 7 days
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days

### Disclosure Policy

- We follow responsible disclosure practices
- We will coordinate with you on the disclosure timeline
- We will credit you for the discovery (unless you prefer anonymity)

## Security Measures

### Code Safety

- All `unsafe` code is forbidden (`#![forbid(unsafe_code)]`)
- Dependencies are regularly updated via Dependabot
- CI runs security-focused lints

### Best Practices

When using lambars:

- Keep your dependencies up to date
- Use the latest stable version when possible
- Review the CHANGELOG for security-related updates

## Scope

This security policy applies to:

- The lambars library (`lambars` crate)
- The lambars-derive macro crate (`lambars-derive` crate)

Third-party dependencies are outside the scope of this policy, but we will work with upstream maintainers if a dependency vulnerability affects lambars.
