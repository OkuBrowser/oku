# Security Policy

1. [Reporting security problems to Oku](#reporting)
2. [Security Point of Contact](#contact)
3. [Incident Response Process](#process)
4. [Vulnerability Management Plans](#vulnerability-management)

<a name="reporting"></a>
## Reporting security problems to Oku

**DO NOT CREATE AN ISSUE** to report a security problem. Instead, please
send an email to limesayahi@gmail.com

<a name="contact"></a>
## Security Point of Contact

The security point of contact is Oku's maintainer, Emil Sayahi. Emil responds to security
incident reports as fast as possible, within one business day at the latest.

<a name="process"></a>
## Incident Response Process

In case an incident is discovered or reported, I will follow the following
process to contain, respond and remediate:

### 1. Containment

The first step is to find out the root cause, nature and scope of the incident.

- Is still ongoing? If yes, first priority is to stop it.
- Is the incident outside of my influence? If yes, first priority is to contain it.
- Find out knows about the incident and who is affected.
- Find out what data was potentially exposed.

### 2. Response

After the initial assessment and containment to my best abilities, I will
document all actions taken in a response plan.

I will create a comment in [the official "Updates" issue](https://github.com/MadeByEmil/oku/issues/3) to inform users about
the incident and what I actions I took to contain it.

### 3. Remediation

Once the incident is confirmed to be resolved, I will summarize the lessons
learned from the incident and create a list of actions I will take to prevent
it from happening again.

<a name="vulnerability-management"></a>
## Vulnerability Management Plans

### Keep dependencies up to date

A large chunk of the code being run on your machine when you start Oku is not Oku itself, 
but, rather, the many dependencies it relies on. Even if Oku itself is secure, one of its dependencies may
have security vulnerabilities; if a dependency has a vulnerability, it will likely be patched, and it is important
that we incorporate those patches into Oku.

### Critical Updates And Security Notices

We learn about critical software updates and security threats from these sources

1. GitHub Security Alerts (alerted through [GitHub Dependabot](https://docs.github.com/en/free-pro-team@latest/github/managing-security-vulnerabilities/about-github-dependabot-security-updates))
  - [GitHub Advisory Database](https://github.com/advisories)
2. [WhiteSource Bolt](https://www.whitesourcesoftware.com/free-developer-tools/bolt)
  - [WhiteSource Vulnerability Database](https://www.whitesourcesoftware.com/vulnerability-database/)
3. [ShiftLeft Scan](https://www.shiftleft.io/scan/) (codebase scanning)
4. [RustSec Advisory Database](https://rustsec.org/) (monitoring for vulnerable dependencies using [`cargo-audit`](https://github.com/RustSec/cargo-audit))